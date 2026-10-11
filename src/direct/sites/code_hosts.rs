use crate::config::DirectFetchConfig;
use url::Url;
#[inline]
#[must_use]
pub fn resolve_code_host_raw_url(
    parsed: &Url,
    host: &str,
    config: &DirectFetchConfig,
) -> Option<String> {
    if contains_host(&config.github_hosts, host) {
        return github_raw_url(parsed, host);
    }
    if contains_host(&config.huggingface_hosts, host) {
        return huggingface_raw_url(parsed, host);
    }
    if contains_host(&config.gitlab_hosts, host) {
        return gitlab_raw_url(parsed, host);
    }
    if contains_host(&config.bitbucket_hosts, host) {
        return bitbucket_raw_url(parsed, host);
    }
    None
}
#[expect(
    clippy::indexing_slicing,
    reason = "GitHub URL segments are length-checked before fixed-position access."
)]
fn github_raw_url(parsed: &Url, host: &str) -> Option<String> {
    if matches!(
        host,
        "raw.githubusercontent.com" | "gist.githubusercontent.com"
    ) {
        return Some(parsed.to_string());
    }
    let parts = path_parts(parsed.path());
    if parts.len() < 5 || !matches!(parts[2].as_str(), "blob" | "raw") {
        return None;
    }
    let file_path = parts[4..].join("/");
    Some(format!(
        "https://raw.githubusercontent.com/{}/{}/{}/{}",
        parts[0], parts[1], parts[3], file_path
    ))
}
#[expect(
    clippy::indexing_slicing,
    reason = "Hugging Face URL segments are length-checked against the marker before slicing."
)]
fn huggingface_raw_url(parsed: &Url, host: &str) -> Option<String> {
    let parts = path_parts(parsed.path());
    let marker = huggingface_marker_index(&parts)?;
    if parts.len() <= marker + 2 {
        return None;
    }
    let file_path = parts[(marker + 2)..].join("/");
    let repo = parts[..marker].join("/");
    Some(format!(
        "https://{host}/{repo}/resolve/{}/{file_path}",
        parts[marker + 1]
    ))
}
#[expect(
    clippy::indexing_slicing,
    reason = "GitLab URL segments are length-checked against the dash marker before slicing."
)]
fn gitlab_raw_url(parsed: &Url, host: &str) -> Option<String> {
    let parts = path_parts(parsed.path());
    let dash = parts.iter().position(|part| part == "-")?;
    if parts.len() <= dash + 3 || !matches!(parts[dash + 1].as_str(), "blob" | "raw") {
        return None;
    }
    let file_path = parts[(dash + 3)..].join("/");
    Some(format!(
        "https://{host}/{}/-/raw/{}/{file_path}",
        parts[..dash].join("/"),
        parts[dash + 2]
    ))
}
#[expect(
    clippy::indexing_slicing,
    reason = "Bitbucket URL segments are length-checked before fixed-position access."
)]
fn bitbucket_raw_url(parsed: &Url, host: &str) -> Option<String> {
    let parts = path_parts(parsed.path());
    if parts.len() < 5 || !matches!(parts[2].as_str(), "src" | "raw") {
        return None;
    }
    let file_path = parts[4..].join("/");
    Some(format!(
        "https://{host}/{}/{}/raw/{}/{}",
        parts[0], parts[1], parts[3], file_path
    ))
}
fn huggingface_marker_index(parts: &[String]) -> Option<usize> {
    let minimum = if matches!(
        parts.first().map(String::as_str),
        Some("datasets" | "spaces")
    ) {
        2
    } else {
        1
    };
    parts
        .iter()
        .enumerate()
        .find(|&(index, part)| {
            index >= minimum && matches!(part.as_str(), "blob" | "raw" | "resolve")
        })
        .map(|(index, _)| index)
}
fn path_parts(path: &str) -> Vec<String> {
    path.split('/')
        .filter(|part| !part.is_empty())
        .map(str::to_owned)
        .collect()
}
fn contains_host(hosts: &[String], host: &str) -> bool {
    hosts
        .iter()
        .any(|configured| configured.eq_ignore_ascii_case(host))
}
