# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.5.1] - 2026-03-14

### Added
- `usage.md` quick-reference guide covering all subcommands, flags, output modes, and usage patterns
- Path filter edge-case tests: spaces and Unicode/emoji in directory names, exact glob segment matching, and double-star patterns through multi-level space-separated paths
- CLI regression tests: `--language` change triggers automatic index rebuild; `--grep` filter is not silently dropped when the target file has a low BM25 rank

## [0.5.0] - 2026-03-14

### Added
- `grep` subcommand: pure regex search over corpus files without requiring a search index
- `--include` / `--exclude` glob pattern flags for path filtering in both `search` and `grep` subcommands
- `--sort` flag (score/path/mtime) for ordering results in `files` and `summary` output modes
- `--language` flag on the `index` subcommand to select the Snowball stemmer language (pl, en, de, fr, es, it, pt, ru, sv, nl, fi, da, hu, ro, tr, or none); defaults to `pl` for backward compatibility
- Hybrid search mode: `--grep` regex filter applied on top of BM25 results in `search` subcommand
- `src/filter.rs` glob-based path filtering module backed by `globset`

### Changed
- Stemmer is now configurable per index instead of being hardcoded to Polish; language choice is persisted in the index manifest
- `search` and `grep` results in `files`/`summary` modes respect the `--sort` order before truncation

### Fixed
- Score threshold pre-filtering removed from chunk search; all candidates are ranked and truncated by `--max-results` after filtering, preventing premature result drops

## [0.4.0] - 2026-03-13

### Added
- `--mode` flag with three output modes: `chunks` (default), `files` (paths + scores only), and `summary` (directory-grouped counts)
- Multi-query support: pass multiple `-q` flags to search for several terms at once, results merged by relevance
- `--max-results` flag to cap the number of results returned
- `search_files` and `search_files_multi` functions for lightweight file-path-only retrieval
- `summarize_by_directory` function grouping file matches by parent directory
- `src/lib.rs` exposing public module API for library consumers
- Integration test suite covering output modes and bug regression scenarios

### Fixed
- Chunk deduplication now merges only truly adjacent or overlapping fragments; gaps are preserved to prevent line-range metadata mismatches
- Index directory excluded from corpus scan to prevent self-indexing
- File modification time tracked at millisecond precision to detect rapid successive edits
- Query term highlighting uses stemmed matching, consistent with the Tantivy analyzer pipeline

## [0.3.1] - 2026-03-12

### Added
- LICENSE file with Commons Clause + MIT terms

### Changed
- License updated from MIT to Commons Clause + MIT, restricting commercial resale
- README updated with PIY (Prompt It Yourself) attribution and correct license reference

## [0.3.0] - 2026-03-12

### Added
- Full GrepRAG lexical search engine with BM25 ranking via Tantivy
- `index` subcommand to build a full-text search index from a corpus directory
- `search` subcommand returning ranked JSON fragments ready for LLM context injection
- Incremental index updates using a manifest (mtime + size tracking); only changed files are re-indexed
- Density-based chunk scoring: chunks with higher query-term density score higher
- Structure-aware deduplication merging overlapping or adjacent fragments from the same file
- Token-budget truncation using cl100k_base tokenizer with partial-chunk support at line boundaries
- Polish Snowball stemmer integrated as a custom Tantivy token filter
- File path boosting (3x) so files whose names match the query rank higher
- Binary file filtering by extension to prevent non-text content from entering the index
- `.gitignore`-aware corpus scanning via the `ignore` crate
- Configurable context window (`--context-lines`), token budget (`--token-budget`), and index path (`--index-dir`)
- `--force` / `--reindex` flags for triggering a full index rebuild
- Structured JSON output with numbered source references (`[1] file.md:15-42`)
- README with usage examples, performance benchmarks, and dependency table
- Project specification document (GrepRAG algorithm description)
- Claude command: `install.md` for tooling setup

### Changed
- `.gitignore` extended with Cargo-managed `/target` exclusion
