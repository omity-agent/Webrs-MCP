#![expect(
    clippy::pub_use,
    reason = "The facade exposes domain types without duplicating their definitions."
)]
pub mod content;
#[cfg(test)]
mod decoding_tests;
pub mod fetch;
pub mod sites;
#[cfg(test)]
mod source_tests;
pub mod target;
use crate::config::DirectFetchConfig;
pub(crate) use fetch::{SharedProbeFetch, fetch_direct_text, shared_probe_fetch};
use sites::{code_hosts, learn, mediawiki, package, stack_overflow};
pub use target::{DirectFetchTarget, ResponseFormat};
#[inline]
#[must_use]
pub fn resolve_direct_fetch_target(
    url: &str,
    config: &DirectFetchConfig,
) -> Option<DirectFetchTarget> {
    let parsed = url::Url::parse(url).ok()?;
    let host = parsed.host_str()?.to_ascii_lowercase();
    if let Some(markdown_url) = learn::markdown_url(&parsed, config) {
        return Some(DirectFetchTarget::markdown(url, markdown_url));
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
