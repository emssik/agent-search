use agent_search::filter::PathFilter;
use agent_search::grep;
use agent_search::index;
use agent_search::search;

use std::fs;
use tempfile::TempDir;

fn setup_corpus(files: &[(&str, &str)]) -> TempDir {
    let tmp = TempDir::new().unwrap();
    let corpus = tmp.path();
    for (name, content) in files {
        let path = corpus.join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&path, content).unwrap();
    }
    tmp
}

fn setup_corpus_with_index(files: &[(&str, &str)]) -> (TempDir, tantivy::Index) {
    let tmp = setup_corpus(files);
    let corpus = tmp.path();
    let idx_dir = corpus.join(".agent-search-index");
    let idx = index::build_index(corpus, &idx_dir, "pl").unwrap();
    (tmp, idx)
}

fn no_filter() -> PathFilter {
    PathFilter::default()
}

// =============================================================================
// Krok 1: Pure grep — grep_files()
// =============================================================================

#[test]
fn test_grep_files_basic() {
    let tmp = setup_corpus(&[
        ("backup.sh", "rsync -avz /src /dest\necho done"),
        ("deploy.sh", "docker push myapp\nrsync logs/ /archive/"),
        ("readme.md", "This project uses docker for deployment"),
    ]);

    let results = grep::grep_files(tmp.path(), "rsync", 100, &no_filter()).unwrap();
    assert_eq!(results.len(), 2, "Should find 2 files with 'rsync'");

    let paths: Vec<&str> = results.iter().map(|f| f.path.as_str()).collect();
    assert!(paths.contains(&"backup.sh"));
    assert!(paths.contains(&"deploy.sh"));

    // Score = number of matching lines
    for fm in &results {
        assert!(fm.score >= 1.0);
    }
}

#[test]
fn test_grep_files_no_matches() {
    let tmp = setup_corpus(&[("hello.txt", "hello world"), ("foo.rs", "fn main() {}")]);

    let results = grep::grep_files(tmp.path(), "xyznonexistent", 100, &no_filter()).unwrap();
    assert!(results.is_empty());
}

#[test]
fn test_grep_files_max_results() {
    let files: Vec<(String, String)> = (0..10)
        .map(|i| {
            (
                format!("file{}.txt", i),
                format!("match_keyword line {}", i),
            )
        })
        .collect();
    let file_refs: Vec<(&str, &str)> = files
        .iter()
        .map(|(a, b)| (a.as_str(), b.as_str()))
        .collect();
    let tmp = setup_corpus(&file_refs);

    let results = grep::grep_files(tmp.path(), "match_keyword", 3, &no_filter()).unwrap();
    assert!(
        results.len() <= 3,
        "Should return at most 3 results, got {}",
        results.len()
    );
}

// =============================================================================
// Krok 1: Pure grep — grep_chunks()
// =============================================================================

#[test]
fn test_grep_chunks_with_context() {
    let lines: Vec<String> = (1..=20).map(|i| format!("line {}", i)).collect();
    // Put a match keyword on line 10 (0-indexed: 9)
    let mut lines = lines;
    lines[9] = "MATCH_TARGET here".to_string();
    let content = lines.join("\n");

    let tmp = setup_corpus(&[("data.txt", &content)]);

    let chunks = grep::grep_chunks(tmp.path(), "MATCH_TARGET", 2, 100, &no_filter()).unwrap();
    assert_eq!(chunks.len(), 1);

    let chunk = &chunks[0];
    assert_eq!(chunk.file_path, "data.txt");
    // 0-indexed line 9, context_lines=2 → window [7, 12) → start_line=8, end_line=12
    assert_eq!(chunk.start_line, 8);
    assert_eq!(chunk.end_line, 12);
    assert!(chunk.content.contains("MATCH_TARGET"));
}

#[test]
fn test_grep_chunks_merges_nearby() {
    let lines: Vec<String> = (1..=20).map(|i| format!("line {}", i)).collect();
    let mut lines = lines;
    // Matches on lines 5 and 7 (0-indexed: 4 and 6)
    lines[4] = "MATCH_A here".to_string();
    lines[6] = "MATCH_A also".to_string();
    let content = lines.join("\n");

    let tmp = setup_corpus(&[("data.txt", &content)]);

    let chunks = grep::grep_chunks(tmp.path(), "MATCH_A", 2, 100, &no_filter()).unwrap();
    // With context_lines=2: window for line 4 is [2,7), window for line 6 is [4,9)
    // These overlap → should be merged into 1 chunk
    assert_eq!(
        chunks.len(),
        1,
        "Should merge overlapping windows into 1 chunk"
    );
    assert!(chunks[0].content.contains("MATCH_A here"));
    assert!(chunks[0].content.contains("MATCH_A also"));
}

#[test]
fn test_grep_chunks_multiple_files() {
    let tmp = setup_corpus(&[
        ("a.txt", "keyword found here"),
        ("b.txt", "another keyword line"),
        ("c.txt", "no match here"),
    ]);

    let chunks = grep::grep_chunks(tmp.path(), "keyword", 0, 100, &no_filter()).unwrap();
    assert_eq!(chunks.len(), 2, "Should find chunks in 2 files");

    let paths: Vec<&str> = chunks.iter().map(|c| c.file_path.as_str()).collect();
    assert!(paths.contains(&"a.txt"));
    assert!(paths.contains(&"b.txt"));
}

#[test]
fn test_grep_invalid_regex() {
    let tmp = setup_corpus(&[("a.txt", "content")]);

    let result = grep::grep_files(tmp.path(), "[bad", 100, &no_filter());
    assert!(result.is_err(), "Should return error for invalid regex");

    let result = grep::grep_chunks(tmp.path(), "[bad", 2, 100, &no_filter());
    assert!(result.is_err(), "Should return error for invalid regex");
}

#[test]
fn test_grep_skips_binary() {
    let tmp = setup_corpus(&[
        ("image.png", "keyword hidden in fake png"),
        ("real.txt", "keyword in real text"),
    ]);

    let results = grep::grep_files(tmp.path(), "keyword", 100, &no_filter()).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].path, "real.txt");
}

#[test]
fn test_grep_respects_gitignore() {
    let tmp = setup_corpus(&[
        ("build/output.txt", "keyword in build output"),
        ("src/main.rs", "keyword in source"),
    ]);

    // Initialize git repo so ignore crate respects .gitignore
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    fs::write(tmp.path().join(".gitignore"), "build/\n").unwrap();

    let results = grep::grep_files(tmp.path(), "keyword", 100, &no_filter()).unwrap();
    let paths: Vec<&str> = results.iter().map(|f| f.path.as_str()).collect();
    assert!(
        !paths.iter().any(|p| p.contains("build/")),
        "Should skip gitignored files"
    );
    assert!(paths.contains(&"src/main.rs"));
}

#[test]
fn test_grep_context_zero() {
    let content = "line 1\nMATCH here\nline 3\nMATCH again\nline 5";
    let tmp = setup_corpus(&[("data.txt", content)]);

    let chunks = grep::grep_chunks(tmp.path(), "MATCH", 0, 100, &no_filter()).unwrap();
    // context_lines=0: each match line is its own window, no merging unless adjacent
    // Lines 1 and 3 (0-indexed) → windows [1,2) and [3,4) → not adjacent → 2 chunks
    assert_eq!(
        chunks.len(),
        2,
        "Should return 2 separate chunks with context_lines=0"
    );
    for chunk in &chunks {
        assert!(chunk.content.contains("MATCH"));
    }
}

#[test]
fn test_grep_case_sensitive() {
    let tmp = setup_corpus(&[(
        "data.txt",
        "Backup is important\nbackup your files\nBACKUP NOW",
    )]);

    // Case-sensitive: "Backup" matches only first line
    let results = grep::grep_files(tmp.path(), "Backup", 100, &no_filter()).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(
        results[0].score, 1.0,
        "Only 1 line matches case-sensitive 'Backup'"
    );

    // Case-insensitive via regex flag
    let results = grep::grep_files(tmp.path(), "(?i)Backup", 100, &no_filter()).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].score, 3.0, "All 3 lines match case-insensitive");
}

// =============================================================================
// Krok 2: Hybrid mode — --grep on search
// =============================================================================

#[test]
fn test_hybrid_chunks_filters() {
    let (_tmp, idx) = setup_corpus_with_index(&[
        (
            "backup_rsync.sh",
            "backup files using rsync\nrsync -avz /src /dest",
        ),
        (
            "backup_tar.sh",
            "backup files using tar\ntar czf backup.tar.gz /src",
        ),
    ]);

    // BM25 for "backup" should find both files
    let chunks = search::search(
        &idx,
        "backup",
        5,
        100,
        &no_filter(),
        Some(snowball_stemmers_rs::Algorithm::Polish),
    )
    .unwrap();
    assert!(chunks.len() >= 2, "BM25 should find chunks in both files");

    // Grep filter: only keep chunks with "rsync"
    let regex = grep::validate_pattern("rsync").unwrap();
    let filtered = grep::filter_chunks_by_regex(chunks, &regex);
    assert!(!filtered.is_empty(), "Should have results after filtering");
    for chunk in &filtered {
        assert!(
            chunk.content.contains("rsync"),
            "All chunks should contain 'rsync'"
        );
    }
}

#[test]
fn test_hybrid_files_filters() {
    let (tmp, idx) = setup_corpus_with_index(&[
        (
            "backup_rsync.sh",
            "backup files using rsync\nrsync -avz /src /dest",
        ),
        (
            "backup_tar.sh",
            "backup files using tar\ntar czf backup.tar.gz /src",
        ),
    ]);

    let files = search::search_files(&idx, "backup", 100, &no_filter()).unwrap();
    assert!(files.len() >= 2);

    let regex = grep::validate_pattern("rsync").unwrap();
    let filtered = grep::filter_files_by_regex(files, &regex, tmp.path());
    assert_eq!(filtered.len(), 1);
    assert!(filtered[0].path.contains("rsync"));
}

#[test]
fn test_hybrid_no_bm25_matches() {
    let (_tmp, idx) = setup_corpus_with_index(&[("hello.txt", "hello world")]);

    let chunks = search::search(
        &idx,
        "xyznonexistent",
        5,
        100,
        &no_filter(),
        Some(snowball_stemmers_rs::Algorithm::Polish),
    )
    .unwrap();
    assert!(chunks.is_empty());

    let regex = grep::validate_pattern("hello").unwrap();
    let filtered = grep::filter_chunks_by_regex(chunks, &regex);
    assert!(
        filtered.is_empty(),
        "No BM25 matches → no results even with grep"
    );
}

#[test]
fn test_hybrid_grep_matches_none() {
    let (_tmp, idx) = setup_corpus_with_index(&[("hello.txt", "hello world greetings")]);

    let chunks = search::search(
        &idx,
        "hello",
        5,
        100,
        &no_filter(),
        Some(snowball_stemmers_rs::Algorithm::Polish),
    )
    .unwrap();
    assert!(!chunks.is_empty(), "BM25 should find results");

    let regex = grep::validate_pattern("xyznonexistent").unwrap();
    let filtered = grep::filter_chunks_by_regex(chunks, &regex);
    assert!(filtered.is_empty(), "Grep matches nothing → empty results");
}

#[test]
fn test_hybrid_invalid_regex() {
    let result = grep::validate_pattern("[bad");
    assert!(result.is_err(), "Should fail fast on invalid regex");
}

#[test]
fn test_hybrid_summary_mode() {
    let (tmp, idx) = setup_corpus_with_index(&[
        ("src/auth.rs", "authenticate user with JWT token"),
        ("src/config.rs", "load configuration settings"),
        ("tests/auth_test.rs", "test JWT authentication flow"),
    ]);

    let files = search::search_files(&idx, "authenticate", 100, &no_filter()).unwrap();
    let regex = grep::validate_pattern("JWT").unwrap();
    let filtered = grep::filter_files_by_regex(files, &regex, tmp.path());

    // Only files containing "JWT" should remain
    for fm in &filtered {
        let content = fs::read_to_string(tmp.path().join(&fm.path)).unwrap();
        assert!(content.contains("JWT"), "Filtered file should contain JWT");
    }
}

// =============================================================================
// Krok 2b: Hybrid mode — anchor regex regression tests (^, $)
// =============================================================================

#[test]
fn test_hybrid_chunks_anchor_caret() {
    let (_tmp, idx) = setup_corpus_with_index(&[
        (
            "rsync_backup.sh",
            "#!/bin/bash\nrsync -avz /src /dest\necho done",
        ),
        (
            "tar_backup.sh",
            "#!/bin/bash\ntar czf backup.tar.gz\necho rsync is not used here",
        ),
    ]);

    // BM25 for "backup" should find both files
    let chunks = search::search(
        &idx,
        "backup",
        5,
        100,
        &no_filter(),
        Some(snowball_stemmers_rs::Algorithm::Polish),
    )
    .unwrap();
    assert!(chunks.len() >= 2, "BM25 should find chunks in both files");

    // ^rsync should only match lines STARTING with "rsync", not "echo rsync..."
    let regex = grep::validate_pattern("^rsync").unwrap();
    let filtered = grep::filter_chunks_by_regex(chunks, &regex);
    assert!(!filtered.is_empty(), "^rsync should match rsync_backup.sh");
    for chunk in &filtered {
        assert!(
            chunk.content.lines().any(|l| l.starts_with("rsync")),
            "Filtered chunk must have a line starting with 'rsync'"
        );
    }
}

#[test]
fn test_hybrid_files_anchor_caret() {
    let (tmp, idx) = setup_corpus_with_index(&[
        (
            "rsync_backup.sh",
            "#!/bin/bash\nrsync -avz /src /dest\necho done",
        ),
        (
            "tar_backup.sh",
            "#!/bin/bash\ntar czf backup.tar.gz\necho rsync is not used here",
        ),
    ]);

    let files = search::search_files(&idx, "backup", 100, &no_filter()).unwrap();
    assert!(files.len() >= 2);

    // ^rsync — per line, so rsync_backup.sh matches (has line starting with rsync)
    // tar_backup.sh does NOT match (rsync appears mid-line in "echo rsync...")
    let regex = grep::validate_pattern("^rsync").unwrap();
    let filtered = grep::filter_files_by_regex(files, &regex, tmp.path());
    assert_eq!(
        filtered.len(),
        1,
        "Only rsync_backup.sh should match ^rsync"
    );
    assert!(filtered[0].path.contains("rsync_backup"));
}

#[test]
fn test_hybrid_chunks_anchor_dollar() {
    let (_tmp, idx) = setup_corpus_with_index(&[
        ("ends_done.sh", "#!/bin/bash\necho processing\necho done"),
        ("mid_done.sh", "#!/bin/bash\necho done processing\nfinished"),
    ]);

    let chunks = search::search(
        &idx,
        "echo",
        5,
        100,
        &no_filter(),
        Some(snowball_stemmers_rs::Algorithm::Polish),
    )
    .unwrap();

    // done$ should match lines ENDING with "done"
    let regex = grep::validate_pattern("done$").unwrap();
    let filtered = grep::filter_chunks_by_regex(chunks, &regex);
    assert!(!filtered.is_empty(), "done$ should match ends_done.sh");
    for chunk in &filtered {
        assert!(
            chunk.content.lines().any(|l| l.ends_with("done")),
            "Filtered chunk must have a line ending with 'done'"
        );
    }
}

// =============================================================================
// Krok 3: Output format tests
// =============================================================================

#[test]
fn test_grep_cmd_chunks_output_format() {
    let tmp = setup_corpus(&[("data.txt", "line 1\nfind_me here\nline 3")]);

    let chunks = grep::grep_chunks(tmp.path(), "find_me", 1, 100, &no_filter()).unwrap();
    assert!(!chunks.is_empty());

    let chunk = &chunks[0];
    assert_eq!(chunk.file_path, "data.txt");
    assert!(chunk.start_line >= 1);
    assert!(chunk.end_line >= chunk.start_line);
    assert!(chunk.content.contains("find_me"));
    assert!(chunk.score > 0.0);
    // source_id is empty before main.rs assigns it
    assert_eq!(chunk.source_id, "");

    // Verify it serializes to expected JSON shape
    let json = serde_json::to_value(chunk).unwrap();
    assert!(json.get("file_path").is_some());
    assert!(json.get("start_line").is_some());
    assert!(json.get("end_line").is_some());
    assert!(json.get("content").is_some());
    assert!(json.get("score").is_some());
    assert!(json.get("source_id").is_some());
}

#[test]
fn test_grep_cmd_files_output_format() {
    let tmp = setup_corpus(&[("data.txt", "find_me here"), ("other.txt", "nothing here")]);

    let files = grep::grep_files(tmp.path(), "find_me", 100, &no_filter()).unwrap();
    assert_eq!(files.len(), 1);

    let fm = &files[0];
    assert_eq!(fm.path, "data.txt");
    assert!(fm.score > 0.0);

    let json = serde_json::to_value(fm).unwrap();
    assert!(json.get("path").is_some());
    assert!(json.get("score").is_some());
}

// =============================================================================
// Krok 4: --include / --exclude glob patterns (grep)
// =============================================================================

#[test]
fn test_grep_include_pattern() {
    let tmp = setup_corpus(&[
        ("docs/guide.md", "search keyword here"),
        ("docs/api.md", "keyword in api docs"),
        ("archive/old.md", "keyword in archive"),
        ("readme.md", "keyword in readme"),
    ]);

    let filter = PathFilter::new(&["docs/**".to_string()], &[]).unwrap();
    let results = grep::grep_files(tmp.path(), "keyword", 100, &filter).unwrap();

    assert_eq!(results.len(), 2, "Should find only 2 files in docs/");
    for fm in &results {
        assert!(
            fm.path.starts_with("docs/"),
            "Should only include docs/, got: {}",
            fm.path
        );
    }
}

#[test]
fn test_grep_exclude_pattern() {
    let tmp = setup_corpus(&[
        ("docs/guide.md", "search keyword here"),
        ("archive/old.md", "keyword in archive"),
        ("readme.md", "keyword in readme"),
    ]);

    let filter = PathFilter::new(&[], &["archive/**".to_string()]).unwrap();
    let results = grep::grep_files(tmp.path(), "keyword", 100, &filter).unwrap();

    for fm in &results {
        assert!(
            !fm.path.starts_with("archive/"),
            "Should exclude archive/, got: {}",
            fm.path
        );
    }
    assert_eq!(results.len(), 2, "Should find 2 files (docs + readme)");
}

#[test]
fn test_grep_include_exclude_combined() {
    let tmp = setup_corpus(&[
        ("docs/guide.md", "keyword here"),
        ("docs/internal/secret.md", "keyword secret"),
        ("archive/old.md", "keyword archive"),
    ]);

    let filter =
        PathFilter::new(&["docs/**".to_string()], &["docs/internal/**".to_string()]).unwrap();
    let results = grep::grep_files(tmp.path(), "keyword", 100, &filter).unwrap();

    assert_eq!(results.len(), 1, "Should find only docs/guide.md");
    assert_eq!(results[0].path, "docs/guide.md");
}

#[test]
fn test_grep_chunks_include() {
    let tmp = setup_corpus(&[
        ("docs/guide.md", "keyword in guide"),
        ("archive/old.md", "keyword in archive"),
    ]);

    let filter = PathFilter::new(&["docs/**".to_string()], &[]).unwrap();
    let chunks = grep::grep_chunks(tmp.path(), "keyword", 0, 100, &filter).unwrap();

    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0].file_path, "docs/guide.md");
}

// =============================================================================
// Krok 4b: --include with spaces and Unicode in paths
// =============================================================================

#[test]
fn test_grep_include_path_with_spaces() {
    let tmp = setup_corpus(&[
        ("my docs/guide.md", "keyword in guide"),
        ("my docs/nested/deep.md", "keyword deep"),
        ("archive/old.md", "keyword in archive"),
    ]);

    let filter = PathFilter::new(&["my docs/**".to_string()], &[]).unwrap();
    let results = grep::grep_files(tmp.path(), "keyword", 100, &filter).unwrap();

    assert_eq!(results.len(), 2, "Should find 2 files in 'my docs/'");
    for fm in &results {
        assert!(
            fm.path.starts_with("my docs/"),
            "Should only include 'my docs/', got: {}",
            fm.path
        );
    }
}

#[test]
fn test_grep_include_path_with_emoji() {
    let tmp = setup_corpus(&[
        ("🚦 project/notes.md", "keyword in notes"),
        ("🚦 project/sub/deep.md", "keyword deep"),
        ("other/file.md", "keyword in other"),
    ]);

    let filter = PathFilter::new(&["🚦 project/**".to_string()], &[]).unwrap();
    let results = grep::grep_files(tmp.path(), "keyword", 100, &filter).unwrap();

    assert_eq!(results.len(), 2, "Should find 2 files in emoji dir");
    for fm in &results {
        assert!(
            fm.path.starts_with("🚦 project/"),
            "Should only include emoji dir, got: {}",
            fm.path
        );
    }
}

#[test]
fn test_grep_include_wildcard_with_spaces_and_emoji() {
    let tmp = setup_corpus(&[
        ("01.Projects/02_MONEY/🚦 ZR - java master/Out/Lekcja 08.md", "keyword java"),
        ("01.Projects/02_MONEY/🚦 ZR - python master/Out/Lekcja A.md", "keyword python"),
        ("01.Projects/other/file.md", "keyword other"),
    ]);

    // Glob segment must match full directory name.
    // "🚦 ZR - java master" requires a wildcard prefix: **/*java master*/**
    let filter = PathFilter::new(&["**/*java master*/**".to_string()], &[]).unwrap();
    let results = grep::grep_files(tmp.path(), "keyword", 100, &filter).unwrap();

    assert_eq!(results.len(), 1, "Should find 1 file matching *java master* glob");
    assert!(results[0].path.contains("java master"));
}

#[test]
fn test_grep_include_exact_segment_does_not_match_partial() {
    let tmp = setup_corpus(&[
        ("prefix-docs/guide.md", "keyword here"),
        ("docs/guide.md", "keyword there"),
    ]);

    // **/docs/** matches only the exact segment "docs", not "prefix-docs"
    let filter = PathFilter::new(&["**/docs/**".to_string()], &[]).unwrap();
    let results = grep::grep_files(tmp.path(), "keyword", 100, &filter).unwrap();

    assert_eq!(results.len(), 1, "Should match only exact 'docs' segment");
    assert_eq!(results[0].path, "docs/guide.md");
}

#[test]
fn test_grep_include_double_star_spaces() {
    let tmp = setup_corpus(&[
        ("a b/c d/file.md", "keyword here"),
        ("a b/other.md", "keyword there"),
        ("normal/file.md", "keyword normal"),
    ]);

    let filter = PathFilter::new(&["**/c d/**".to_string()], &[]).unwrap();
    let results = grep::grep_files(tmp.path(), "keyword", 100, &filter).unwrap();

    assert_eq!(results.len(), 1, "Should match through dirs with spaces");
    assert!(results[0].path.contains("c d/"));
}
