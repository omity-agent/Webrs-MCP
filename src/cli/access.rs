use super::Cli;
use crate::{
    Result, error::AppError, mcp::tools::ToolCredentials, page::reader::ReaderCredentials,
    search::SearchCredentials,
};
pub(super) fn credentials(cli: &Cli) -> Result<ToolCredentials> {
    let reader = match (cli.jina.as_deref(), cli.tinyfish.as_deref()) {
        (Some(_jina), Some(_tinyfish)) => {
            return Err(AppError::config(
                "Provide at most one remote reader API key: --jina-api-key or --tinyfish-api-key.",
            ));
        }
        (Some(key), None) => Some(ReaderCredentials::Jina(key.to_owned())),
        (None, Some(key)) => Some(ReaderCredentials::TinyFish(key.to_owned())),
        (None, None) => None,
    };
    if cli
        .jina
        .as_deref()
        .or(cli.tinyfish.as_deref())
        .is_some_and(|key| key.trim().is_empty())
    {
        return Err(AppError::config("Remote reader API key must not be empty."));
    }
    Ok(ToolCredentials {
        search: search_credentials(cli)?,
        reader,
    })
}
fn search_credentials(cli: &Cli) -> Result<Option<SearchCredentials>> {
    let selected = match (cli.exa.as_deref(), cli.octen.as_deref()) {
        (Some(_exa), Some(_octen)) => {
            return Err(AppError::config(
                "Provide at most one search API key: --exa-api-key or --octen-api-key, not both.",
            ));
        }
        (Some(key), None) => Some(SearchCredentials::Exa(key.to_owned())),
        (None, Some(key)) => Some(SearchCredentials::Octen(key.to_owned())),
        (None, None) => None,
    };
    if selected
        .as_ref()
        .is_some_and(|credentials| credentials.api_key().trim().is_empty())
    {
        return Err(AppError::config("Search API key must not be empty."));
    }
    Ok(selected)
}
