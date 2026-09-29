mod embedded;
mod protocol;
mod providers;
pub mod types;
mod validation;
use crate::Result;
pub type AppConfig = types::AppConfig;
pub type ChunkingConfig = types::ChunkingConfig;
pub type DirectFetchConfig = types::DirectFetchConfig;
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
#[inline]
#[must_use]
pub const fn default_yaml() -> &'static str {
    embedded::DEFAULT_YAML
}
#[inline]
pub fn load_embedded() -> Result<AppConfig> {
    let config = embedded::load()?;
    config.validate()?;
    Ok(config)
}
