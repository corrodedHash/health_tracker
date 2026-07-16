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
        .post_run_gpx(b"<gpx></gpx>", started_at, 1000.0, Duration::from_mins(10))
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
        .post_run_gpx(b"<gpx></gpx>", started_at, 1000.0, Duration::from_mins(10))
        .await;

    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("500"),
        "error should mention status code: {err}"
    );
}
