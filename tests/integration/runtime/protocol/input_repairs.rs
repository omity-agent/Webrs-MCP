use crate::assertions::JsonProbe as _;
use crate::{
    rpc::{Endpoint, response},
    upstream::Upstream,
};
use reqwest::StatusCode;
use rmcp::serde_json::json;
#[tokio::test]
async fn normalized_requests_reach_the_provider_with_visible_repairs_and_raw_output() {
    let upstream = Upstream :: json (& json ! ({ "results" : [{ "url" : "https://example.org" , "title" : "A&B <title>" , "highlights" : ["raw <match> & text"] }] })) . await ;
    let endpoint = Endpoint::configured(upstream.server.settings()).await;
    let reply = response(
        endpoint
            .tool(
                "search_query",
                &json!({"Request": {
                    "Query": "one", "Domain": "Example.org", "Category": "Research Papers"
                }}),
            )
            .header("x-exa-api-key", "key"),
        StatusCode::OK,
    )
    .await;
    assert_eq!(reply.at("/result/isError"), &json!(false), "{reply}");
    assert_eq!(
        reply.at("/result/structuredContent/results/0/title"),
        &json!("A&B <title>")
    );
    assert_eq!(
        reply.at("/result/structuredContent/results/0/highlight"),
        &json!("raw <match> & text")
    );
    let text = reply.text("/result/content/0/text");
    assert!(text.contains("<TITLE>\nA&B <title>\n</TITLE>"));
    assert!(text.contains("<HIGHLIGHT>\nraw <match> & text\n</HIGHLIGHT>"));
    let warnings = reply.list("/result/structuredContent/warning");
    for fragment in ["requests", "domains", "research paper"] {
        assert!(
            warnings
                .iter()
                .any(|warning| warning.as_str().unwrap().contains(fragment)),
            "{warnings:?}"
        );
        assert!(text.contains(fragment));
    }
    let calls = upstream.requests.lock().await.clone();
    assert_eq!(calls.len(), 1);
    let captured = calls.first().unwrap();
    assert_eq!(captured.body.at("/query"), &json!("one"));
    assert_eq!(captured.body.at("/includeDomains"), &json!(["example.org"]));
    assert_eq!(captured.body.at("/category"), &json!("research paper"));
    for (arguments, expected) in [
        (
            json ! ({ "outer" : [{ "inner" : { "term" : "inferred query" } }] }),
            "inferred query",
        ),
        (
            json ! ({ "requests" : [{ "q" : { "payload" : [{ "text" : "nested query" }] } }] }),
            "nested query",
        ),
    ] {
        upstream.requests.lock().await.clear();
        let normalized = response(
            endpoint
                .tool("search_query", &arguments)
                .header("x-exa-api-key", "key"),
            StatusCode::OK,
        )
        .await;
        assert_eq!(
            normalized.at("/result/isError"),
            &json!(false),
            "{normalized}"
        );
        let repaired_calls = upstream.requests.lock().await.clone();
        assert_eq!(repaired_calls.len(), 1);
        assert_eq!(
            repaired_calls.first().unwrap().body.at("/query"),
            &json!(expected)
        );
    }
}
