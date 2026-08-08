use crate::{Result, config::validation, error::AppError};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ServerConfig {
    pub name: String,
    pub instructions: String,
    pub host: String,
    pub port: u16,
    pub log_level: String,
    pub json_response: bool,
    pub allowed_hosts: Vec<String>,
    pub allowed_origins: Vec<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HeaderConfig {
    pub exa_api_key: String,
    pub jina_api_key: String,
    pub tinyfish_api_key: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SearchConfig {
    pub endpoint: String,
    pub num_results: u32,
    #[serde(rename = "type")]
    pub search_type: String,
    pub highlights_max_characters: u32,
    pub max_age_hours: u64,
    pub livecrawl_timeout: u32,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HttpConfig {
    pub user_agent: String,
    pub timeout_seconds: f64,
    pub direct_fetch_timeout_seconds: f64,
    pub max_redirects: usize,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct JinaViewportConfig {
    pub width: u32,
    pub height: u32,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct JinaConfig {
    pub endpoint: String,
    pub accept: String,
    pub return_format: String,
    pub engine: String,
    pub locale: String,
    pub no_cache: bool,
    pub respond_with: String,
    pub retain_images: String,
    pub with_shadow_dom: bool,
    pub arxiv_pdf_url_prefix: String,
    pub arxiv_html_url_prefix: String,
    pub viewport: JinaViewportConfig,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TinyFishConfig {
    pub endpoint: String,
    pub format: String,
    pub per_url_timeout_ms: u64,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ChunkingConfig {
    pub tokenizer: String,
    pub chunk_tokens: usize,
    pub overlap_ratio: f64,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FindConfig {
    pub default_snippet_tokens: usize,
    pub max_matches_per_page: usize,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DirectFetchConfig {
    pub max_bytes: usize,
    pub similarity_threshold: f64,
    pub github_hosts: Vec<String>,
    pub huggingface_hosts: Vec<String>,
    pub gitlab_hosts: Vec<String>,
    pub bitbucket_hosts: Vec<String>,
    pub microsoft_learn_hosts: Vec<String>,
    pub stack_overflow_hosts: Vec<String>,
    pub stack_overflow_api_url_template: String,
    pub npm_hosts: Vec<String>,
    pub npm_registry_url_prefix: String,
    pub wikimedia_domains: Vec<String>,
    pub wikimedia_api_path: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SsrfConfig {
    pub block_private_networks: bool,
    pub block_local_hostnames: bool,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub headers: HeaderConfig,
    pub search: SearchConfig,
    pub http: HttpConfig,
    pub jina: JinaConfig,
    pub tinyfish: TinyFishConfig,
    pub chunking: ChunkingConfig,
    pub find: FindConfig,
    pub direct_fetch: DirectFetchConfig,
    pub ssrf: SsrfConfig,
}
impl AppConfig {
    #[inline]
    pub fn validate(&self) -> Result<()> {
        validation::positive(&self.search.num_results, "search.num_results")?;
        validation::positive(
            &self.search.highlights_max_characters,
            "search.highlights_max_characters",
        )?;
        validation::positive(&self.search.livecrawl_timeout, "search.livecrawl_timeout")?;
        validation::positive(&self.chunking.chunk_tokens, "chunking.chunk_tokens")?;
        validation::positive(
            &self.find.default_snippet_tokens,
            "find.default_snippet_tokens",
        )?;
        validation::positive(&self.find.max_matches_per_page, "find.max_matches_per_page")?;
        validation::positive(&self.direct_fetch.max_bytes, "direct_fetch.max_bytes")?;
        validation::threshold(
            self.direct_fetch.similarity_threshold,
            "direct_fetch.similarity_threshold",
        )?;
        validation::positive_float(self.http.timeout_seconds, "http.timeout_seconds")?;
        validation::positive_float(
            self.http.direct_fetch_timeout_seconds,
            "http.direct_fetch_timeout_seconds",
        )?;
        if !(0.0_f64..1.0_f64).contains(&self.chunking.overlap_ratio) {
            return Err(AppError::config(
                "chunking.overlap_ratio must be >= 0 and < 1",
            ));
        }
        validation::header_value(&self.http.user_agent, "http.user_agent")?;
        validation::endpoint(&self.search.endpoint, "search.endpoint")?;
        validation::endpoint(&self.jina.endpoint, "jina.endpoint")?;
        validation::endpoint(&self.tinyfish.endpoint, "tinyfish.endpoint")?;
        validation::positive(
            &self.tinyfish.per_url_timeout_ms,
            "tinyfish.per_url_timeout_ms",
        )?;
        if self.tinyfish.per_url_timeout_ms > 110_000 {
            return Err(AppError::config(
                "tinyfish.per_url_timeout_ms must be <= 110000",
            ));
        }
        if self.tinyfish.format != "markdown" {
            return Err(AppError::config("tinyfish.format must be markdown"));
        }
        validation::endpoint(&self.jina.arxiv_pdf_url_prefix, "jina.arxiv_pdf_url_prefix")?;
        validation::endpoint(
            &self.jina.arxiv_html_url_prefix,
            "jina.arxiv_html_url_prefix",
        )?;
        validation::endpoint(
            &self.direct_fetch.npm_registry_url_prefix,
            "direct_fetch.npm_registry_url_prefix",
        )?;
        validation::template_endpoint(
            &self.direct_fetch.stack_overflow_api_url_template,
            "direct_fetch.stack_overflow_api_url_template",
            "{question_id}",
        )?;
        validation::path_prefix(
            &self.direct_fetch.wikimedia_api_path,
            "direct_fetch.wikimedia_api_path",
        )?;
        Ok(())
    }
}
