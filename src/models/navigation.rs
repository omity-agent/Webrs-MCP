use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, Serialize)]
pub struct ChunkSelection {
    pub chunk: usize,
}
#[derive(Clone, Copy, Debug, JsonSchema, Serialize)]
pub struct PagePartition {
    pub chunk: usize,
    pub total_chunks: usize,
}
#[derive(Clone, Copy, Debug, JsonSchema, Serialize)]
pub struct SplitCount {
    pub total_chunks: usize,
}
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, Serialize)]
#[expect(
    clippy::empty_structs_with_brackets,
    reason = "Flattened unpartitioned contracts serialize as an empty object rather than null."
)]
pub struct Unpartitioned {}
#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
pub struct OpenRequest<Selection = Option<ChunkSelection>> {
    pub url: String,
    #[serde(flatten)]
    pub selection: Selection,
}
#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
pub struct OpenArguments<Selection = Option<ChunkSelection>> {
    pub requests: Vec<OpenRequest<Selection>>,
}
#[derive(Clone, Debug, JsonSchema, Serialize)]
#[schemars(deny_unknown_fields)]
pub struct OpenPage<Partition = Option<PagePartition>> {
    #[serde(flatten)]
    pub partition: Partition,
    pub content: String,
}
#[derive(Clone, Debug, JsonSchema, Serialize)]
#[serde(untagged)]
#[schemars(deny_unknown_fields)]
#[expect(
    clippy::exhaustive_enums,
    reason = "Open results have exactly one success or failure payload."
)]
pub enum OpenResult<Partition = Option<PagePartition>> {
    Success {
        url: String,
        page: OpenPage<Partition>,
    },
    Failure {
        url: String,
        error: String,
    },
}
#[derive(Clone, Debug, JsonSchema, Serialize)]
#[schemars(deny_unknown_fields)]
pub struct OpenResponse<Partition = Option<PagePartition>> {
    pub results: Vec<OpenResult<Partition>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<Vec<String>>,
}
