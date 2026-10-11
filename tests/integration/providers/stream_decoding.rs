use crate::upstream::Upstream;
use alloc::sync::Arc;
use axum::{body::Bytes, http::StatusCode};
use web_rs::{net::secure_client, page::jina::JinaReaderClient};
#[tokio::test]
async fn jina_http_contract_handles_json_and_complete_sse_frames() {
    for (content_type, body, expected) in [
        (
            "application/json",
            r#"{"content":"plain text"}"#,
            "plain text",
        ),
        (
            "application/json",
            r#"{"data":{"markdown":"nested text"}}"#,
            "nested text",
        ),
        (
            "Text/Event-Stream; charset=utf-8",
            concat!(
                ": keepalive\r\n",
                "event: result\r\n",
                "data: {\"data\":{\"markdown\":\"first\"}}\r\n\r\n",
                "data: plain\r\ndata: final\r\n\r\n",
                "data: [DONE]\r\n\r\n"
            ),
            "plain\nfinal",
        ),
    ] {
        let upstream = Upstream::start(
            StatusCode::OK,
            content_type,
            Bytes::from_static(body.as_bytes()),
        )
        .await;
        let settings = upstream.server.settings();
        let reader = JinaReaderClient::new(
            Arc::new(settings.clone()),
            secure_client(&settings.http, &settings.ssrf).unwrap(),
        );
        assert_eq!(
            reader
                .read_markdown("https://arxiv.org/pdf/2501.00001", "jina-key")
                .await
                .unwrap(),
            expected
        );
        let requests = upstream.requests.lock().await.clone();
        assert_eq!(requests.len(), 1);
        let captured = requests.first().unwrap();
        assert_eq!(
            captured.headers.get("authorization").unwrap(),
            "Bearer jina-key"
        );
        assert_eq!(
            captured.body.get("url").unwrap(),
            "https://arxiv.org/html/2501.00001"
        );
        assert_eq!(
            captured.body.get("viewport").unwrap(),
            &rmcp::serde_json::to_value(&settings.jina.viewport).unwrap()
        );
    }
}
#[tokio::test]
async fn jina_rejects_missing_content_and_parser_overflow_without_partial_success() {
    for (content_type, body, expected) in [
        ("application/json", "{}".to_owned(), "unsupported response"),
        (
            "text/event-stream",
            "data: [DONE]\n\n".to_owned(),
            "no page content",
        ),
        (
            "text/event-stream",
            format!("data: {}\n\n", "x".repeat(16 * 1024 * 1024 + 1)),
            "SSE parsing failed",
        ),
    ] {
        let upstream = Upstream::start(StatusCode::OK, content_type, Bytes::from(body)).await;
        let mut settings = upstream.server.settings();
        settings.http.max_response_bytes = 20 * 1024 * 1024;
        let reader = JinaReaderClient::new(
            Arc::new(settings.clone()),
            secure_client(&settings.http, &settings.ssrf).unwrap(),
        );
        let error = reader
            .read_markdown("https://example.org/", "key")
            .await
            .unwrap_err();
        assert!(error.client_message().contains(expected), "{error}");
    }
}
