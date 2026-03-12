use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Chunk {
    pub source_id: String,
    pub file_path: String,
    pub start_line: usize,
    pub end_line: usize,
    pub content: String,
    pub score: f32,
}

#[derive(Debug, Serialize)]
pub struct SearchOutput {
    pub query: String,
    pub total_candidates: usize,
    pub returned_chunks: usize,
    pub token_count: usize,
    pub sources: Vec<String>,
    pub chunks: Vec<Chunk>,
}
