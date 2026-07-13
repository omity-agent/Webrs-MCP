use crate::{
    Result,
    error::AppError,
    models::{
        FindMatch, FindResponse, OpenPage, OpenResponse, OpenResult, SearchQueryResponse,
        SearchResult,
    },
};
use sonic_rs::Value;
#[cfg(test)]
mod tests;
mod writer;
use writer::PseudoXml;
pub(crate) enum ToolOutput {
    Search(SearchQueryResponse),
    Open(OpenResponse),
    Find(FindResponse),
}
impl ToolOutput {
    #[expect(
        clippy::pattern_type_mismatch,
        reason = "Matching borrowed output variants avoids cloning response models."
    )]
    pub(crate) fn structured(&self) -> Result<Value> {
        match self {
            Self::Search(response) => to_value(response),
            Self::Open(response) => to_value(response),
            Self::Find(response) => to_value(response),
        }
    }
    #[must_use]
    #[expect(
        clippy::pattern_type_mismatch,
        reason = "Matching borrowed output variants avoids cloning response models."
    )]
    pub(crate) fn standard_text(&self) -> String {
        match self {
            Self::Search(response) => search_text(response),
            Self::Open(response) => open_text(response),
            Self::Find(response) => find_text(response),
        }
    }
}
fn to_value<T>(value: &T) -> Result<Value>
where
    T: serde::Serialize,
{
    sonic_rs::to_value(value)
        .map_err(|error| AppError::internal(format!("failed to encode tool response: {error}")))
}
fn search_text(response: &SearchQueryResponse) -> String {
    let mut xml = PseudoXml::new();
    if response.results.is_empty() {
        xml.empty("RESULTS");
    } else {
        xml.open("RESULTS");
        for result in &response.results {
            search_result(&mut xml, result);
        }
        xml.close("RESULTS");
    }
    warnings(&mut xml, response.warning.as_deref());
    xml.finish()
}
fn search_result(xml: &mut PseudoXml, result: &SearchResult) {
    xml.open("RESULT");
    if let Some(title) = result.title.as_deref() {
        xml.text("TITLE", title);
    }
    if let Some(date) = result.date.as_deref() {
        xml.text("DATE", date);
    }
    xml.text("URL", &result.url);
    xml.text("HIGHLIGHT", &result.highlight);
    xml.close("RESULT");
}
#[expect(
    clippy::pattern_type_mismatch,
    reason = "Matching borrowed open results avoids cloning page and error payloads."
)]
fn open_text(response: &OpenResponse) -> String {
    let mut xml = PseudoXml::new();
    if response.results.is_empty() {
        xml.empty("RESULTS");
    } else {
        xml.open("RESULTS");
        for result in &response.results {
            xml.open("RESULT");
            match result {
                OpenResult::Success { url, page } => {
                    xml.text("URL", url);
                    open_page(&mut xml, page);
                }
                OpenResult::Failure { url, error } => {
                    xml.text("URL", url);
                    xml.text("ERROR", error);
                }
            }
            xml.close("RESULT");
        }
        xml.close("RESULTS");
    }
    warnings(&mut xml, response.warning.as_deref());
    xml.finish()
}
fn open_page(xml: &mut PseudoXml, page: &OpenPage) {
    xml.open("PAGE");
    xml.number("CHUNK", page.chunk);
    xml.number("TOTAL_CHUNKS", page.total_chunks);
    xml.text("CONTENT", &page.content);
    xml.close("PAGE");
}
fn find_text(response: &FindResponse) -> String {
    let mut xml = PseudoXml::new();
    if response.pages.is_empty() {
        xml.empty("PAGES");
    } else {
        xml.open("PAGES");
        for page in &response.pages {
            xml.open("PAGE");
            xml.number("TOTAL_CHUNKS", page.total_chunks);
            if page.matches.is_empty() {
                xml.empty("MATCHES");
            } else {
                xml.open("MATCHES");
                for found in &page.matches {
                    find_match(&mut xml, found);
                }
                xml.close("MATCHES");
            }
            xml.close("PAGE");
        }
        xml.close("PAGES");
    }
    warnings(&mut xml, response.warning.as_deref());
    xml.finish()
}
fn find_match(xml: &mut PseudoXml, found: &FindMatch) {
    xml.open("MATCH");
    xml.number("CHUNK", found.chunk);
    xml.text("SNIPPET", &found.snippet);
    xml.close("MATCH");
}
fn warnings(xml: &mut PseudoXml, warning_values: Option<&[String]>) {
    let Some(values) = warning_values else {
        return;
    };
    if values.is_empty() {
        xml.empty("WARNINGS");
        return;
    }
    xml.open("WARNINGS");
    for warning in values {
        xml.text("WARNING", warning);
    }
    xml.close("WARNINGS");
}
