use crate::{
    Result,
    cli::{RuntimeOptions, Transport, WorkMode, oneshot::Invocation},
    config::AppConfig,
    error::AppError,
    mcp::{self, handler::health, http_service, tools::ToolCredentials},
};
use axum::{Router, routing::get};
use core::net::SocketAddr;
use rmcp::{serve_server, transport::stdio};
use std::io::Write as _;
use tokio::net::TcpListener;
use tracing::info;
const STREAMABLE_HTTP_PATH: &str = "/mcp";
const HEALTH_PATH: &str = "/health";
#[expect(
    clippy::missing_inline_in_public_items,
    reason = "The async server entrypoint composes HTTP and stdio MCP services."
)]
pub async fn run(config: AppConfig, options: RuntimeOptions) -> anyhow::Result<()> {
    options.validate().map_err(anyhow::Error::from)?;
    if let Some(invocation) = options.invocation {
        return run_command(config, options.credentials, invocation).await;
    }
    match options.transport {
        Transport::Http => run_http(config).await,
        Transport::Stdio => run_stdio(config, options.mode, options.credentials).await,
    }
}
async fn run_command(
    config: AppConfig,
    credentials: ToolCredentials,
    invocation: Invocation,
) -> anyhow::Result<()> {
    let service = mcp::local_service(&config, WorkMode::Response, credentials)?;
    let (kind, arguments) = invocation.request.into_arguments()?;
    let output = service
        .call(
            config.tools.name(kind),
            Some(arguments),
            &http::HeaderMap::new(),
        )
        .await?;
    writeln!(
        std::io::stdout().lock(),
        "{}",
        output.formatted(invocation.format)?
    )?;
    if let Some(message) = output.failure() {
        return Err(AppError::client(message).into());
    }
    Ok(())
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
    let service = mcp::local_service(&config, mode, credentials)?;
    info!("web MCP server listening on stdio");
    let running = serve_server(service, stdio()).await?;
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
