use anyhow::{Context, Result, bail};
use ignore::WalkBuilder;
use serde::{Deserialize, Serialize};
use snowball_stemmers_rs::Algorithm;
use std::collections::HashMap;
use std::path::Path;
use std::time::SystemTime;
use tantivy::schema::{IndexRecordOption, Schema, TextFieldIndexing, TextOptions};
use tantivy::tokenizer::{LowerCaser, RemoveLongFilter, SimpleTokenizer, TextAnalyzer};
use tantivy::{Index, IndexWriter, Term, doc};

use crate::stemmer::StemmerFilter;

pub(crate) const MAX_FILE_SIZE: u64 = 1_048_576; // 1MB
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
    #[serde(default)]
    language: Option<String>,
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

fn register_tokenizer(index: &Index, algorithm: Option<Algorithm>) {
    match algorithm {
        Some(algo) => {
            let tokenizer = TextAnalyzer::builder(SimpleTokenizer::default())
                .filter(RemoveLongFilter::limit(80))
                .filter(LowerCaser)
                .filter(StemmerFilter::new(algo))
                .build();
            index.tokenizers().register(TOKENIZER_NAME, tokenizer);
        }
        None => {
            let tokenizer = TextAnalyzer::builder(SimpleTokenizer::default())
                .filter(RemoveLongFilter::limit(80))
                .filter(LowerCaser)
                .build();
            index.tokenizers().register(TOKENIZER_NAME, tokenizer);
        }
    }
}

fn mtime_millis(metadata: &std::fs::Metadata) -> u64 {
    metadata
        .modified()
        .unwrap_or(SystemTime::UNIX_EPOCH)
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
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

pub(crate) fn is_binary_ext(path: &Path) -> bool {
    let ext = path.extension().map(|e| e.to_string_lossy().to_lowercase());
    matches!(
        ext.as_deref(),
        Some(
            "png"
                | "jpg"
                | "jpeg"
                | "gif"
                | "bmp"
                | "ico"
                | "svg"
                | "webp"
                | "mp3"
                | "mp4"
                | "avi"
                | "mov"
                | "mkv"
                | "wav"
                | "flac"
                | "zip"
                | "tar"
                | "gz"
                | "bz2"
                | "xz"
                | "7z"
                | "rar"
                | "exe"
                | "dll"
                | "so"
                | "dylib"
                | "o"
                | "a"
                | "wasm"
                | "pdf"
                | "doc"
                | "docx"
                | "xls"
                | "xlsx"
                | "bin"
                | "dat"
                | "db"
                | "sqlite"
        )
    )
}

/// Resolve a language string to a stemmer algorithm.
/// Returns Ok(Some(algo)) for known languages, Ok(None) for "none", Err for unknown.
pub fn resolve_language(lang: &str) -> Result<Option<Algorithm>> {
    match lang {
        "pl" => Ok(Some(Algorithm::Polish)),
        "en" => Ok(Some(Algorithm::English)),
        "de" => Ok(Some(Algorithm::German)),
        "fr" => Ok(Some(Algorithm::French)),
        "es" => Ok(Some(Algorithm::Spanish)),
        "it" => Ok(Some(Algorithm::Italian)),
        "pt" => Ok(Some(Algorithm::Portuguese)),
        "ru" => Ok(Some(Algorithm::Russian)),
        "sv" => Ok(Some(Algorithm::Swedish)),
        "nl" => Ok(Some(Algorithm::Dutch)),
        "fi" => Ok(Some(Algorithm::Finnish)),
        "da" => Ok(Some(Algorithm::Danish)),
        "hu" => Ok(Some(Algorithm::Hungarian)),
        "ro" => Ok(Some(Algorithm::Romanian)),
        "tr" => Ok(Some(Algorithm::Turkish)),
        "none" => Ok(None),
        _ => bail!(
            "Unsupported language: '{}'. Supported: pl, en, de, fr, es, it, pt, ru, sv, nl, fi, da, hu, ro, tr, none",
            lang
        ),
    }
}

/// Read the language stored in an index's manifest. Defaults to "pl" for backward compat.
pub fn read_index_language(index_path: &Path) -> String {
    let manifest = load_manifest(index_path);
    manifest.language.unwrap_or_else(|| "pl".to_string())
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

/// Scan corpus directory, return map of rel_path -> FileEntry for indexable files.
/// Excludes the index directory to prevent self-indexing.
fn scan_corpus(corpus_path: &Path, index_path: &Path) -> HashMap<String, FileEntry> {
    let mut current = HashMap::new();
    let index_canonical = index_path.canonicalize().ok();
    let walker = WalkBuilder::new(corpus_path)
        .hidden(true)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .filter_entry(move |entry| {
            if let Some(ref idx_canon) = index_canonical {
                if let Ok(entry_canon) = entry.path().canonicalize() {
                    if entry_canon.starts_with(idx_canon) {
                        return false;
                    }
                }
            }
            true
        })
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
                mtime: mtime_millis(&metadata),
                size: metadata.len(),
            },
        );
    }
    current
}

pub fn open_index(index_path: &Path) -> Result<Index> {
    let manifest = load_manifest(index_path);
    let lang = manifest.language.as_deref().unwrap_or("pl");
    let algorithm = resolve_language(lang)?;

    let index = Index::open_in_dir(index_path).context("Failed to open existing index")?;
    register_tokenizer(&index, algorithm);
    Ok(index)
}

/// Full rebuild from scratch
pub fn build_index(corpus_path: &Path, index_path: &Path, language: &str) -> Result<Index> {
    let algorithm = resolve_language(language)?;
    let schema = build_schema();

    if index_path.exists() {
        std::fs::remove_dir_all(index_path)?;
    }
    std::fs::create_dir_all(index_path)?;

    let index = Index::create_in_dir(index_path, schema.clone())?;
    register_tokenizer(&index, algorithm);

    let mut writer: IndexWriter = index.writer(WRITER_HEAP)?;
    let path_field = schema.get_field(FIELD_PATH).unwrap();
    let body_field = schema.get_field(FIELD_BODY).unwrap();

    let current = scan_corpus(corpus_path, index_path);
    let mut file_count = 0u64;

    for rel_path in current.keys() {
        if index_file(&writer, path_field, body_field, corpus_path, rel_path)? {
            file_count += 1;
        }
    }

    writer.commit()?;
    save_manifest(
        index_path,
        &Manifest {
            files: current,
            language: Some(language.to_string()),
        },
    )?;
    eprintln!("Indexed {} files into {}", file_count, index_path.display());
    Ok(index)
}

/// Incremental update: add new/changed, remove deleted
pub fn update_index(corpus_path: &Path, index_path: &Path) -> Result<(Index, bool)> {
    let old_manifest = load_manifest(index_path);
    let language = old_manifest.language.clone();
    let current = scan_corpus(corpus_path, index_path);

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
    save_manifest(
        index_path,
        &Manifest {
            files: current,
            language,
        },
    )?;
    eprintln!(
        "Updated index: +{} added/changed, -{} removed",
        added,
        to_remove.len()
    );
    Ok((index, true))
}
