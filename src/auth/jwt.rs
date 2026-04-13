use axum::body::Body;
use axum::http::{header, Request, Response, StatusCode};
use futures_util::{future::BoxFuture, FutureExt};
use jsonwebtoken::{decode, errors::ErrorKind, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use std::task::{Context, Poll};
use tower::{Layer, Service};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
}

#[derive(Clone)]
pub struct JwtAuthLayer {
    secret: String,
}

impl JwtAuthLayer {
    pub fn new(secret: String) -> Self {
        Self { secret }
    }
}

impl<S> Layer<S> for JwtAuthLayer {
    type Service = JwtAuth<S>;

    fn layer(&self, inner: S) -> Self::Service {
        JwtAuth {
            inner,
            secret: self.secret.clone(),
        }
    }
}

#[derive(Clone)]
pub struct JwtAuth<S> {
    inner: S,
    secret: String,
}

impl<S> Service<Request<axum::body::Body>> for JwtAuth<S>
where
    S: Service<Request<axum::body::Body>, Response = Response<Body>> + Clone + Send + 'static,
    S::Future: Send + 'static,
    S::Error: Send + 'static,
{
    type Response = Response<Body>;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<axum::body::Body>) -> Self::Future {
        let auth_header = req
            .headers()
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix("Bearer "))
            .map(str::trim)
            .map(String::from);

        let secret = self.secret.clone();
        let mut inner = self.inner.clone();

        async move {
            let token = match auth_header {
                Some(token) if !token.is_empty() => token,
                _ => return Ok(unauthorized_response("Missing or invalid Authorization header")),
            };

            let validation = Validation::default();
            let key = DecodingKey::from_secret(secret.as_ref());
            let token_data = decode::<Claims>(&token, &key, &validation);

            match token_data {
                Ok(data) => {
                    req.extensions_mut().insert(data.claims);
                    inner.call(req).await
                }
                Err(err) => {
                    let message = match *err.kind() {
                        ErrorKind::ExpiredSignature => "Token expired",
                        _ => "Invalid token",
                    };
                    Ok(unauthorized_response(message))
                }
            }
        }
        .boxed()
    }
}

fn unauthorized_response(message: &str) -> Response<Body> {
    let body = serde_json::json!({"error": message}).to_string();
    Response::builder()
        .status(StatusCode::UNAUTHORIZED)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap()
}
