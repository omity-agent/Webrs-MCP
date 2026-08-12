use crate::{Result, config::DirectFetchConfig, error::AppError};
use serde::{Deserialize, Deserializer, de::IgnoredAny};
use url::Url;
#[derive(Deserialize)]
struct MediaWikiResponse {
    error: Option<MediaWikiApiError>,
    query: Option<MediaWikiQuery>,
}
#[derive(Deserialize)]
struct MediaWikiApiError {
    code: Option<String>,
}
#[derive(Deserialize)]
struct MediaWikiQuery {
    #[serde(default, deserialize_with = "presence")]
    badrevids: bool,
    pages: Option<Vec<MediaWikiPage>>,
}
#[derive(Deserialize)]
struct MediaWikiPage {
    #[serde(default, deserialize_with = "presence")]
    missing: bool,
    revisions: Option<Vec<MediaWikiRevision>>,
}
#[derive(Deserialize)]
struct MediaWikiRevision {
    slots: Option<MediaWikiSlots>,
}
#[derive(Deserialize)]
struct MediaWikiSlots {
    main: Option<MediaWikiMainSlot>,
}
#[derive(Deserialize)]
struct MediaWikiMainSlot {
    content: Option<String>,
}
#[must_use]
#[inline]
pub fn resolve_mediawiki_api_url(parsed: &Url, config: &DirectFetchConfig) -> Option<String> {
    let host = parsed.host_str()?.to_ascii_lowercase();
    let (selector, api_path) = if is_wikimedia_host(&host, &config.wikimedia_domains) {
        (
            wikimedia_selector(parsed)?,
            config.wikimedia_api_path.clone(),
        )
    } else if is_fandom_host(&host) {
        fandom_selector_and_api_path(parsed)?
    } else {
        return None;
    };
    let mut api = Url::parse(&format!("https://{host}{api_path}")).ok()?;
    append_api_parameters(&mut api, &selector);
    Some(api.to_string())
}
#[inline]
pub fn extract_mediawiki_content(body: &[u8]) -> Result<String> {
    let response: MediaWikiResponse = sonic_rs::from_slice(body)
        .map_err(|_error| AppError::client("MediaWiki API returned an invalid response object."))?;
    if let Some(error) = response.error {
        let message = error.code.map_or_else(
            || "MediaWiki API rejected the page request.".to_owned(),
            |code| format!("MediaWiki API rejected the page request ({code})."),
        );
        return Err(AppError::client(message));
    }
    let query = response
        .query
        .ok_or_else(|| AppError::client("MediaWiki API response is missing query results."))?;
    if query.badrevids {
        return Err(AppError::client("MediaWiki revision was not found."));
    }
    let pages = query
        .pages
        .ok_or_else(|| AppError::client("MediaWiki API did not return exactly one page."))?;
    let [page]: [MediaWikiPage; 1] = pages
        .try_into()
        .map_err(|_pages| AppError::client("MediaWiki API did not return exactly one page."))?;
    extract_page_content(page)
}
fn extract_page_content(page: MediaWikiPage) -> Result<String> {
    if page.missing {
        return Err(AppError::client("MediaWiki page was not found."));
    }
    let revisions = page
        .revisions
        .ok_or_else(|| AppError::client("MediaWiki API response is missing page revisions."))?;
    let [revision]: [MediaWikiRevision; 1] = revisions.try_into().map_err(|_revisions| {
        AppError::client("MediaWiki API response is missing page revisions.")
    })?;
    revision
        .slots
        .and_then(|slots| slots.main)
        .and_then(|main| main.content)
        .map(|content| content.replace("\r\n", "\n"))
        .ok_or_else(|| AppError::client("MediaWiki API response is missing page content."))
}
fn presence<'de, D>(deserializer: D) -> core::result::Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    IgnoredAny::deserialize(deserializer).map(|_ignored| true)
}
fn wikimedia_selector(parsed: &Url) -> Option<(String, String)> {
    if let Some(title) = parsed.path().strip_prefix("/wiki/") {
        return page_selector(parsed, Some(percent_decode(title)));
    }
    if parsed.path() == "/w/index.php" {
        return page_selector(parsed, Some(first_query_value(parsed, "title")?));
    }
    None
}
fn fandom_selector_and_api_path(parsed: &Url) -> Option<((String, String), String)> {
    let (prefix, title) = if let Some(article) = fandom_article_path(parsed.path()) {
        article
    } else {
        (
            fandom_index_prefix(parsed.path())?,
            first_query_value(parsed, "title")?,
        )
    };
    Some((
        page_selector(parsed, Some(title))?,
        format!("{prefix}/api.php"),
    ))
}
fn page_selector(parsed: &Url, title: Option<String>) -> Option<(String, String)> {
    if let Some(oldid) = first_query_value(parsed, "oldid") {
        return Some(("revids".to_owned(), oldid));
    }
    if let Some(curid) = first_query_value(parsed, "curid") {
        return Some(("pageids".to_owned(), curid));
    }
    title.map(|value| ("titles".to_owned(), value))
}
fn append_api_parameters(api: &mut Url, selector: &(String, String)) {
    let mut pairs = api.query_pairs_mut();
    pairs
        .append_pair("action", "query")
        .append_pair("prop", "revisions")
        .append_pair("rvprop", "content")
        .append_pair("rvslots", "main")
        .append_pair(&selector.0, &selector.1);
    if selector.0 == "titles" {
        pairs.append_pair("redirects", "1");
    }
    pairs
        .append_pair("format", "json")
        .append_pair("formatversion", "2");
}
fn fandom_article_path(path: &str) -> Option<(String, String)> {
    let parts: Vec<&str> = path.split('/').collect();
    if parts.get(1).is_some_and(|part| *part == "wiki") {
        let title = parts.get(2..)?;
        if !title.is_empty() {
            return Some((String::new(), percent_decode(&title.join("/"))));
        }
    }
    if parts.get(2).is_some_and(|part| *part == "wiki") {
        let prefix = parts.get(1)?;
        let title = parts.get(3..)?;
        if !title.is_empty() {
            return Some((format!("/{prefix}"), percent_decode(&title.join("/"))));
        }
    }
    None
}
fn fandom_index_prefix(path: &str) -> Option<String> {
    let parts: Vec<&str> = path.split('/').collect();
    if parts.as_slice() == ["", "index.php"] || parts.as_slice() == ["", "w", "index.php"] {
        return Some(String::new());
    }
    if parts.len() == 3 && parts.get(2).is_some_and(|part| *part == "index.php") {
        let prefix = parts.get(1)?;
        return Some(format!("/{prefix}"));
    }
    None
}
fn first_query_value(parsed: &Url, name: &str) -> Option<String> {
    parsed
        .query_pairs()
        .find(|pair| pair.0.as_ref() == name)
        .map(|(_, value)| value.into_owned())
}
fn is_wikimedia_host(host: &str, domains: &[String]) -> bool {
    domains
        .iter()
        .any(|domain| host == domain || host.ends_with(&format!(".{domain}")))
}
fn is_fandom_host(host: &str) -> bool {
    host != "www.fandom.com" && host.ends_with(".fandom.com")
}
fn percent_decode(value: &str) -> String {
    percent_encoding::percent_decode_str(value)
        .decode_utf8_lossy()
        .into_owned()
}
