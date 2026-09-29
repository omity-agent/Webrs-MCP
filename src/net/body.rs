use crate::{Result, error::AppError};
use bytes::Bytes;
use http_body_util::{BodyExt as _, LengthLimitError, Limited};
pub(crate) async fn collect(response: reqwest::Response, max_bytes: usize) -> Result<Bytes> {
    let limit = u64::try_from(max_bytes)
        .map_err(|error| AppError::config(format!("HTTP response limit is too large: {error}")))?;
    if response
        .content_length()
        .is_some_and(|length| length > limit)
    {
        return Err(limit_error(max_bytes));
    }
    let body = reqwest::Body::from(response);
    let collected = Limited::new(body, max_bytes)
        .collect()
        .await
        .map_err(|error| {
            if error.is::<LengthLimitError>() {
                return limit_error(max_bytes);
            }
            match error.downcast::<reqwest::Error>() {
                Ok(request_error) => AppError::from(*request_error),
                Err(other) => AppError::internal(format!("failed to read HTTP response: {other}")),
            }
        })?;
    Ok(collected.to_bytes())
}
fn limit_error(max_bytes: usize) -> AppError {
    AppError::client(format!(
        "HTTP response body exceeded the configured {max_bytes} byte limit."
    ))
}
