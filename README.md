# agent-search

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
# Search with automatic index update
agent-search search -c /path/to/corpus -q "authentication flow"

# With index rebuild
agent-search search -c /path/to/corpus -q "error handling" --reindex

# Custom parameters
agent-search search -c /path/to/corpus -q "query" \
  --context-lines 15 \
  --token-budget 8192
```

### Options

| Flag | Default | Description |
|---|---|---|
| `--context-lines` | 10 | Lines of context around each hit |
| `--token-budget` | 4096 | Maximum token count in output |
| `--index-dir` | `<corpus>/.agent-search-index` | Index directory |
| `--force` / `--reindex` | — | Force full index rebuild |

### Output (JSON)

```json
{
  "query": "authentication flow",
  "total_candidates": 12,
  "returned_chunks": 3,
  "token_count": 1847,
  "sources": [
    "[1] src/auth.rs:15-42",
    "[2] src/middleware.rs:88-103",
    "[3] src/config.rs:1-20"
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

MIT
