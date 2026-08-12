use crate::direct::{DirectFetchTarget, resolve_direct_fetch_target};
use url::Url;
pub(super) fn direct_fetch_targets(
    url: &str,
    config: &crate::config::DirectFetchConfig,
) -> Vec<DirectFetchTarget> {
    let markdown_targets = core::iter::once(markdown_accept_target(url))
        .chain(markdown_direct_fetch_targets(url))
        .collect::<Vec<_>>();
    if let Some(target) = resolve_direct_fetch_target(url, config) {
        return core::iter::once(target).chain(markdown_targets).collect();
    }
    markdown_targets
}
fn markdown_accept_target(url: &str) -> DirectFetchTarget {
    DirectFetchTarget::markdown_accept(url)
}
fn markdown_direct_fetch_targets(url: &str) -> Vec<DirectFetchTarget> {
    let probe_url = replace_path_suffix(url, &random_missing_suffix());
    let replaced = replace_extension_with_markdown(url);
    let mut candidates = Vec::with_capacity(3);
    if replaced.is_none() {
        candidates.push(replace_path_suffix(url, ".md"));
    }
    candidates.push(index_markdown_url(url));
    candidates.extend(replaced);
    dedup(candidates)
        .into_iter()
        .map(|candidate| markdown_direct_fetch_target(url, candidate, &probe_url))
        .collect()
}
fn markdown_direct_fetch_target(
    original_url: &str,
    request_url: String,
    probe_url: &str,
) -> DirectFetchTarget {
    let mut target = DirectFetchTarget::text(original_url, request_url);
    target.required_content_type = Some("text/markdown".to_owned());
    target.similarity_probe_url = Some(probe_url.to_owned());
    target
}
pub(super) fn replace_path_suffix(url: &str, suffix: &str) -> String {
    let Ok(mut parsed) = Url::parse(url) else {
        return url.to_owned();
    };
    let path = parsed.path().trim_end_matches('/');
    let next_path = if path.is_empty() {
        format!("/{}", suffix.trim_start_matches('.'))
    } else {
        format!("{path}{suffix}")
    };
    parsed.set_path(&next_path);
    parsed.to_string()
}
fn index_markdown_url(url: &str) -> String {
    let Ok(mut parsed) = Url::parse(url) else {
        return url.to_owned();
    };
    let path = parsed.path().trim_end_matches('/');
    let base_path =
        extension_dot_index(path).map_or(path, |dot_index| path.get(..dot_index).unwrap_or(path));
    let next_path = if path.is_empty() {
        "/index.md".to_owned()
    } else {
        format!("{base_path}/index.md")
    };
    parsed.set_path(&next_path);
    parsed.to_string()
}
fn replace_extension_with_markdown(url: &str) -> Option<String> {
    let mut parsed = Url::parse(url).ok()?;
    let path = parsed.path().trim_end_matches('/');
    let dot_index = extension_dot_index(path)?;
    let prefix = path
        .get(..dot_index)
        .map_or_else(|| path.to_owned(), str::to_owned);
    parsed.set_path(&format!("{prefix}.md"));
    Some(parsed.to_string())
}
fn extension_dot_index(path: &str) -> Option<usize> {
    let slash_index = path.rfind('/').unwrap_or(0);
    let dot_index = path.rfind('.')?;
    if dot_index <= slash_index || dot_index == path.len().saturating_sub(1) {
        return None;
    }
    Some(dot_index)
}
fn dedup(values: Vec<String>) -> Vec<String> {
    let mut unique = Vec::with_capacity(values.len());
    for value in values {
        if !unique.contains(&value) {
            unique.push(value);
        }
    }
    unique
}
fn random_missing_suffix() -> String {
    format!(".{}", nanoid::nanoid!())
}
