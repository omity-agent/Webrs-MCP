use crate::assertions::JsonProbe as _;
use crate::{channel, listener::LocalServer};
use axum::{Json, Router, routing::post};
use rmcp::serde_json::{Value, json};
use web_rs::{
    cli::WorkMode,
    mcp::tools::{ToolCredentials, ToolService},
    search::SearchCredentials,
};
async fn search_service() -> (LocalServer, ToolService) {
    let server = LocalServer::start(Router::new().route("/api", post(search_response))).await;
    let service = ToolService::new_with_credentials(
        server.settings(),
        ToolCredentials {
            search: Some(SearchCredentials::Exa("key".to_owned())),
            reader: None,
        },
        WorkMode::Response,
    )
    .unwrap();
    (server, service)
}
async fn search_response(Json(body): Json<Value>) -> Json<Value> {
    let query = body.text("/query");
    let count: usize = match query {
        "alpha" => 12,
        "beta" => 2,
        "empty" => 0,
        unexpected => panic!("unexpected search query: {unexpected}"),
    };
    let results = (0..count)
        .map(|index| {
            json!({
                "title": format!("{query} {index}"),
                "publishedDate": "2025-10-01",
                "url": format!("https://example.org/{query}/{index}"),
                "highlights": [format!("{query} & <{index}>")],
            })
        })
        .collect::<Vec<_>>();
    Json(json!({ "results": results }))
}
#[tokio::test]
async fn search_restarts_numbering_per_query_and_keeps_structured_results_flat() {
    let (server, service) = search_service().await;
    let reply = channel::call(
        service,
        "tools/call",
        json!({
            "name": "search_query",
            "arguments": {
                "requests": [{"q": "alpha"}, {"q": "empty"}, {"q": "beta"}],
            },
        }),
    )
    .await;
    assert_eq!(reply.at("/result/isError"), &json!(false));
    let text = reply.text("/result/content/0/text");
    assert_eq!(text.matches("<0>\n<TITLE>").count(), 2);
    assert!(text.contains("</11>\n<0>\n<TITLE>\nbeta 0"));
    let results = reply.list("/result/structuredContent/results");
    assert_eq!(results.len(), 14);
    assert_eq!(results.first().unwrap().text("/title"), "alpha 0");
    assert_eq!(results.last().unwrap().text("/title"), "beta 1");
    assert_eq!(server.uris().await.len(), 3);
}
