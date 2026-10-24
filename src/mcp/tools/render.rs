use crate::{
    Result,
    cli::oneshot::OutputFormat,
    error::AppError,
    models::{
        FilesystemResponse, FindMatch, FindPage, FindResponse, OpenPage, OpenResponse, OpenResult,
        SearchQueryResponse, SearchResult,
    },
};
mod writer;
use writer::PseudoXml;
#[derive(serde :: Serialize)]
#[serde(untagged)]
pub(crate) enum ToolOutput {
    Search(SearchQueryResponse),
    Open(OpenResponse),
    Find(FindResponse),
    Filesystem(FilesystemResponse),
}
impl ToolOutput {
    pub(crate) fn structured(&self) -> Result<rmcp::serde_json::Value> {
        rmcp::serde_json::to_value(self)
            .map_err(|error| AppError::internal(format!("failed to encode tool response: {error}")))
    }
    pub(crate) fn formatted(&self, format: OutputFormat) -> Result<String> {
        match format {
            OutputFormat::Xml => Ok(self.standard_text()),
            OutputFormat::Json => sonic_rs::to_string(self).map_err(|error| {
                AppError::internal(format!("failed to encode tool response: {error}"))
            }),
        }
    }
    #[expect(
        clippy::pattern_type_mismatch,
        reason = "Borrowing output variants keeps failure messages in their response models."
    )]
    pub(crate) fn failure(&self) -> Option<&str> {
        match self {
            Self::Open(response) => response.results.iter().find_map(|result| match result {
                OpenResult::Failure { url: _, error } => Some(error.as_str()),
                OpenResult::Success { url: _, page: _ } => None,
            }),
            Self::Filesystem(response) => {
                (!response.error.is_empty()).then_some(response.error.as_str())
            }
            Self::Search(_) | Self::Find(_) => None,
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
            Self::Filesystem(response) => filesystem_text(response),
        }
    }
}
fn filesystem_text(response: &FilesystemResponse) -> String {
    let mut xml = PseudoXml::new();
    xml.text("LOCATION", &response.output_path);
    xml.text("ERROR", &response.error);
    xml.text("WARNING", &response.warning);
    xml.finish()
}
fn search_text(response: &SearchQueryResponse) -> String {
    let mut xml = PseudoXml::new();
    for group in &response.groups {
        xml.indexed(group, search_result);
    }
    warnings(&mut xml, response.warning.as_deref());
    xml.finish()
}
fn search_result(xml: &mut PseudoXml, result: &SearchResult) {
    if let Some(title) = result.title.as_deref() {
        xml.text("TITLE", title);
    }
    if let Some(date) = result.date.as_deref() {
        xml.text("DATE", date);
    }
    xml.text("URL", &result.url);
    xml.text("HIGHLIGHT", &result.highlight);
}
fn open_text(response: &OpenResponse) -> String {
    let mut xml = PseudoXml::new();
    xml.indexed(&response.results, open_result);
    warnings(&mut xml, response.warning.as_deref());
    xml.finish()
}
#[expect(
    clippy::pattern_type_mismatch,
    reason = "Matching borrowed open results avoids cloning page and error payloads."
)]
fn open_result(xml: &mut PseudoXml, result: &OpenResult) {
    match result {
        OpenResult::Success { url, page } => {
            xml.text("URL", url);
            open_page(xml, page);
        }
        OpenResult::Failure { url, error } => {
            xml.text("URL", url);
            xml.text("ERROR", error);
        }
    }
}
fn open_page(xml: &mut PseudoXml, page: &OpenPage) {
    xml.open("PAGE");
    if let Some(partition) = page.partition {
        xml.number("CHUNK", partition.chunk);
        xml.number("TOTAL_CHUNKS", partition.total_chunks);
    }
    xml.text("CONTENT", &page.content);
    xml.close("PAGE");
}
fn find_text(response: &FindResponse) -> String {
    let mut xml = PseudoXml::new();
    xml.indexed(&response.pages, find_page);
    warnings(&mut xml, response.warning.as_deref());
    xml.finish()
}
fn find_page(xml: &mut PseudoXml, page: &FindPage) {
    if let Some(count) = page.count {
        xml.number("TOTAL_CHUNKS", count.total_chunks);
    }
    if page.matches.is_empty() {
        xml.empty("MATCHES");
    } else {
        xml.open("MATCHES");
        xml.indexed(&page.matches, find_match);
        xml.close("MATCHES");
    }
}
fn find_match(xml: &mut PseudoXml, found: &FindMatch) {
    if let Some(selection) = found.selection {
        xml.number("CHUNK", selection.chunk);
    }
    xml.text("SNIPPET", &found.snippet);
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
