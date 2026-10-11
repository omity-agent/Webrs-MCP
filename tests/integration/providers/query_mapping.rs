use crate::upstream::{Upstream, query};
use rmcp::serde_json::json;
use web_rs::{models::SearchCategory, search::SearchCredentials};
#[tokio::test]
async fn octen_maps_filters_results_and_warnings() {
    let upstream = Upstream :: json (& json ! ({ "code" : 0_i64 , "msg" : "success" , "request_id" : "test" , "data" : { "results" : [{ "title" : "Page" , "url" : "https://example.org/" , "highlight" : "Useful\n\n\ntext" , "time_published" : "2026-01-01T00:00:00Z" }] } , "meta" : { "warning" : "Partial coverage" } })) . await ;
    let client = upstream.search_client();
    let mut request = query("你好 search");
    request.recency = Some(7);
    request.domains = Some(vec!["https://Example.org/path".to_owned()]);
    request.category = Some(SearchCategory::News);
    let before = chrono::Utc::now() - chrono::Days::new(7);
    let batch = client
        .search_many(
            &[request],
            &SearchCredentials::Octen("octen-key".to_owned()),
            true,
        )
        .await
        .unwrap();
    let result = batch.groups.first().unwrap().first().unwrap();
    assert_eq!(result.title.as_deref(), Some("Page"));
    assert_eq!(result.date.as_deref(), Some("2026-01-01T00:00:00Z"));
    assert_eq!(result.highlight, "Useful\n\ntext");
    assert!(
        batch
            .warnings
            .iter()
            .any(|warning| warning.contains("category"))
    );
    assert!(
        batch
            .warnings
            .iter()
            .any(|warning| warning.contains("Partial coverage"))
    );
    let captured = upstream.requests.lock().await.first().unwrap().clone();
    assert_eq!(captured.headers.get("x-api-key").unwrap(), "octen-key");
    let octen = upstream.server.settings().search.octen;
    for (path, value) in [
        ("/query", json!("你好 search")),
        ("/count", json!(10_u32)),
        ("/include_domains", json!(["example.org"])),
        ("/time_basis", json!("published")),
        ("/highlight/enable", json!(true)),
        ("/highlight/max_tokens", json!(octen.highlights_max_tokens)),
        ("/full_content/enable", json!(false)),
        ("/format", json!(octen.format)),
        ("/safesearch", json!(octen.safesearch)),
    ] {
        assert_eq!(captured.body.pointer(path), Some(&value));
    }
    assert!(captured.body.get("category").is_none());
    let since = chrono::DateTime::parse_from_rfc3339(
        captured.body.get("start_time").unwrap().as_str().unwrap(),
    )
    .unwrap();
    let after = chrono::Utc::now() - chrono::Days::new(7);
    assert!((before.timestamp()..=after.timestamp()).contains(&since.timestamp()));
}
#[tokio::test]
async fn exa_preserves_payload_and_result_contract() {
    let upstream = Upstream :: json (& json ! ({ "results" : [{ "title" : "Exa" , "url" : "https://example.org/" , "publishedDate" : "2026-01-01" , "highlights" : ["first\n\n\n" , "\nsecond"] }] })) . await ;
    let client = upstream.search_client();
    let mut request = query("exa");
    request.category = Some(SearchCategory::News);
    request.recency = Some(1);
    request.domains = Some(vec!["EXAMPLE.org".to_owned()]);
    let key = SearchCredentials::Exa("exa-key".to_owned());
    let batch = client.search_many(&[request], &key, true).await.unwrap();
    assert_eq!(batch.warnings, Vec::<String>::new());
    assert_eq!(
        batch.groups.first().unwrap().first().unwrap().highlight,
        "first\n\nsecond"
    );
    let captured = upstream.requests.lock().await.first().unwrap().clone();
    assert_eq!(captured.headers.get("x-api-key").unwrap(), "exa-key");
    for (path, value) in [
        ("/type", json!("deep")),
        ("/numResults", json!(10_u32)),
        ("/category", json!("news")),
        ("/includeDomains", json!(["example.org"])),
        ("/contents/highlights/maxCharacters", json!(600_u32)),
    ] {
        assert_eq!(captured.body.pointer(path), Some(&value));
    }
    assert_eq!(
        captured
            .body
            .get("startPublishedDate")
            .unwrap()
            .as_str()
            .unwrap()
            .len(),
        10
    );
}
#[tokio::test]
async fn url_only_search_disables_content_for_both_backends() {
    for (key, reply, path, expected) in [
        (
            SearchCredentials::Exa("exa".to_owned()),
            json ! ({ "results" : [] }),
            "/contents",
            None,
        ),
        (
            SearchCredentials::Octen("octen".to_owned()),
            json ! ({ "code" : 0_i64 , "msg" : "ok" , "data" : { "results" : [] } }),
            "/highlight/enable",
            Some(json!(false)),
        ),
    ] {
        let upstream = Upstream::json(&reply).await;
        let client = upstream.search_client();
        let batch = client
            .search_many(&[query("first"), query("second")], &key, false)
            .await
            .unwrap();
        assert_eq!(batch.groups.len(), 2);
        let requests = upstream.requests.lock().await;
        assert_eq!(requests.len(), 2);
        for captured in requests.iter() {
            assert_eq!(captured.body.pointer(path), expected.as_ref());
        }
    }
}
