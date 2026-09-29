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
#[global_allocator]
static ALLOC: snmalloc_rs::SnMalloc = snmalloc_rs::SnMalloc;
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub type Result<T> = core::result::Result<T, error::AppError>;
