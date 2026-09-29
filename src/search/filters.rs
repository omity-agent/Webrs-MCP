use crate::{Result, error::AppError};
use chrono::{DateTime, Days, Utc};
use url::Url;
pub(super) fn domains(values: Option<&[String]>) -> Result<Option<Vec<String>>> {
    let Some(entries) = values else {
        return Ok(None);
    };
    let normalized = entries
        .iter()
        .map(|value| domain(value))
        .collect::<Result<Vec<_>>>()?;
    Ok((!normalized.is_empty()).then_some(normalized))
}
fn domain(value: &str) -> Result<String> {
    let trimmed = value.trim();
    let input = if trimmed.contains("://") {
        trimmed.to_owned()
    } else {
        format!("https://{trimmed}")
    };
    let parsed = Url::parse(&input)
        .map_err(|error| AppError::client(format!("Invalid search domain: {error}")))?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(AppError::client(
            "Search domains require HTTP or HTTPS URLs.",
        ));
    }
    parsed
        .host_str()
        .map(str::to_ascii_lowercase)
        .ok_or_else(|| AppError::client("Search domain has no hostname."))
}
pub(super) fn since(recency: Option<u64>) -> Result<Option<DateTime<Utc>>> {
    recency
        .map(|days| {
            Utc::now().checked_sub_days(Days::new(days)).ok_or_else(|| {
                AppError::client("Search recency is outside the supported date range.")
            })
        })
        .transpose()
}
