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
    assert!(captured.headers.get("exa-beta").is_none());
    let search = upstream.server.settings().search;
    let octen = &search.octen;
    for (path, value) in [
        ("/query", json!("你好 search")),
        ("/count", json!(search.num_results)),
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
async fn exa_requests_dynamic_highlights_and_preserves_result_contract() {
    let upstream = Upstream::json(&json!({
        "results": [
            {
                "title": "Exa",
                "url": "https://example.org/",
                "publishedDate": "2026-01-01",
                "highlights": ["first\n\n\n", "\nsecond"]
            },
            {
                "title": "Another result",
                "url": "https://example.org/another",
                "highlights": []
            }
        ]
    }))
    .await;
    let client = upstream.search_client();
    let mut request = query("exa");
    request.category = Some(SearchCategory::News);
    request.recency = Some(1);
    request.domains = Some(vec!["EXAMPLE.org".to_owned()]);
    let key = SearchCredentials::Exa("exa-key".to_owned());
    let batch = client.search_many(&[request], &key, true).await.unwrap();
    assert_eq!(batch.warnings, Vec::<String>::new());
    let mut results = batch.groups.first().unwrap().iter();
    assert_eq!(results.len(), 2);
    let first = results.next().unwrap();
    assert_eq!(first.title.as_deref(), Some("Exa"));
    assert_eq!(first.date.as_deref(), Some("2026-01-01"));
    assert_eq!(first.url, "https://example.org/");
    assert_eq!(first.highlight, "first\n\nsecond");
    let second = results.next().unwrap();
    assert_eq!(second.url, "https://example.org/another");
    assert_eq!(second.highlight, "");
    let captured = upstream.requests.lock().await.first().unwrap().clone();
    assert_eq!(captured.headers.get("x-api-key").unwrap(), "exa-key");
    assert_eq!(
        captured.headers.get("exa-beta").unwrap(),
        "dynamic-highlights-2026-08-28"
    );
    let search = upstream.server.settings().search;
    for (path, value) in [
        ("/query", json!("exa")),
        ("/type", json!("deep")),
        ("/numResults", json!(search.num_results)),
        ("/category", json!("news")),
        ("/includeDomains", json!(["example.org"])),
        (
            "/contents",
            json!({
                "highlights": {"dynamic": true},
                "maxAgeHours": search.exa.max_age_hours,
                "livecrawlTimeout": search.exa.livecrawl_timeout
            }),
        ),
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
async fn exa_sends_configured_beta_for_each_query() {
    let upstream = Upstream::json(&json!({"results": []})).await;
    let mut settings = upstream.server.settings();
    settings.search.exa.dynamic_highlights_beta = "configured-dynamic-highlights".to_owned();
    let client = web_rs::search::SearchClient::new(
        &settings,
        web_rs::net::secure_client(&settings.http, &settings.ssrf).unwrap(),
    );
    client
        .search_many(
            &[query("first"), query("second")],
            &SearchCredentials::Exa("exa".to_owned()),
            true,
        )
        .await
        .unwrap();
    let requests = upstream.requests.lock().await;
    assert_eq!(requests.len(), 2);
    for captured in requests.iter() {
        assert_eq!(
            captured.headers.get("exa-beta").unwrap(),
            settings.search.exa.dynamic_highlights_beta.as_str()
        );
        assert_eq!(
            captured.body.pointer("/contents/highlights"),
            Some(&json!({"dynamic": true}))
        );
    }
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
            assert!(captured.headers.get("exa-beta").is_none());
        }
    }
}
