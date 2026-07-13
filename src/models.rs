use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(rename_all = "kebab-case")]
#[expect(
    clippy::exhaustive_enums,
    reason = "Search categories are the closed set accepted by the upstream API."
)]
pub enum SearchCategory {
    Company,
    #[serde(rename = "research paper")]
    ResearchPaper,
    News,
    Pdf,
    #[serde(rename = "personal site")]
    PersonalSite,
    #[serde(rename = "financial report")]
    FinancialReport,
    People,
}
impl SearchCategory {
    #[expect(
        clippy::pattern_type_mismatch,
        reason = "Matching borrowed enum variants avoids copying the public model value."
    )]
    #[inline]
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Company => "company",
            Self::ResearchPaper => "research paper",
            Self::News => "news",
            Self::Pdf => "pdf",
            Self::PersonalSite => "personal site",
            Self::FinancialReport => "financial report",
            Self::People => "people",
        }
    }
}
#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
pub struct SearchQueryRequest {
    pub q: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recency: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domains: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<SearchCategory>,
}
#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
pub struct SearchQueryArguments {
    pub requests: Vec<SearchQueryRequest>,
}
#[derive(Clone, Debug, JsonSchema, Serialize)]
#[schemars(deny_unknown_fields)]
pub struct SearchResult {
    pub title: Option<String>,
    pub date: Option<String>,
    pub url: String,
    pub highlight: String,
}
#[derive(Clone, Debug, JsonSchema, Serialize)]
#[schemars(deny_unknown_fields)]
pub struct SearchQueryResponse {
    pub results: Vec<SearchResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<Vec<String>>,
}
#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
pub struct OpenRequest {
    pub url: String,
    pub chunk: usize,
}
#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
pub struct OpenArguments {
    pub requests: Vec<OpenRequest>,
}
#[derive(Clone, Debug, JsonSchema, Serialize)]
#[schemars(deny_unknown_fields)]
pub struct OpenPage {
    pub chunk: usize,
    pub total_chunks: usize,
    pub content: String,
}
#[derive(Clone, Debug, JsonSchema, Serialize)]
#[serde(untagged)]
#[schemars(deny_unknown_fields)]
#[expect(
    clippy::exhaustive_enums,
    reason = "Open results have exactly one success or failure payload."
)]
pub enum OpenResult {
    Success { url: String, page: OpenPage },
    Failure { url: String, error: String },
}
#[derive(Clone, Debug, JsonSchema, Serialize)]
#[schemars(deny_unknown_fields)]
pub struct OpenResponse {
    pub results: Vec<OpenResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<Vec<String>>,
}
#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
pub struct FindRequest {
    pub url: String,
    #[schemars(description = "Regex are allowed")]
    pub pattern: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snippet_tokens: Option<usize>,
}
#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
pub struct FindArguments {
    pub requests: Vec<FindRequest>,
}
#[derive(Clone, Debug, JsonSchema, Serialize)]
#[schemars(deny_unknown_fields)]
pub struct FindMatch {
    pub chunk: usize,
    pub snippet: String,
}
#[derive(Clone, Debug, JsonSchema, Serialize)]
#[schemars(deny_unknown_fields)]
pub struct FindPage {
    pub total_chunks: usize,
    pub matches: Vec<FindMatch>,
}
#[derive(Clone, Debug, JsonSchema, Serialize)]
#[schemars(deny_unknown_fields)]
pub struct FindResponse {
    pub pages: Vec<FindPage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<Vec<String>>,
}
