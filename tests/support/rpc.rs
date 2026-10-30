use crate::{
    envelope::{STATELESS, request},
    listener::LocalServer,
};
use core::time::Duration;
use reqwest::{Client, RequestBuilder, StatusCode};
use rmcp::serde_json::{Value, json};
use web_rs::{app, config::AppConfig};
pub(crate) struct Endpoint {
    pub server: LocalServer,
    pub client: Client,
}
impl Endpoint {
    pub fn tool(&self, name: &str, arguments: &Value) -> RequestBuilder {
        self.post(
            &request(
                STATELESS,
                "tools/call",
                json!({
                "name": name, "arguments": arguments
                    }),
            ),
            STATELESS,
        )
    }
    pub async fn configured(settings: AppConfig) -> Self {
        Self::listen(app::router(settings).unwrap()).await
    }
    pub async fn listen(router: axum::Router) -> Self {
        Self {
            server: LocalServer::start(router).await,
            client: Client::builder()
                .no_proxy()
                .timeout(Duration::from_secs(10))
                .build()
                .unwrap(),
        }
    }
    pub fn post(&self, body: &Value, version: &str) -> RequestBuilder {
        let mut builder = self
            .client
            .post(format!("{}/mcp", self.server.address))
            .header("accept", "application/json, text/event-stream")
            .header("mcp-protocol-version", version)
            .json(body);
        if version == STATELESS {
            builder = builder.header("mcp-method", body.get("method").unwrap().as_str().unwrap());
            if let Some(name) = body.pointer("/params/name").and_then(Value::as_str) {
                builder = builder.header("mcp-name", name);
            }
        }
        builder
    }
}
pub(crate) async fn response(builder: RequestBuilder, expected: StatusCode) -> Value {
    let reply = builder.send().await.unwrap();
    decode(reply, expected, None).await
}
pub(crate) async fn response_as(
    builder: RequestBuilder,
    expected: StatusCode,
    json_response: bool,
) -> Value {
    let reply = builder.send().await.unwrap();
    decode(reply, expected, Some(json_response)).await
}
async fn decode(
    reply: reqwest::Response,
    expected: StatusCode,
    json_response: Option<bool>,
) -> Value {
    let status = reply.status();
    assert!(!reply.headers().contains_key("mcp-session-id"));
    let is_sse = reply
        .headers()
        .get("content-type")
        .unwrap()
        .to_str()
        .unwrap()
        .starts_with("text/event-stream");
    if let Some(wants_json) = json_response {
        assert_eq!(!is_sse, wants_json, "response ignored server.json_response");
    }
    let body = reply.text().await.unwrap();
    assert_eq!(status, expected, "{body}");
    let encoded = if is_sse {
        body.lines()
            .find_map(|line| line.strip_prefix("data:"))
            .unwrap()
            .trim()
    } else {
        &body
    };
    sonic_rs::from_str(encoded).unwrap_or_else(|error| panic!("{error}: {body}"))
}
