use crate::{Result, config::DirectFetchConfig, error::AppError};
use url::Url;
pub(in crate::direct) fn markdown_url(parsed: &Url, config: &DirectFetchConfig) -> Option<String> {
    if !matches_host(parsed, config) {
        return None;
    }
    let mut segments = parsed
        .path_segments()?
        .filter(|segment| !segment.is_empty());
    let first = decoded_segment(segments.next()?)?;
    let section = if is_locale(&first) {
        decoded_segment(segments.next()?)?
    } else {
        first
    };
    segments.next()?;
    if config
        .microsoft_learn_excluded_sections
        .iter()
        .any(|excluded| excluded.eq_ignore_ascii_case(&section))
    {
        return None;
    }
    let mut next = parsed.clone();
    let pairs = parsed
        .query_pairs()
        .filter(|pair| !pair.0.eq_ignore_ascii_case("accept"))
        .map(|(key, value)| (key.into_owned(), value.into_owned()));
    next.query_pairs_mut()
        .clear()
        .extend_pairs(pairs)
        .append_pair("accept", "text/markdown");
    Some(next.to_string())
}
pub(crate) fn markdown_body<'markdown>(
    source_url: &str,
    markdown: &'markdown str,
    config: &DirectFetchConfig,
) -> Result<&'markdown str> {
    let parsed = Url::parse(source_url)
        .map_err(|error| AppError::internal(format!("invalid fetched page URL: {error}")))?;
    if !matches_host(&parsed, config) {
        return Ok(markdown);
    }
    Ok(front_matter_prefix(markdown)
        .and_then(|prefix| markdown.get(prefix..))
        .unwrap_or(markdown))
}
fn matches_host(parsed: &Url, config: &DirectFetchConfig) -> bool {
    parsed.host_str().is_some_and(|host| {
        config
            .microsoft_learn_hosts
            .iter()
            .any(|configured| configured.eq_ignore_ascii_case(host))
    })
}
fn front_matter_prefix(markdown: &str) -> Option<usize> {
    let bom_length = if markdown.starts_with('\u{FEFF}') {
        '\u{FEFF}'.len_utf8()
    } else {
        0
    };
    let source = markdown.get(bom_length..)?;
    let mut lines = source.split_inclusive('\n');
    let opening = lines.next()?;
    if !is_front_matter_delimiter(opening) {
        return None;
    }
    let mut consumed = opening.len();
    for metadata_line in lines {
        consumed += metadata_line.len();
        if is_front_matter_delimiter(metadata_line) {
            let mut body_start = bom_length + consumed;
            while let Some(blank_line) = markdown
                .get(body_start..)
                .and_then(|body| body.split_inclusive('\n').next())
            {
                if !blank_line.trim_matches([' ', '\t', '\r', '\n']).is_empty() {
                    break;
                }
                body_start += blank_line.len();
            }
            return Some(body_start);
        }
    }
    None
}
fn is_front_matter_delimiter(line: &str) -> bool {
    line.trim_end_matches(['\r', '\n'])
        .trim_end_matches([' ', '\t'])
        == "---"
}
fn is_locale(segment: &str) -> bool {
    segment.split_once('-').is_some_and(|(language, suffix)| {
        let region = if let Some((script, region)) = suffix.split_once('-') {
            if script.len() != 4 || !script.bytes().all(|byte| byte.is_ascii_alphabetic()) {
                return false;
            }
            region
        } else {
            suffix
        };
        (2..=3).contains(&language.len())
            && language.bytes().all(|byte| byte.is_ascii_alphabetic())
            && ((region.len() == 2 && region.bytes().all(|byte| byte.is_ascii_alphabetic()))
                || (region.len() == 3 && region.bytes().all(|byte| byte.is_ascii_digit())))
    })
}
fn decoded_segment(segment: &str) -> Option<alloc::borrow::Cow<'_, str>> {
    percent_encoding::percent_decode_str(segment)
        .decode_utf8()
        .ok()
}
