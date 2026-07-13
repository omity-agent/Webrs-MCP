use crate::{
    Result,
    config::AppConfig,
    net::SecureHttpClient,
    page::{jina::JinaReaderClient, tinyfish::TinyFishFetchClient},
};
use futures::future::join_all;
#[derive(Clone)]
pub struct PageReader {
    jina: JinaReaderClient,
    tinyfish: TinyFishFetchClient,
}
#[derive(Clone, Debug, Eq, PartialEq)]
#[expect(
    clippy::exhaustive_enums,
    reason = "Reader credentials are closed because supported upstream readers are configured here."
)]
pub enum ReaderCredentials {
    Jina(String),
    TinyFish(String),
}
impl PageReader {
    #[inline]
    #[must_use]
    pub fn new(config: AppConfig, http: SecureHttpClient) -> Self {
        Self {
            jina: JinaReaderClient::new(config.clone(), http.clone()),
            tinyfish: TinyFishFetchClient::new(config, http),
        }
    }
    #[expect(
        clippy::missing_inline_in_public_items,
        reason = "Remote reader calls perform async HTTP I/O and are not inline candidates."
    )]
    #[expect(
        clippy::pattern_type_mismatch,
        reason = "Matching borrowed credentials avoids cloning request API keys."
    )]
    pub async fn read_markdown(
        &self,
        url: &str,
        credentials: &ReaderCredentials,
    ) -> Result<String> {
        match credentials {
            ReaderCredentials::Jina(api_key) => self.jina.read_markdown(url, api_key).await,
            ReaderCredentials::TinyFish(api_key) => self.tinyfish.read_markdown(url, api_key).await,
        }
    }
    #[expect(
        clippy::missing_inline_in_public_items,
        reason = "Batch reader calls perform async HTTP I/O and are not inline candidates."
    )]
    #[expect(
        clippy::pattern_type_mismatch,
        reason = "Matching borrowed credentials avoids cloning request API keys."
    )]
    pub async fn read_markdown_many(
        &self,
        urls: &[String],
        credentials: &ReaderCredentials,
    ) -> Result<Vec<Result<String>>> {
        match credentials {
            ReaderCredentials::Jina(api_key) => {
                let reads = urls.iter().map(|url| self.jina.read_markdown(url, api_key));
                Ok(join_all(reads).await)
            }
            ReaderCredentials::TinyFish(api_key) => {
                self.tinyfish.read_markdown_many(urls, api_key).await
            }
        }
    }
}
