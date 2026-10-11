use alloc::sync::Arc;
use axum::{
    Router,
    extract::{Request, State},
    http::header::ACCEPT,
    middleware::{self, Next},
    response::Response,
};
use tokio::{net::TcpListener, sync::Mutex, task::JoinHandle};
use web_rs::{config, net::secure_client, page::PageFetcher};
#[derive(Clone, Debug)]
pub(super) struct Visit {
    pub uri: String,
    pub accept: String,
}
pub(super) struct Website {
    pub address: String,
    pub visits: Arc<Mutex<Vec<Visit>>>,
    pub task: JoinHandle<()>,
}
impl Website {
    pub async fn start(router: Router) -> Self {
        let visits = Arc::new(Mutex::new(Vec::new()));
        let observed = router.layer(middleware::from_fn_with_state(
            Arc::clone(&visits),
            record_visit,
        ));
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
    pub fn settings(&self) -> config::AppConfig {
        let mut settings = config::load_embedded().unwrap();
        settings.ssrf.block_private_networks = false;
        settings.ssrf.block_local_hostnames = false;
        settings.jina.endpoint = format!("{}/reader", self.address);
        settings
    }
    pub fn fetcher(&self, settings: &config::AppConfig) -> PageFetcher {
        assert!(settings.jina.endpoint.starts_with(&self.address));
        PageFetcher::new(
            Arc::new(settings.clone()),
            secure_client(&settings.http, &settings.ssrf).unwrap(),
        )
    }
}
impl Drop for Website {
    fn drop(&mut self) {
        self.task.abort();
    }
}
async fn record_visit(
    State(visits): State<Arc<Mutex<Vec<Visit>>>>,
    request: Request,
    next: Next,
) -> Response {
    visits.lock().await.push(Visit {
        uri: request.uri().to_string(),
        accept: request
            .headers()
            .get(ACCEPT)
            .map_or_else(String::new, |value| value.to_str().unwrap().to_owned()),
    });
    next.run(request).await
}
