use super::SecureHttpClient;
use crate::{Result, error::AppError};
use core::{future::Future, time::Duration};
impl SecureHttpClient {
    pub(super) async fn with_response<Output, Process, Pending>(
        &self,
        builder: reqwest::RequestBuilder,
        timeout_seconds: f64,
        process: Process,
    ) -> Result<Output>
    where
        Process: FnOnce(reqwest::Response) -> Pending,
        Pending: Future<Output = Result<Output>>,
    {
        let (client, request) = builder.build_split();
        let prepared = request?;
        let pending = async {
            let _permit = self.requests.acquire().await.map_err(|error| {
                AppError::internal(format!("HTTP request limiter was closed: {error}"))
            })?;
            let response = client.execute(prepared).await?;
            process(response).await
        };
        tokio::time::timeout(duration(timeout_seconds)?, pending)
            .await
            .map_err(|_elapsed| timeout_error())?
    }
}
pub(super) fn duration(seconds: f64) -> Result<Duration> {
    let parsed = Duration::try_from_secs_f64(seconds)
        .map_err(|error| AppError::config(format!("HTTP timeout is invalid: {error}")))?;
    if parsed.is_zero() {
        return Err(AppError::config("HTTP timeout must be positive"));
    }
    Ok(parsed)
}
pub(super) fn timeout_error() -> AppError {
    AppError::Upstream("upstream request timed out. Retry later.".to_owned())
}
