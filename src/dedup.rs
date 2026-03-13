use crate::types::Chunk;
use std::collections::HashMap;

/// Merge overlapping or adjacent chunks from the same file.
/// Chunks with a gap of <= 3 lines are merged into one continuous block.
pub fn merge_chunks(chunks: Vec<Chunk>) -> Vec<Chunk> {
    // Group by file path
    let mut by_file: HashMap<String, Vec<Chunk>> = HashMap::new();
    for chunk in chunks {
        by_file
            .entry(chunk.file_path.clone())
            .or_default()
            .push(chunk);
    }

    let mut result = Vec::new();

    for (file_path, mut file_chunks) in by_file {
        file_chunks.sort_by_key(|c| c.start_line);

        let mut merged: Vec<Chunk> = Vec::new();

        for chunk in file_chunks {
            if let Some(last) = merged.last_mut() {
                // Merge only if truly overlapping or directly adjacent (no gap).
                // We cannot merge across gaps because we don't have the gap lines'
                // content — that would create a mismatch between line range metadata
                // and actual content.
                if chunk.start_line <= last.end_line + 1 {
                    if chunk.end_line > last.end_line {
                        // Extend content: append only the new lines
                        let overlap = last.end_line.saturating_sub(chunk.start_line.saturating_sub(1));
                        let new_lines: Vec<&str> = chunk
                            .content
                            .lines()
                            .skip(overlap)
                            .collect();
                        if !new_lines.is_empty() {
                            last.content.push('\n');
                            last.content.push_str(&new_lines.join("\n"));
                        }
                        last.end_line = chunk.end_line;
                    }
                    last.score = last.score.max(chunk.score);
                    continue;
                }
            }
            merged.push(Chunk {
                file_path: file_path.clone(),
                ..chunk
            });
        }

        result.extend(merged);
    }

    // Sort by score descending
    result.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    result
}
