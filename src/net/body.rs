use crate::{Result, error::AppError};
use axum::body::Bytes;
pub(crate) async fn collect(
    mut response: reqwest::Response,
    limit: Option<usize>,
) -> Result<Bytes> {
    match limit {
        Some(max_bytes) => collect_limited(&mut response, max_bytes).await,
        None => Ok(response.bytes().await?),
    }
}
async fn collect_limited(response: &mut reqwest::Response, max_bytes: usize) -> Result<Bytes> {
    reject_declared_oversize(response, max_bytes)?;
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        append_chunk(&mut body, &chunk, max_bytes)?;
    }
    Ok(body.into())
}
fn reject_declared_oversize(response: &reqwest::Response, max_bytes: usize) -> Result<()> {
    let Some(content_length) = response.content_length() else {
        return Ok(());
    };
    let max_bytes_u64 = u64::try_from(max_bytes).map_err(|error| {
        AppError::internal(format!(
            "HTTP response body limit conversion failed: {error}"
        ))
    })?;
    if content_length <= max_bytes_u64 {
        return Ok(());
    }
    Err(body_limit_error(max_bytes))
}
fn append_chunk(body: &mut Vec<u8>, chunk: &[u8], max_bytes: usize) -> Result<()> {
    let next_len = body
        .len()
        .checked_add(chunk.len())
        .ok_or_else(|| AppError::internal("HTTP response body length overflowed usize"))?;
    if next_len > max_bytes {
        return Err(body_limit_error(max_bytes));
    }
    body.try_reserve(chunk.len()).map_err(|error| {
        AppError::internal(format!(
            "failed to reserve memory for HTTP response body: {error}"
        ))
    })?;
    body.extend_from_slice(chunk);
    Ok(())
}
fn body_limit_error(max_bytes: usize) -> AppError {
    AppError::client(format!(
        "HTTP response body exceeded the configured {max_bytes} byte limit."
    ))
}
#[cfg(test)]
mod tests;
