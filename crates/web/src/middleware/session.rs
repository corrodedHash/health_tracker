use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use axum::body::Body;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::{Request, StatusCode};
use cookie::Key;
use tower::{Layer, Service};
use uuid::Uuid;

use health_auth::session::parse_session_cookie;

#[derive(Debug, Clone)]
pub struct UserId(pub Uuid);

impl<S> FromRequestParts<S> for UserId
where
    S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<Self>()
            .cloned()
            .ok_or(StatusCode::UNAUTHORIZED)
    }
}

#[derive(Clone)]
pub struct SessionAuthLayer {
    cookie_key: Key,
}

impl SessionAuthLayer {
    pub const fn new(cookie_key: Key) -> Self {
        Self { cookie_key }
    }
}

impl<S> Layer<S> for SessionAuthLayer {
    type Service = SessionAuthService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        SessionAuthService {
            inner,
            cookie_key: self.cookie_key.clone(),
        }
    }
}

#[derive(Clone)]
pub struct SessionAuthService<S> {
    inner: S,
    cookie_key: Key,
}

impl<S> Service<Request<Body>> for SessionAuthService<S>
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
        let key = self.cookie_key.clone();

        Box::pin(async move {
            let user_id = req
                .headers()
                .get("cookie")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| parse_session_cookie(s, &key).ok())
                .and_then(|d| d.user_id.parse::<Uuid>().ok())
                .map(UserId);

            let (mut parts, body) = req.into_parts();
            if let Some(uid) = user_id {
                parts.extensions.insert(uid);
            }
            let req = Request::from_parts(parts, body);

            inner.call(req).await
        })
    }
}
