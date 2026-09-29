use super::fixture::Upstream;
use core::time::Duration;
use rmcp::{
    serde_json::{Value, json},
    service::serve_directly,
};
use tokio::io::{AsyncBufReadExt as _, AsyncWriteExt as _, BufReader, duplex};
use web_rs::{
    cli::WorkMode,
    mcp::tools::{ToolCredentials, ToolService},
    search::SearchCredentials,
};
#[tokio::test]
async fn filesystem_search_delivers_cached_pages_and_all_warnings() {
    let upstream = Upstream :: json (& json ! ({ "code" : 0_i64 , "msg" : "success" , "data" : { "results" : [{ "url" : "https://example.org/page" }] } , "meta" : { "warning" : "Partial coverage" } })) . await ;
    let output = tempfile::tempdir().unwrap();
    let cached = output.path().join("cached");
    tokio::fs::create_dir(&cached).await.unwrap();
    tokio::fs::write(cached.join(".url.txt"), "https://example.org/page")
        .await
        .unwrap();
    tokio::fs::write(cached.join("content.txt"), "Full cached page")
        .await
        .unwrap();
    let service = ToolService::new_with_credentials(
        upstream.config(),
        ToolCredentials {
            search: Some(SearchCredentials::Octen("octen-key".to_owned())),
            reader: None,
        },
        WorkMode::Filesystem,
    )
    .unwrap();
    let (client_io, server_io) = duplex(0x0001_0000);
    let running = serve_directly(service, server_io, None);
    let mut client = BufReader::new(client_io);
    let request = json ! ({ "jsonrpc" : "2.0" , "id" : 1_u64 , "method" : "tools/call" , "params" : { "name" : "search_query" , "_meta" : { "io.modelcontextprotocol/protocolVersion" : "2026-07-28" } , "arguments" : { "output_path" : output . path () . to_str () . unwrap () , "requests" : [{ "q" : "test" , "category" : "news" }] } } });
    let mut encoded = sonic_rs::to_vec(&request).unwrap();
    encoded.push(b'\n');
    client.write_all(&encoded).await.unwrap();
    let mut line = String::new();
    tokio::time::timeout(Duration::from_secs(10), client.read_line(&mut line))
        .await
        .unwrap()
        .unwrap();
    let response: Value = sonic_rs::from_str(&line).unwrap();
    let result = response.pointer("/result/structuredContent").unwrap();
    assert_eq!(result.get("error"), Some(&json!("")), "{response}");
    let warning = result.get("warning").unwrap().as_str().unwrap();
    assert!(warning.contains("category"));
    assert!(warning.contains("Partial coverage"));
    let destination = std::path::Path::new(result.get("output_path").unwrap().as_str().unwrap());
    assert_eq!(
        tokio::fs::read_to_string(destination.join("0/0/content.txt"))
            .await
            .unwrap(),
        "Full cached page"
    );
    assert_eq!(
        tokio::fs::read_to_string(destination.join("0/0/.url.txt"))
            .await
            .unwrap(),
        "https://example.org/page"
    );
    let captured = upstream.requests.lock().await.first().unwrap().clone();
    assert_eq!(
        captured.body.pointer("/highlight/enable"),
        Some(&json!(false))
    );
    assert_eq!(
        captured.body.pointer("/full_content/enable"),
        Some(&json!(false))
    );
    running.cancel().await.unwrap();
}
