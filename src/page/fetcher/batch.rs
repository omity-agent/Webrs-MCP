use super::{PageContent, PageFetcher, PageSource};
use crate::{Result, error::AppError, page::reader::ReaderCredentials};
use futures::future::join_all;
#[cfg(test)]
mod tests;
impl PageFetcher {
    #[expect(
        clippy::missing_inline_in_public_items,
        reason = "Batch page fetching coordinates direct HTTP attempts and optional remote reader calls."
    )]
    pub async fn fetch_many(
        &self,
        urls: &[String],
        credentials: Option<&ReaderCredentials>,
    ) -> Result<Vec<PageContent>> {
        self.fetch_many_partial(urls, credentials)
            .await
            .into_iter()
            .collect()
    }
    #[expect(
        clippy::missing_inline_in_public_items,
        reason = "Partial batch fetching coordinates independent async work and retains per-request errors."
    )]
    #[expect(
        clippy::pattern_type_mismatch,
        reason = "Matching borrowed reader credentials selects the TinyFish batch path without cloning API keys."
    )]
    pub async fn fetch_many_partial(
        &self,
        urls: &[String],
        credentials: Option<&ReaderCredentials>,
    ) -> Vec<Result<PageContent>> {
        match credentials {
            Some(reader_credentials @ ReaderCredentials::TinyFish(_)) => {
                self.fetch_many_tinyfish(urls, reader_credentials).await
            }
            None | Some(ReaderCredentials::Jina(_)) => {
                let fetches = urls.iter().map(|url| self.fetch(url, credentials));
                join_all(fetches).await
            }
        }
    }
    async fn fetch_many_tinyfish(
        &self,
        urls: &[String],
        credentials: &ReaderCredentials,
    ) -> Vec<Result<PageContent>> {
        let direct_pages = join_all(urls.iter().map(|url| self.fetch_direct(url))).await;
        let missing_urls = direct_pages
            .iter()
            .zip(urls)
            .filter(|entry| entry.0.as_ref().is_ok_and(Option::is_none))
            .map(|entry| entry.1.clone())
            .collect::<Vec<_>>();
        if missing_urls.is_empty() {
            return finish_direct_pages(direct_pages);
        }
        let reader_results = self
            .reader
            .read_markdown_many(&missing_urls, credentials)
            .await;
        merge_reader_pages(direct_pages, missing_urls, reader_results)
    }
}
fn merge_reader_pages(
    direct_pages: Vec<Result<Option<PageContent>>>,
    missing_urls: Vec<String>,
    reader_results: Result<Vec<Result<String>>>,
) -> Vec<Result<PageContent>> {
    let ordered_results = normalize_reader_results(missing_urls.len(), reader_results);
    let mut reader_pages = missing_urls
        .into_iter()
        .zip(ordered_results)
        .map(|(url, result)| {
            result.map(|markdown| PageContent {
                url,
                source: PageSource::Reader,
                markdown,
            })
        });
    direct_pages
        .into_iter()
        .map(|direct| match direct {
            Ok(Some(page)) => Ok(page),
            Ok(None) => reader_pages
                .next()
                .unwrap_or_else(|| Err(AppError::internal("page reader result was missing"))),
            Err(error) => Err(error),
        })
        .collect()
}
fn normalize_reader_results(
    expected: usize,
    reader_results: Result<Vec<Result<String>>>,
) -> Vec<Result<String>> {
    match reader_results {
        Ok(results) if results.len() == expected => results,
        Ok(_results) => {
            let error =
                AppError::internal("TinyFish batch response count did not match requested URLs");
            repeat_error(expected, &error)
        }
        Err(error) => repeat_error(expected, &error),
    }
}
fn finish_direct_pages(direct_pages: Vec<Result<Option<PageContent>>>) -> Vec<Result<PageContent>> {
    direct_pages
        .into_iter()
        .map(|result| {
            let page = result?;
            page.ok_or_else(|| AppError::internal("page fetch result was missing"))
        })
        .collect()
}
fn repeat_error<Item>(count: usize, error: &AppError) -> Vec<Result<Item>> {
    core::iter::repeat_with(|| Err(error.clone()))
        .take(count)
        .collect()
}
