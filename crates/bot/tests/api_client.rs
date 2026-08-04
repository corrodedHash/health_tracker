//! Integration tests for `ApiClient::post_run_gpx` against a `wiremock`
//! mock server (item 5.29).

#![allow(clippy::unwrap_used, reason = "tests")]

use std::time::Duration;

use health_bot::api_client::{ApiClient, ApiConfig, ReqwestApiClient};
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn post_run_gpx_success() {
    let server = MockServer::start().await;
    let id = uuid::Uuid::new_v4();

    Mock::given(method("POST"))
        .and(path("/api/runs/gpx"))
        .and(header("authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": id.to_string(),
        })))
        .mount(&server)
        .await;

    let config = ApiConfig {
        base_url: server.uri(),
        token: "test-token".into(),
    };
    let client = ReqwestApiClient::new(config);

    let started_at = chrono::DateTime::parse_from_rfc3339("2026-07-16T08:00:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc);

    let result = client
        .post_run_gpx(
            "test-token",
            b"<gpx></gpx>",
            started_at,
            1000.0,
            Duration::from_mins(10),
        )
        .await
        .unwrap();

    assert_eq!(result, id);
}

#[tokio::test]
async fn post_run_gpx_uses_sender_token() {
    let server = MockServer::start().await;
    let id = uuid::Uuid::new_v4();

    Mock::given(method("POST"))
        .and(path("/api/runs/gpx"))
        .and(header("authorization", "Bearer alice-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": id.to_string(),
        })))
        .mount(&server)
        .await;

    let config = ApiConfig {
        base_url: server.uri(),
        token: "service-token".into(),
    };
    let client = ReqwestApiClient::new(config);

    let started_at = chrono::DateTime::parse_from_rfc3339("2026-07-16T08:00:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc);

    let result = client
        .post_run_gpx(
            "alice-token",
            b"<gpx></gpx>",
            started_at,
            1000.0,
            Duration::from_mins(10),
        )
        .await
        .unwrap();

    assert_eq!(result, id);
}

#[tokio::test]
async fn post_run_gpx_server_error_propagates() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/runs/gpx"))
        .respond_with(ResponseTemplate::new(500).set_body_string("internal error"))
        .mount(&server)
        .await;

    let config = ApiConfig {
        base_url: server.uri(),
        token: "test-token".into(),
    };
    let client = ReqwestApiClient::new(config);

    let started_at = chrono::DateTime::parse_from_rfc3339("2026-07-16T08:00:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc);

    let result = client
        .post_run_gpx(
            "test-token",
            b"<gpx></gpx>",
            started_at,
            1000.0,
            Duration::from_mins(10),
        )
        .await;

    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("500"),
        "error should mention status code: {err}"
    );
}

#[tokio::test]
async fn post_run_gpx_unauthorized_is_typed() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/runs/gpx"))
        .respond_with(ResponseTemplate::new(401).set_body_string("unauthorized"))
        .mount(&server)
        .await;

    let config = ApiConfig {
        base_url: server.uri(),
        token: "test-token".into(),
    };
    let client = ReqwestApiClient::new(config);

    let started_at = chrono::DateTime::parse_from_rfc3339("2026-07-16T08:00:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc);

    let err = client
        .post_run_gpx(
            "test-token",
            b"<gpx></gpx>",
            started_at,
            1000.0,
            Duration::from_mins(10),
        )
        .await
        .unwrap_err();

    let api_err = err.downcast_ref::<health_bot::api_client::ApiError>();
    assert!(
        api_err.is_some_and(health_bot::api_client::ApiError::is_unauthorized),
        "expected an unauthorized ApiError, got: {err:#}"
    );
}

#[tokio::test]
async fn create_link_returns_code_and_url() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/links"))
        .and(header("authorization", "Bearer service-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "code": "abc123",
            "url": "https://health.example.com/link?code=abc123",
        })))
        .mount(&server)
        .await;

    let config = ApiConfig {
        base_url: server.uri(),
        token: "service-token".into(),
    };
    let client = ReqwestApiClient::new(config);

    let link = client.create_link().await.unwrap();
    assert_eq!(link.code, "abc123");
    assert_eq!(link.url, "https://health.example.com/link?code=abc123");
}

#[tokio::test]
async fn poll_link_pending() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/links/abc123"))
        .and(header("authorization", "Bearer service-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": "pending",
        })))
        .mount(&server)
        .await;

    let config = ApiConfig {
        base_url: server.uri(),
        token: "service-token".into(),
    };
    let client = ReqwestApiClient::new(config);

    let poll = client.poll_link("abc123").await.unwrap();
    assert_eq!(poll.status, health_bot::api_client::LinkStatus::Pending);
    assert!(poll.token.is_none());
}

#[tokio::test]
async fn poll_link_accepted_with_token() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/links/abc123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": "accepted",
            "token": "fresh-user-token",
        })))
        .mount(&server)
        .await;

    let config = ApiConfig {
        base_url: server.uri(),
        token: "service-token".into(),
    };
    let client = ReqwestApiClient::new(config);

    let poll = client.poll_link("abc123").await.unwrap();
    assert_eq!(poll.status, health_bot::api_client::LinkStatus::Accepted);
    assert_eq!(poll.token.as_deref(), Some("fresh-user-token"));
}

#[tokio::test]
async fn poll_link_expired() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/links/abc123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": "expired",
        })))
        .mount(&server)
        .await;

    let config = ApiConfig {
        base_url: server.uri(),
        token: "service-token".into(),
    };
    let client = ReqwestApiClient::new(config);

    let poll = client.poll_link("abc123").await.unwrap();
    assert_eq!(poll.status, health_bot::api_client::LinkStatus::Expired);
    assert!(poll.token.is_none());
}
