use anyhow::Result;
use ignore::WalkBuilder;
use regex::Regex;
use std::path::Path;

use crate::filter::PathFilter;
use crate::index::{MAX_FILE_SIZE, is_binary_ext};
use crate::types::{Chunk, FileMatch};

/// Validate and compile a regex pattern.
pub fn validate_pattern(pattern: &str) -> Result<Regex> {
    Regex::new(pattern).map_err(|e| anyhow::anyhow!("Invalid regex pattern '{}': {}", pattern, e))
}

/// Walk corpus files, respecting gitignore, binary filters, and path filters.
/// Returns an iterator of (relative_path, absolute_path) pairs.
fn walk_corpus<'a>(
    corpus: &'a Path,
    filter: &'a PathFilter,
) -> impl Iterator<Item = (String, std::path::PathBuf)> + 'a {
    let corpus_path = corpus.to_path_buf();
    WalkBuilder::new(corpus)
        .hidden(true)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .build()
        .filter_map(|e| e.ok())
        .filter(|entry| entry.file_type().map_or(false, |ft| ft.is_file()))
        .filter(|entry| !is_binary_ext(entry.path()))
        .filter(|entry| {
            entry
                .metadata()
                .map(|m| m.len() <= MAX_FILE_SIZE && m.len() > 0)
                .unwrap_or(false)
        })
        .filter_map(move |entry| {
            let rel = entry
                .path()
                .strip_prefix(&corpus_path)
                .unwrap_or(entry.path())
                .to_string_lossy()
                .to_string();
            if filter.matches(&rel) {
                Some((rel, entry.into_path()))
            } else {
                None
            }
        })
}

/// Pure grep over corpus files. Returns files with match count as score.
pub fn grep_files(
    corpus: &Path,
    pattern: &str,
    max_results: usize,
    filter: &PathFilter,
) -> Result<Vec<FileMatch>> {
    let regex = validate_pattern(pattern)?;
    let mut results = Vec::new();

    for (rel_path, abs_path) in walk_corpus(corpus, filter) {
        let content = match std::fs::read_to_string(&abs_path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let match_count = content.lines().filter(|line| regex.is_match(line)).count();
        if match_count > 0 {
            results.push(FileMatch {
                path: rel_path,
                score: match_count as f32,
            });
        }
    }

    results.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    results.truncate(max_results);
    Ok(results)
}

/// Pure grep over corpus files. Returns chunks with context around matches.
pub fn grep_chunks(
    corpus: &Path,
    pattern: &str,
    context_lines: usize,
    max_results: usize,
    filter: &PathFilter,
) -> Result<Vec<Chunk>> {
    let regex = validate_pattern(pattern)?;
    let mut chunks = Vec::new();

    for (rel_path, abs_path) in walk_corpus(corpus, filter) {
        let content = match std::fs::read_to_string(&abs_path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let lines: Vec<&str> = content.lines().collect();
        let total_lines = lines.len();

        // Find matching line indices
        let hit_lines: Vec<usize> = lines
            .iter()
            .enumerate()
            .filter(|(_, line)| regex.is_match(line))
            .map(|(i, _)| i)
            .collect();

        if hit_lines.is_empty() {
            continue;
        }

        // Build context windows
        let mut windows: Vec<(usize, usize)> = hit_lines
            .iter()
            .map(|&hit| {
                let start = hit.saturating_sub(context_lines);
                let end = (hit + context_lines + 1).min(total_lines);
                (start, end)
            })
            .collect();

        // Merge overlapping/adjacent windows
        windows.sort();
        let mut merged: Vec<(usize, usize)> = Vec::new();
        for (start, end) in windows {
            if let Some(last) = merged.last_mut() {
                if start <= last.1 {
                    last.1 = last.1.max(end);
                    continue;
                }
            }
            merged.push((start, end));
        }

        // Create chunks from merged windows
        for (start, end) in merged {
            let chunk_lines = &lines[start..end];
            let chunk_content = chunk_lines.join("\n");

            let chunk_hits = (start..end).filter(|&i| hit_lines.contains(&i)).count();
            let density = chunk_hits as f32 / chunk_lines.len().max(1) as f32;

            chunks.push(Chunk {
                source_id: String::new(),
                file_path: rel_path.clone(),
                start_line: start + 1,
                end_line: end,
                content: chunk_content,
                score: density,
            });
        }
    }

    chunks.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    chunks.truncate(max_results);
    Ok(chunks)
}

/// Filter BM25 chunks by regex — keep only chunks where at least one line matches the pattern.
/// Uses per-line matching (same semantics as pure grep) so anchors like ^ and $ work correctly.
pub fn filter_chunks_by_regex(chunks: Vec<Chunk>, regex: &Regex) -> Vec<Chunk> {
    chunks
        .into_iter()
        .filter(|chunk| chunk.content.lines().any(|line| regex.is_match(line)))
        .collect()
}

/// Filter BM25 file matches by regex — keep only files where at least one line matches the pattern.
/// Uses per-line matching (same semantics as pure grep) so anchors like ^ and $ work correctly.
pub fn filter_files_by_regex(
    files: Vec<FileMatch>,
    regex: &Regex,
    corpus: &Path,
) -> Vec<FileMatch> {
    files
        .into_iter()
        .filter(|fm| {
            let full_path = corpus.join(&fm.path);
            match std::fs::read_to_string(&full_path) {
                Ok(content) => content.lines().any(|line| regex.is_match(line)),
                Err(_) => false,
            }
        })
        .collect()
}
