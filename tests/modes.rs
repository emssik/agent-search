use agent_search::index;
use agent_search::search;

use std::fs;
use tempfile::TempDir;

fn setup_corpus(files: &[(&str, &str)]) -> (TempDir, tantivy::Index) {
    let tmp = TempDir::new().unwrap();
    let corpus = tmp.path();
    for (name, content) in files {
        let path = corpus.join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&path, content).unwrap();
    }
    let idx_dir = corpus.join(".agent-search-index");
    let idx = index::build_index(corpus, &idx_dir).unwrap();
    (tmp, idx)
}

// =============================================================================
// Krok 1: search_files() + --mode files
// =============================================================================

#[test]
fn test_files_mode_returns_file_matches() {
    let (_tmp, idx) = setup_corpus(&[
        ("auth.rs", "fn authenticate(user: &str) { /* auth logic */ }"),
        ("config.rs", "fn load_config() { /* config loading */ }"),
        ("auth_test.rs", "fn test_authenticate() { authenticate(\"admin\"); }"),
    ]);

    let results = search::search_files(&idx, "authenticate", 10).unwrap();

    assert!(!results.is_empty(), "Should find files matching 'authenticate'");
    // Should return FileMatch with path and score, but no content
    for fm in &results {
        assert!(!fm.path.is_empty());
        assert!(fm.score > 0.0);
    }
    // auth.rs and auth_test.rs should match
    let paths: Vec<&str> = results.iter().map(|f| f.path.as_str()).collect();
    assert!(paths.contains(&"auth.rs"), "Should find auth.rs");
    assert!(paths.contains(&"auth_test.rs"), "Should find auth_test.rs");
}

#[test]
fn test_files_mode_no_results() {
    let (_tmp, idx) = setup_corpus(&[
        ("hello.txt", "hello world"),
    ]);

    let results = search::search_files(&idx, "xyznonexistent", 10).unwrap();
    assert!(results.is_empty(), "Should return empty for non-matching query");
}

// =============================================================================
// Krok 2: --mode summary (directory facets)
// =============================================================================

#[test]
fn test_summary_groups_by_directory() {
    let (_tmp, idx) = setup_corpus(&[
        ("src/auth.rs", "fn authenticate(user: &str) {}"),
        ("src/auth_middleware.rs", "fn auth_middleware() { authenticate() }"),
        ("tests/auth_test.rs", "fn test_auth() { authenticate() }"),
    ]);

    let file_matches = search::search_files(&idx, "authenticate", 10).unwrap();
    assert!(!file_matches.is_empty());

    let groups = search::summarize_by_directory(file_matches);
    assert!(!groups.is_empty());

    // Should have groups for "src" and "tests"
    let dir_names: Vec<&str> = groups.iter().map(|g| g.directory.as_str()).collect();
    assert!(dir_names.contains(&"src"), "Should have 'src' group");
    assert!(dir_names.contains(&"tests"), "Should have 'tests' group");

    // src group should have 2 files
    let src_group = groups.iter().find(|g| g.directory == "src").unwrap();
    assert_eq!(src_group.count, 2);
    assert!(src_group.top_score > 0.0);
    assert_eq!(src_group.files.len(), 2);
}

// =============================================================================
// Krok 3: Multi-query
// =============================================================================

#[test]
fn test_multi_query_files_merges_scores() {
    let (_tmp, idx) = setup_corpus(&[
        ("both.rs", "fn authenticate() {} fn authorize() {}"),
        ("auth_only.rs", "fn authenticate(user: &str) {}"),
        ("authz_only.rs", "fn authorize(role: &str) {}"),
    ]);

    let results = search::search_files_multi(
        &idx,
        &["authenticate", "authorize"],
        10,
    ).unwrap();

    assert!(!results.is_empty());

    // both.rs should appear (matched by both queries)
    let both = results.iter().find(|f| f.path == "both.rs");
    assert!(both.is_some(), "both.rs should be in results");

    // both.rs should have highest score (matched by both queries → max score)
    let paths: Vec<&str> = results.iter().map(|f| f.path.as_str()).collect();
    assert!(paths.contains(&"both.rs"));
}

#[test]
fn test_multi_query_chunks_merges_results() {
    let (_tmp, idx) = setup_corpus(&[
        ("auth.rs", "fn authenticate(user: &str) { /* logic */ }"),
        ("authz.rs", "fn authorize(role: &str) { /* logic */ }"),
    ]);

    let results = search::search_multi(&idx, &["authenticate", "authorize"], 2, 10).unwrap();

    assert!(!results.is_empty());
    // Should have chunks from both files
    let paths: Vec<&str> = results.iter().map(|c| c.file_path.as_str()).collect();
    assert!(paths.contains(&"auth.rs") || paths.contains(&"authz.rs"),
        "Should have results from at least one file");
}

// =============================================================================
// Krok 4: --max-results
// =============================================================================

#[test]
fn test_max_results_limits_output() {
    let files: Vec<(String, String)> = (0..10)
        .map(|i| (format!("file{}.rs", i), format!("fn search_function_{}() {{}}", i)))
        .collect();
    let file_refs: Vec<(&str, &str)> = files.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();

    let (_tmp, idx) = setup_corpus(&file_refs);

    let results = search::search_files(&idx, "search_function", 3).unwrap();
    assert!(results.len() <= 3, "Should return at most 3 results, got {}", results.len());
}

#[test]
fn test_max_results_search_chunks() {
    let files: Vec<(String, String)> = (0..10)
        .map(|i| (format!("file{}.rs", i), format!("fn search_function_{}() {{}}", i)))
        .collect();
    let file_refs: Vec<(&str, &str)> = files.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();

    let (_tmp, idx) = setup_corpus(&file_refs);

    let results = search::search(&idx, "search_function", 2, 3).unwrap();
    assert!(results.len() <= 3, "Should return at most 3 results, got {}", results.len());
}
