use super::acceptance::receive;
use crate::listener::LocalServer;
use alloc::sync::Arc;
use axum::{Json, Router, http::StatusCode, routing::get};
use rmcp::serde_json::{Value, json};
use web_rs::{
    config,
    direct::{ResponseFormat, resolve_direct_fetch_target},
    net::secure_client,
    page::PageFetcher,
};
#[tokio::test]
async fn source_routing_preserves_raw_paths_and_registry_content_priorities() {
    let defaults = config::load_embedded().unwrap();
    let raw = resolve_direct_fetch_target(
        "https://github.com/org/project/blob/main/README.md",
        &defaults.direct_fetch,
    )
    .unwrap();
    assert_eq!(
        raw.request_url,
        "https://raw.githubusercontent.com/org/project/main/README.md"
    );
    assert_eq!(
        receive(
            raw.response_format,
            StatusCode::OK,
            Some("text/plain"),
            b"<html>raw source</html>"
        )
        .await
        .unwrap(),
        "<html>raw source</html>"
    );
    let server = LocalServer :: start (Router :: new () . route ("/package/@types/node" , get (| | async { ([("content-type" , "text/html")] , "<html>Package</html>") }) ,) . route ("/registry/{package}" , get (| | async { Json (json ! ({ "_id" : "demo" , "versions" : { "1" : { } } , "repository" : { "url" : "https://example.org/project" } , "readme" : "# Demo" , "description" : "Package" , "name" : "@types/node" })) }) ,) ,) . await ;
    let mut settings = server.settings();
    settings.direct_fetch.npm_hosts = vec!["127.0.0.1".to_owned()];
    settings.direct_fetch.npm_registry_url_prefix = format!("{}/registry/", server.address);
    let fetcher = PageFetcher::new(
        Arc::new(settings.clone()),
        secure_client(&settings.http, &settings.ssrf).unwrap(),
    );
    let page = fetcher
        .fetch(&format!("{}/package/@types/node", server.address), None)
        .await
        .unwrap();
    assert ! (page . markdown . starts_with ("{\n  \"name\": \"@types/node\",\n  \"description\": \"Package\",\n  \"readme\": \"# Demo\",\n  \"repository\"") , "{}" , page . markdown);
    let decoded: Value = sonic_rs::from_str(&page.markdown).unwrap();
    assert_eq!(decoded.get("_id"), Some(&json!("demo")));
    assert_eq!(decoded.get("versions"), Some(&json ! ({ "1" : { } })));
    assert!(page.markdown.find("\"_id\"").unwrap() < page.markdown.find("\"versions\"").unwrap());
    assert!(
        server
            .visits
            .lock()
            .await
            .iter()
            .any(|visit| visit.uri == "/registry/@types%2Fnode")
    );
}
#[tokio::test]
async fn structured_sources_extract_complete_content_and_surface_api_failures() {
    let question = receive (ResponseFormat :: StackOverflowQuestionJson , StatusCode :: OK , Some ("application/json") , br#"{"items":[{"title":"How?","body_markdown":"Question","comments":[{}],"answers":[{"body_markdown":"Answer","comments":[{}]}]}],"has_more":false}"# ,) . await . unwrap () ;
    let decoded: Value = sonic_rs::from_str(&question).unwrap();
    assert_eq!(
        decoded,
        json ! ({ "question" : { "title" : "How?" , "body" : "Question" } , "answers" : ["Answer"] })
    );
    let wiki = receive (ResponseFormat :: MediaWikiApi , StatusCode :: OK , Some ("application/json") , br#"{"query":{"pages":[{"revisions":[{"slots":{"main":{"content":"line1\r\nline2\r\n"}}}]}]}}"# ,) . await . unwrap () ;
    assert_eq!(wiki, "line1\nline2\n");
    for (format, body, expected) in [
        (
            ResponseFormat::StackOverflowQuestionJson,
            br#"{"items":[],"has_more":true}"#.as_slice(),
            "complete answer list",
        ),
        (
            ResponseFormat::MediaWikiApi,
            br#"{"error":{"code":"badtitle"}}"#.as_slice(),
            "badtitle",
        ),
        (
            ResponseFormat::MediaWikiApi,
            br#"{"query":{"pages":[{"missing":true}]}}"#.as_slice(),
            "not found",
        ),
    ] {
        let error = receive(format, StatusCode::OK, Some("application/json"), body)
            .await
            .unwrap_err();
        assert!(error.client_message().contains(expected), "{error}");
    }
}
