pub(crate) mod body;
pub mod client;
pub(crate) mod resolver;
pub mod ssrf;
#[cfg(test)]
mod tests;
mod transport;
pub type FetchResponse = client::FetchResponse;
pub type SecureHttpClient = client::SecureHttpClient;
pub type SsrfGuard = ssrf::SsrfGuard;
use crate::{
    Result,
    config::{HttpConfig, SsrfConfig},
};
#[inline]
pub fn secure_client(http: &HttpConfig, ssrf: &SsrfConfig) -> Result<SecureHttpClient> {
    SecureHttpClient::new(http, SsrfGuard::new(ssrf.clone()))
}
#[inline]
pub(crate) fn secure_client_from_config(
    config: &crate::config::AppConfig,
) -> Result<SecureHttpClient> {
    secure_client(&config.http, &config.ssrf)
}
