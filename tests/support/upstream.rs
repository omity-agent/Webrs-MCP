use crate::listener::LocalServer;
use alloc::sync::Arc;
use axum::{
    Router,
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode, header::CONTENT_TYPE},
    routing::post,
};
use rmcp::serde_json::Value;
use tokio::sync::Mutex;
use web_rs::{models::SearchQueryRequest, net::secure_client, search::SearchClient};
#[derive(Clone)]
pub(crate) struct Captured {
    pub headers: HeaderMap,
    pub body: Value,
}
#[derive(Clone)]
struct Reply {
    body: Bytes,
    status: StatusCode,
    content_type: &'static str,
    requests: Arc<Mutex<Vec<Captured>>>,
}
pub(crate) struct Upstream {
    pub server: LocalServer,
    pub requests: Arc<Mutex<Vec<Captured>>>,
}
impl Upstream {
    pub async fn start(status: StatusCode, content_type: &'static str, body: Bytes) -> Self {
        let requests = Arc::new(Mutex::new(Vec::new()));
        let router = Router::new()
            .route("/api", post(capture))
            .with_state(Reply {
                body,
                status,
                content_type,
                requests: Arc::clone(&requests),
            });
        Self {
            server: LocalServer::start(router).await,
            requests,
        }
    }
    pub async fn json(body: &Value) -> Self {
        Self::start(
            StatusCode::OK,
            "application/json",
            Bytes::from(sonic_rs::to_vec(body).unwrap()),
        )
        .await
    }
    pub fn search_client(&self) -> SearchClient {
        let settings = self.server.settings();
        SearchClient::new(
            &settings,
            secure_client(&settings.http, &settings.ssrf).unwrap(),
        )
    }
}
async fn capture(
    State(reply): State<Reply>,
    headers: HeaderMap,
    body: Bytes,
) -> (StatusCode, [(http::HeaderName, &'static str); 1], Bytes) {
    reply.requests.lock().await.push(Captured {
        headers,
        body: sonic_rs::from_slice(&body).unwrap(),
    });
    (
        reply.status,
        [(CONTENT_TYPE, reply.content_type)],
        reply.body,
    )
}
pub(crate) fn query(text: &str) -> SearchQueryRequest {
    SearchQueryRequest {
        q: text.to_owned(),
        recency: None,
        domains: None,
        category: None,
    }
}
