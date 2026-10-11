use crate::{channel, upstream::Upstream};
use rmcp::serde_json::json;
use web_rs::{
    cli::WorkMode,
    mcp::tools::{ToolCredentials, ToolService},
    search::SearchCredentials,
};
#[tokio::test]
async fn filesystem_search_delivers_cached_pages_and_retains_all_warnings_without_page_text() {
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
        upstream.server.settings(),
        ToolCredentials {
            search: Some(SearchCredentials::Octen("key".to_owned())),
            reader: None,
        },
        WorkMode::Filesystem,
    )
    .unwrap();
    let reply = channel :: call (service , "tools/call" , json ! ({ "name" : "search_query" , "arguments" : { "output_path" : output . path () . to_str () . unwrap () , "requests" : [{ "q" : "test" , "category" : "news" }] } })) . await ;
    let result = reply.pointer("/result/structuredContent").unwrap();
    assert_eq!(result.get("error"), Some(&json!("")));
    let warning = result.get("warning").unwrap().as_str().unwrap();
    assert!(
        warning.contains("category") && warning.contains("Partial coverage"),
        "{warning}"
    );
    let destination = std::path::Path::new(result.get("output_path").unwrap().as_str().unwrap());
    assert!(destination.is_absolute() && destination.starts_with(output.path()));
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
    assert!(
        !sonic_rs::to_string(&reply)
            .unwrap()
            .contains("Full cached page")
    );
    let requests = upstream.requests.lock().await.clone();
    assert_eq!(requests.len(), 1);
    let captured = requests.first().unwrap();
    assert_eq!(
        captured.body.pointer("/highlight/enable"),
        Some(&json!(false))
    );
    assert_eq!(
        captured.body.pointer("/full_content/enable"),
        Some(&json!(false))
    );
}
