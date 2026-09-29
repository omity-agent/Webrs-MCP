use super::{
    pipe::Process,
    wire::{LEGACY, MODERN, initialize, request},
};
use rmcp::serde_json::json;
use std::path::Path;
#[tokio::test]
async fn filesystem_open_delivers_cached_content_through_both_stdio_versions() {
    let output = tempfile::tempdir().unwrap();
    let cache = output.path().join("cached");
    tokio::fs::create_dir(&cache).await.unwrap();
    tokio::fs::write(cache.join(".url.txt"), "https://example.org/cached")
        .await
        .unwrap();
    tokio::fs::write(cache.join("content.txt"), "Saved page")
        .await
        .unwrap();
    for version in [LEGACY, MODERN] {
        let mut process = Process::start("filesystem");
        if version == LEGACY {
            let reply = process.call(&initialize(version)).await;
            assert_eq!(
                reply.pointer("/result/protocolVersion"),
                Some(&json!(version))
            );
            process
                .send(&json ! ({ "jsonrpc" : "2.0" , "method" : "notifications/initialized" }))
                .await;
        }
        let result = process . call (& request (version , "tools/call" , json ! ({ "name" : "open" , "arguments" : { "output_path" : output . path () . to_str () . unwrap () , "urls" : ["https://example.org/cached"] } }))) . await ;
        assert_eq!(
            result.pointer("/result/structuredContent/error"),
            Some(&json!(""))
        );
        let path = Path::new(
            result
                .pointer("/result/structuredContent/output_path")
                .unwrap()
                .as_str()
                .unwrap(),
        );
        assert_eq!(
            tokio::fs::read_to_string(path.join("0/content.txt"))
                .await
                .unwrap(),
            "Saved page"
        );
        assert_eq!(
            tokio::fs::read_to_string(path.join("0/.url.txt"))
                .await
                .unwrap(),
            "https://example.org/cached"
        );
        assert_eq!(
            result.pointer("/result/resultType"),
            (version == MODERN).then_some(&json!("complete"))
        );
        process.close().await;
    }
}
