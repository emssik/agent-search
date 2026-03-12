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
                // Merge if overlapping or gap <= 3 lines
                if chunk.start_line <= last.end_line + 4 {
                    if chunk.end_line > last.end_line {
                        // Extend content: append only the new lines
                        let new_lines: Vec<&str> = chunk
                            .content
                            .lines()
                            .skip(last.end_line.saturating_sub(chunk.start_line - 1))
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
