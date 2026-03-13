mod dedup;
mod index;
mod search;
mod stemmer;
mod truncate;
mod types;

use anyhow::{bail, Result};
use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

use types::{FilesOutput, SearchOutput, SummaryOutput};

#[derive(Debug, Clone, Default, ValueEnum)]
enum OutputMode {
    /// Return content chunks (default)
    #[default]
    Chunks,
    /// Return only file paths and scores
    Files,
    /// Return directory-grouped summary
    Summary,
}

#[derive(Parser)]
#[command(name = "agent-search", about = "GrepRAG lexical search agent")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Build search index from a corpus directory
    Index {
        /// Path to the corpus directory
        #[arg(short, long)]
        corpus: PathBuf,

        /// Path to store the index (default: <corpus>/.agent-search-index)
        #[arg(short, long)]
        index_dir: Option<PathBuf>,

        /// Force full rebuild (ignore manifest)
        #[arg(long)]
        force: bool,
    },
    /// Search the indexed corpus
    Search {
        /// Path to the corpus directory
        #[arg(short, long)]
        corpus: PathBuf,

        /// Search query (can be specified multiple times for multi-query)
        #[arg(short, long, num_args = 1..)]
        query: Vec<String>,

        /// Lines of context around each match
        #[arg(long, default_value = "10")]
        context_lines: usize,

        /// Maximum tokens in output
        #[arg(long, default_value = "4096")]
        token_budget: usize,

        /// Path to the index directory (default: <corpus>/.agent-search-index)
        #[arg(short, long)]
        index_dir: Option<PathBuf>,

        /// Rebuild index before searching
        #[arg(long)]
        reindex: bool,

        /// Output mode: chunks (default), files, or summary
        #[arg(long, value_enum, default_value = "chunks")]
        mode: OutputMode,

        /// Maximum number of results
        #[arg(long, default_value = "100")]
        max_results: usize,
    },
}

fn resolve_index_path(corpus: &PathBuf, index_dir: &Option<PathBuf>) -> PathBuf {
    index_dir
        .clone()
        .unwrap_or_else(|| corpus.join(".agent-search-index"))
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Index {
            corpus,
            index_dir,
            force,
        } => {
            if !corpus.is_dir() {
                bail!(
                    "Corpus path does not exist or is not a directory: {}",
                    corpus.display()
                );
            }
            let idx_path = resolve_index_path(&corpus, &index_dir);
            if force || !idx_path.exists() {
                index::build_index(&corpus, &idx_path)?;
            } else {
                index::update_index(&corpus, &idx_path)?;
            }
        }
        Commands::Search {
            corpus,
            query,
            context_lines,
            token_budget,
            index_dir,
            reindex,
            mode,
            max_results,
        } => {
            let idx_path = resolve_index_path(&corpus, &index_dir);

            if !corpus.is_dir() {
                bail!("Corpus path does not exist: {}", corpus.display());
            }

            let idx = if reindex || !idx_path.exists() {
                eprintln!("Building index...");
                index::build_index(&corpus, &idx_path)?
            } else {
                let (index, _changed) = index::update_index(&corpus, &idx_path)?;
                index
            };

            let query_display = query.join(" | ");
            let query_refs: Vec<&str> = query.iter().map(|s| s.as_str()).collect();

            match mode {
                OutputMode::Files => {
                    let files = if query_refs.len() == 1 {
                        search::search_files(&idx, query_refs[0], max_results)?
                    } else {
                        search::search_files_multi(&idx, &query_refs, max_results)?
                    };
                    let output = FilesOutput {
                        query: query_display,
                        total_files: files.len(),
                        files,
                    };
                    println!("{}", serde_json::to_string_pretty(&output)?);
                }
                OutputMode::Summary => {
                    let files = if query_refs.len() == 1 {
                        search::search_files(&idx, query_refs[0], max_results)?
                    } else {
                        search::search_files_multi(&idx, &query_refs, max_results)?
                    };
                    let total_files = files.len();
                    let directories = search::summarize_by_directory(files);
                    let output = SummaryOutput {
                        query: query_display,
                        total_files,
                        directories,
                    };
                    println!("{}", serde_json::to_string_pretty(&output)?);
                }
                OutputMode::Chunks => {
                    let chunks = if query_refs.len() == 1 {
                        search::search(&idx, query_refs[0], context_lines, max_results)?
                    } else {
                        search::search_multi(&idx, &query_refs, context_lines, max_results)?
                    };
                    let total_candidates = chunks.len();

                    let merged = dedup::merge_chunks(chunks);
                    let (mut final_chunks, token_count) =
                        truncate::truncate_to_budget(merged, token_budget)?;

                    let mut sources = Vec::new();
                    for (i, chunk) in final_chunks.iter_mut().enumerate() {
                        let id = format!("[{}]", i + 1);
                        let reference = format!(
                            "{} {}:{}-{}",
                            id, chunk.file_path, chunk.start_line, chunk.end_line
                        );
                        chunk.source_id = id;
                        sources.push(reference);
                    }

                    let output = SearchOutput {
                        query: query_display,
                        total_candidates,
                        returned_chunks: final_chunks.len(),
                        token_count,
                        sources,
                        chunks: final_chunks,
                    };
                    println!("{}", serde_json::to_string_pretty(&output)?);
                }
            }
        }
    }

    Ok(())
}
