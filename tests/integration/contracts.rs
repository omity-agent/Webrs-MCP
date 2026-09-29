use super::fixture::{Upstream, query};
use axum::{body::Bytes, http::StatusCode};
use rmcp::serde_json::json;
use web_rs::{
    models::SearchCategory,
    search::{SearchClient, SearchCredentials},
};
#[tokio::test]
async fn octen_maps_filters_results_and_warnings() {
    let upstream = Upstream :: json (& json ! ({ "code" : 0_i64 , "msg" : "success" , "request_id" : "test" , "data" : { "results" : [{ "title" : "Page" , "url" : "https://example.org/" , "highlight" : "Useful text" , "time_published" : "2026-01-01T00:00:00Z" }] } , "meta" : { "warning" : "Partial coverage" } })) . await ;
    let client = SearchClient::new(&upstream.config()).unwrap();
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
    assert_eq!(result.highlight, "Useful text");
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
    for (path, value) in [
        ("/query", json!("你好 search")),
        ("/count", json!(10_u32)),
        ("/include_domains", json!(["example.org"])),
        ("/time_basis", json!("published")),
        ("/highlight/enable", json!(true)),
        ("/highlight/max_tokens", json!(512_u32)),
        ("/full_content/enable", json!(false)),
        ("/format", json!("text")),
        ("/safesearch", json!("strict")),
    ] {
        assert_eq!(captured.body.pointer(path), Some(&value));
    }
    assert!(captured.body.get("category").is_none());
    let since = chrono::DateTime::parse_from_rfc3339(
        captured.body.get("start_time").unwrap().as_str().unwrap(),
    )
    .unwrap();
    assert!((since.timestamp() - before.timestamp()).abs() <= 2);
}
#[tokio::test]
async fn exa_preserves_payload_and_result_contract() {
    let upstream = Upstream :: json (& json ! ({ "results" : [{ "title" : "Exa" , "url" : "https://example.org/" , "publishedDate" : "2026-01-01" , "highlights" : ["first" , "second"] }] })) . await ;
    let client = SearchClient::new(&upstream.config()).unwrap();
    let mut request = query("exa");
    request.category = Some(SearchCategory::News);
    request.recency = Some(1);
    request.domains = Some(vec!["EXAMPLE.org".to_owned()]);
    let key = SearchCredentials::Exa("exa-key".to_owned());
    let batch = client.search_many(&[request], &key, true).await.unwrap();
    assert_eq!(batch.warnings, Vec::<String>::new());
    assert_eq!(
        batch.groups.first().unwrap().first().unwrap().highlight,
        "first\nsecond"
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
        let client = SearchClient::new(&upstream.config()).unwrap();
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
#[tokio::test]
async fn octen_rejects_upstream_failures() {
    for (status, body, expected) in [
        (
            StatusCode::OK,
            r#"{"code":403,"msg":"Insufficient balance","request_id":"req-test"}"#,
            "Insufficient balance",
        ),
        (StatusCode::OK, r#"{"code":0,"msg":"ok"}"#, "missing data"),
        (
            StatusCode::OK,
            r#"{"code":0,"msg":"ok","data":{}}"#,
            "malformed JSON",
        ),
        (StatusCode::OK, "not JSON", "malformed JSON"),
        (StatusCode::UNAUTHORIZED, "{}", "API key"),
        (StatusCode::TOO_MANY_REQUESTS, "{}", "rate limit"),
    ] {
        let upstream = Upstream::start(
            status,
            "application/json",
            Bytes::from_static(body.as_bytes()),
        )
        .await;
        let client = SearchClient::new(&upstream.config()).unwrap();
        let error = client
            .search_many(
                &[query("test")],
                &SearchCredentials::Octen("key".to_owned()),
                true,
            )
            .await
            .err()
            .unwrap();
        assert!(error.client_message().contains(expected), "{error}");
    }
}
#[tokio::test]
async fn invalid_filters_are_not_silently_discarded() {
    let upstream = Upstream::json(&json!({})).await;
    let client = SearchClient::new(&upstream.config()).unwrap();
    let mut huge_recency = query("test");
    huge_recency.recency = Some(u64::MAX);
    let mut invalid_domain = query("test");
    invalid_domain.domains = Some(vec!["https://".to_owned()]);
    for request in [huge_recency, invalid_domain, query(&"界".repeat(501))] {
        assert!(
            client
                .search_many(
                    &[request],
                    &SearchCredentials::Octen("key".to_owned()),
                    true
                )
                .await
                .is_err()
        );
    }
    assert!(upstream.requests.lock().await.is_empty());
}
