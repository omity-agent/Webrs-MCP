use super::validation;
use crate::{Result, error::AppError};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SearchConfig {
    pub num_results: u32,
    pub exa: ExaConfig,
    pub octen: OctenConfig,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExaConfig {
    pub endpoint: String,
    #[serde(rename = "type")]
    pub search_type: String,
    pub highlights_max_characters: u32,
    pub max_age_hours: u64,
    pub livecrawl_timeout: u32,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OctenConfig {
    pub endpoint: String,
    pub highlights_max_tokens: u32,
    pub format: String,
    pub safesearch: String,
}
impl SearchConfig {
    pub(super) fn validate(&self) -> Result<()> {
        if !(1..=100).contains(&self.num_results) {
            return Err(AppError::config(
                "search.num_results must be between 1 and 100",
            ));
        }
        validation::endpoint(&self.exa.endpoint, "search.exa.endpoint")?;
        validation::positive(
            &self.exa.highlights_max_characters,
            "search.exa.highlights_max_characters",
        )?;
        validation::positive(&self.exa.livecrawl_timeout, "search.exa.livecrawl_timeout")?;
        validation::endpoint(&self.octen.endpoint, "search.octen.endpoint")?;
        if !(100..=20_000).contains(&self.octen.highlights_max_tokens) {
            return Err(AppError::config(
                "search.octen.highlights_max_tokens must be between 100 and 20000",
            ));
        }
        if !matches!(self.octen.format.as_str(), "text" | "markdown") {
            return Err(AppError::config(
                "search.octen.format must be text or markdown",
            ));
        }
        if !matches!(self.octen.safesearch.as_str(), "strict" | "off") {
            return Err(AppError::config(
                "search.octen.safesearch must be strict or off",
            ));
        }
        Ok(())
    }
}
