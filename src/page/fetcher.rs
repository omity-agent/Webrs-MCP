use crate::{
    Result,
    config::AppConfig,
    direct::{
        DirectFetchTarget, SharedProbeFetch, fetch_direct_text, fetch_direct_text_with_probe,
        shared_probe_fetch,
    },
    error::AppError,
    net::{SecureHttpClient, SsrfGuard, guard, secure_client_from_config},
    page::reader::{PageReader, ReaderCredentials},
};
use alloc::sync::Arc;
use futures::{StreamExt as _, stream::FuturesUnordered};
use std::collections::HashMap;
use targets::direct_fetch_targets;
use tracing::warn;
use url::Url;
mod batch;
mod targets;
#[cfg(test)]
mod tests;
#[derive(Clone, Debug)]
pub struct PageContent {
    pub url: String,
    pub source: PageSource,
    pub markdown: Arc<str>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[expect(
    clippy::exhaustive_enums,
    reason = "Page sources are closed because callers branch on the configured readers."
)]
pub enum PageSource {
    Direct,
    Reader,
}
#[derive(Clone)]
pub struct PageFetcher {
    config: AppConfig,
    http: SecureHttpClient,
    guard: SsrfGuard,
    reader: PageReader,
}
impl PageFetcher {
    #[inline]
    pub fn new(config: AppConfig) -> Result<Self> {
        let http = secure_client_from_config(&config)?;
        let guard = guard(&config.ssrf);
        let reader = PageReader::new(config.clone(), http.clone());
        Ok(Self {
            config,
            http,
            guard,
            reader,
        })
    }
    #[expect(
        clippy::missing_inline_in_public_items,
        reason = "Page fetching performs async network I/O and is not an inline candidate."
    )]
    pub async fn fetch(
        &self,
        url: &str,
        credentials: Option<&ReaderCredentials>,
    ) -> Result<PageContent> {
        if let Some(page) = self.fetch_direct(url).await? {
            return Ok(page);
        }
        let Some(reader_credentials) = credentials else {
            return Err(self.missing_reader_credentials_error());
        };
        let markdown = self.reader.read_markdown(url, reader_credentials).await?;
        Ok(PageContent {
            url: url.to_owned(),
            source: PageSource::Reader,
            markdown: markdown.into(),
        })
    }
    async fn fetch_direct(&self, url: &str) -> Result<Option<PageContent>> {
        let parsed =
            Url::parse(url).map_err(|error| AppError::client(format!("Invalid URL: {error}")))?;
        self.guard.validate_url(&parsed).await?;
        let targets = direct_fetch_targets(url, &self.config.direct_fetch);
        let probes = self.shared_probe_fetches(&targets)?;
        let mut attempts = targets
            .into_iter()
            .map(|target| {
                let request_url = target.request_url.clone();
                let probe_fetch = target
                    .similarity_probe_url
                    .as_ref()
                    .and_then(|probe_url| probes.get(probe_url).cloned());
                async move {
                    let result = if let Some(probe) = probe_fetch {
                        fetch_direct_text_with_probe(
                            &self.http,
                            &target,
                            &self.config.direct_fetch,
                            &self.config.http,
                            probe,
                        )
                        .await
                    } else {
                        fetch_direct_text(
                            &self.http,
                            &target,
                            &self.config.direct_fetch,
                            &self.config.http,
                        )
                        .await
                    };
                    (request_url, result)
                }
            })
            .collect::<FuturesUnordered<_>>();
        let mut errors = Vec::new();
        while let Some((request_url, result)) = attempts.next().await {
            match result {
                Ok(markdown) => {
                    return Ok(Some(PageContent {
                        url: url.to_owned(),
                        source: PageSource::Direct,
                        markdown: markdown.into(),
                    }));
                }
                Err(error) => errors.push((request_url, error)),
            }
        }
        for (request_url, error) in errors {
            warn!(
                "Direct fetch failed for {}: {}",
                request_url,
                error.client_message()
            );
        }
        Ok(None)
    }
    fn shared_probe_fetches(
        &self,
        targets: &[DirectFetchTarget],
    ) -> Result<HashMap<String, SharedProbeFetch>> {
        let mut probes = HashMap::new();
        for target in targets {
            let Some(probe_url) = target.similarity_probe_url.as_deref() else {
                continue;
            };
            if probes.contains_key(probe_url) {
                continue;
            }
            let probe = shared_probe_fetch(
                self.http.clone(),
                probe_url.to_owned(),
                target,
                &self.config.direct_fetch,
                &self.config.http,
            )?;
            probes.insert(probe_url.to_owned(), probe);
        }
        Ok(probes)
    }
    fn missing_reader_credentials_error(&self) -> AppError {
        AppError::client(format!(
            "Missing required header: {} or {}. URLs that cannot be directly fetched require one remote reader API key.",
            self.config.headers.jina_api_key, self.config.headers.tinyfish_api_key
        ))
    }
}
