use crate::{Result, error::AppError};
use reqwest::header::HeaderValue;
use url::Url;
pub fn positive<T>(value: &T, path: &str) -> Result<()>
where
    T: PartialOrd + From<u8>,
{
    if value > &T::from(0_u8) {
        return Ok(());
    }
    Err(AppError::config(format!("{path} must be positive")))
}
pub fn positive_float(value: f64, path: &str) -> Result<()> {
    if value.is_finite() && value > 0.0_f64 {
        return Ok(());
    }
    Err(AppError::config(format!("{path} must be positive")))
}
pub fn threshold(value: f64, path: &str) -> Result<()> {
    if (0.0_f64..=1.0_f64).contains(&value) {
        return Ok(());
    }
    Err(AppError::config(format!("{path} must be between 0 and 1")))
}
pub fn header_value(value: &str, path: &str) -> Result<()> {
    HeaderValue::from_str(value)
        .map(|_header| ())
        .map_err(|error| AppError::config(format!("{path}: {error}")))
}
pub fn endpoint(value: &str, path: &str) -> Result<()> {
    let url = Url::parse(value).map_err(|error| AppError::config(format!("{path}: {error}")))?;
    if matches!(url.scheme(), "http" | "https") && url.host_str().is_some() {
        return Ok(());
    }
    Err(AppError::config(format!(
        "{path} must be an absolute HTTP or HTTPS URL"
    )))
}
pub fn template_endpoint(value: &str, path: &str, placeholder: &str) -> Result<()> {
    if !value.contains(placeholder) {
        return Err(AppError::config(format!(
            "{path} must contain {placeholder}"
        )));
    }
    endpoint(&value.replace(placeholder, "1"), path)
}
pub fn path_prefix(value: &str, path: &str) -> Result<()> {
    if value.starts_with('/') {
        return Ok(());
    }
    Err(AppError::config(format!("{path} must start with /")))
}
pub fn tinyfish(config: &crate::config::TinyFishConfig) -> Result<()> {
    positive(
        &config.max_urls_per_request,
        "tinyfish.max_urls_per_request",
    )?;
    if config.max_urls_per_request > 10 {
        return Err(AppError::config(
            "tinyfish.max_urls_per_request must be <= 10",
        ));
    }
    positive(&config.per_url_timeout_ms, "tinyfish.per_url_timeout_ms")?;
    if config.per_url_timeout_ms > 110_000 {
        return Err(AppError::config(
            "tinyfish.per_url_timeout_ms must be <= 110000",
        ));
    }
    if config.format != "markdown" {
        return Err(AppError::config("tinyfish.format must be markdown"));
    }
    Ok(())
}
