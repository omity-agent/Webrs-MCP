pub mod handler;
pub(crate) mod processing;
pub mod schemas;
pub mod server;
pub mod tools;
use crate::{Result, config::AppConfig};
use rmcp::transport::{
    StreamableHttpServerConfig,
    streamable_http_server::{session::never::NeverSessionManager, tower::StreamableHttpService},
};
pub(crate) type HttpMcpService = StreamableHttpService<tools::ToolService, NeverSessionManager>;
#[cfg(test)]
mod tests;
pub(crate) fn http_service(config: &AppConfig) -> Result<HttpMcpService> {
    let tools = tools::ToolService::new(config.clone())?;
    Ok(StreamableHttpService::new(
        move || Ok(tools.clone()),
        NeverSessionManager::default().into(),
        streamable_http_config(config),
    ))
}
#[inline]
pub(crate) fn stdio_service(
    config: &AppConfig,
    credentials: tools::ToolCredentials,
) -> Result<tools::ToolService> {
    let mut stdio_config = config.clone();
    stdio_config.ssrf.block_private_networks = false;
    stdio_config.ssrf.block_local_hostnames = false;
    tools::ToolService::new_with_credentials(stdio_config, credentials)
}
#[must_use]
pub(crate) fn streamable_http_config(config: &AppConfig) -> StreamableHttpServerConfig {
    StreamableHttpServerConfig::default()
        .with_legacy_session_mode(false)
        .with_stateless_protocol_metadata_required(true)
        .with_json_response(config.server.json_response)
        .with_allowed_hosts(config.server.allowed_hosts.clone())
        .with_allowed_origins(config.server.allowed_origins.clone())
}
