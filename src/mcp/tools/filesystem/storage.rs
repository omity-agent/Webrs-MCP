use crate::{Result, error::AppError, models::FilesystemResponse, page::PageContent};
use nanoid::nanoid;
use path_slash::PathExt as _;
use std::{
    collections::HashMap,
    io::ErrorKind,
    path::{Path, PathBuf},
};
use tokio::fs;
use walkdir::WalkDir;
pub(super) const ID_ALPHABET: [char; 36] = [
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i',
    'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z',
];
pub(super) const ID_LENGTH: usize = 8;
const MAX_ID_ATTEMPTS: usize = 32;
pub(super) struct Invocation {
    root: PathBuf,
    cache: HashMap<String, CachedPage>,
}
struct CachedPage {
    content: PathBuf,
    url: PathBuf,
}
impl Invocation {
    pub(super) async fn create(output_path: &str) -> Result<Self> {
        let base = PathBuf::from(output_path);
        fs::create_dir_all(&base)
            .await
            .map_err(|error| io_error("create output_path", &base, &error))?;
        let cache_root = base.clone();
        let cache = tokio::task::spawn_blocking(move || index_cache(&cache_root))
            .await
            .map_err(|error| {
                AppError::internal(format!("cache indexing task failed: {error}"))
            })??;
        let root = reserve_directory(&base).await?;
        Ok(Self { root, cache })
    }
    pub(super) async fn prepare<'path>(
        &self,
        paths: impl Iterator<Item = &'path PathBuf>,
    ) -> Result<()> {
        for path in paths {
            let directory = self.root.join(path);
            fs::create_dir_all(&directory)
                .await
                .map_err(|error| io_error("create result directory", &directory, &error))?;
        }
        Ok(())
    }
    pub(super) fn is_cached(&self, url: &str) -> bool {
        self.cache.contains_key(url)
    }
    pub(super) async fn copy_cached(&self, directory: &Path, url: &str) -> Result<()> {
        let cached = self
            .cache
            .get(url)
            .ok_or_else(|| AppError::internal("cached page disappeared from the index"))?;
        let destination = self.root.join(directory);
        copy_file(&cached.content, &destination.join("content.txt")).await?;
        copy_file(&cached.url, &destination.join(".url.txt")).await
    }
    pub(super) async fn write_page(
        &self,
        directory: &Path,
        url: &str,
        page: &PageContent,
    ) -> Result<()> {
        let destination = self.root.join(directory);
        let content_path = destination.join("content.txt");
        let url_path = destination.join(".url.txt");
        fs::write(&content_path, page.markdown.as_bytes())
            .await
            .map_err(|error| io_error("write page content", &content_path, &error))?;
        fs::write(&url_path, url.as_bytes())
            .await
            .map_err(|error| io_error("write page URL", &url_path, &error))
    }
    pub(super) fn response(&self, error: Option<String>) -> FilesystemResponse {
        FilesystemResponse {
            output_path: self.root.to_slash_lossy().into_owned(),
            error: error.unwrap_or_default(),
            warning: String::new(),
        }
    }
}
async fn reserve_directory(base: &Path) -> Result<PathBuf> {
    for _attempt in 0..MAX_ID_ATTEMPTS {
        let root = base.join(nanoid!(ID_LENGTH, &ID_ALPHABET));
        match fs::create_dir(&root).await {
            Ok(()) => return Ok(root),
            Err(error) if error.kind() == ErrorKind::AlreadyExists => {}
            Err(error) => return Err(io_error("create invocation directory", &root, &error)),
        }
    }
    Err(AppError::internal(
        "failed to allocate a unique invocation directory",
    ))
}
fn index_cache(root: &Path) -> Result<HashMap<String, CachedPage>> {
    let mut cache = HashMap::new();
    for raw_entry in WalkDir::new(root).follow_links(false) {
        let entry = raw_entry.map_err(|error| {
            AppError::client(format!("failed to scan {}: {error}", root.display()))
        })?;
        if entry.file_name() != ".url.txt" {
            continue;
        }
        let metadata = std::fs::metadata(entry.path())
            .map_err(|error| io_error("inspect cached page URL", entry.path(), &error))?;
        if !metadata.is_file() {
            return Err(AppError::client(format!(
                "cached page URL marker is not a file: {}",
                entry.path().display()
            )));
        }
        let url_path = entry.into_path();
        let content_path = url_path.with_file_name("content.txt");
        let content_metadata = std::fs::metadata(&content_path)
            .map_err(|error| io_error("inspect cached page content", &content_path, &error))?;
        if !content_metadata.is_file() {
            return Err(AppError::client(format!(
                "cached page content is not a file: {}",
                content_path.display()
            )));
        }
        let url = std::fs::read_to_string(&url_path)
            .map_err(|error| io_error("read cached page URL", &url_path, &error))?;
        let normalized_url = url.trim();
        if normalized_url.is_empty() {
            return Err(AppError::client(format!(
                "cached page URL is empty: {}",
                url_path.display()
            )));
        }
        cache
            .entry(normalized_url.to_owned())
            .or_insert(CachedPage {
                content: content_path,
                url: url_path,
            });
    }
    Ok(cache)
}
async fn copy_file(source: &Path, destination: &Path) -> Result<()> {
    fs::copy(source, destination)
        .await
        .map(|_bytes| ())
        .map_err(|error| io_error("copy cached page", destination, &error))
}
fn io_error(operation: &str, path: &Path, error: &std::io::Error) -> AppError {
    AppError::client(format!("{operation} at {} failed: {error}", path.display()))
}
