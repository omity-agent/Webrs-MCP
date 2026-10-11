use super::{docs_rs_url_with_path, is_docs_rs_host};
use crate::config::DirectFetchConfig;
use url::Url;
#[inline]
#[must_use]
pub fn resolve_rustdoc_json_url(
    parsed: &Url,
    host: &str,
    config: &DirectFetchConfig,
) -> Option<String> {
    if !is_docs_rs_host(host, config) {
        return None;
    }
    let parts = parsed
        .path_segments()?
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    let (crate_name, version, target) = crate_route(&parts)?;
    let target_suffix = target.map_or_else(String::new, |value| format!("/{value}"));
    Some(docs_rs_url_with_path(
        parsed,
        &format!("/crate/{crate_name}/{version}{target_suffix}/json"),
    ))
}
fn crate_route<'path>(
    parts: &'path [&str],
) -> Option<(&'path str, &'path str, Option<&'path str>)> {
    if parts.first() == Some(&"crate") {
        let crate_name = valid_segment(parts.get(1).copied()?)?;
        let version = valid_segment(parts.get(2).copied()?)?;
        return documented_route(crate_name, version, parts.get(3..)?);
    }
    let crate_name = valid_segment(parts.first().copied()?)?;
    let version = valid_segment(parts.get(1).copied()?)?;
    if is_reserved_page(crate_name) {
        return None;
    }
    documented_route(crate_name, version, parts.get(2..)?)
}
#[expect(
    clippy::pattern_type_mismatch,
    reason = "Route segments are matched through a borrowed URL path."
)]
fn documented_route<'path>(
    crate_name: &'path str,
    version: &'path str,
    rest: &[&'path str],
) -> Option<(&'path str, &'path str, Option<&'path str>)> {
    match rest {
        [] | ["json"] => Some((crate_name, version, None)),
        [target, "json"] => Some((crate_name, version, valid_segment(target))),
        _ if rest
            .first()
            .is_some_and(|section| is_reserved_page(section)) =>
        {
            None
        }
        _ => Some((crate_name, version, None)),
    }
}
fn valid_segment(value: &str) -> Option<&str> {
    (!value.is_empty() && !matches!(value, "." | "..") && !value.contains('/')).then_some(value)
}
fn is_reserved_page(value: &str) -> bool {
    matches!(
        value,
        "about"
            | "builds"
            | "crate"
            | "download"
            | "features"
            | "favicon.ico"
            | "releases"
            | "robots.txt"
            | "source"
    )
}
