use anyhow::Result;
use std::collections::HashSet;
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::Value;
use tantivy::{Index, TantivyDocument};

use crate::index::{FIELD_BODY, FIELD_PATH};
use crate::types::Chunk;

const MAX_RESULTS: usize = 100;
const PATH_BOOST: f32 = 3.0;
const SCORE_THRESHOLD_RATIO: f32 = 0.5;
const MIN_RESULTS: usize = 3;
const NO_HIT_WEIGHT: f32 = 0.6;
const DENSITY_WEIGHT: f32 = 0.4;
const MERGE_GAP: usize = 3;

fn line_matches_any(line: &str, terms_lower: &[String]) -> bool {
    let ll = line.to_lowercase();
    terms_lower.iter().any(|t| ll.contains(t.as_str()))
}

pub fn search(index: &Index, query_str: &str, context_lines: usize) -> Result<Vec<Chunk>> {
    let reader = index.reader()?;
    let searcher = reader.searcher();
    let schema = index.schema();

    let body_field = schema.get_field(FIELD_BODY).unwrap();
    let path_field = schema.get_field(FIELD_PATH).unwrap();

    let mut query_parser = QueryParser::for_index(index, vec![body_field, path_field]);
    query_parser.set_field_boost(path_field, PATH_BOOST);
    let query = query_parser.parse_query(query_str)?;

    let top_docs = searcher.search(&query, &TopDocs::with_limit(MAX_RESULTS))?;

    if top_docs.is_empty() {
        return Ok(vec![]);
    }

    let max_score = top_docs[0].0;
    let min_score = top_docs.last().unwrap().0;
    let threshold = min_score + (max_score - min_score) * SCORE_THRESHOLD_RATIO;

    // Lowercase query terms once
    let terms_lower: Vec<String> = query_str
        .split_whitespace()
        .map(|t| t.to_lowercase())
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
            .filter(|(_, line)| line_matches_any(line, &terms_lower))
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
