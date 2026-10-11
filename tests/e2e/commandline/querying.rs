use crate::{
    listener::LocalServer,
    process::{assert_success, invoke, renamed_settings, settings_file, stdout},
};
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
async fn configured_search_runs_once_per_command_and_outputs_default_xml_or_standalone_json() {
    let captures = Captures::default();
    let server = LocalServer::start(
        Router::new()
            .route("/api", post(answer_query))
            .with_state(Arc::clone(&captures)),
    )
    .await;
    let mut settings = server.settings();
    settings.tools = renamed_settings().tools;
    let file = settings_file(&settings);
    for format in [None, Some("json")] {
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
        let output = invoke(file.path(), &arguments).await;
        assert_success(&output);
        let text = stdout(&output);
        if format.is_some() {
            let response: Value = sonic_rs::from_str(text).unwrap();
            assert_eq!(
                response.pointer("/results/0/title"),
                Some(&json!("A&B <title>"))
            );
            assert_eq!(
                response.pointer("/results/0/highlight"),
                Some(&json!("Found & <match>"))
            );
            assert_eq!(
                response.get("results").unwrap().as_array().unwrap().len(),
                1
            );
            assert!(response.get("jsonrpc").is_none() && response.get("content").is_none());
        } else {
            assert_eq!(
                text,
                concat!(
                    "<RESULTS>\n<RESULT>\n",
                    "<TITLE>\nA&B <title>\n</TITLE>\n",
                    "<URL>\nhttps://example.org/\n</URL>\n",
                    "<HIGHLIGHT>\nFound & <match>\n</HIGHLIGHT>\n",
                    "</RESULT>\n</RESULTS>\n"
                )
            );
        }
    }
    let requests = captures.lock().await;
    assert_eq!(requests.len(), 2);
    for capture in requests.iter() {
        let headers = &capture.0;
        let body = &capture.1;
        assert_eq!(headers.get("x-api-key").unwrap(), "cli-key");
        assert_eq!(body.get("query"), Some(&json!("one query")));
        assert_eq!(body.get("includeDomains"), Some(&json!(["example.org"])));
        assert_eq!(body.get("category"), Some(&json!("news")));
    }
}
#[tokio::test]
async fn provider_and_ignored_filter_warnings_survive_both_cli_output_formats() {
    let server = LocalServer :: start (Router :: new () . route ("/api" , post (| | async { Json (json ! ({ "code" : 0_i64 , "msg" : "ok" , "data" : { "results" : [] } , "meta" : { "warning" : "Provider notice" } })) }))) . await ;
    let mut settings = renamed_settings();
    settings.search.octen.endpoint = format!("{}/api", server.address);
    let file = settings_file(&settings);
    for format in ["xml", "json"] {
        let output = invoke(
            file.path(),
            &[
                "lookup",
                "--q",
                "test",
                "--octen-api-key",
                "key",
                "--category",
                "news",
                "--format",
                format,
            ],
        )
        .await;
        assert_success(&output);
        let text = stdout(&output);
        assert!(
            text.contains("Provider notice") && text.contains("category"),
            "{text}"
        );
        if format == "json" {
            let response: Value = sonic_rs::from_str(text).unwrap();
            assert_eq!(
                response.get("warning").unwrap().as_array().unwrap().len(),
                2
            );
            assert_eq!(response.get("results"), Some(&json!([])));
        } else {
            assert!(text.contains("<WARNINGS>\n<WARNING>"));
        }
    }
    assert_eq!(server.visits.lock().await.len(), 2);
}
