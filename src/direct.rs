pub mod code_hosts;
pub mod content;
pub mod fetch;
pub mod mediawiki;
pub mod package;
pub mod stack_overflow;
pub mod target;
#[cfg(test)]
mod tests;
use crate::config::DirectFetchConfig;
pub type DirectFetchTarget = target::DirectFetchTarget;
pub type ResponseFormat = target::ResponseFormat;
pub type SharedProbeFetch = fetch::SharedProbeFetch;
#[inline]
#[must_use]
pub fn resolve_direct_fetch_target(
    url: &str,
    config: &DirectFetchConfig,
) -> Option<DirectFetchTarget> {
    let parsed = url::Url::parse(url).ok()?;
    let host = parsed.host_str()?.to_ascii_lowercase();
    if contains(&config.microsoft_learn_hosts, &host) {
        return Some(DirectFetchTarget::text(
            url,
            microsoft_learn_markdown_url(&parsed),
        ));
    }
    if let Some(raw_url) = code_hosts::resolve_code_host_raw_url(&parsed, &host, config) {
        return Some(DirectFetchTarget::text(url, raw_url));
    }
    if let Some(api_url) = stack_overflow::resolve_stack_overflow_api_url(&parsed, config) {
        return Some(DirectFetchTarget::stack_overflow_question(url, api_url));
    }
    if let Some(registry) = package::resolve_package_registry_target(&parsed, config) {
        return Some(DirectFetchTarget::package(
            url,
            registry.request_url,
            registry.json_fields_first,
            registry.json_fields_last,
        ));
    }
    mediawiki::resolve_mediawiki_api_url(&parsed, config)
        .map(|request_url| DirectFetchTarget::mediawiki(url, request_url))
}
#[expect(
    clippy::missing_inline_in_public_items,
    reason = "The async direct fetch facade performs HTTP I/O and is not an inline candidate."
)]
pub async fn fetch_direct_text(
    client: &crate::net::SecureHttpClient,
    target: &DirectFetchTarget,
    direct_config: &crate::config::DirectFetchConfig,
    http_config: &crate::config::HttpConfig,
) -> crate::Result<String> {
    fetch::fetch_direct_text(client, target, direct_config, http_config).await
}
#[expect(
    clippy::missing_inline_in_public_items,
    reason = "The async direct fetch facade performs HTTP I/O and is not an inline candidate."
)]
pub async fn fetch_direct_text_with_probe(
    client: &crate::net::SecureHttpClient,
    target: &DirectFetchTarget,
    direct_config: &crate::config::DirectFetchConfig,
    http_config: &crate::config::HttpConfig,
    probe_fetch: SharedProbeFetch,
) -> crate::Result<String> {
    fetch::fetch_direct_text_with_probe(client, target, direct_config, http_config, probe_fetch)
        .await
}
#[inline]
pub fn shared_probe_fetch(
    client: crate::net::SecureHttpClient,
    probe_url: String,
    target: &DirectFetchTarget,
    direct_config: &crate::config::DirectFetchConfig,
    http_config: &crate::config::HttpConfig,
) -> crate::Result<SharedProbeFetch> {
    fetch::shared_probe_fetch(client, probe_url, target, direct_config, http_config)
}
fn microsoft_learn_markdown_url(parsed: &url::Url) -> String {
    let mut pairs: Vec<(String, String)> = parsed
        .query_pairs()
        .filter(|pair| !pair.0.as_ref().eq_ignore_ascii_case("accept"))
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect();
    pairs.push(("accept".to_owned(), "text/markdown".to_owned()));
    let mut next = parsed.clone();
    next.query_pairs_mut().clear().extend_pairs(pairs);
    next.to_string()
}
fn contains(values: &[String], value: &str) -> bool {
    values
        .iter()
        .any(|configured| configured.eq_ignore_ascii_case(value))
}
