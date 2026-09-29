use super::fixture::Upstream;
use axum::{body::Bytes, http::StatusCode};
use web_rs::{net::secure_client, page::jina::JinaReaderClient};
#[tokio::test]
async fn oversized_reader_event_is_reported_as_an_error() {
    let body = format!("data: {}\n\n", "x".repeat(16 * 1024 * 1024 + 1));
    let upstream = Upstream::start(StatusCode::OK, "text/event-stream", Bytes::from(body)).await;
    let config = upstream.config();
    let reader = JinaReaderClient::new(
        config.clone(),
        secure_client(&config.http, &config.ssrf).unwrap(),
    );
    let result = reader
        .read_markdown("https://example.org/", "jina-key")
        .await;
    assert!(result.is_err(), "SSE parser overflow was silently accepted");
}
