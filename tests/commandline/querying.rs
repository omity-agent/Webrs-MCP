use super::harness::{Site, assert_success, invoke, renamed_config, settings_file, stdout};
use alloc::sync::Arc;
use axum::{Json, Router, extract::State, http::HeaderMap, routing::post};
use rmcp::serde_json::{Value, json};
use tokio::sync::Mutex;
type Captures = Arc<Mutex<Vec<(HeaderMap, Value)>>>;
async fn answer_query(
    State(captured): State<Captures>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Json<Value> {
    captured.lock().await.push((headers, body));
    Json(
        json ! ({ "results" : [{ "url" : "https://example.org/" , "title" : "A&B <title>" , "highlights" : ["Found & <match>"] }] }),
    )
}
#[tokio::test]
async fn search_runs_once_and_selects_xml_or_json_without_protocol_wrapping() {
    let captures = Captures::default();
    let site = Site::listen(
        Router::new()
            .route("/query", post(answer_query))
            .with_state(Arc::clone(&captures)),
    )
    .await;
    let mut settings = renamed_config();
    settings.search.exa.endpoint = format!("{}/query", site.address);
    let file = settings_file(&settings);
    for format in [None, Some("xml"), Some("json")] {
        let mut arguments = vec![
            "lookup",
            "--q",
            "one query",
            "--exa-api-key",
            "cli-key",
            "--domains",
            "example.org",
            "--category",
            "news",
        ];
        if let Some(value) = format {
            arguments.extend(["--format", value]);
        }
        let output = invoke(&file, &arguments).await;
        assert_success(&output);
        let text = stdout(&output);
        if format == Some("json") {
            let response: Value = sonic_rs::from_str(text).unwrap();
            assert_eq!(
                response.pointer("/results/0/title"),
                Some(&json!("A&B <title>"))
            );
            assert_eq!(
                response.pointer("/results/0/highlight"),
                Some(&json!("Found & <match>"))
            );
            assert!(response.get("jsonrpc").is_none());
            assert!(response.get("content").is_none());
        } else {
            assert!(text.starts_with("<RESULTS>\n<RESULT>\n"));
            assert!(text.contains("<TITLE>\nA&B <title>\n</TITLE>"));
            assert!(text.contains("<HIGHLIGHT>\nFound & <match>\n</HIGHLIGHT>"));
        }
    }
    let requests = captures.lock().await;
    assert_eq!(requests.len(), 3);
    for captured in &*requests {
        assert_eq!(captured.0.get("x-api-key").unwrap(), "cli-key");
        assert_eq!(captured.1.get("query"), Some(&json!("one query")));
        assert_eq!(
            captured.1.get("includeDomains"),
            Some(&json!(["example.org"]))
        );
        assert_eq!(captured.1.get("category"), Some(&json!("news")));
    }
}
#[tokio::test]
async fn octen_warnings_are_included_in_both_cli_formats() {
    let site = Site :: listen (Router :: new () . route ("/query" , post (| | async { Json (json ! ({ "code" : 0_i64 , "msg" : "ok" , "data" : { "results" : [] } , "meta" : { "warning" : "Provider notice" } })) }))) . await ;
    let mut settings = renamed_config();
    settings.search.octen.endpoint = format!("{}/query", site.address);
    let file = settings_file(&settings);
    for format in ["xml", "json"] {
        let output = invoke(
            &file,
            &[
                "lookup",
                "--q",
                "test",
                "--octen-api-key",
                "octen-key",
                "--category",
                "news",
                "--format",
                format,
            ],
        )
        .await;
        assert_success(&output);
        assert!(stdout(&output).contains("Provider notice"));
        if format == "json" {
            let response: Value = sonic_rs::from_str(stdout(&output)).unwrap();
            assert_eq!(
                response.get("warning").unwrap().as_array().unwrap().len(),
                2
            );
        } else {
            assert!(stdout(&output).contains("<WARNINGS>"));
        }
    }
}
