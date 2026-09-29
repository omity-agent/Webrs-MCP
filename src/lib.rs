extern crate alloc;
pub mod app;
pub mod arguments;
pub mod cli;
pub mod config;
pub mod direct;
pub mod error;
pub mod mcp;
pub mod models;
pub mod net;
pub mod page;
pub mod search;
use mimalloc::MiMalloc;
#[global_allocator]
static GLOBAL_ALLOCATOR: MiMalloc = MiMalloc;
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub type Result<T> = core::result::Result<T, error::AppError>;
