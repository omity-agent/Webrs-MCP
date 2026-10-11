use crate::{
    envelope::{STATELESS, request},
    rpc::{Endpoint, response},
    upstream::Upstream,
};
use axum::Router;
use reqwest::StatusCode;
use rmcp::{
    serde_json::json,
    transport::{
        StreamableHttpServerConfig,
        streamable_http_server::{
            session::never::NeverSessionManager, tower::StreamableHttpService,
        },
    },
};
use web_rs::{
    cli::WorkMode,
    mcp::tools::{ToolCredentials, ToolService},
    page::reader::ReaderCredentials,
    search::SearchCredentials,
};
#[tokio::test]
async fn credentials_are_request_scoped_and_invalid_headers_never_use_defaults() {
    let upstream = Upstream::json(
        &json ! ({ "results" : [] , "code" : 0_i64 , "msg" : "ok" , "data" : { "results" : [] } }),
    )
    .await;
    let service = ToolService::new_with_credentials(
        upstream.server.settings(),
        ToolCredentials {
            search: Some(SearchCredentials::Exa("default-key".to_owned())),
            reader: Some(ReaderCredentials::Jina("default-reader".to_owned())),
        },
        WorkMode::Response,
    )
    .unwrap();
    let http = StreamableHttpService::new(
        move || Ok(service.clone()),
        NeverSessionManager::default().into(),
        StreamableHttpServerConfig::default()
            .with_json_response(true)
            .with_stateless_protocol_metadata_required(true),
    );
    let endpoint = Endpoint::listen(Router::new().nest_service("/mcp", http)).await;
    let body = request(
        STATELESS,
        "tools/call",
        json ! ({ "name" : "search_query" , "arguments" : { "requests" : [{ "q" : "test" }] } }),
    );
    let (exa, octen) = tokio::join!(
        response(
            endpoint
                .post(&body, STATELESS)
                .header("x-exa-api-key", "request-exa"),
            StatusCode::OK
        ),
        response(
            endpoint
                .post(&body, STATELESS)
                .header("x-octen-api-key", "request-octen"),
            StatusCode::OK
        ),
    );
    for reply in <[_; 2]>::from((exa, octen)) {
        assert_eq!(
            reply.pointer("/result/isError"),
            Some(&json!(false)),
            "{reply}"
        );
    }
    let fallback = response(endpoint.post(&body, STATELESS), StatusCode::OK).await;
    assert_eq!(fallback.pointer("/result/isError"), Some(&json!(false)));
    let mut keys = upstream
        .requests
        .lock()
        .await
        .iter()
        .map(|captured| {
            captured
                .headers
                .get("x-api-key")
                .unwrap()
                .to_str()
                .unwrap()
                .to_owned()
        })
        .collect::<Vec<_>>();
    keys.sort_unstable();
    assert_eq!(keys, ["default-key", "request-exa", "request-octen"]);
    for headers in [
        vec![("x-exa-api-key", "")],
        vec![("x-exa-api-key", "one"), ("x-exa-api-key", "two")],
        vec![("x-exa-api-key", "exa"), ("x-octen-api-key", "octen")],
    ] {
        let mut builder = endpoint.post(&body, STATELESS);
        for (name, value) in headers {
            builder = builder.header(name, value);
        }
        let reply = response(builder, StatusCode::OK).await;
        assert_eq!(
            reply.pointer("/result/isError"),
            Some(&json!(true)),
            "{reply}"
        );
        assert!(
            reply.pointer("/result/structuredContent").is_none(),
            "{reply}"
        );
    }
    assert_eq!(upstream.requests.lock().await.len(), 3);
    let open = request(
        STATELESS,
        "tools/call",
        json ! ({ "name" : "open" , "arguments" : { "requests" : [{ "url" : format ! ("{}/unvisited" , upstream . server . address) , "chunk" : 0_u64 }] } }),
    );
    for headers in [
        vec![("x-jina-api-key", "")],
        vec![("x-tinyfish-api-key", "one"), ("x-tinyfish-api-key", "two")],
        vec![
            ("x-jina-api-key", "jina"),
            ("x-tinyfish-api-key", "tinyfish"),
        ],
    ] {
        let mut builder = endpoint.post(&open, STATELESS);
        for (name, value) in headers {
            builder = builder.header(name, value);
        }
        let reply = response(builder, StatusCode::OK).await;
        assert_eq!(
            reply.pointer("/result/isError"),
            Some(&json!(true)),
            "{reply}"
        );
        assert!(reply.pointer("/result/structuredContent").is_none());
    }
    assert_eq!(upstream.server.visits.lock().await.len(), 3);
}
