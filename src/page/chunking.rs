use crate::{Result, config::ChunkingConfig, error::AppError};
use core::num::NonZeroUsize;
use num_traits::ToPrimitive as _;
use tiktoken::CoreBpe;
#[derive(Clone, Debug)]
pub struct TextChunk {
    pub index: usize,
    pub content: String,
}
#[derive(Clone)]
pub struct TokenChunker {
    encoder: &'static CoreBpe,
    chunk_tokens: usize,
    max_concurrent_tasks: NonZeroUsize,
    overlap_tokens: usize,
}
impl TokenChunker {
    #[inline]
    pub fn new(config: &ChunkingConfig) -> Result<Self> {
        let encoder = tiktoken::get_encoding(&config.tokenizer)
            .ok_or_else(|| AppError::config(format!("unknown tokenizer: {}", config.tokenizer)))?;
        let max_concurrent_tasks = NonZeroUsize::new(config.max_concurrent_tasks)
            .ok_or_else(|| AppError::config("chunking.max_concurrent_tasks must be positive"))?;
        Ok(Self {
            encoder,
            chunk_tokens: config.chunk_tokens,
            max_concurrent_tasks,
            overlap_tokens: overlap_tokens(config)?,
        })
    }
    #[inline]
    #[must_use]
    pub const fn max_concurrent_tasks(&self) -> usize {
        self.max_concurrent_tasks.get()
    }
    #[inline]
    #[must_use]
    pub fn count_tokens(&self, text: &str) -> usize {
        self.encoder.encode(text).len()
    }
    #[expect(
        clippy::missing_inline_in_public_items,
        reason = "Splitting text allocates chunks and decodes token ranges, so inlining is not useful."
    )]
    pub fn split(&self, text: &str) -> Result<Vec<TextChunk>> {
        let tokens = self.encoder.encode(text);
        if tokens.len() <= self.chunk_tokens {
            return Ok(vec![TextChunk {
                index: 0,
                content: text.to_owned(),
            }]);
        }
        let mut chunks = Vec::new();
        let step = self.chunk_tokens.saturating_sub(self.overlap_tokens).max(1);
        let mut start = 0;
        while start < tokens.len() {
            let end = tokens.len().min(start.saturating_add(self.chunk_tokens));
            let token_window = tokens.get(start..end).ok_or_else(|| {
                AppError::internal("token chunk range was outside tokenizer output")
            })?;
            chunks.push(TextChunk {
                index: chunks.len(),
                content: self.decode(token_window)?,
            });
            if end == tokens.len() {
                break;
            }
            start = start.saturating_add(step);
        }
        Ok(chunks)
    }
    #[expect(
        clippy::missing_inline_in_public_items,
        reason = "Snippet extraction performs tokenization and allocation, so inlining is not useful."
    )]
    pub fn snippet_around_span(
        &self,
        text: &str,
        start: usize,
        end: usize,
        max_tokens: usize,
    ) -> Result<String> {
        let before_text = text
            .get(..start)
            .ok_or_else(|| AppError::internal("snippet start was not a character boundary"))?;
        let match_text = text
            .get(start..end)
            .ok_or_else(|| AppError::internal("snippet range was not a character boundary"))?;
        let after_text = text
            .get(end..)
            .ok_or_else(|| AppError::internal("snippet end was not a character boundary"))?;
        let before_tokens = self.encoder.encode(before_text);
        let match_tokens = self.encoder.encode(match_text);
        let after_tokens = self.encoder.encode(after_text);
        if match_tokens.len() >= max_tokens {
            let selected = match_tokens.get(..max_tokens).ok_or_else(|| {
                AppError::internal("snippet token limit exceeded match token range")
            })?;
            return self.decode(selected);
        }
        let remaining = max_tokens.saturating_sub(match_tokens.len());
        #[expect(
            clippy::integer_division,
            clippy::integer_division_remainder_used,
            reason = "The remaining token budget is intentionally split evenly around the match."
        )]
        let mut left_count = before_tokens.len().min(remaining / 2);
        let mut right_count = after_tokens.len().min(remaining.saturating_sub(left_count));
        let unused = remaining
            .saturating_sub(left_count)
            .saturating_sub(right_count);
        let extra_left = before_tokens.len().saturating_sub(left_count).min(unused);
        left_count = left_count.saturating_add(extra_left);
        right_count = right_count.saturating_add(
            after_tokens
                .len()
                .saturating_sub(right_count)
                .min(unused.saturating_sub(extra_left)),
        );
        let mut selected = tail(&before_tokens, left_count);
        selected.extend_from_slice(&match_tokens);
        let right_tokens = after_tokens.get(..right_count).ok_or_else(|| {
            AppError::internal("right snippet token range exceeded tokenizer output")
        })?;
        selected.extend_from_slice(right_tokens);
        self.decode(&selected)
    }
    fn decode(&self, tokens: &[u32]) -> Result<String> {
        self.encoder
            .decode_to_string(tokens)
            .map_err(|error| AppError::internal(format!("token decode failed: {error}")))
    }
}
fn overlap_tokens(config: &ChunkingConfig) -> Result<usize> {
    let chunk_tokens = config
        .chunk_tokens
        .to_f64()
        .ok_or_else(|| AppError::config("chunking.chunk_tokens is too large"))?;
    (chunk_tokens * config.overlap_ratio)
        .floor()
        .to_usize()
        .ok_or_else(|| AppError::config("chunking.overlap_ratio produced too many tokens"))
}
fn tail(items: &[u32], count: usize) -> Vec<u32> {
    if count == 0 {
        return Vec::new();
    }
    let start = items.len().saturating_sub(count);
    if let Some(tail) = items.get(start..) {
        return tail.to_vec();
    }
    Vec::new()
}
