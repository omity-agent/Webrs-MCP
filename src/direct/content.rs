use crate::{
    Result,
    direct::{
        sites::{
            docs_rs::format_rustdoc_json, mediawiki::extract_mediawiki_content,
            package::format_package_registry_json,
            stack_overflow::format_stack_overflow_question_json,
        },
        target::{DirectFetchTarget, ResponseFormat},
    },
    error::{AppError, http_service_error},
};
use reqwest::header::{CONTENT_TYPE, HeaderMap};
use sonic_rs::{JsonValueMutTrait as _, JsonValueTrait as _, Value};
use std::sync::LazyLock;
static HTML_DOCUMENT: LazyLock<Result<fancy_regex::Regex>> = LazyLock::new(|| {
    fancy_regex::Regex::new(
        r"(?is)\A(?:\s|\u{FEFF}|<!--.*?-->)*(?:<!doctype\s+html(?:\s|>)|<html(?:\s|>))",
    )
    .map_err(|error| AppError::internal(format!("invalid HTML document signature: {error}")))
});
#[derive(Clone, Copy)]
enum ResponsePurpose {
    Content,
    MissingProbe,
}
#[expect(
    clippy::missing_inline_in_public_items,
    reason = "Direct content extraction can parse and reserialize response bodies."
)]
pub fn extract_content(
    target: &DirectFetchTarget,
    status_code: u16,
    headers: &HeaderMap,
    body: &[u8],
) -> Result<String> {
    extract_response(target, status_code, headers, body, ResponsePurpose::Content)?
        .ok_or_else(|| AppError::internal("accepted direct content was missing"))
}
pub(crate) fn extract_probe_content(
    target: &DirectFetchTarget,
    status_code: u16,
    headers: &HeaderMap,
    body: &[u8],
) -> Result<Option<String>> {
    extract_response(
        target,
        status_code,
        headers,
        body,
        ResponsePurpose::MissingProbe,
    )
}
fn extract_response(
    target: &DirectFetchTarget,
    status_code: u16,
    headers: &HeaderMap,
    body: &[u8],
    purpose: ResponsePurpose,
) -> Result<Option<String>> {
    match (purpose, status_code) {
        (_, 200) => {}
        (ResponsePurpose::MissingProbe, 404 | 410) => return Ok(None),
        _ => return Err(http_service_error("direct fetch", status_code)),
    }
    ensure_content_type(target.response_format, headers)?;
    let content = match target.response_format {
        ResponseFormat::Markdown | ResponseFormat::Text => {
            decode_text(target.response_format, body)
        }
        ResponseFormat::RustdocJson => format_rustdoc_json(headers, body),
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
    }?;
    Ok(Some(content))
}
fn decode_text(format: ResponseFormat, body: &[u8]) -> Result<String> {
    if !content_inspector::inspect(body).is_text() {
        return Err(AppError::client("Direct fetch returned binary content."));
    }
    let text = core::str::from_utf8(body).map_err(|error| {
        AppError::client(format!("Direct fetch returned invalid UTF-8: {error}"))
    })?;
    if matches!(format, ResponseFormat::Markdown)
        && HTML_DOCUMENT
            .as_ref()
            .map_err(Clone::clone)?
            .is_match(text)
            .map_err(|error| {
                AppError::internal(format!(
                    "failed to inspect direct Markdown content: {error}"
                ))
            })?
    {
        return Err(AppError::client(
            "Direct fetch returned an HTML document instead of Markdown.",
        ));
    }
    Ok(text.to_owned())
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
fn ensure_content_type(format: ResponseFormat, headers: &HeaderMap) -> Result<()> {
    let expected = match format {
        ResponseFormat::Markdown => "text/markdown",
        ResponseFormat::Text => "non-HTML textual content",
        ResponseFormat::RustdocJson => "application/json, application/gzip, or application/zstd",
        ResponseFormat::MediaWikiApi
        | ResponseFormat::PackageRegistryJson
        | ResponseFormat::StackOverflowQuestionJson => "application/json",
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
    let accepted = match format {
        ResponseFormat::Markdown => actual.essence_str() == "text/markdown",
        ResponseFormat::Text => {
            actual.essence_str() != "application/xhtml+xml"
                && ((actual.type_() == mime::TEXT && actual.subtype() != mime::HTML)
                    || is_json(&actual)
                    || actual.essence_str() == "application/xml"
                    || actual.suffix() == Some(mime::XML))
        }
        ResponseFormat::RustdocJson => matches!(
            actual.essence_str(),
            "application/json" | "application/gzip" | "application/zstd"
        ),
        ResponseFormat::MediaWikiApi
        | ResponseFormat::PackageRegistryJson
        | ResponseFormat::StackOverflowQuestionJson => is_json(&actual),
    };
    if accepted {
        return Ok(());
    }
    Err(AppError::client(format!(
        "Direct fetch returned Content-Type {content_type}; expected {expected}."
    )))
}
fn is_json(content_type: &mime::Mime) -> bool {
    content_type.type_() == mime::APPLICATION
        && (content_type.subtype() == mime::JSON || content_type.suffix() == Some(mime::JSON))
}
