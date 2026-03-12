use std::sync::OnceLock;

use anyhow::Result;
use tiktoken_rs::{cl100k_base, CoreBPE};

use crate::types::Chunk;

fn tokenizer() -> &'static CoreBPE {
    static BPE: OnceLock<CoreBPE> = OnceLock::new();
    BPE.get_or_init(|| cl100k_base().expect("failed to load cl100k_base tokenizer"))
}

/// Truncate chunks to fit within a token budget.
/// Chunks should be pre-sorted by score (descending).
/// Returns (kept_chunks, total_token_count).
pub fn truncate_to_budget(chunks: Vec<Chunk>, token_budget: usize) -> Result<(Vec<Chunk>, usize)> {
    let bpe = tokenizer();
    let mut kept = Vec::new();
    let mut total_tokens = 0usize;

    for chunk in chunks {
        let chunk_tokens = bpe.encode_ordinary(&chunk.content).len();

        if total_tokens + chunk_tokens > token_budget {
            // Try to include a partial chunk (cut at line boundary)
            let remaining = token_budget.saturating_sub(total_tokens);
            if remaining > 50 {
                // Worth including a partial chunk
                let lines: Vec<&str> = chunk.content.lines().collect();
                let mut partial_content = String::new();
                let mut partial_tokens = 0;
                let mut last_line_idx = 0;

                for (i, line) in lines.iter().enumerate() {
                    let line_tokens = bpe.encode_ordinary(line).len() + 1; // +1 for newline
                    if partial_tokens + line_tokens > remaining {
                        break;
                    }
                    if !partial_content.is_empty() {
                        partial_content.push('\n');
                    }
                    partial_content.push_str(line);
                    partial_tokens += line_tokens;
                    last_line_idx = i;
                }

                if !partial_content.is_empty() {
                    total_tokens += partial_tokens;
                    kept.push(Chunk {
                        end_line: chunk.start_line + last_line_idx,
                        content: partial_content,
                        ..chunk
                    });
                }
            }
            break;
        }

        total_tokens += chunk_tokens;
        kept.push(chunk);
    }

    Ok((kept, total_tokens))
}
