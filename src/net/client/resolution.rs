use super::{
    SecureHttpClient,
    request_budget::{duration, timeout_error},
};
use crate::Result;
use reqwest::{Url, header::ACCEPT};
impl SecureHttpClient {
    #[expect(
        clippy::missing_inline_in_public_items,
        reason = "URL resolution follows guarded HTTP redirects within one request budget."
    )]
    pub async fn resolve_url(&self, url: &str, timeout_seconds: f64) -> Result<Url> {
        let builder = self
            .direct
            .get(self.checked_url(url)?)
            .header(ACCEPT, "text/html");
        self.with_response(builder, timeout_seconds, |response| {
            core::future::ready(Ok(response.url().clone()))
        })
        .await
    }
    #[expect(
        clippy::missing_inline_in_public_items,
        reason = "URL validation performs DNS I/O within the configured time budget."
    )]
    pub async fn validate_url(&self, url: &Url, timeout_seconds: f64) -> Result<()> {
        tokio::time::timeout(duration(timeout_seconds)?, self.guard.validate_url(url))
            .await
            .map_err(|_elapsed| timeout_error())?
    }
}
