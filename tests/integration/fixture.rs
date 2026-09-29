use alloc::sync::Arc;
use axum::{
    Router,
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode, header::CONTENT_TYPE},
    routing::post,
};
use rmcp::serde_json::Value;
use tokio::{net::TcpListener, sync::Mutex, task::JoinHandle};
use web_rs::{config, models::SearchQueryRequest};
pub(super) struct Listener {
    pub address: String,
    pub task: JoinHandle<()>,
}
impl Listener {
    pub async fn start(router: Router) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = format!("http://{}", listener.local_addr().unwrap());
        let task = tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });
        Self { address, task }
    }
}
impl Drop for Listener {
    fn drop(&mut self) {
        self.task.abort();
    }
}
#[derive(Clone)]
pub(super) struct Captured {
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
pub(super) struct Upstream {
    pub server: Listener,
    pub requests: Arc<Mutex<Vec<Captured>>>,
}
impl Upstream {
    pub async fn start(status: StatusCode, content_type: &'static str, body: Bytes) -> Self {
        let requests = Arc::new(Mutex::new(Vec::new()));
        let reply = Reply {
            body,
            status,
            content_type,
            requests: Arc::clone(&requests),
        };
        let router = Router::new()
            .route("/search", post(capture))
            .with_state(reply);
        Self {
            server: Listener::start(router).await,
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
    pub fn config(&self) -> config::AppConfig {
        let mut config = config::load_embedded().unwrap();
        config.ssrf.block_local_hostnames = false;
        config.ssrf.block_private_networks = false;
        config.search.exa.endpoint = format!("{}/search", self.server.address);
        config.search.octen.endpoint = format!("{}/search", self.server.address);
        config.jina.endpoint = format!("{}/search", self.server.address);
        config
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
pub(super) fn query(text: &str) -> SearchQueryRequest {
    SearchQueryRequest {
        q: text.to_owned(),
        recency: None,
        domains: None,
        category: None,
    }
}
