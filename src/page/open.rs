use crate::{
    Result,
    error::AppError,
    models::{ChunkSelection, OpenPage, PagePartition},
    page::{PageContent, TokenChunker},
    warnings::Warnings,
};
#[expect(
    clippy::missing_inline_in_public_items,
    reason = "Opening a page copies its full content or selects a token chunk."
)]
pub fn open_page(
    page: &PageContent,
    selection: Option<ChunkSelection>,
    request_index: usize,
    chunker: &TokenChunker,
    warnings: &mut Warnings,
) -> Result<OpenPage> {
    let Some(ChunkSelection { chunk: chunk_index }) = selection else {
        return Ok(OpenPage {
            partition: None,
            content: page.markdown.to_string(),
        });
    };
    let chunks = chunker.split(&page.markdown)?;
    let selected = if chunk_index >= chunks.len() {
        warnings.push(format!(
            "\"requests[{request_index}].chunk\" must be between 0 and {}; using 0",
            chunks.len().saturating_sub(1)
        ));
        chunks
            .first()
            .ok_or_else(|| AppError::internal("page split produced no chunks"))?
    } else {
        chunks
            .get(chunk_index)
            .ok_or_else(|| AppError::internal("validated page chunk was missing"))?
    };
    Ok(OpenPage {
        partition: Some(PagePartition {
            chunk: selected.index,
            total_chunks: chunks.len(),
        }),
        content: selected.content.clone(),
    })
}
