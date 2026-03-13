# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
