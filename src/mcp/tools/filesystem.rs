use super::{
    ToolOutput, ToolService,
    identity::{reader_credentials, required_api_key},
};
use crate::{
    Result,
    error::AppError,
    models::{FilesystemResponse, FilesystemSearchArguments},
    page::{PageContent, reader::ReaderCredentials},
};
use axum::http::HeaderMap;
use futures::future::join_all;
use sonic_rs::Value;
use std::path::PathBuf;
use storage::Invocation;
mod arguments;
mod storage;
#[cfg(test)]
mod tests;
struct PageTarget {
    url: String,
    directory: PathBuf,
    label: String,
}
impl ToolService {
    pub(super) async fn filesystem_call(
        &self,
        name: &str,
        arguments: Option<Value>,
        headers: &HeaderMap,
    ) -> ToolOutput {
        let result = match name {
            "search_query" => self.filesystem_search(arguments, headers).await,
            "open" => self.filesystem_open(arguments, headers).await,
            other => Err(AppError::client(format!("Unknown tool: {other}"))),
        };
        ToolOutput::Filesystem(result.unwrap_or_else(|error| failure_response(&error)))
    }
    async fn filesystem_search(
        &self,
        raw: Option<Value>,
        headers: &HeaderMap,
    ) -> Result<FilesystemResponse> {
        let arguments = arguments::search(raw)?;
        let invocation = Invocation::create(&arguments.output_path).await?;
        let result = self
            .search_into_invocation(arguments, headers, &invocation)
            .await;
        Ok(invocation.response(result.err().map(|error| error.client_message())))
    }
    async fn search_into_invocation(
        &self,
        arguments: FilesystemSearchArguments,
        headers: &HeaderMap,
        invocation: &Invocation,
    ) -> Result<()> {
        let request_directories = (0..arguments.requests.len())
            .map(|index| PathBuf::from(index.to_string()))
            .collect::<Vec<_>>();
        invocation.prepare(request_directories.iter()).await?;
        let api_key = required_api_key(
            headers,
            &self.config.headers.exa_api_key,
            self.credentials.exa_api_key.as_deref(),
        )?;
        let groups = self
            .search
            .search_urls_many(&arguments.requests, &api_key)
            .await?;
        let targets = groups
            .into_iter()
            .enumerate()
            .flat_map(|(request_index, urls)| {
                urls.into_iter()
                    .enumerate()
                    .map(move |(result_index, url)| PageTarget {
                        url,
                        directory: PathBuf::from(request_index.to_string())
                            .join(result_index.to_string()),
                        label: format!("requests[{request_index}].results[{result_index}]"),
                    })
            })
            .collect::<Vec<_>>();
        let credentials = reader_credentials(
            headers,
            &self.config.headers,
            self.credentials.reader.clone(),
        )?;
        materialize(self, invocation, targets, credentials.as_ref()).await
    }
    async fn filesystem_open(
        &self,
        raw: Option<Value>,
        headers: &HeaderMap,
    ) -> Result<FilesystemResponse> {
        let arguments = arguments::open(raw)?;
        let invocation = Invocation::create(&arguments.output_path).await?;
        let credentials = reader_credentials(
            headers,
            &self.config.headers,
            self.credentials.reader.clone(),
        )?;
        let targets = arguments
            .urls
            .into_iter()
            .enumerate()
            .map(|(index, url)| PageTarget {
                url,
                directory: PathBuf::from(index.to_string()),
                label: format!("urls[{index}]"),
            })
            .collect();
        let errors = materialize(self, &invocation, targets, credentials.as_ref())
            .await
            .err()
            .map(|error| error.client_message());
        Ok(invocation.response(errors))
    }
}
async fn materialize(
    service: &ToolService,
    invocation: &Invocation,
    targets: Vec<PageTarget>,
    credentials: Option<&ReaderCredentials>,
) -> Result<()> {
    invocation
        .prepare(targets.iter().map(|target| &target.directory))
        .await?;
    let pending = targets
        .iter()
        .enumerate()
        .filter(|entry| !invocation.is_cached(&entry.1.url))
        .map(|entry| (entry.0, entry.1.url.clone()))
        .collect::<Vec<_>>();
    let urls = pending
        .iter()
        .map(|entry| entry.1.clone())
        .collect::<Vec<_>>();
    let fetched = service
        .page_fetcher
        .fetch_many_partial(&urls, credentials)
        .await;
    let mut pages: Vec<Option<Result<PageContent>>> = vec![None; targets.len()];
    for (entry, fetched_page) in pending.into_iter().zip(fetched) {
        let slot = pages
            .get_mut(entry.0)
            .ok_or_else(|| AppError::internal("pending page index was out of bounds"))?;
        *slot = Some(fetched_page);
    }
    let writes = targets.iter().zip(pages).map(|(target, page)| async move {
        let result = match page {
            Some(Ok(fetched_page)) => {
                invocation
                    .write_page(&target.directory, &target.url, &fetched_page)
                    .await
            }
            Some(Err(error)) => Err(error),
            None => invocation.copy_cached(&target.directory, &target.url).await,
        };
        result.map_err(|error| format!("{}: {}", target.label, error.client_message()))
    });
    let errors = join_all(writes)
        .await
        .into_iter()
        .filter_map(core::result::Result::err)
        .collect::<Vec<_>>();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(AppError::client(errors.join("\n")))
    }
}
fn failure_response(error: &AppError) -> FilesystemResponse {
    FilesystemResponse {
        output_path: String::new(),
        error: error.client_message(),
        warning: String::new(),
    }
}
