use rmcp::model::CacheScope;
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProtocolConfig {
    pub tools_list_cache: ToolListCacheConfig,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ToolListCacheConfig {
    pub ttl_ms: u64,
    pub scope: CacheScope,
}
