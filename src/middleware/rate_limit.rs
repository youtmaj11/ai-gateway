use axum::body::Body;
use axum::http::{Request, Response, StatusCode};
use redis::aio::ConnectionManager;
use redis::{AsyncCommands, Client, RedisError};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tower::{Layer, Service};

#[derive(Clone)]
pub struct RateLimitLayer {
    shared: Arc<RateLimitState>,
}

#[derive(Clone)]
pub struct RateLimitState {
    redis: Arc<ConnectionManager>,
    limit: u32,
    window_seconds: u64,
}

impl RateLimitLayer {
    pub async fn new(redis_url: &str, limit: u32, window_seconds: u64) -> Result<Self, RedisError> {
        let client = Client::open(redis_url)?;
        let manager = ConnectionManager::new(client).await?;
        Ok(Self {
            shared: Arc::new(RateLimitState {
                redis: Arc::new(manager),
                limit,
                window_seconds,
            }),
        })
    }
}

impl<S> Layer<S> for RateLimitLayer {
    type Service = RateLimitMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RateLimitMiddleware {
            inner,
            shared: self.shared.clone(),
        }
    }
}

#[derive(Clone)]
pub struct RateLimitMiddleware<S> {
    inner: S,
    shared: Arc<RateLimitState>,
}

impl<S> Service<Request<Body>> for RateLimitMiddleware<S>
where
    S: Service<Request<Body>, Response = Response<Body>> + Clone + Send + 'static,
    S::Future: Send + 'static,
    S::Error: Send + 'static,
{
    type Response = Response<Body>;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let mut inner = self.inner.clone();
        let shared = self.shared.clone();
        let key = Self::client_identifier(&req);

        Box::pin(async move {
            match Self::check_rate_limit(shared, &key).await {
                Ok(allowed) => {
                    if allowed {
                        inner.call(req).await
                    } else {
                        let response = Response::builder()
                            .status(StatusCode::TOO_MANY_REQUESTS)
                            .body(Body::empty())
                            .expect("response build failed");
                        Ok(response)
                    }
                }
                Err(_) => inner.call(req).await,
            }
        })
    }
}

impl<S> RateLimitMiddleware<S> {
    fn client_identifier(req: &Request<Body>) -> String {
        req.headers()
            .get("x-forwarded-for")
            .and_then(|value| value.to_str().ok())
            .map(|s| s.split(',').next().unwrap_or(s).trim().to_string())
            .unwrap_or_else(|| "unknown".to_string())
    }

    async fn check_rate_limit(shared: Arc<RateLimitState>, identifier: &str) -> Result<bool, RedisError> {
        let mut conn = (*shared.redis).clone();
        let key = format!("rate_limit:{}", identifier);
        let count: u32 = conn.incr(&key, 1u32).await?;
        if count == 1 {
            let _: () = conn.expire(&key, shared.window_seconds as i64).await?;
        }
        Ok(count <= shared.limit)
    }
}
