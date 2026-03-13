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

#[derive(Debug, Clone, Serialize)]
pub struct FileMatch {
    pub path: String,
    pub score: f32,
}

#[derive(Debug, Serialize)]
pub struct FilesOutput {
    pub query: String,
    pub total_files: usize,
    pub files: Vec<FileMatch>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DirGroup {
    pub directory: String,
    pub count: usize,
    pub top_score: f32,
    pub files: Vec<FileMatch>,
}

#[derive(Debug, Serialize)]
pub struct SummaryOutput {
    pub query: String,
    pub total_files: usize,
    pub directories: Vec<DirGroup>,
}

#[derive(Debug, Clone, Default)]
pub enum SortOrder {
    #[default]
    Score,
    Path,
    Mtime,
}
