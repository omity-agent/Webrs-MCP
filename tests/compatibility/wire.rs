use core::time::Duration;
use reqwest::{Client, RequestBuilder, StatusCode};
use rmcp::serde_json::{Value, json};
use tokio::{net::TcpListener, task::JoinHandle};
use web_rs::{app, config};
pub(super) const LEGACY: &str = "2025-06-18";
pub(super) const MODERN: &str = "2026-07-28";
pub(super) struct Endpoint {
    pub(super) url: String,
    pub(super) client: Client,
    task: JoinHandle<()>,
}
impl Endpoint {
    pub(super) async fn new(json_response: bool) -> Self {
        let mut settings = config::load_embedded().unwrap();
        settings.server.json_response = json_response;
        Self::configured(settings).await
    }
    pub(super) async fn configured(settings: config::AppConfig) -> Self {
        Self::listen(app::router(settings).unwrap()).await
    }
    pub(super) async fn listen(router: axum::Router) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/mcp", listener.local_addr().unwrap());
        let task = tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });
        Self {
            url,
            client: Client::builder()
                .no_proxy()
                .timeout(Duration::from_secs(10))
                .build()
                .unwrap(),
            task,
        }
    }
    pub(super) fn post(&self, body: &Value, version: Option<&str>) -> RequestBuilder {
        let mut builder = self
            .client
            .post(&self.url)
            .header("accept", "application/json, text/event-stream")
            .json(body);
        if let Some(selected) = version {
            builder = builder.header("mcp-protocol-version", selected);
        }
        if version == Some(MODERN) {
            builder = builder.header("mcp-method", body.get("method").unwrap().as_str().unwrap());
            if let Some(name) = body.pointer("/params/name").and_then(Value::as_str) {
                builder = builder.header("mcp-name", name);
            }
        }
        builder
    }
}
impl Drop for Endpoint {
    fn drop(&mut self) {
        self.task.abort();
    }
}
pub(super) fn request(version: &str, method: &str, mut params: Value) -> Value {
    if version == MODERN {
        params . as_object_mut () . unwrap () . insert ("_meta" . to_owned () , json ! ({ "io.modelcontextprotocol/protocolVersion" : version , "io.modelcontextprotocol/clientCapabilities" : { } }) ,) ;
    }
    json ! ({ "jsonrpc" : "2.0" , "id" : 1_u64 , "method" : method , "params" : params })
}
pub(super) fn initialize(version: &str) -> Value {
    json ! ({ "jsonrpc" : "2.0" , "id" : 0_u64 , "method" : "initialize" , "params" : { "protocolVersion" : version , "capabilities" : { } , "clientInfo" : { "name" : "wire-test" , "version" : "1" } } })
}
pub(super) async fn response(builder: RequestBuilder, status: StatusCode) -> Value {
    let reply = builder.send().await.unwrap();
    assert_eq!(reply.status(), status);
    assert!(!reply.headers().contains_key("mcp-session-id"));
    let is_sse = reply
        .headers()
        .get("content-type")
        .unwrap()
        .to_str()
        .unwrap()
        .starts_with("text/event-stream");
    let body = reply.text().await.unwrap();
    if is_sse {
        let data = body
            .lines()
            .find_map(|line| line.strip_prefix("data:"))
            .unwrap();
        sonic_rs::from_str(data.trim()).unwrap()
    } else {
        sonic_rs::from_str(&body).unwrap_or_else(|error| panic!("{error}: {body}"))
    }
}
pub(super) fn assert_tools(reply: &Value, version: &str, filesystem: bool) {
    let result = reply.get("result").unwrap_or_else(|| panic!("{reply}"));
    let tools = result.get("tools").unwrap().as_array().unwrap();
    let names = tools
        .iter()
        .map(|tool| tool.get("name").unwrap().as_str().unwrap())
        .collect::<Vec<_>>();
    if filesystem {
        assert_eq!(names, ["search_query", "open"]);
    } else {
        assert_eq!(names, ["search_query", "open", "find"]);
    }
    for tool in tools {
        assert!(tool.get("inputSchema").is_some());
        assert!(tool.get("outputSchema").is_some());
    }
    if version == MODERN {
        assert_eq!(result.get("resultType"), Some(&json!("complete")));
        assert_eq!(result.get("ttlMs"), Some(&json!(3_600_000_u64)));
        assert_eq!(result.get("cacheScope"), Some(&json!("public")));
    } else {
        for key in ["resultType", "ttlMs", "cacheScope"] {
            assert!(result.get(key).is_none(), "{reply}");
        }
    }
}
