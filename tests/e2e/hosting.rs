use crate::assertions::JsonProbe as _;
use crate::{
    envelope::{STATELESS, request},
    listener::LocalServer,
    process::{renamed_settings, settings_file, start_http},
};
use axum::{Json, Router, routing::post};
use core::time::Duration;
use reqwest::Client;
use rmcp::serde_json::{Value, json};
use tokio::{
    net::TcpListener,
    time::{sleep, timeout},
};
#[tokio::test]
async fn http_mcp_requires_request_headers_even_when_environment_keys_are_present() {
    let upstream = LocalServer::start(
        Router::new().route("/api", post(|| async { Json(json!({ "results": [] })) })),
    )
    .await;
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let mut settings = upstream.settings();
    settings.tools = renamed_settings().tools;
    settings.server.host = "127.0.0.1".to_owned();
    settings.server.port = listener.local_addr().unwrap().port();
    drop(listener);
    let address = format!("http://{}:{}", settings.server.host, settings.server.port);
    let file = settings_file(&settings);
    let mut child = start_http(
        &file,
        &["--no-chunking"],
        &[
            ("EXA_API_KEY", "env-exa"),
            ("OCTEN_API_KEY", "env-octen"),
            ("JINA_API_KEY", ""),
        ],
    );
    let client = Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap();
    timeout(Duration::from_secs(15), async {
        loop {
            assert!(
                child.try_wait().unwrap().is_none(),
                "HTTP MCP stopped during startup"
            );
            if let Ok(response) = client.get(format!("{address}/health")).send().await
                && response.status().is_success()
            {
                break;
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .unwrap();
    for (name, headers, failed) in [
        ("lookup", vec![], true),
        ("lookup", vec![("x-exa-api-key", "request-key")], false),
        ("lookup", vec![("x-exa-api-key", "")], true),
        (
            "lookup",
            vec![("x-exa-api-key", "one"), ("x-exa-api-key", "two")],
            true,
        ),
        (
            "lookup",
            vec![("x-exa-api-key", "exa"), ("x-octen-api-key", "octen")],
            true,
        ),
        ("browse", vec![("x-jina-api-key", "")], true),
        (
            "browse",
            vec![("x-tinyfish-api-key", "one"), ("x-tinyfish-api-key", "two")],
            true,
        ),
        (
            "browse",
            vec![
                ("x-jina-api-key", "jina"),
                ("x-tinyfish-api-key", "tinyfish"),
            ],
            true,
        ),
    ] {
        let arguments = if name == "lookup" {
            json!({"requests": [{"q": "test"}]})
        } else {
            json!({"requests": [{"url": "https://example.org/", "chunk": 0_u64}]})
        };
        let body = request(
            STATELESS,
            "tools/call",
            json!({"name": name, "arguments": arguments}),
        );
        let mut builder = client
            .post(format!("{address}/mcp"))
            .header("accept", "application/json, text/event-stream")
            .header("mcp-protocol-version", STATELESS)
            .header("mcp-method", "tools/call")
            .header("mcp-name", name)
            .json(&body);
        for (header, value) in headers {
            builder = builder.header(header, value);
        }
        let response = builder.send().await.unwrap();
        assert!(response.status().is_success(), "{response:?}");
        let text = response.text().await.unwrap();
        let reply: Value = sonic_rs::from_str(&text).unwrap();
        assert_eq!(reply.at("/result/isError"), &json!(failed), "{reply}");
    }
    assert_eq!(upstream.uris().await.len(), 1);
    let visits = upstream.visits.lock().await;
    assert_eq!(visits.len(), 1);
    assert_eq!(
        visits.first().unwrap().headers.get("x-api-key").unwrap(),
        "request-key"
    );
    drop(visits);
    child.kill().await.unwrap();
}
