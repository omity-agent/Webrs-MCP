use crate::{Result, error::AppError};
use flate2::read::GzDecoder;
use reqwest::header::{CONTENT_TYPE, HeaderMap};
use rustdoc_types::{Crate, FORMAT_VERSION, Id, ItemEnum};
use std::{collections::HashSet, io::Read as _};
mod items;
mod signatures;
#[expect(
    clippy::missing_inline_in_public_items,
    reason = "Rustdoc JSON conversion decompresses, deserializes, and renders a complete document."
)]
pub fn format_rustdoc_json(headers: &HeaderMap, body: &[u8]) -> Result<String> {
    let content_type = headers
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| AppError::client("Rustdoc JSON response has no valid Content-Type."))?;
    let decoded = decode_payload(content_type, body)?;
    let document: Crate = sonic_rs::from_slice(&decoded)
        .map_err(|error| AppError::client(format!("Rustdoc JSON is malformed: {error}")))?;
    if document.format_version > FORMAT_VERSION {
        return Err(AppError::client(format!(
            "Rustdoc JSON format version {} is newer than the supported version {FORMAT_VERSION}.",
            document.format_version
        )));
    }
    render_document(&document)
}
fn decode_payload(content_type: &str, body: &[u8]) -> Result<Vec<u8>> {
    let media_type = content_type.split(';').next().map_or("", str::trim);
    match media_type {
        "application/json" => Ok(body.to_owned()),
        "application/gzip" => {
            let mut decoder = GzDecoder::new(body);
            let mut decoded = Vec::new();
            decoder.read_to_end(&mut decoded).map_err(|error| {
                AppError::client(format!("Rustdoc JSON gzip is malformed: {error}"))
            })?;
            Ok(decoded)
        }
        "application/zstd" => zstd::stream::decode_all(body)
            .map_err(|error| AppError::client(format!("Rustdoc JSON zstd is malformed: {error}"))),
        _ => Err(AppError::client(format!(
            "Rustdoc JSON returned unsupported Content-Type {content_type}."
        ))),
    }
}
#[expect(
    clippy::pattern_type_mismatch,
    reason = "The root item is inspected through a shared Rustdoc document reference."
)]
fn render_document(document: &Crate) -> Result<String> {
    let root = document
        .index
        .get(&document.root)
        .ok_or_else(|| AppError::client("Rustdoc JSON does not contain its root item."))?;
    let ItemEnum::Module(module) = &root.inner else {
        return Err(AppError::client("Rustdoc JSON root item is not a module."));
    };
    let items = &module.items;
    let crate_name = document
        .external_crates
        .get(&root.crate_id)
        .map(|item| item.name.as_str())
        .or(root.name.as_deref())
        .map_or("crate", |name| name);
    let mut renderer = Renderer {
        document,
        local_crate_id: root.crate_id,
        output: String::new(),
        visited: HashSet::new(),
    };
    renderer.heading(1, &format!("`{crate_name}`"));
    if let Some(version) = document.crate_version.as_deref() {
        renderer.line(&format!("Version: `{version}`"));
    }
    renderer.line(&format!("Target: `{}`", document.target.triple));
    renderer.blank();
    renderer.docs(root.docs.as_deref());
    renderer.heading(2, "API");
    items::render_items(&mut renderer, items, 3, false)?;
    Ok(format!("{}\n", renderer.output.trim_end()))
}
pub(super) struct Renderer<'document> {
    pub(super) document: &'document Crate,
    pub(super) local_crate_id: u32,
    pub(super) output: String,
    pub(super) visited: HashSet<Id>,
}
impl Renderer<'_> {
    pub(super) fn line(&mut self, value: &str) {
        self.output.push_str(value);
        self.output.push('\n');
    }
    pub(super) fn blank(&mut self) {
        if !self.output.ends_with("\n\n") {
            self.output.push('\n');
        }
    }
    pub(super) fn heading(&mut self, level: usize, value: &str) {
        self.blank();
        self.line(&format!("{} {value}", "#".repeat(level.clamp(1, 6))));
        self.blank();
    }
    pub(super) fn code(&mut self, value: &str) {
        self.line("```rust");
        self.line(value);
        self.line("```");
        self.blank();
    }
    pub(super) fn docs(&mut self, source: Option<&str>) {
        let Some(docs) = source.map(str::trim).filter(|value| !value.is_empty()) else {
            return;
        };
        self.output.push_str(docs);
        self.output.push_str("\n\n");
    }
    pub(super) fn deprecation(&mut self, item: &rustdoc_types::Item) {
        let Some(deprecation) = item.deprecation.as_ref() else {
            return;
        };
        let mut message = String::from("> **Deprecated**");
        if let Some(since) = deprecation.since.as_deref() {
            message.push_str(" since `");
            message.push_str(since);
            message.push('`');
        }
        if let Some(note) = deprecation.note.as_deref() {
            message.push_str(": ");
            message.push_str(note);
        }
        self.line(&message);
        self.blank();
    }
}
