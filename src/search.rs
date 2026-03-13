use anyhow::Result;
use snowball_stemmers_rs::{Algorithm, Stemmer};
use std::collections::HashSet;
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::Value;
use tantivy::{Index, TantivyDocument};

use crate::index::{FIELD_BODY, FIELD_PATH};
use crate::types::{Chunk, DirGroup, FileMatch};


const PATH_BOOST: f32 = 3.0;
const SCORE_THRESHOLD_RATIO: f32 = 0.5;
const MIN_RESULTS: usize = 3;
const NO_HIT_WEIGHT: f32 = 0.6;
const DENSITY_WEIGHT: f32 = 0.4;
const MERGE_GAP: usize = 3;

/// Check if a line matches any of the pre-stemmed query terms.
/// Both line words and query terms are lowercased and stemmed to match
/// the Tantivy analyzer pipeline (SimpleTokenizer → LowerCaser → PolishStemmer).
fn line_matches_any(line: &str, stemmed_terms: &[String], stemmer: &Stemmer) -> bool {
    let ll = line.to_lowercase();
    for word in ll.split_whitespace() {
        let stemmed_word = stemmer.stem(word);
        if stemmed_terms.iter().any(|t| *t == stemmed_word.as_ref()) {
            return true;
        }
    }
    false
}

pub fn search(index: &Index, query_str: &str, context_lines: usize, max_results: usize) -> Result<Vec<Chunk>> {
    let reader = index.reader()?;
    let searcher = reader.searcher();
    let schema = index.schema();

    let body_field = schema.get_field(FIELD_BODY).unwrap();
    let path_field = schema.get_field(FIELD_PATH).unwrap();

    let mut query_parser = QueryParser::for_index(index, vec![body_field, path_field]);
    query_parser.set_field_boost(path_field, PATH_BOOST);
    let query = query_parser.parse_query(query_str)?;

    let top_docs = searcher.search(&query, &TopDocs::with_limit(max_results))?;

    if top_docs.is_empty() {
        return Ok(vec![]);
    }

    let max_score = top_docs[0].0;
    let min_score = top_docs.last().unwrap().0;
    let threshold = min_score + (max_score - min_score) * SCORE_THRESHOLD_RATIO;

    // Stem and lowercase query terms once (matching the Tantivy analyzer pipeline)
    let stemmer = Stemmer::create(Algorithm::Polish);
    let stemmed_terms: Vec<String> = query_str
        .split_whitespace()
        .map(|t| stemmer.stem(&t.to_lowercase()).to_string())
        .collect();

    let mut chunks = Vec::new();

    for (score, doc_address) in &top_docs {
        if *score < threshold && chunks.len() > MIN_RESULTS {
            break;
        }

        let doc: TantivyDocument = searcher.doc(*doc_address)?;

        let file_path = doc
            .get_first(path_field)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let body = doc
            .get_first(body_field)
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let lines: Vec<&str> = body.lines().collect();
        let total_lines = lines.len();

        // Find hit lines once, reuse for chunk scoring
        let hit_set: HashSet<usize> = lines
            .iter()
            .enumerate()
            .filter(|(_, line)| line_matches_any(line, &stemmed_terms, &stemmer))
            .map(|(i, _)| i)
            .collect();

        if hit_set.is_empty() {
            let end = total_lines.min(context_lines * 2);
            let content = lines[..end].join("\n");
            chunks.push(Chunk {
                source_id: String::new(),
                file_path,
                start_line: 1,
                end_line: end,
                content,
                score: score * NO_HIT_WEIGHT,
            });
            continue;
        }

        // Build context windows around hit lines
        let mut windows: Vec<(usize, usize)> = hit_set
            .iter()
            .map(|&hit| {
                let start = hit.saturating_sub(context_lines);
                let end = (hit + context_lines + 1).min(total_lines);
                (start, end)
            })
            .collect();

        // Merge overlapping windows
        windows.sort();
        let mut merged: Vec<(usize, usize)> = Vec::new();
        for (start, end) in windows {
            if let Some(last) = merged.last_mut() {
                if start <= last.1 + MERGE_GAP {
                    last.1 = last.1.max(end);
                    continue;
                }
            }
            merged.push((start, end));
        }

        for (start, end) in merged {
            let chunk_lines = &lines[start..end];
            let content = chunk_lines.join("\n");

            // Reuse hit_set for density scoring (no re-lowercasing)
            let chunk_hits = (start..end).filter(|i| hit_set.contains(i)).count();
            let chunk_len = chunk_lines.len().max(1) as f32;
            let density = chunk_hits as f32 / chunk_len;
            let chunk_score = score * (1.0 - DENSITY_WEIGHT + DENSITY_WEIGHT * density);

            chunks.push(Chunk {
                source_id: String::new(),
                file_path: file_path.clone(),
                start_line: start + 1,
                end_line: end,
                content,
                score: chunk_score,
            });
        }
    }

    Ok(chunks)
}

/// Search returning only file paths and scores (no content extraction).
pub fn search_files(index: &Index, query_str: &str, max_results: usize) -> Result<Vec<FileMatch>> {
    let reader = index.reader()?;
    let searcher = reader.searcher();
    let schema = index.schema();

    let body_field = schema.get_field(FIELD_BODY).unwrap();
    let path_field = schema.get_field(FIELD_PATH).unwrap();

    let mut query_parser = QueryParser::for_index(index, vec![body_field, path_field]);
    query_parser.set_field_boost(path_field, PATH_BOOST);
    let query = query_parser.parse_query(query_str)?;

    let top_docs = searcher.search(&query, &TopDocs::with_limit(max_results))?;

    let mut results = Vec::new();
    for (score, doc_address) in &top_docs {
        let doc: TantivyDocument = searcher.doc(*doc_address)?;
        let file_path = doc
            .get_first(path_field)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        results.push(FileMatch {
            path: file_path,
            score: *score,
        });
    }

    Ok(results)
}

/// Multi-query file search: run each query, merge results by path (max score).
pub fn search_files_multi(index: &Index, queries: &[&str], max_results: usize) -> Result<Vec<FileMatch>> {
    let mut merged: std::collections::HashMap<String, f32> = std::collections::HashMap::new();

    for query_str in queries {
        let results = search_files(index, query_str, max_results)?;
        for fm in results {
            let entry = merged.entry(fm.path).or_insert(0.0);
            *entry = entry.max(fm.score);
        }
    }

    let mut files: Vec<FileMatch> = merged
        .into_iter()
        .map(|(path, score)| FileMatch { path, score })
        .collect();
    files.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    files.truncate(max_results);
    Ok(files)
}

/// Multi-query chunk search: run each query, collect and dedup chunks.
pub fn search_multi(index: &Index, queries: &[&str], context_lines: usize, max_results: usize) -> Result<Vec<Chunk>> {
    let mut all_chunks = Vec::new();
    for query_str in queries {
        let chunks = search(index, query_str, context_lines, max_results)?;
        all_chunks.extend(chunks);
    }
    // Dedup by (file_path, start_line) keeping higher score
    let mut seen: std::collections::HashMap<(String, usize), usize> = std::collections::HashMap::new();
    let mut deduped: Vec<Chunk> = Vec::new();
    for chunk in all_chunks {
        let key = (chunk.file_path.clone(), chunk.start_line);
        if let Some(&idx) = seen.get(&key) {
            if chunk.score > deduped[idx].score {
                deduped[idx] = chunk;
            }
        } else {
            seen.insert(key, deduped.len());
            deduped.push(chunk);
        }
    }
    deduped.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    deduped.truncate(max_results);
    Ok(deduped)
}

/// Group file matches by parent directory.
pub fn summarize_by_directory(files: Vec<FileMatch>) -> Vec<DirGroup> {
    let mut groups: std::collections::HashMap<String, Vec<FileMatch>> = std::collections::HashMap::new();
    for fm in files {
        let dir = std::path::Path::new(&fm.path)
            .parent()
            .map(|p| {
                let s = p.to_string_lossy().to_string();
                if s.is_empty() { ".".to_string() } else { s }
            })
            .unwrap_or_else(|| ".".to_string());
        groups.entry(dir).or_default().push(fm);
    }

    let mut result: Vec<DirGroup> = groups
        .into_iter()
        .map(|(directory, files)| {
            let count = files.len();
            let top_score = files.iter().map(|f| f.score).fold(0.0f32, f32::max);
            DirGroup {
                directory,
                count,
                top_score,
                files,
            }
        })
        .collect();
    result.sort_by(|a, b| b.top_score.partial_cmp(&a.top_score).unwrap_or(std::cmp::Ordering::Equal));
    result
}
