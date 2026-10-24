use crate::{
    Result, config::EnvironmentConfig, error::AppError, mcp::tools::ToolCredentials,
    page::reader::ReaderCredentials, search::SearchCredentials,
};
pub(super) fn credentials(config: &EnvironmentConfig) -> Result<ToolCredentials> {
    let jina = api_key(&config.jina)?;
    let tinyfish = api_key(&config.tinyfish)?;
    let reader = match (jina, tinyfish) {
        (Some(_jina), Some(_tinyfish)) => {
            return Err(AppError::config(format!(
                "Set at most one remote reader API key environment variable: {} or {}.",
                config.jina, config.tinyfish
            )));
        }
        (Some(key), None) => Some(ReaderCredentials::Jina(key)),
        (None, Some(key)) => Some(ReaderCredentials::TinyFish(key)),
        (None, None) => None,
    };
    Ok(ToolCredentials {
        search: search_credentials(config)?,
        reader,
    })
}
fn search_credentials(config: &EnvironmentConfig) -> Result<Option<SearchCredentials>> {
    let exa = api_key(&config.exa)?;
    let octen = api_key(&config.octen)?;
    match (exa, octen) {
        (Some(_exa), Some(_octen)) => Err(AppError::config(format!(
            "Set at most one search API key: {} or {}.",
            config.exa, config.octen
        ))),
        (Some(key), None) => Ok(Some(SearchCredentials::Exa(key))),
        (None, Some(key)) => Ok(Some(SearchCredentials::Octen(key))),
        (None, None) => Ok(None),
    }
}
fn api_key(name: &str) -> Result<Option<String>> {
    match std::env::var(name) {
        Ok(key) if key.trim().is_empty() => Err(AppError::config(format!(
            "API key {name} must not be empty."
        ))),
        Ok(key) => Ok(Some(key)),
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(std::env::VarError::NotUnicode(_)) => Err(AppError::config(format!(
            "API key {name} is not valid Unicode."
        ))),
    }
}
