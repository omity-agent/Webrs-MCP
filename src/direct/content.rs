use crate::{
    Result,
    config::DirectFetchConfig,
    direct::{
        mediawiki::extract_mediawiki_content,
        package::format_package_registry_json,
        stack_overflow::format_stack_overflow_question_json,
        target::{DirectFetchTarget, ResponseFormat},
    },
    error::{AppError, http_service_error},
};
use reqwest::header::{CONTENT_TYPE, HeaderMap};
use sonic_rs::{JsonValueMutTrait as _, JsonValueTrait as _, Value};
#[expect(
    clippy::missing_inline_in_public_items,
    reason = "Direct content extraction can parse and reserialize response bodies."
)]
pub fn extract_content(
    target: &DirectFetchTarget,
    status_code: u16,
    headers: &HeaderMap,
    body: &[u8],
    direct_config: &DirectFetchConfig,
) -> Result<String> {
    if status_code >= 400 {
        return Err(http_service_error("direct fetch", status_code));
    }
    if body.len() > direct_config.max_bytes {
        return Err(AppError::client(format!(
            "Direct content is larger than the allowed {} bytes.",
            direct_config.max_bytes
        )));
    }
    ensure_text_content(target, body)?;
    ensure_required_content_type(target, headers)?;
    match target.response_format {
        ResponseFormat::Text => Ok(String::from_utf8_lossy(body).into_owned()),
        ResponseFormat::MediaWikiApi => extract_mediawiki_content(body),
        ResponseFormat::PackageRegistryJson => {
            let payload = package_registry_payload(body)?;
            format_package_registry_json(
                &payload,
                &target.json_fields_first,
                &target.json_fields_last,
            )
        }
        ResponseFormat::StackOverflowQuestionJson => format_stack_overflow_question_json(body),
    }
}
fn ensure_text_content(target: &DirectFetchTarget, body: &[u8]) -> Result<()> {
    if !matches!(target.response_format, ResponseFormat::Text)
        || content_inspector::inspect(body).is_text()
    {
        return Ok(());
    }
    Err(AppError::client("Direct fetch returned binary content."))
}
fn package_registry_payload(body: &[u8]) -> Result<Value> {
    let mut payload: Value = sonic_rs::from_slice(body)
        .map_err(|_error| AppError::client("Package registry returned malformed JSON."))?;
    normalize_crlf(&mut payload);
    Ok(payload)
}
fn normalize_crlf(value: &mut Value) {
    if let Some(text) = value.as_str() {
        if text.contains('\r') {
            let normalized = text.replace("\r\n", "\n");
            *value = Value::from(normalized.as_str());
        }
    } else if let Some(array) = value.as_array_mut() {
        for item in array.as_mut_slice() {
            normalize_crlf(item);
        }
    } else if let Some(object) = value.as_object_mut() {
        for (_, item) in object.iter_mut() {
            normalize_crlf(item);
        }
    }
}
fn ensure_required_content_type(target: &DirectFetchTarget, headers: &HeaderMap) -> Result<()> {
    let Some(expected) = target.required_content_type.as_deref() else {
        return Ok(());
    };
    let Some(content_type) = headers
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
    else {
        return Err(AppError::client(format!(
            "Direct fetch returned no Content-Type header; expected {expected}."
        )));
    };
    let actual = content_type.parse::<mime::Mime>().map_err(|error| {
        AppError::client(format!(
            "Direct fetch returned invalid Content-Type {content_type}: {error}."
        ))
    })?;
    let expected_mime = expected.parse::<mime::Mime>().map_err(|error| {
        AppError::internal(format!(
            "configured direct fetch Content-Type {expected} is invalid: {error}"
        ))
    })?;
    if actual.essence_str() == expected_mime.essence_str() {
        return Ok(());
    }
    Err(AppError::client(format!(
        "Direct fetch returned Content-Type {content_type}; expected {expected}."
    )))
}
