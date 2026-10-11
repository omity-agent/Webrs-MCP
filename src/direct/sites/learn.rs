use crate::config::DirectFetchConfig;
use url::Url;
pub(in crate::direct) fn markdown_url(parsed: &Url, config: &DirectFetchConfig) -> Option<String> {
    let host = parsed.host_str()?;
    if !config
        .microsoft_learn_hosts
        .iter()
        .any(|configured| configured.eq_ignore_ascii_case(host))
    {
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
