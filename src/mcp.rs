pub mod handler;
pub mod processing;
pub mod schemas;
pub mod server;
pub mod tools;
use crate::{Result, cli::WorkMode, config::AppConfig};
use axum::{Router, extract::Request};
use rmcp::{
    model::ProtocolVersion,
    transport::{
        StreamableHttpServerConfig,
        streamable_http_server::{
            session::never::NeverSessionManager, tower::StreamableHttpService,
        },
    },
};
#[cfg(test)]
mod tests;
pub(crate) fn http_service(config: &AppConfig) -> Result<Router> {
    let tools = tools::ToolService::new(config.clone())?;
    let modern = StreamableHttpService::new(
        move || Ok(tools.clone()),
        NeverSessionManager::default().into(),
        streamable_http_config(config),
    );
    let mut legacy = modern.clone();
    legacy.config.stateless_protocol_metadata_required = false;
    Ok(Router::new().fallback(move |request: Request| {
        let selected = if request
            .headers()
            .get("mcp-protocol-version")
            .is_some_and(|version| version == ProtocolVersion::V_2025_06_18.as_str())
        {
            legacy.clone()
        } else {
            modern.clone()
        };
        async move { selected.handle(request).await }
    }))
}
#[inline]
pub(crate) fn local_service(
    config: &AppConfig,
    mode: WorkMode,
    credentials: tools::ToolCredentials,
) -> Result<tools::ToolService> {
    let mut local_config = config.clone();
    local_config.ssrf.block_private_networks = false;
    local_config.ssrf.block_local_hostnames = false;
    tools::ToolService::new_with_credentials(local_config, credentials, mode)
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
