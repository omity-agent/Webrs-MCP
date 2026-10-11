use crate::{
    Result,
    arguments::{
        aliases,
        request::{RequestMap, RequestNormalizer},
        support::{parse_json_container, requests_entry, warn_unused_top_fields},
    },
    error::AppError,
    warnings::Warnings,
};
use sonic_rs::{JsonContainerTrait as _, Value};
#[inline]
pub fn normalize_requests(
    raw: Option<Value>,
    fields: &'static [aliases::FieldSpec],
    primary_field: Option<&'static str>,
) -> Result<(Vec<RequestMap>, Warnings)> {
    let mut warnings = Warnings::new();
    let Some(arguments) = raw.map(parse_json_container) else {
        return Err(AppError::client(
            "Invalid request: requests is required and must be an array",
        ));
    };
    if let Some(object) = arguments.as_object() {
        if let Some((key, value)) = requests_entry(object) {
            if key != "requests" {
                warnings.push_unique(format!("use \"requests\" instead of \"{key}\""));
            }
            warn_unused_top_fields(object, &key, &mut warnings);
            let requests = requests_from_value(value, fields, primary_field, &mut warnings);
            return Ok((requests, warnings));
        }
        let normalizer = RequestNormalizer::new(fields, primary_field);
        let keys = object.iter().map(|(key, _)| key);
        if aliases::looks_like_request(fields, keys) {
            warnings.push_unique("wrap the request object in the \"requests\" array".to_owned());
            let request = normalizer.normalize_object(&arguments, object, &mut warnings, "request");
            return Ok((vec![request], warnings));
        }
        if let Some(request) = normalizer.singleton_request(&arguments, &mut warnings, "request") {
            return Ok((vec![request], warnings));
        }
        return Err(AppError::client(
            "Invalid request: requests is required and must be an array",
        ));
    }
    let requests = requests_from_value(arguments, fields, primary_field, &mut warnings);
    Ok((requests, warnings))
}
fn requests_from_value(
    value: Value,
    fields: &'static [aliases::FieldSpec],
    primary_field: Option<&'static str>,
    warnings: &mut Warnings,
) -> Vec<RequestMap> {
    let parsed = parse_json_container(value);
    let normalizer = RequestNormalizer::new(fields, primary_field);
    if let Some(array) = parsed.as_array() {
        let mut requests = Vec::with_capacity(array.len());
        for (index, item) in array.iter().enumerate() {
            requests.push(normalizer.normalize_value(
                item.clone(),
                warnings,
                &format!("requests[{index}]"),
            ));
        }
        return requests;
    }
    warnings.push_unique("pass \"requests\" as an array");
    vec![normalizer.normalize_value(parsed, warnings, "requests[0]")]
}
