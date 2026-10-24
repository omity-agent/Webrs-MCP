use super::navigation::{ChunkSelection, SplitCount};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
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
pub struct FindMatch<Selection = Option<ChunkSelection>> {
    #[serde(flatten)]
    pub selection: Selection,
    pub snippet: String,
}
#[derive(Clone, Debug, JsonSchema, Serialize)]
#[schemars(deny_unknown_fields)]
pub struct FindPage<Count = Option<SplitCount>, Selection = Option<ChunkSelection>> {
    #[serde(flatten)]
    pub count: Count,
    pub matches: Vec<FindMatch<Selection>>,
}
#[derive(Clone, Debug, JsonSchema, Serialize)]
#[schemars(deny_unknown_fields)]
pub struct FindResponse<Count = Option<SplitCount>, Selection = Option<ChunkSelection>> {
    pub pages: Vec<FindPage<Count, Selection>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<Vec<String>>,
}
