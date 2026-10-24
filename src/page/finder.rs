use crate::{
    Result,
    config::FindConfig,
    error::AppError,
    models::{ChunkSelection, FindMatch, FindPage, SplitCount},
    page::{PageContent, TokenChunker},
};
use fancy_regex::Regex;
#[expect(
    clippy::missing_inline_in_public_items,
    reason = "Page search loops over chunks and regex matches, so inlining is not useful."
)]
pub fn find_in_page(
    page: &PageContent,
    regex: &Regex,
    snippet_tokens: usize,
    chunker: &TokenChunker,
    config: &FindConfig,
    chunking: bool,
) -> Result<FindPage> {
    let chunks = chunking
        .then(|| chunker.split(&page.markdown))
        .transpose()?;
    let count = chunks.as_ref().map(|parts| SplitCount {
        total_chunks: parts.len(),
    });
    let regions = chunks.iter().flatten().map(|chunk| {
        (
            Some(ChunkSelection { chunk: chunk.index }),
            chunk.content.as_str(),
        )
    });
    let full_page = (!chunking).then_some((None, page.markdown.as_ref()));
    let mut matches = Vec::new();
    for (selection, content) in regions.chain(full_page) {
        for found_result in regex.find_iter(content) {
            let found = found_result.map_err(|error| {
                AppError::client(format!("regular expression match failed: {error}"))
            })?;
            matches.push(FindMatch {
                selection,
                snippet: chunker.snippet_around_span(
                    content,
                    found.start(),
                    found.end(),
                    snippet_tokens,
                )?,
            });
            if matches.len() >= config.max_matches_per_page {
                return Ok(FindPage { count, matches });
            }
        }
    }
    Ok(FindPage { count, matches })
}
