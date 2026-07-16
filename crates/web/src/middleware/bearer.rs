use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use axum::body::Body;
use axum::http::Request;
use sqlx::PgPool;
use tower::{Layer, Service};

use health_db::{ApiTokenRepository, SqlxRepository};

use super::session::UserId;

#[derive(Clone)]
pub struct BearerAuthLayer {
    pool: PgPool,
}

impl BearerAuthLayer {
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl<S> Layer<S> for BearerAuthLayer {
    type Service = BearerAuthService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        BearerAuthService {
            inner,
            pool: self.pool.clone(),
        }
    }
}

#[derive(Clone)]
pub struct BearerAuthService<S> {
    inner: S,
    pool: PgPool,
}

impl<S> Service<Request<Body>> for BearerAuthService<S>
where
    S: Service<Request<Body>, Response = axum::response::Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let mut inner = self.inner.clone();
        let pool = self.pool.clone();

        let token = req
            .headers()
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.strip_prefix("Bearer "))
            .map(str::to_owned);

        Box::pin(async move {
            let uid = match token {
                Some(t) => {
                    let repo = SqlxRepository::new(pool.clone());
                    repo.verify(&t).await.ok().flatten()
                }
                None => None,
            };

            let (mut parts, body) = req.into_parts();
            if let Some(uid) = uid {
                parts.extensions.insert(UserId(uid));
            }
            let req = Request::from_parts(parts, body);

            inner.call(req).await
        })
    }
}
