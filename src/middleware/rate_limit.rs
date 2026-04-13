use axum::body::{Body, Bytes, HttpBody};
use axum::extract::connect_info::ConnectInfo;
use axum::http::{Request, Response, StatusCode};
use crate::auth::jwt::Claims;
use redis::aio::ConnectionManager;
use redis::{AsyncCommands, Client, RedisError};
use std::future::Future;
use std::net::SocketAddr;
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

impl<S, B> Service<Request<Body>> for RateLimitMiddleware<S>
where
    S: Service<Request<Body>, Response = Response<B>> + Clone + Send + 'static,
    B: HttpBody<Data = Bytes> + Send + 'static,
    B::Error: Into<axum::BoxError>,
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
                        let response = inner.call(req).await?;
                        Ok(response.map(Body::new))
                    } else {
                        let response = Response::builder()
                            .status(StatusCode::TOO_MANY_REQUESTS)
                            .body(Body::empty())
                            .expect("response build failed");
                        Ok(response)
                    }
                }
                Err(_) => {
                    let response = inner.call(req).await?;
                    Ok(response.map(Body::new))
                }
            }
        })
    }
}

impl<S> RateLimitMiddleware<S> {
    fn client_identifier(req: &Request<Body>) -> String {
        if let Some(claims) = req.extensions().get::<Claims>() {
            return format!("user:{}", claims.sub);
        }

        if let Some(header) = req.headers().get("x-forwarded-for") {
            if let Ok(value) = header.to_str() {
                let ip = value.split(',').next().unwrap_or(value).trim();
                if !ip.is_empty() {
                    return format!("ip:{}", ip);
                }
            }
        }

        if let Some(header) = req.headers().get("x-real-ip") {
            if let Ok(value) = header.to_str() {
                let ip = value.trim();
                if !ip.is_empty() {
                    return format!("ip:{}", ip);
                }
            }
        }

        if let Some(ConnectInfo(addr)) = req.extensions().get::<ConnectInfo<SocketAddr>>() {
            return format!("ip:{}", addr.ip());
        }

        "unknown".to_string()
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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{HeaderValue, Request};

    #[test]
    fn client_identifier_prefers_user_claims_over_ip() {
        let mut request = Request::builder()
            .uri("/")
            .body(Body::empty())
            .unwrap();

        request.extensions_mut().insert(Claims {
            sub: "user123".to_string(),
            exp: 0,
        });
        request.headers_mut().insert(
            "x-forwarded-for",
            HeaderValue::from_static("192.0.2.1"),
        );

        assert_eq!(RateLimitMiddleware::<()>::client_identifier(&request), "user:user123");
    }

    #[test]
    fn client_identifier_uses_x_forwarded_for_when_no_claims() {
        let mut request = Request::builder()
            .uri("/")
            .body(Body::empty())
            .unwrap();

        request.headers_mut().insert(
            "x-forwarded-for",
            HeaderValue::from_static("192.0.2.1, 198.51.100.1"),
        );

        assert_eq!(RateLimitMiddleware::<()>::client_identifier(&request), "ip:192.0.2.1");
    }
}
