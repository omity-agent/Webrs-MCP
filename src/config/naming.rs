use crate::{Result, error::AppError};
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ToolKind {
    Search,
    Open,
    Find,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ToolNames {
    pub search_query: String,
    pub open: String,
    pub find: String,
}
impl ToolNames {
    #[inline]
    #[must_use]
    pub fn name(&self, kind: ToolKind) -> &str {
        match kind {
            ToolKind::Search => &self.search_query,
            ToolKind::Open => &self.open,
            ToolKind::Find => &self.find,
        }
    }
    #[inline]
    #[must_use]
    pub fn resolve(&self, name: &str) -> Option<ToolKind> {
        [ToolKind::Search, ToolKind::Open, ToolKind::Find]
            .into_iter()
            .find(|kind| self.name(*kind) == name)
    }
    pub(super) fn validate(&self) -> Result<()> {
        let entries = [
            ("tools.search_query", self.search_query.as_str()),
            ("tools.open", self.open.as_str()),
            ("tools.find", self.find.as_str()),
        ];
        let mut seen = std::collections::HashSet::new();
        for (field, name) in entries {
            if !name
                .bytes()
                .next()
                .is_some_and(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
                || !name
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
                || name == "help"
            {
                return Err(AppError::config(format!(
                    "{field} must start with an ASCII letter, digit, or underscore, contain only ASCII letters, digits, underscores, hyphens, or dots, and not be 'help'"
                )));
            }
            if name == "mcp" && field != "tools.find" {
                return Err(AppError::config(format!(
                    "{field} cannot be 'mcp', which is reserved for the MCP server command."
                )));
            }
            if !seen.insert(name) {
                return Err(AppError::config(format!(
                    "{field} duplicates another tool name: {name}"
                )));
            }
        }
        Ok(())
    }
}
