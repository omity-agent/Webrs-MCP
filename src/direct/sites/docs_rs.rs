use crate::config::DirectFetchConfig;
use url::Url;
mod render;
mod route;
pub use render::format_rustdoc_json;
pub use route::resolve_rustdoc_json_url;
#[inline]
#[must_use]
pub fn is_docs_rs_host(host: &str, config: &DirectFetchConfig) -> bool {
    config
        .docs_rs_hosts
        .iter()
        .any(|configured| configured.eq_ignore_ascii_case(host))
}
#[inline]
#[must_use]
pub fn docs_rs_url_with_path(parsed: &Url, path: &str) -> String {
    let mut json_url = parsed.clone();
    json_url.set_path(path);
    json_url.set_query(None);
    json_url.set_fragment(None);
    json_url.to_string()
}
