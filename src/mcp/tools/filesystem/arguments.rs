use crate::{
    Result,
    error::AppError,
    models::{FilesystemOpenArguments, FilesystemSearchArguments},
};
use serde::de::DeserializeOwned;
use sonic_rs::Value;
use url::Url;
pub(super) fn search(raw: Option<Value>) -> Result<FilesystemSearchArguments> {
    let arguments: FilesystemSearchArguments = deserialize(raw)?;
    validate_output_path(&arguments.output_path)?;
    for (index, request) in arguments.requests.iter().enumerate() {
        if request.q.trim().is_empty() {
            return Err(invalid(format!("requests[{index}].q must not be empty")));
        }
        if let Some(domains) = request.domains.as_deref() {
            for domain_entry in domains.iter().enumerate() {
                if domain_entry.1.trim().is_empty() {
                    return Err(invalid(format!(
                        "requests[{index}].domains[{}] must not be empty",
                        domain_entry.0
                    )));
                }
            }
        }
    }
    Ok(arguments)
}
pub(super) fn open(raw: Option<Value>) -> Result<FilesystemOpenArguments> {
    let arguments: FilesystemOpenArguments = deserialize(raw)?;
    validate_output_path(&arguments.output_path)?;
    for (index, url) in arguments.urls.iter().enumerate() {
        validate_url(url, index)?;
    }
    Ok(arguments)
}
fn deserialize<T>(raw: Option<Value>) -> Result<T>
where
    T: DeserializeOwned,
{
    let value = raw.ok_or_else(|| invalid("arguments are required"))?;
    sonic_rs::from_value(&value)
        .map_err(|error| invalid(format!("arguments do not match the tool schema: {error}")))
}
fn validate_output_path(path: &str) -> Result<()> {
    if path.trim().is_empty() {
        return Err(invalid("output_path must not be empty"));
    }
    Ok(())
}
fn validate_url(value: &str, index: usize) -> Result<()> {
    let parsed = Url::parse(value).map_err(|_error| {
        invalid(format!(
            "urls[{index}] must be an absolute HTTP or HTTPS URL"
        ))
    })?;
    if matches!(parsed.scheme(), "http" | "https") && parsed.host_str().is_some() {
        return Ok(());
    }
    Err(invalid(format!(
        "urls[{index}] must be an absolute HTTP or HTTPS URL"
    )))
}
fn invalid(message: impl core::fmt::Display) -> AppError {
    AppError::client(format!("Invalid request: {message}"))
}
