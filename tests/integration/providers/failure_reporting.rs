use crate::upstream::{Upstream, query};
use axum::{body::Bytes, http::StatusCode};
use rmcp::serde_json::json;
use web_rs::{error::AppError, search::SearchCredentials};
#[tokio::test]
async fn provider_failures_remain_actionable_without_disclosing_response_bodies() {
    for (status, body, expected) in [
        (
            StatusCode::OK,
            r#"{"code":403,"msg":"Insufficient balance","request_id":"req-test"}"#,
            "Insufficient balance",
        ),
        (StatusCode::OK, r#"{"code":0,"msg":"ok"}"#, "missing data"),
        (
            StatusCode::OK,
            r#"{"code":0,"msg":"ok","data":{}}"#,
            "malformed JSON",
        ),
        (StatusCode::OK, "private-upstream-detail", "malformed JSON"),
        (
            StatusCode::UNAUTHORIZED,
            "private-upstream-detail",
            "API key",
        ),
        (
            StatusCode::TOO_MANY_REQUESTS,
            "private-upstream-detail",
            "rate limit",
        ),
    ] {
        let upstream = Upstream::start(
            status,
            "application/json",
            Bytes::from_static(body.as_bytes()),
        )
        .await;
        let error = upstream
            .search_client()
            .search_many(
                &[query("test")],
                &SearchCredentials::Octen("key".to_owned()),
                true,
            )
            .await
            .err()
            .unwrap();
        assert!(matches!(error, AppError::Client(_)), "{error}");
        let message = error.client_message();
        assert!(message.contains(expected), "{message}");
        assert!(!message.contains("private-upstream-detail"), "{message}");
        assert_eq!(upstream.requests.lock().await.len(), 1);
    }
}
#[tokio::test]
async fn invalid_filters_fail_before_network_io() {
    let upstream = Upstream::json(&json!({})).await;
    let client = upstream.search_client();
    let mut huge_recency = query("test");
    huge_recency.recency = Some(u64::MAX);
    let mut invalid_domain = query("test");
    invalid_domain.domains = Some(vec!["https://".to_owned()]);
    for (request, expected) in [
        (huge_recency, "recency"),
        (invalid_domain, "domain"),
        (query(&"界".repeat(501)), "500"),
    ] {
        let error = client
            .search_many(
                &[request],
                &SearchCredentials::Octen("key".to_owned()),
                true,
            )
            .await
            .err()
            .unwrap();
        assert!(matches!(error, AppError::Client(_)), "{error}");
        assert!(error.client_message().contains(expected), "{error}");
    }
    assert!(upstream.requests.lock().await.is_empty());
}
