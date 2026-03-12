use anyhow::{Context, Result};
use ignore::WalkBuilder;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::time::SystemTime;
use tantivy::schema::{IndexRecordOption, Schema, TextFieldIndexing, TextOptions};
use tantivy::tokenizer::{LowerCaser, RemoveLongFilter, SimpleTokenizer, TextAnalyzer};
use tantivy::{doc, Index, IndexWriter, Term};

use crate::stemmer::PolishStemmerFilter;

const MAX_FILE_SIZE: u64 = 1_048_576; // 1MB
const TOKENIZER_NAME: &str = "pl_stem";
const MANIFEST_FILE: &str = "manifest.json";
const WRITER_HEAP: usize = 50_000_000; // 50MB

pub const FIELD_PATH: &str = "path";
pub const FIELD_BODY: &str = "body";

#[derive(Serialize, Deserialize, Clone)]
struct FileEntry {
    mtime: u64,
    size: u64,
}

#[derive(Serialize, Deserialize, Default)]
struct Manifest {
    files: HashMap<String, FileEntry>,
}

fn build_schema() -> Schema {
    let mut builder = Schema::builder();
    let text_indexing = TextFieldIndexing::default()
        .set_tokenizer(TOKENIZER_NAME)
        .set_index_option(IndexRecordOption::WithFreqsAndPositions);
    let text_options = TextOptions::default()
        .set_indexing_options(text_indexing)
        .set_stored();
    builder.add_text_field(FIELD_PATH, text_options.clone());
    builder.add_text_field(FIELD_BODY, text_options);
    builder.build()
}

fn register_tokenizer(index: &Index) {
    let tokenizer = TextAnalyzer::builder(SimpleTokenizer::default())
        .filter(RemoveLongFilter::limit(80))
        .filter(LowerCaser)
        .filter(PolishStemmerFilter)
        .build();
    index.tokenizers().register(TOKENIZER_NAME, tokenizer);
}

fn mtime_secs(metadata: &std::fs::Metadata) -> u64 {
    metadata
        .modified()
        .unwrap_or(SystemTime::UNIX_EPOCH)
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn load_manifest(index_path: &Path) -> Manifest {
    let path = index_path.join(MANIFEST_FILE);
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_manifest(index_path: &Path, manifest: &Manifest) -> Result<()> {
    let path = index_path.join(MANIFEST_FILE);
    let json = serde_json::to_string(manifest)?;
    std::fs::write(path, json)?;
    Ok(())
}

fn is_binary_ext(path: &Path) -> bool {
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase());
    matches!(
        ext.as_deref(),
        Some(
            "png" | "jpg" | "jpeg" | "gif" | "bmp" | "ico" | "svg" | "webp"
                | "mp3" | "mp4" | "avi" | "mov" | "mkv" | "wav" | "flac"
                | "zip" | "tar" | "gz" | "bz2" | "xz" | "7z" | "rar"
                | "exe" | "dll" | "so" | "dylib" | "o" | "a"
                | "wasm" | "pdf" | "doc" | "docx" | "xls" | "xlsx"
                | "bin" | "dat" | "db" | "sqlite"
        )
    )
}

/// Read file and add to index. Returns true if added.
fn index_file(
    writer: &IndexWriter,
    path_field: tantivy::schema::Field,
    body_field: tantivy::schema::Field,
    corpus_path: &Path,
    rel_path: &str,
) -> Result<bool> {
    let full_path = corpus_path.join(rel_path);
    let content = match std::fs::read_to_string(&full_path) {
        Ok(c) if !c.is_empty() => c,
        _ => return Ok(false),
    };
    writer.add_document(doc!(
        path_field => rel_path,
        body_field => content,
    ))?;
    Ok(true)
}

/// Scan corpus directory, return map of rel_path -> FileEntry for indexable files
fn scan_corpus(corpus_path: &Path) -> HashMap<String, FileEntry> {
    let mut current = HashMap::new();
    let walker = WalkBuilder::new(corpus_path)
        .hidden(true)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .build();

    for entry in walker.into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().map_or(false, |ft| ft.is_file()) {
            continue;
        }
        let metadata = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        if metadata.len() > MAX_FILE_SIZE || metadata.len() == 0 {
            continue;
        }
        if is_binary_ext(entry.path()) {
            continue;
        }
        let rel_path = entry
            .path()
            .strip_prefix(corpus_path)
            .unwrap_or(entry.path())
            .to_string_lossy()
            .to_string();
        current.insert(
            rel_path,
            FileEntry {
                mtime: mtime_secs(&metadata),
                size: metadata.len(),
            },
        );
    }
    current
}

pub fn open_index(index_path: &Path) -> Result<Index> {
    let index = Index::open_in_dir(index_path).context("Failed to open existing index")?;
    register_tokenizer(&index);
    Ok(index)
}

/// Full rebuild from scratch
pub fn build_index(corpus_path: &Path, index_path: &Path) -> Result<Index> {
    let schema = build_schema();

    if index_path.exists() {
        std::fs::remove_dir_all(index_path)?;
    }
    std::fs::create_dir_all(index_path)?;

    let index = Index::create_in_dir(index_path, schema.clone())?;
    register_tokenizer(&index);

    let mut writer: IndexWriter = index.writer(WRITER_HEAP)?;
    let path_field = schema.get_field(FIELD_PATH).unwrap();
    let body_field = schema.get_field(FIELD_BODY).unwrap();

    let current = scan_corpus(corpus_path);
    let mut file_count = 0u64;

    for rel_path in current.keys() {
        if index_file(&writer, path_field, body_field, corpus_path, rel_path)? {
            file_count += 1;
        }
    }

    writer.commit()?;
    save_manifest(index_path, &Manifest { files: current })?;
    eprintln!("Indexed {} files into {}", file_count, index_path.display());
    Ok(index)
}

/// Incremental update: add new/changed, remove deleted
pub fn update_index(corpus_path: &Path, index_path: &Path) -> Result<(Index, bool)> {
    let old_manifest = load_manifest(index_path);
    let current = scan_corpus(corpus_path);

    let mut to_add: Vec<&String> = Vec::new();
    let mut to_remove: Vec<&String> = Vec::new();

    for (path, entry) in &current {
        match old_manifest.files.get(path) {
            Some(old) if old.mtime == entry.mtime && old.size == entry.size => {}
            _ => to_add.push(path),
        }
    }
    for path in old_manifest.files.keys() {
        if !current.contains_key(path) {
            to_remove.push(path);
        }
    }

    if to_add.is_empty() && to_remove.is_empty() {
        return Ok((open_index(index_path)?, false));
    }

    let index = open_index(index_path)?;
    let schema = index.schema();
    let path_field = schema.get_field(FIELD_PATH).unwrap();
    let body_field = schema.get_field(FIELD_BODY).unwrap();

    let mut writer: IndexWriter = index.writer(WRITER_HEAP)?;

    for path in to_remove.iter().chain(to_add.iter()) {
        writer.delete_term(Term::from_field_text(path_field, path));
    }

    let mut added = 0u64;
    for path in &to_add {
        if index_file(&writer, path_field, body_field, corpus_path, path)? {
            added += 1;
        }
    }

    writer.commit()?;
    save_manifest(index_path, &Manifest { files: current })?;
    eprintln!(
        "Updated index: +{} added/changed, -{} removed",
        added,
        to_remove.len()
    );
    Ok((index, true))
}
