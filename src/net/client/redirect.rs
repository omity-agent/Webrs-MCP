use crate::{Result, error::AppError};
use reqwest::{Url, header::HeaderValue};
pub(super) fn redirect_target(location: Option<&HeaderValue>, base: &Url) -> Result<Option<Url>> {
    let Some(raw) = location else {
        return Ok(None);
    };
    let value = raw
        .to_str()
        .map_err(|_error| AppError::client("Redirect location is not valid UTF-8."))?;
    base.join(value)
        .map(Some)
        .map_err(|error| AppError::client(format!("Redirect location is invalid: {error}")))
}
