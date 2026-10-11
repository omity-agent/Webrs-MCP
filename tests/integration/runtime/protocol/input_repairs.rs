use crate::{
    envelope::{STATELESS, request},
    rpc::{Endpoint, response},
    upstream::Upstream,
};
use reqwest::StatusCode;
use rmcp::serde_json::json;
#[tokio::test]
async fn normalized_requests_reach_the_provider_with_visible_repairs_and_raw_output() {
    let upstream = Upstream :: json (& json ! ({ "results" : [{ "url" : "https://example.org" , "title" : "A&B <title>" , "highlights" : ["raw <match> & text"] }] })) . await ;
    let endpoint = Endpoint::configured(upstream.server.settings()).await;
    let body = request(
        STATELESS,
        "tools/call",
        json ! ({ "name" : "search_query" , "arguments" : { "Request" : { "Query" : "one" , "Domain" : "Example.org" , "Category" : "Research Papers" } } }),
    );
    let reply = response(
        endpoint
            .post(&body, STATELESS)
            .header("x-exa-api-key", "key"),
        StatusCode::OK,
    )
    .await;
    assert_eq!(
        reply.pointer("/result/isError"),
        Some(&json!(false)),
        "{reply}"
    );
    assert_eq!(
        reply.pointer("/result/structuredContent/results/0/title"),
        Some(&json!("A&B <title>"))
    );
    assert_eq!(
        reply.pointer("/result/structuredContent/results/0/highlight"),
        Some(&json!("raw <match> & text"))
    );
    let text = reply
        .pointer("/result/content/0/text")
        .unwrap()
        .as_str()
        .unwrap();
    assert!(text.contains("<TITLE>\nA&B <title>\n</TITLE>"));
    assert!(text.contains("<HIGHLIGHT>\nraw <match> & text\n</HIGHLIGHT>"));
    let warnings = reply
        .pointer("/result/structuredContent/warning")
        .unwrap()
        .as_array()
        .unwrap();
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
    assert_eq!(captured.body.get("query"), Some(&json!("one")));
    assert_eq!(
        captured.body.get("includeDomains"),
        Some(&json!(["example.org"]))
    );
    assert_eq!(
        captured.body.get("category"),
        Some(&json!("research paper"))
    );
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
        let nested = request(
            STATELESS,
            "tools/call",
            json ! ({ "name" : "search_query" , "arguments" : arguments }),
        );
        let normalized = response(
            endpoint
                .post(&nested, STATELESS)
                .header("x-exa-api-key", "key"),
            StatusCode::OK,
        )
        .await;
        assert_eq!(
            normalized.pointer("/result/isError"),
            Some(&json!(false)),
            "{normalized}"
        );
        let repaired_calls = upstream.requests.lock().await.clone();
        assert_eq!(repaired_calls.len(), 1);
        assert_eq!(
            repaired_calls.first().unwrap().body.get("query"),
            Some(&json!(expected))
        );
    }
}
