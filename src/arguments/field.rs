use crate::arguments::{aliases, singleton::single_valid_string, support::parse_json_container};
use crate::warnings::Warnings;
use sonic_rs::{JsonContainerTrait as _, JsonValueTrait as _, Value};
pub(crate) fn normalize_field(
    name: &str,
    value: Value,
    warnings: &mut Warnings,
    path: &str,
) -> Value {
    let parsed = parse_json_container(value);
    if name == "url" {
        return normalize_url(normalize_text(parsed), warnings, path);
    }
    if name == "category" {
        return normalize_category(parsed, warnings, path);
    }
    if name == "domains" && !parsed.is_null() && parsed.as_array().is_none() {
        warnings.push_unique(format!("pass \"{path}\" as an array"));
    }
    if name == "q" {
        let query = normalize_text(parsed);
        warn_site_query(&query, warnings, path);
        return query;
    }
    parsed
}
fn normalize_text(value: Value) -> Value {
    if value.as_str().is_some() {
        return value;
    }
    let Some(text) = single_valid_string(&value) else {
        return value;
    };
    Value::from(text.as_str())
}
fn normalize_url(value: Value, warnings: &mut Warnings, path: &str) -> Value {
    let Some(text) = value.as_str() else {
        return value;
    };
    let trimmed = text.trim();
    if trimmed != text {
        warnings.push_unique(format!("remove surrounding whitespace from \"{path}\""));
    }
    if trimmed.is_empty() || trimmed.contains("://") {
        return Value::from(trimmed);
    }
    let normalized = format!("https://{trimmed}");
    warnings.push_unique(format!(
        "include a URL scheme for \"{path}\"; interpreted as \"{normalized}\""
    ));
    Value::from(normalized.as_str())
}
fn normalize_category(value: Value, warnings: &mut Warnings, path: &str) -> Value {
    let Some(text) = value.as_str() else {
        return value;
    };
    let Some(category) = aliases::canonical_category(text) else {
        return value;
    };
    if category != text {
        warnings.push_unique(format!(
            "use \"{category}\" instead of \"{text}\" for \"{path}\""
        ));
    }
    Value::from(category)
}
fn warn_site_query(value: &Value, warnings: &mut Warnings, path: &str) {
    let Some(text) = value.as_str() else {
        return;
    };
    if text.to_ascii_lowercase().contains("site:") {
        warnings.push_unique(format!(
            "use \"domains\" instead of site: syntax in \"{path}\""
        ));
    }
}
