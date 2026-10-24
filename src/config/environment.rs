use crate::{Result, error::AppError};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EnvironmentConfig {
    pub exa: String,
    pub octen: String,
    pub jina: String,
    pub tinyfish: String,
}
impl EnvironmentConfig {
    pub(super) fn validate(&self) -> Result<()> {
        let entries = [
            ("environment.exa", self.exa.as_str()),
            ("environment.octen", self.octen.as_str()),
            ("environment.jina", self.jina.as_str()),
            ("environment.tinyfish", self.tinyfish.as_str()),
        ];
        let mut seen = std::collections::HashSet::new();
        for (field, name) in entries {
            if !name
                .bytes()
                .next()
                .is_some_and(|byte| byte.is_ascii_alphabetic() || byte == b'_')
                || !name
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
            {
                return Err(AppError::config(format!(
                    "{field} must be a portable environment variable name."
                )));
            }
            if !seen.insert(name.to_ascii_uppercase()) {
                return Err(AppError::config(format!(
                    "{field} duplicates another API key environment variable: {name}"
                )));
            }
        }
        Ok(())
    }
}
