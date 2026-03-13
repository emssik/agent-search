# agent-search

> This project is published as part of [PIY — Prompt It Yourself](https://blog.atdpath.com/piy), a personal initiative launched on March 8, 2026, encouraging others to build their own apps with the help of LLMs.

Vector-free search engine for AI agents. Indexes a corpus of text files and returns the most relevant fragments as JSON, ready to inject into an LLM context.

Built on the **GrepRAG** architecture — lexical search with BM25 ranking, deduplication, and token budget truncation.

## How it works

```
Corpus → Tantivy Index → BM25 Search → Chunk Extraction → Dedup → Token Truncation → JSON
```

1. **Indexing** — scans a directory (respects `.gitignore`), tokenizes with a Polish Snowball stemmer, writes a Tantivy index
2. **BM25 Search** — document ranking with file path boosting (x3)
3. **Chunk Extraction** — extracts fragments with context around hits, density-based scoring per chunk
4. **Deduplication** — merges overlapping/adjacent fragments from the same file
5. **Truncation** — trims results to a token budget (default 4096), with source references `[1]`, `[2]`...

## Installation

```bash
cargo build --release
```

Binary: `target/release/agent-search`

## Usage

### Indexing

```bash
# Build index (incremental, or full rebuild with --force)
agent-search index -c /path/to/corpus
agent-search index -c /path/to/corpus --force
```

### Searching

```bash
# Default (chunks mode) — returns content fragments with context
agent-search search -c ./corpus -q "authentication flow"

# Files mode — returns only file paths + scores (no content, minimal tokens)
agent-search search -c ./corpus --mode files -q "Spring Security"

# Summary mode — groups results by directory with file counts
agent-search search -c ./corpus --mode summary -q "Docker"

# Multi-query — searches for multiple terms, merges results
agent-search search -c ./corpus -q "MockMvc" -q "TestRestTemplate"

# Limit results
agent-search search -c ./corpus --mode files -q "test" --max-results 5

# Custom chunk parameters
agent-search search -c ./corpus -q "query" \
  --context-lines 15 \
  --token-budget 8192

# Rebuild index before searching
agent-search search -c ./corpus -q "error handling" --reindex
```

### Output modes

**`--mode chunks`** (default) — full content fragments with context lines, deduplication, and token budget truncation. Best for injecting into LLM context.

**`--mode files`** — file paths and BM25 scores only. No content extraction. Use when you need to know *which* files are relevant without spending tokens on content.

**`--mode summary`** — file matches grouped by parent directory with counts and top scores. Use to understand *where* in the codebase a topic lives.

### Multi-query

Pass multiple `-q` flags to search for several terms at once. Results are merged by file path (highest score wins for files/summary modes) or deduplicated by position (for chunks mode).

```bash
agent-search search -c ./corpus -q "authenticate" -q "authorize" --mode files
```

### Options

| Flag | Default | Description |
|---|---|---|
| `--mode` | `chunks` | Output mode: `chunks`, `files`, or `summary` |
| `--max-results` | 100 | Maximum number of results |
| `--context-lines` | 10 | Lines of context around each hit (chunks mode) |
| `--token-budget` | 4096 | Maximum token count in output (chunks mode) |
| `--index-dir` | `<corpus>/.agent-search-index` | Index directory |
| `--force` / `--reindex` | — | Force full index rebuild |

### Output examples

**Chunks mode** (default):

```json
{
  "query": "authentication flow",
  "total_candidates": 12,
  "returned_chunks": 3,
  "token_count": 1847,
  "sources": [
    "[1] src/auth.rs:15-42",
    "[2] src/middleware.rs:88-103"
  ],
  "chunks": [
    {
      "source_id": "[1]",
      "file_path": "src/auth.rs",
      "start_line": 15,
      "end_line": 42,
      "content": "...",
      "score": 8.42
    }
  ]
}
```

**Files mode**:

```json
{
  "query": "authentication",
  "total_files": 3,
  "files": [
    { "path": "src/auth.rs", "score": 8.42 },
    { "path": "src/middleware.rs", "score": 5.11 },
    { "path": "tests/auth_test.rs", "score": 3.87 }
  ]
}
```

**Summary mode**:

```json
{
  "query": "authentication",
  "total_files": 3,
  "directories": [
    {
      "directory": "src",
      "count": 2,
      "top_score": 8.42,
      "files": [
        { "path": "src/auth.rs", "score": 8.42 },
        { "path": "src/middleware.rs", "score": 5.11 }
      ]
    },
    {
      "directory": "tests",
      "count": 1,
      "top_score": 3.87,
      "files": [
        { "path": "tests/auth_test.rs", "score": 3.87 }
      ]
    }
  ]
}
```

## Performance

| Operation | Time |
|---|---|
| Full indexing (1400 Markdown files) | ~0.5s |
| Incremental update (only new/changed) | near-instant |
| Search | ~50ms |

Benchmarked on a Mac Studio M2 with an Obsidian vault of ~1400 notes.

## Dependencies

| Crate | Role |
|---|---|
| `tantivy` | Full-text search engine (BM25) |
| `snowball_stemmers_rs` | Polish stemmer (Snowball algorithm) |
| `tiktoken-rs` | Token counting (cl100k_base) |
| `ignore` | File scanning with `.gitignore` support |
| `clap` | CLI |
| `serde` / `serde_json` | JSON serialization |
| `anyhow` | Error handling |

## Incremental indexing

The index is updated incrementally based on a manifest (`manifest.json`) that stores `mtime` and `size` for each file. On subsequent runs, only new/changed files are indexed and deleted files are removed from the index.

## License

Commons Clause + MIT — see [LICENSE](LICENSE)
