use crate::{
    Result,
    cli::{Transport, WorkMode},
    config::AppConfig,
    mcp::{self, handler::health, http_service, tools::ToolCredentials},
};
use axum::{Router, routing::get};
use core::net::SocketAddr;
use rmcp::{service::serve_directly, transport::stdio};
use tokio::net::TcpListener;
use tracing::info;
const STREAMABLE_HTTP_PATH: &str = "/mcp";
const HEALTH_PATH: &str = "/health";
#[expect(
    clippy::missing_inline_in_public_items,
    reason = "The async server entrypoint composes HTTP and stdio MCP services."
)]
pub async fn run(
    config: AppConfig,
    transport: Transport,
    mode: WorkMode,
    credentials: ToolCredentials,
) -> anyhow::Result<()> {
    if transport == Transport::Http && mode == WorkMode::Filesystem {
        anyhow::bail!("filesystem mode can only be used with --transport stdio");
    }
    match transport {
        Transport::Http => run_http(config).await,
        Transport::Stdio => run_stdio(config, mode, credentials).await,
    }
}
async fn run_http(config: AppConfig) -> anyhow::Result<()> {
    let address = SocketAddr::new(config.server.host.parse()?, config.server.port);
    let router = router(config.clone()).map_err(anyhow::Error::from)?;
    let listener = TcpListener::bind(address).await?;
    info!("web MCP server listening on http://{address}{STREAMABLE_HTTP_PATH}");
    axum::serve(listener, router).await?;
    Ok(())
}
#[expect(
    clippy::missing_inline_in_public_items,
    reason = "The async server entrypoint performs MCP stdio I/O and is not an inline candidate."
)]
pub async fn run_stdio(
    config: AppConfig,
    mode: WorkMode,
    credentials: ToolCredentials,
) -> anyhow::Result<()> {
    let service = mcp::stdio_service(&config, mode, credentials)?;
    info!("web MCP server listening on stdio");
    let running = serve_directly(service, stdio(), None);
    let quit_reason = running.waiting().await?;
    info!(?quit_reason, "web MCP stdio server stopped");
    Ok(())
}
#[inline]
pub fn router(config: AppConfig) -> Result<Router> {
    let service = http_service(&config)?;
    Ok(Router::new()
        .route(HEALTH_PATH, get(health))
        .nest_service(STREAMABLE_HTTP_PATH, service)
        .with_state(config))
}
