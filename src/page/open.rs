use crate::{
    Result,
    error::AppError,
    models::OpenPage,
    page::{PageContent, TokenChunker},
    warnings::Warnings,
};
#[expect(
    clippy::missing_inline_in_public_items,
    reason = "Open-page chunk selection allocates chunks and keeps the response model terminology."
)]
pub fn open_page_chunk(
    page: &PageContent,
    chunk_index: usize,
    request_index: usize,
    chunker: &TokenChunker,
    warnings: &mut Warnings,
) -> Result<OpenPage> {
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
        chunk: selected.index,
        total_chunks: chunks.len(),
        content: selected.content.clone(),
    })
}
