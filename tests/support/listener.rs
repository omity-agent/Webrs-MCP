use alloc::sync::Arc;
use axum::{
    Router,
    extract::{Request, State},
    http::HeaderMap,
    middleware::{self, Next},
    response::Response,
};
use tokio::{net::TcpListener, sync::Mutex, task::JoinHandle};
use web_rs::config::{self, AppConfig};
#[derive(Clone, Debug)]
pub(crate) struct Visit {
    pub uri: String,
    pub headers: HeaderMap,
}
pub(crate) struct LocalServer {
    pub address: String,
    pub visits: Arc<Mutex<Vec<Visit>>>,
    pub task: JoinHandle<()>,
}
impl LocalServer {
    pub async fn uris(&self) -> Vec<String> {
        self.visits
            .lock()
            .await
            .iter()
            .map(|visit| visit.uri.clone())
            .collect()
    }
    pub async fn start(router: Router) -> Self {
        let visits = Arc::new(Mutex::new(Vec::new()));
        let observed = router.layer(middleware::from_fn_with_state(Arc::clone(&visits), record));
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = format!("http://{}", listener.local_addr().unwrap());
        let task = tokio::spawn(async move {
            axum::serve(listener, observed).await.unwrap();
        });
        Self {
            address,
            visits,
            task,
        }
    }
    pub fn settings(&self) -> AppConfig {
        let mut settings = config::load_embedded().unwrap();
        settings.ssrf.block_private_networks = false;
        settings.ssrf.block_local_hostnames = false;
        let endpoint = format!("{}/api", self.address);
        settings.search.exa.endpoint.clone_from(&endpoint);
        settings.search.octen.endpoint.clone_from(&endpoint);
        settings.jina.endpoint.clone_from(&endpoint);
        settings.tinyfish.endpoint = endpoint;
        settings
    }
}
impl Drop for LocalServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}
async fn record(
    State(visits): State<Arc<Mutex<Vec<Visit>>>>,
    request: Request,
    next: Next,
) -> Response {
    visits.lock().await.push(Visit {
        uri: request.uri().to_string(),
        headers: request.headers().clone(),
    });
    next.run(request).await
}
