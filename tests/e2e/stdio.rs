use crate::{
    envelope::{HANDSHAKE, STATELESS, initialize, request},
    listener::LocalServer,
    process::{PAGE, Process, renamed_settings, settings_file},
};
use axum::{
    Json, Router,
    routing::{get, post},
};
use rmcp::serde_json::{Value, json};
use std::path::{Path, PathBuf};
#[tokio::test]
async fn stdio_negotiation_and_stateless_metadata_keep_the_process_usable_until_eof() {
    let file = settings_file(&renamed_settings());
    let mut handshake = Process::start(&file, "response");
    let initialized = handshake.call(&initialize(HANDSHAKE)).await;
    assert_eq!(
        initialized.pointer("/result/protocolVersion"),
        Some(&json!(HANDSHAKE))
    );
    handshake
        .send(&json ! ({ "jsonrpc" : "2.0" , "method" : "notifications/initialized" }))
        .await;
    let listed = handshake
        .call(&request(HANDSHAKE, "tools/list", json!({})))
        .await;
    assert_eq!(tool_names(&listed), ["lookup", "browse", "scan"]);
    handshake.close().await;
    let mut stateless = Process::start(&file, "response");
    let discovered = stateless
        .call(&request(STATELESS, "server/discover", json!({})))
        .await;
    assert_eq!(
        discovered.pointer("/result/supportedVersions"),
        Some(&json!([HANDSHAKE, STATELESS]))
    );
    let missing = stateless
        .call(&request(HANDSHAKE, "tools/list", json!({})))
        .await;
    assert_eq!(missing.pointer("/error/code"), Some(&json!(-32602_i64)));
    let recovered = stateless
        .call(&request(STATELESS, "tools/list", json!({})))
        .await;
    assert_eq!(tool_names(&recovered), ["lookup", "browse", "scan"]);
    stateless.close().await;
}
#[tokio::test]
async fn filesystem_process_isolates_invocations_and_reuses_cached_pages_after_the_origin_stops() {
    let server = LocalServer::start(Router::new().route(
        "/page",
        get(|| async { ([("content-type", "text/markdown")], PAGE) }),
    ))
    .await;
    let settings = renamed_settings();
    assert!(settings.ssrf.block_private_networks && settings.ssrf.block_local_hostnames);
    let file = settings_file(&settings);
    let directory = tempfile::tempdir().unwrap();
    let url = format!("{}/page", server.address);
    let missing = format!("{}/missing", server.address);
    let mut process = Process::start(&file, "filesystem");
    let listed = process
        .call(&request(STATELESS, "tools/list", json!({})))
        .await;
    assert_eq!(tool_names(&listed), ["lookup", "browse"]);
    let first = process . call (& request (STATELESS , "tools/call" , json ! ({ "name" : "browse" , "arguments" : { "output_path" : directory . path () . to_str () . unwrap () , "urls" : [url , url , missing] } }))) . await ;
    let first_path = output_path(&first, directory.path());
    assert!(
        first
            .pointer("/result/structuredContent/error")
            .unwrap()
            .as_str()
            .unwrap()
            .starts_with("urls[2]: Missing required header:"),
        "{first}"
    );
    for index in ["0", "1"] {
        assert_saved(&first_path.join(index), &url).await;
    }
    assert!(!sonic_rs::to_string(&first).unwrap().contains(PAGE));
    drop(server);
    let second = process . call (& request (STATELESS , "tools/call" , json ! ({ "name" : "browse" , "arguments" : { "output_path" : directory . path () . to_str () . unwrap () , "urls" : [url] } }))) . await ;
    assert_eq!(
        second.pointer("/result/structuredContent/error"),
        Some(&json!(""))
    );
    let second_path = output_path(&second, directory.path());
    assert_ne!(first_path, second_path);
    assert_saved(&second_path.join("0"), &url).await;
    assert_saved(&first_path.join("0"), &url).await;
    let rejected = process . call (& request (STATELESS , "tools/call" , json ! ({ "name" : "browse" , "arguments" : { "output_path" : directory . path () . to_str () . unwrap () , "urls" : ["file:///secret"] } }))) . await ;
    assert!(
        rejected
            .pointer("/result/structuredContent/error")
            .unwrap()
            .as_str()
            .unwrap()
            .contains("HTTP or HTTPS")
    );
    process.close().await;
}
fn tool_names(reply: &Value) -> Vec<&str> {
    reply
        .pointer("/result/tools")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .map(|tool| tool.get("name").unwrap().as_str().unwrap())
        .collect()
}
#[tokio::test]
async fn stdio_mcp_search_uses_environment_credentials() {
    let server = LocalServer::start(
        Router::new().route("/api", post(|| async { Json(json!({ "results": [] })) })),
    )
    .await;
    let mut settings = server.settings();
    settings.tools = renamed_settings().tools;
    let file = settings_file(&settings);
    let mut process = Process::start_with_env(&file, "response", &[("EXA_API_KEY", "stdio-key")]);
    let reply = process
        .call(&request(
            STATELESS,
            "tools/call",
            json!({ "name": "lookup", "arguments": { "requests": [{ "q": "test" }] } }),
        ))
        .await;
    assert_eq!(
        reply.pointer("/result/isError"),
        Some(&json!(false)),
        "{reply}"
    );
    let visits = server.visits.lock().await;
    assert_eq!(visits.len(), 1);
    assert_eq!(
        visits.first().unwrap().headers.get("x-api-key").unwrap(),
        "stdio-key"
    );
    drop(visits);
    process.close().await;
}
fn output_path(reply: &Value, root: &Path) -> PathBuf {
    let result = reply.pointer("/result/structuredContent").unwrap();
    assert_eq!(result.as_object().unwrap().len(), 3);
    let path = result.get("output_path").unwrap().as_str().unwrap();
    assert!(!path.contains('\\'));
    let directory = PathBuf::from(path);
    assert!(directory.is_absolute() && directory.parent() == Some(root));
    let name = directory.file_name().unwrap().to_str().unwrap();
    assert_eq!(name.len(), 8);
    assert!(
        name.bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
    );
    directory
}
async fn assert_saved(directory: &Path, url: &str) {
    assert_eq!(
        tokio::fs::read_to_string(directory.join("content.txt"))
            .await
            .unwrap(),
        PAGE
    );
    assert_eq!(
        tokio::fs::read_to_string(directory.join(".url.txt"))
            .await
            .unwrap(),
        url
    );
}
