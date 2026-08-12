pub mod exa;
mod payload;
pub type ExaSearchClient = exa::ExaSearchClient;
use crate::{Result, config::AppConfig};
#[inline]
pub(crate) fn client(config: &AppConfig) -> Result<ExaSearchClient> {
    ExaSearchClient::new(config)
}
#[must_use]
#[inline]
pub(crate) const fn provider_name() -> &'static str {
    "Exa"
}
#[must_use]
#[inline]
pub(crate) const fn api_key_header() -> &'static str {
    "x-api-key"
}
