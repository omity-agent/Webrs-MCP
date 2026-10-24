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
    let query = body.get("query").unwrap().as_str().unwrap();
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
    assert_eq!(reply.pointer("/result/isError"), Some(&json!(false)));
    let expected_text = [("alpha", 12_usize), ("beta", 2_usize)]
        .into_iter()
        .flat_map(|(query, count)| {
            (0..count).map(move |index| {
                format!(
                    "<{index}>\n<TITLE>\n{query} {index}\n</TITLE>\n\
                     <DATE>\n2025-10-01\n</DATE>\n\
                     <URL>\nhttps://example.org/{query}/{index}\n</URL>\n\
                     <HIGHLIGHT>\n{query} & <{index}>\n</HIGHLIGHT>\n</{index}>"
                )
            })
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert_eq!(
        reply.pointer("/result/content/0/text"),
        Some(&json!(expected_text))
    );
    let expected_results = [("alpha", 12_usize), ("beta", 2_usize)]
        .into_iter()
        .flat_map(|(query, count)| {
            (0..count).map(move |index| {
                json!({
                    "title": format!("{query} {index}"),
                    "date": "2025-10-01",
                    "url": format!("https://example.org/{query}/{index}"),
                    "highlight": format!("{query} & <{index}>"),
                })
            })
        })
        .collect::<Vec<_>>();
    assert_eq!(
        reply.pointer("/result/structuredContent"),
        Some(&json!({ "results": expected_results }))
    );
    assert_eq!(server.visits.lock().await.len(), 3);
}
#[tokio::test]
async fn empty_results_produce_empty_text_or_only_their_warnings() {
    let (_server, service) = search_service().await;
    let empty_results = json!({ "results": [] });
    let empty_pages = json!({ "pages": [] });
    for (name, arguments, expected) in [
        ("search_query", json!({ "requests": [] }), &empty_results),
        (
            "search_query",
            json!({ "requests": [{ "q": "empty" }] }),
            &empty_results,
        ),
        ("open", json!({ "requests": [] }), &empty_results),
        ("find", json!({ "requests": [] }), &empty_pages),
    ] {
        let reply = channel::call(
            service.clone(),
            "tools/call",
            json!({ "name": name, "arguments": arguments }),
        )
        .await;
        assert_eq!(reply.pointer("/result/isError"), Some(&json!(false)));
        assert_eq!(reply.pointer("/result/content/0/text"), Some(&json!("")));
        assert_eq!(reply.pointer("/result/structuredContent"), Some(expected));
    }
    let warned = channel::call(
        service,
        "tools/call",
        json!({
            "name": "search_query",
            "arguments": { "requests": [{ "query": "empty" }] },
        }),
    )
    .await;
    assert_eq!(warned.pointer("/result/isError"), Some(&json!(false)));
    assert_eq!(
        warned.pointer("/result/structuredContent/results"),
        Some(&json!([]))
    );
    let warning = warned
        .pointer("/result/structuredContent/warning/0")
        .unwrap()
        .as_str()
        .unwrap();
    assert_eq!(
        warned.pointer("/result/content/0/text"),
        Some(&json!(format!(
            "<WARNINGS>\n<WARNING>\n{warning}\n</WARNING>\n</WARNINGS>"
        )))
    );
}
