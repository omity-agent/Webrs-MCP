use crate::assertions::JsonProbe as _;
use crate::{
    listener::LocalServer,
    process::{
        PAGE, assert_success, invoke, invoke_with_env, renamed_settings, settings_file, stdout,
    },
};
use axum::{
    Json, Router,
    routing::{get, post},
};
use rmcp::serde_json::{Value, json};
#[tokio::test]
async fn configured_open_delivers_full_local_content_in_json_and_xml() {
    let server = LocalServer::start(Router::new().route(
        "/page",
        get(|| async { ([("content-type", "text/markdown")], PAGE) }),
    ))
    .await;
    let mut settings = renamed_settings();
    settings.chunking.chunk_tokens = 2;
    let file = settings_file(&settings);
    let url = format!("{}/page", server.address);
    for format in ["json", "agent"] {
        let opened = invoke(file.path(), &["browse", &url, "--format", format]).await;
        assert_success(&opened);
        if format == "json" {
            let response: Value = sonic_rs::from_str(stdout(&opened)).unwrap();
            assert_eq!(
                response,
                json!({ "results": [{ "url": url, "page": { "content": PAGE } }] })
            );
        } else {
            assert_eq!(
                stdout(&opened),
                format!(
                    "<0>\n<URL>\n{url}\n</URL>\n<PAGE>\n<CONTENT>\n{PAGE}\n</CONTENT>\n</PAGE>\n</0>\n"
                )
            );
        }
    }
    assert!(
        server
            .visits
            .lock()
            .await
            .iter()
            .all(|visit| !visit.headers.contains_key("range"))
    );
}
#[tokio::test]
async fn remote_readers_use_api_keys_from_environment_variables() {
    let server = LocalServer::start(
        Router::new()
            .route(
                "/jina",
                post(|| async { Json(json!({ "data": { "content": PAGE } })) }),
            )
            .route(
                "/tinyfish",
                post(|Json(body): Json<Value>| async move {
                    Json(json!({
                        "results": [{ "url": body.at("/urls/0"), "text": PAGE }],
                        "errors": []
                    }))
                }),
            ),
    )
    .await;
    let mut settings = server.settings();
    settings.tools = renamed_settings().tools;
    settings.jina.endpoint = format!("{}/jina", server.address);
    settings.tinyfish.endpoint = format!("{}/tinyfish", server.address);
    let file = settings_file(&settings);
    let url = format!("{}/missing", server.address);
    for (variable, route, header, expected) in [
        (
            "JINA_API_KEY",
            "/jina",
            "authorization",
            "Bearer reader-key",
        ),
        ("TINYFISH_API_KEY", "/tinyfish", "x-api-key", "reader-key"),
    ] {
        let output = invoke_with_env(
            file.path(),
            &["browse", &url, "--format", "json"],
            &[(variable, "reader-key")],
        )
        .await;
        assert_success(&output);
        let response: Value = sonic_rs::from_str(stdout(&output)).unwrap();
        assert_eq!(response.at("/results/0/page/content"), &json!(PAGE));
        let visits = server.visits.lock().await;
        let request = visits.iter().find(|visit| visit.uri == route).unwrap();
        assert_eq!(request.headers.get(header).unwrap(), expected);
        drop(visits);
    }
}
