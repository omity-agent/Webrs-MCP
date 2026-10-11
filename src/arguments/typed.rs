use crate::{
    Result,
    arguments::{
        Normalized,
        aliases::{FIND_FIELDS, OPEN_FIELDS, SEARCH_FIELDS},
        normalizer::normalize_requests,
        request::RequestMap,
    },
    error::AppError,
    models::{
        FindArguments, FindRequest, OpenArguments, OpenRequest, SearchQueryArguments,
        SearchQueryRequest,
    },
    warnings::Warnings,
};
use sonic_rs::Value;
use validation::{
    integer_value, is_empty, optional_category, optional_string_list, optional_usize,
    required_http_url, required_string, required_usize,
};
mod validation;
#[inline]
pub fn search_arguments(raw: Option<Value>) -> Result<Normalized<SearchQueryArguments>> {
    let (maps, warnings) = normalize_requests(raw, SEARCH_FIELDS, Some("q"))?;
    let mut requests = Vec::with_capacity(maps.len());
    for (index, map) in maps.iter().enumerate() {
        requests.push(SearchQueryRequest {
            q: required_string(map, "q", &format!("requests[{index}].q"))?,
            recency: optional_usize(map, "recency", &format!("requests[{index}].recency"))?
                .map(u64::try_from)
                .transpose()
                .map_err(|_overflow| {
                    AppError::client("Invalid request: requests[].recency is too large")
                })?,
            domains: optional_string_list(map, "domains", &format!("requests[{index}].domains"))?,
            category: optional_category(map, &format!("requests[{index}].category"))?,
        });
    }
    Ok(Normalized::new(SearchQueryArguments { requests }, warnings))
}
#[inline]
pub fn open_arguments(raw: Option<Value>) -> Result<Normalized<OpenArguments>> {
    let (mut maps, mut warnings) = normalize_requests(raw, OPEN_FIELDS, Some("url"))?;
    normalize_open_chunks(&mut maps, &mut warnings);
    let mut requests = Vec::with_capacity(maps.len());
    for (index, map) in maps.iter().enumerate() {
        let url = required_http_url(map, "url", &format!("requests[{index}].url"))?;
        let chunk = required_usize(map, "chunk", &format!("requests[{index}].chunk"))?;
        requests.push(OpenRequest { url, chunk });
    }
    Ok(Normalized::new(OpenArguments { requests }, warnings))
}
#[inline]
pub fn find_arguments(raw: Option<Value>) -> Result<Normalized<FindArguments>> {
    let (maps, warnings) = normalize_requests(raw, FIND_FIELDS, None)?;
    let mut requests = Vec::with_capacity(maps.len());
    for (index, map) in maps.iter().enumerate() {
        requests.push(FindRequest {
            url: required_http_url(map, "url", &format!("requests[{index}].url"))?,
            pattern: required_string(map, "pattern", &format!("requests[{index}].pattern"))?,
            snippet_tokens: optional_usize(
                map,
                "snippet_tokens",
                &format!("requests[{index}].snippet_tokens"),
            )?,
        });
    }
    Ok(Normalized::new(FindArguments { requests }, warnings))
}
fn normalize_open_chunks(maps: &mut [RequestMap], warnings: &mut Warnings) {
    for (index, map) in maps.iter_mut().enumerate() {
        let path = format!("requests[{index}].chunk");
        match map.get("chunk") {
            None => {
                map.insert("chunk".to_owned(), Value::from(0_u64));
                warnings.push_unique(format!("\"{path}\" is required; using 0"));
            }
            Some(value) if is_empty(value) => {
                map.insert("chunk".to_owned(), Value::from(0_u64));
                warnings.push_unique(format!("\"{path}\" is empty; using 0"));
            }
            Some(value) if integer_value(value).is_some_and(|number| number < 0) => {
                map.insert("chunk".to_owned(), Value::from(0_u64));
                warnings.push_unique(format!(
                    "\"{path}\" must be greater than or equal to 0; using 0"
                ));
            }
            Some(_) => {}
        }
    }
}
