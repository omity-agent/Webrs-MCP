pub(crate) mod body;
pub mod client;
pub(crate) mod resolver;
pub mod ssrf;
#[cfg(test)]
mod tests;
pub type FetchResponse = client::FetchResponse;
pub type SecureHttpClient = client::SecureHttpClient;
pub type SsrfGuard = ssrf::SsrfGuard;
use crate::{
    Result,
    config::{HttpConfig, SsrfConfig},
};
#[inline]
pub fn secure_client(http: &HttpConfig, ssrf: &SsrfConfig) -> Result<SecureHttpClient> {
    SecureHttpClient::new(
        http.max_redirects,
        http.max_concurrent_requests,
        &http.user_agent,
        SsrfGuard::new(ssrf.clone()),
    )
}
#[must_use]
#[inline]
pub(crate) fn guard(ssrf: &SsrfConfig) -> SsrfGuard {
    SsrfGuard::new(ssrf.clone())
}
#[inline]
pub(crate) fn secure_client_from_config(
    config: &crate::config::AppConfig,
) -> Result<SecureHttpClient> {
    secure_client(&config.http, &config.ssrf)
}
