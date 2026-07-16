#![allow(clippy::unwrap_used, clippy::expect_used)]

use axum::body::Body;
use axum::extract::FromRequestParts;
use axum::http::{Request, StatusCode, header};
use tower::{Layer, ServiceExt};

use crate::middleware::session::{SessionAuthLayer, UserId};

fn cookie_key() -> cookie::Key {
    cookie::Key::from(&[0u8; 64][..])
}

fn signed_cookie(key: &cookie::Key, user_id: &str) -> String {
    let data = health_auth::session::SessionData {
        user_id: user_id.to_owned(),
    };
    let cookie_str =
        health_auth::session::create_session_cookie(&data, key, time::Duration::hours(1), false)
            .expect("failed to create cookie");

    let parsed = cookie::Cookie::parse_encoded(cookie_str).expect("failed to parse cookie");
    format!("{}={}", parsed.name(), parsed.value())
}

fn test_state() -> crate::state::AppState {
    let pool = sqlx::PgPool::connect_lazy("postgresql://unused").unwrap();
    crate::state::AppState {
        pool,
        config: crate::config::Config {
            database_url: String::new(),
            cookie_key: String::new(),
            listen_addr: String::new(),
            static_dir: None,
            oidc: None,
            dev_auto_login: false,
            cookie_secure: false,
        },
        cookie_key: cookie_key(),
        oidc_bundle: None,
    }
}

#[tokio::test]
async fn session_auth_stamps_user_id() {
    let key = cookie_key();
    let cookie = signed_cookie(&key, "550e8400-e29b-41d4-a716-446655440000");

    let layer = SessionAuthLayer::new(key);
    let svc = layer.layer(axum::Router::new().route("/test", axum::routing::get(|| async {})));

    let req = Request::builder()
        .uri("/test")
        .header(header::COOKIE, &cookie)
        .body(Body::empty())
        .unwrap();

    let response = svc.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn session_auth_passes_without_cookie() {
    let key = cookie_key();
    let layer = SessionAuthLayer::new(key);
    let svc = layer.layer(axum::Router::new().route("/test", axum::routing::get(|| async {})));

    let req = Request::builder().uri("/test").body(Body::empty()).unwrap();

    let response = svc.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn session_auth_rejects_invalid_signature() {
    let key = cookie_key();
    let other_key = cookie::Key::from(&[1u8; 64][..]);
    let cookie = signed_cookie(&other_key, "550e8400-e29b-41d4-a716-446655440000");

    let layer = SessionAuthLayer::new(key);
    let svc = layer.layer(axum::Router::new().route("/test", axum::routing::get(|| async {})));

    let req = Request::builder()
        .uri("/test")
        .header(header::COOKIE, &cookie)
        .body(Body::empty())
        .unwrap();

    let response = svc.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn user_id_extractor_fails_without_extension() {
    let req = Request::builder().uri("/").body(Body::empty()).unwrap();

    let (mut parts, _body) = req.into_parts();
    let result = UserId::from_request_parts(&mut parts, &()).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn user_id_extractor_succeeds_with_extension() {
    let req = Request::builder().uri("/").body(Body::empty()).unwrap();

    let (mut parts, _body) = req.into_parts();
    let uid = UserId("550e8400-e29b-41d4-a716-446655440000".parse().unwrap());
    parts.extensions.insert(uid.clone());

    let result = UserId::from_request_parts(&mut parts, &()).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().0, uid.0);
}

#[tokio::test]
async fn session_cookie_round_trip_via_middleware() {
    use crate::middleware::session::UserId;

    let key = cookie_key();
    let user_id = "550e8400-e29b-41d4-a716-446655440000";
    let cookie = signed_cookie(&key, user_id);

    let layer = SessionAuthLayer::new(key);
    let svc = layer.layer(axum::Router::new().route(
        "/test",
        axum::routing::get(|UserId(uid): UserId| async move { uid.to_string() }),
    ));

    let req = Request::builder()
        .uri("/test")
        .header(header::COOKIE, &cookie)
        .body(Body::empty())
        .unwrap();

    let response = svc.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    assert_eq!(body.as_ref(), user_id.as_bytes());
}

#[tokio::test]
async fn unauthenticated_api_returns_401() {
    let state = test_state();
    let app = crate::routes::build_router(state);

    let req = Request::builder()
        .uri("/api/exercise-sessions")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn auth_login_errors_without_oidc_config() {
    let state = test_state();
    let app = crate::routes::build_router(state);

    let req = Request::builder()
        .uri("/auth/login")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn auth_logout_returns_200() {
    let state = test_state();
    let app = crate::routes::build_router(state);

    let req = Request::builder()
        .uri("/auth/logout")
        .method("POST")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn delete_token_returns_401_without_session() {
    let state = test_state();
    let app = crate::routes::build_router(state);

    let req = Request::builder()
        .uri("/api/tokens/550e8400-e29b-41d4-a716-446655440000")
        .method("DELETE")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn unauthenticated_bearer_route_returns_401() {
    let state = test_state();
    let app = crate::routes::build_router(state);

    let req = Request::builder()
        .uri("/api/runs/gpx")
        .method("POST")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
