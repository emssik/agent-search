mod dedup;
mod filter;
mod grep;
mod index;
mod search;
mod stemmer;
mod truncate;
mod types;

use anyhow::{Result, bail};
use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

use filter::PathFilter;
use types::{FilesOutput, SearchOutput, SortOrder, SummaryOutput};

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

#[derive(Debug, Clone, Default, ValueEnum)]
enum CliSortOrder {
    /// Sort by relevance score (default)
    #[default]
    Score,
    /// Sort alphabetically by file path
    Path,
    /// Sort by modification time (newest first)
    Mtime,
}

impl From<CliSortOrder> for SortOrder {
    fn from(s: CliSortOrder) -> Self {
        match s {
            CliSortOrder::Score => SortOrder::Score,
            CliSortOrder::Path => SortOrder::Path,
            CliSortOrder::Mtime => SortOrder::Mtime,
        }
    }
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

        /// Stemmer language (default: pl)
        #[arg(long)]
        language: Option<String>,
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

        /// Regex pattern to filter BM25 results (hybrid mode)
        #[arg(long)]
        grep: Option<String>,

        /// Include only files matching these glob patterns
        #[arg(long)]
        include: Vec<String>,

        /// Exclude files matching these glob patterns
        #[arg(long)]
        exclude: Vec<String>,

        /// Sort order for results (files/summary modes)
        #[arg(long, value_enum, default_value = "score")]
        sort: CliSortOrder,
    },
    /// Pure regex search over corpus files (no index required)
    Grep {
        /// Path to the corpus directory
        #[arg(short, long)]
        corpus: PathBuf,

        /// Regex pattern to search for
        #[arg(short, long)]
        pattern: String,

        /// Lines of context around each match
        #[arg(long, default_value = "2")]
        context_lines: usize,

        /// Maximum number of results
        #[arg(long, default_value = "100")]
        max_results: usize,

        /// Output mode: chunks (default), files, or summary
        #[arg(long, value_enum, default_value = "chunks")]
        mode: OutputMode,

        /// Maximum tokens in output
        #[arg(long, default_value = "4096")]
        token_budget: usize,

        /// Include only files matching these glob patterns
        #[arg(long)]
        include: Vec<String>,

        /// Exclude files matching these glob patterns
        #[arg(long)]
        exclude: Vec<String>,

        /// Sort order for results (files/summary modes)
        #[arg(long, value_enum, default_value = "score")]
        sort: CliSortOrder,
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
            language,
        } => {
            if !corpus.is_dir() {
                bail!(
                    "Corpus path does not exist or is not a directory: {}",
                    corpus.display()
                );
            }
            let idx_path = resolve_index_path(&corpus, &index_dir);
            if force || !idx_path.exists() {
                let lang = match language {
                    Some(lang) => {
                        index::resolve_language(&lang)?;
                        lang
                    }
                    None => {
                        if idx_path.exists() {
                            index::read_index_language(&idx_path)
                        } else {
                            "pl".to_string()
                        }
                    }
                };
                index::build_index(&corpus, &idx_path, &lang)?;
            } else {
                if let Some(lang) = language {
                    index::resolve_language(&lang)?;
                    let current_lang = index::read_index_language(&idx_path);
                    if current_lang != lang {
                        eprintln!(
                            "Language changed from '{}' to '{}', rebuilding index...",
                            current_lang, lang
                        );
                        index::build_index(&corpus, &idx_path, &lang)?;
                    } else {
                        index::update_index(&corpus, &idx_path)?;
                    }
                } else {
                    index::update_index(&corpus, &idx_path)?;
                }
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
            grep: grep_pattern,
            include,
            exclude,
            sort,
        } => {
            let idx_path = resolve_index_path(&corpus, &index_dir);
            let path_filter = PathFilter::new(&include, &exclude)?;
            let sort_order: SortOrder = sort.into();

            if !corpus.is_dir() {
                bail!("Corpus path does not exist: {}", corpus.display());
            }

            let idx = if reindex || !idx_path.exists() {
                eprintln!("Building index...");
                let lang = if idx_path.exists() {
                    index::read_index_language(&idx_path)
                } else {
                    "pl".to_string()
                };
                index::build_index(&corpus, &idx_path, &lang)?
            } else {
                let (index, _changed) = index::update_index(&corpus, &idx_path)?;
                index
            };

            // Read language from manifest for stemmer
            let lang = index::read_index_language(&idx_path);
            let algorithm = index::resolve_language(&lang)?;

            let query_display = query.join(" | ");
            let query_refs: Vec<&str> = query.iter().map(|s| s.as_str()).collect();

            // Validate grep regex early (fail fast)
            let grep_regex = match &grep_pattern {
                Some(pat) => Some(grep::validate_pattern(pat)?),
                None => None,
            };
            let candidate_max_results = if grep_regex.is_some() {
                idx.reader()?.searcher().num_docs() as usize
            } else {
                max_results
            };
            let candidate_max_results = candidate_max_results.max(max_results);

            match mode {
                OutputMode::Files => {
                    let files = if query_refs.len() == 1 {
                        search::search_files(
                            &idx,
                            query_refs[0],
                            candidate_max_results,
                            &path_filter,
                        )?
                    } else {
                        search::search_files_multi(
                            &idx,
                            &query_refs,
                            candidate_max_results,
                            &path_filter,
                        )?
                    };
                    let mut files = match &grep_regex {
                        Some(re) => grep::filter_files_by_regex(files, re, &corpus),
                        None => files,
                    };
                    search::sort_file_matches(&mut files, &sort_order, &corpus);
                    files.truncate(max_results);
                    let output = FilesOutput {
                        query: query_display,
                        total_files: files.len(),
                        files,
                    };
                    println!("{}", serde_json::to_string_pretty(&output)?);
                }
                OutputMode::Summary => {
                    let files = if query_refs.len() == 1 {
                        search::search_files(
                            &idx,
                            query_refs[0],
                            candidate_max_results,
                            &path_filter,
                        )?
                    } else {
                        search::search_files_multi(
                            &idx,
                            &query_refs,
                            candidate_max_results,
                            &path_filter,
                        )?
                    };
                    let files = match &grep_regex {
                        Some(re) => grep::filter_files_by_regex(files, re, &corpus),
                        None => files,
                    };
                    let mut files = files;
                    search::sort_file_matches(&mut files, &sort_order, &corpus);
                    files.truncate(max_results);
                    let total_files = files.len();
                    let mut directories = search::summarize_by_directory(files);
                    search::sort_dir_groups(&mut directories, &sort_order, &corpus);
                    let output = SummaryOutput {
                        query: query_display,
                        total_files,
                        directories,
                    };
                    println!("{}", serde_json::to_string_pretty(&output)?);
                }
                OutputMode::Chunks => {
                    let chunks = if query_refs.len() == 1 {
                        search::search(
                            &idx,
                            query_refs[0],
                            context_lines,
                            candidate_max_results,
                            &path_filter,
                            algorithm,
                        )?
                    } else {
                        search::search_multi(
                            &idx,
                            &query_refs,
                            context_lines,
                            candidate_max_results,
                            &path_filter,
                            algorithm,
                        )?
                    };
                    let mut chunks = match &grep_regex {
                        Some(re) => grep::filter_chunks_by_regex(chunks, re),
                        None => chunks,
                    };
                    chunks.sort_by(|a, b| {
                        b.score
                            .partial_cmp(&a.score)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    });
                    chunks.truncate(max_results);
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
        Commands::Grep {
            corpus,
            pattern,
            context_lines,
            max_results,
            mode,
            token_budget,
            include,
            exclude,
            sort,
        } => {
            if !corpus.is_dir() {
                bail!("Corpus path does not exist: {}", corpus.display());
            }

            // Validate regex early
            grep::validate_pattern(&pattern)?;

            let path_filter = PathFilter::new(&include, &exclude)?;
            let sort_order: SortOrder = sort.into();

            match mode {
                OutputMode::Files => {
                    let mut files = grep::grep_files(&corpus, &pattern, max_results, &path_filter)?;
                    search::sort_file_matches(&mut files, &sort_order, &corpus);
                    let output = FilesOutput {
                        query: pattern,
                        total_files: files.len(),
                        files,
                    };
                    println!("{}", serde_json::to_string_pretty(&output)?);
                }
                OutputMode::Summary => {
                    let mut files = grep::grep_files(&corpus, &pattern, max_results, &path_filter)?;
                    search::sort_file_matches(&mut files, &sort_order, &corpus);
                    let total_files = files.len();
                    let mut directories = search::summarize_by_directory(files);
                    search::sort_dir_groups(&mut directories, &sort_order, &corpus);
                    let output = SummaryOutput {
                        query: pattern,
                        total_files,
                        directories,
                    };
                    println!("{}", serde_json::to_string_pretty(&output)?);
                }
                OutputMode::Chunks => {
                    let chunks = grep::grep_chunks(
                        &corpus,
                        &pattern,
                        context_lines,
                        max_results,
                        &path_filter,
                    )?;
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
                        query: pattern,
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
