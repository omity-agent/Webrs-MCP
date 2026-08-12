use super::{
    TinyFishFetchClient,
    response::{extract_markdowns, service_error},
};
use crate::{Result, error::AppError, net::secure_client_from_config};
use alloc::sync::Arc;
use axum::{Json, Router, extract::State, routing::post};
use serde::Deserialize;
use tokio::sync::Mutex;
#[test]
#[expect(
    clippy::panic_in_result_fn,
    reason = "The test uses assertions while Result keeps setup failures readable."
)]
fn response_extracts_requested_result_text() -> Result<()> {
    let body = br##"{
        "results": [
            {
                "url": "https://example.com",
                "text": "# Example"
            }
        ],
        "errors": []
    }"##;
    let urls = vec!["https://example.com".to_owned()];
    let markdowns = extract_markdowns(&urls, body)?;
    let markdown = markdowns
        .first()
        .ok_or_else(|| AppError::internal("expected one TinyFish result"))?;
    assert_eq!(markdowns.len(), 1);
    assert_eq!(markdown.as_deref().map_err(Clone::clone)?, "# Example");
    Ok(())
}
#[test]
fn service_error_preserves_tinyfish_diagnostics() {
    let body = br#"{
        "error": {
            "code": "INVALID_INPUT",
            "message": "Too many URLs. Maximum is 10."
        },
        "request_id": "request-123"
    }"#;
    assert_eq!(
        service_error(400, body).client_message(),
        "TinyFish returned HTTP 400: INVALID_INPUT: Too many URLs. Maximum is 10. Request ID: request-123."
    );
}
#[derive(Deserialize)]
struct TestPayload {
    urls: Vec<String>,
}
async fn fetch_batch(
    State(batch_sizes): State<Arc<Mutex<Vec<usize>>>>,
    Json(payload): Json<TestPayload>,
) -> Json<rmcp::serde_json::Value> {
    batch_sizes.lock().await.push(payload.urls.len());
    let results = payload
        .urls
        .into_iter()
        .map(|url| rmcp :: serde_json :: json ! ({ "url" : url , "text" : "# Page" }))
        .collect::<Vec<_>>();
    Json(rmcp :: serde_json :: json ! ({ "results" : results , "errors" : [] }))
}
#[tokio::test]
#[expect(
    clippy::panic_in_result_fn,
    reason = "Assertions verify HTTP batch boundaries while Result keeps async setup failures readable."
)]
async fn client_splits_requests_at_tinyfish_limit() -> Result<()> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|error| AppError::internal(format!("test listener failed: {error}")))?;
    let address = listener
        .local_addr()
        .map_err(|error| AppError::internal(format!("test address failed: {error}")))?;
    let batch_sizes = Arc::new(Mutex::new(Vec::<usize>::new()));
    let router = Router::new()
        .route("/", post(fetch_batch))
        .with_state(Arc::clone(&batch_sizes));
    let server = tokio::spawn(async move {
        axum::serve(listener, router)
            .await
            .unwrap_or_else(|error| panic!("test server failed: {error}"));
    });
    let mut config = crate::config::load_embedded()?;
    config.tinyfish.endpoint = format!("http://{address}/");
    config.ssrf.block_private_networks = false;
    config.ssrf.block_local_hostnames = false;
    let client = TinyFishFetchClient::new(config.clone(), secure_client_from_config(&config)?);
    let urls = (0_usize..21_usize)
        .map(|index| format!("https://example.com/{index}"))
        .collect::<Vec<_>>();
    let results = client.read_markdown_many(&urls, "key").await?;
    server.abort();
    let mut observed_sizes: Vec<usize> = batch_sizes.lock().await.clone();
    observed_sizes.sort_unstable();
    assert_eq!(observed_sizes, [1, 10, 10]);
    assert_eq!(results.len(), urls.len());
    assert!(results.into_iter().all(|result| result.is_ok()));
    Ok(())
}
#[test]
#[expect(
    clippy::panic_in_result_fn,
    reason = "The test uses assertions while Result keeps setup failures readable."
)]
fn response_extracts_batch_results_in_request_order() -> Result<()> {
    let body = br##"{
        "results": [
            {
                "url": "https://example.com/b",
                "text": "# B"
            },
            {
                "url": "https://example.com/a",
                "text": "# A"
            }
        ],
        "errors": []
    }"##;
    let urls = vec![
        "https://example.com/a".to_owned(),
        "https://example.com/b".to_owned(),
    ];
    let markdowns = extract_markdowns(&urls, body)?;
    let first = markdowns
        .first()
        .ok_or_else(|| AppError::internal("first TinyFish result was missing"))?;
    let second = markdowns
        .get(1)
        .ok_or_else(|| AppError::internal("second TinyFish result was missing"))?;
    assert_eq!(markdowns.len(), 2);
    assert_eq!(first.as_deref().map_err(Clone::clone)?, "# A");
    assert_eq!(second.as_deref().map_err(Clone::clone)?, "# B");
    Ok(())
}
#[test]
#[expect(
    clippy::panic_in_result_fn,
    reason = "The test uses assertions while Result keeps setup failures readable."
)]
fn response_preserves_mixed_results_and_errors_in_request_order() -> Result<()> {
    let body = br##"{
        "results": [
            {
                "url": "https://example.com/b",
                "text": "# B"
            }
        ],
        "errors": [
            {
                "url": "https://example.com/a",
                "error": "bot_blocked",
                "status": null
            }
        ]
    }"##;
    let urls = vec![
        "https://example.com/a".to_owned(),
        "https://example.com/b".to_owned(),
        "https://example.com/missing".to_owned(),
    ];
    let markdowns = extract_markdowns(&urls, body)?;
    let first = markdowns
        .first()
        .ok_or_else(|| AppError::internal("first TinyFish result was missing"))?;
    let second = markdowns
        .get(1)
        .ok_or_else(|| AppError::internal("second TinyFish result was missing"))?;
    let third = markdowns
        .get(2)
        .ok_or_else(|| AppError::internal("third TinyFish result was missing"))?;
    assert_eq!(markdowns.len(), 3);
    let first_error = first.as_ref().unwrap_err().client_message();
    assert_eq!(
        first_error,
        "TinyFish could not fetch https://example.com/a: bot_blocked."
    );
    assert_eq!(second.as_deref().map_err(Clone::clone)?, "# B");
    let third_error = third.as_ref().unwrap_err().client_message();
    assert_eq!(
        third_error,
        "TinyFish returned no content for the requested URL: https://example.com/missing."
    );
    Ok(())
}
