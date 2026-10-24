mod environment;
mod naming;
mod protocol;
mod providers;
pub mod types;
pub(crate) mod validation;
use crate::Result;
use std::path::Path;
pub type AppConfig = types::AppConfig;
pub type ChunkingConfig = types::ChunkingConfig;
pub type DirectFetchConfig = types::DirectFetchConfig;
pub type EnvironmentConfig = environment::EnvironmentConfig;
pub type FindConfig = types::FindConfig;
pub type HeaderConfig = types::HeaderConfig;
pub type HttpConfig = types::HttpConfig;
pub type JinaConfig = types::JinaConfig;
pub type JinaViewportConfig = types::JinaViewportConfig;
pub type ProtocolConfig = protocol::ProtocolConfig;
pub type ExaConfig = providers::ExaConfig;
pub type OctenConfig = providers::OctenConfig;
pub type SearchConfig = providers::SearchConfig;
pub type ServerConfig = types::ServerConfig;
pub type SsrfConfig = types::SsrfConfig;
pub type TinyFishConfig = types::TinyFishConfig;
pub type ToolListCacheConfig = protocol::ToolListCacheConfig;
pub type ToolKind = naming::ToolKind;
pub type ToolNames = naming::ToolNames;
#[inline]
#[must_use]
pub const fn default_yaml() -> &'static str {
    include_str!("../config/default.yaml")
}
#[inline]
pub fn load_embedded() -> Result<AppConfig> {
    parse(default_yaml(), "embedded config/default.yaml")
}
#[inline]
pub fn load(path: &Path) -> Result<AppConfig> {
    let yaml = std::fs::read_to_string(path).map_err(|error| {
        crate::error::AppError::config(format!("failed to read {}: {error}", path.display()))
    })?;
    parse(&yaml, &path.display().to_string())
}
fn parse(yaml: &str, source: &str) -> Result<AppConfig> {
    let config: AppConfig = serde_saphyr::from_str(yaml).map_err(|error| {
        crate::error::AppError::config(format!("YAML {source} is invalid: {error}"))
    })?;
    config.validate()?;
    Ok(config)
}
