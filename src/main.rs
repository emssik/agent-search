mod dedup;
mod index;
mod search;
mod stemmer;
mod truncate;
mod types;

use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

use types::SearchOutput;

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

        /// Search query
        #[arg(short, long)]
        query: String,

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

            // Step 2+3: Search with BM25 ranking + context extraction
            let chunks = search::search(&idx, &query, context_lines)?;
            let total_candidates = chunks.len();

            // Step 4: Structure-aware deduplication
            let merged = dedup::merge_chunks(chunks);

            // Step 5: Truncate to token budget
            let (mut final_chunks, token_count) =
                truncate::truncate_to_budget(merged, token_budget)?;

            // Assign source IDs and build references
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
                query,
                total_candidates,
                returned_chunks: final_chunks.len(),
                token_count,
                sources,
                chunks: final_chunks,
            };

            println!("{}", serde_json::to_string_pretty(&output)?);
        }
    }

    Ok(())
}
