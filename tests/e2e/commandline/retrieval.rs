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
async fn configured_open_delivers_real_local_content_with_a_positional_url_and_chunk_parameter() {
    let server = LocalServer::start(Router::new().route(
        "/page",
        get(|| async { ([("content-type", "text/markdown")], PAGE) }),
    ))
    .await;
    let file = settings_file(&renamed_settings());
    let url = format!("{}/page", server.address);
    let opened = invoke(
        file.path(),
        &["browse", &url, "--chunk", "99", "--format", "json"],
    )
    .await;
    assert_success(&opened);
    let response: Value = sonic_rs::from_str(stdout(&opened)).unwrap();
    assert_eq!(
        response.pointer("/results/0/page/content"),
        Some(&json!(PAGE))
    );
    assert_eq!(
        response.pointer("/results/0/page/chunk"),
        Some(&json!(0_u64))
    );
    assert_eq!(
        response.pointer("/results/0/page/total_chunks"),
        Some(&json!(1_u64))
    );
    assert!(
        response
            .pointer("/warning/0")
            .unwrap()
            .as_str()
            .unwrap()
            .contains("using 0")
    );
    assert_eq!(
        server
            .visits
            .lock()
            .await
            .iter()
            .filter(|visit| visit.uri == "/page")
            .count(),
        2
    );
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
                        "results": [{ "url": body.pointer("/urls/0").unwrap(), "text": PAGE }],
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
        assert_eq!(
            response.pointer("/results/0/page/content"),
            Some(&json!(PAGE))
        );
        let visits = server.visits.lock().await;
        let request = visits.iter().find(|visit| visit.uri == route).unwrap();
        assert_eq!(request.headers.get(header).unwrap(), expected);
        drop(visits);
    }
}
