#![expect(
    clippy::pub_use,
    reason = "The page facade exposes implementations directly instead of maintaining forwarding functions."
)]
pub mod chunking;
pub mod fetcher;
pub mod finder;
pub mod jina;
pub mod open;
pub mod reader;
pub mod tinyfish;
pub use self::{
    chunking::{TextChunk, TokenChunker},
    fetcher::{PageContent, PageFetcher},
    finder::find_in_page,
    open::open_page,
};
