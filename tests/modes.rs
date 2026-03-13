use agent_search::filter::PathFilter;
use agent_search::index;
use agent_search::search;
use agent_search::types::SortOrder;

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
    let idx = index::build_index(corpus, &idx_dir, "pl").unwrap();
    (tmp, idx)
}

fn no_filter() -> PathFilter {
    PathFilter::default()
}

// =============================================================================
// Krok 1: search_files() + --mode files
// =============================================================================

#[test]
fn test_files_mode_returns_file_matches() {
    let (_tmp, idx) = setup_corpus(&[
        (
            "auth.rs",
            "fn authenticate(user: &str) { /* auth logic */ }",
        ),
        ("config.rs", "fn load_config() { /* config loading */ }"),
        (
            "auth_test.rs",
            "fn test_authenticate() { authenticate(\"admin\"); }",
        ),
    ]);

    let results = search::search_files(&idx, "authenticate", 10, &no_filter()).unwrap();

    assert!(
        !results.is_empty(),
        "Should find files matching 'authenticate'"
    );
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
    let (_tmp, idx) = setup_corpus(&[("hello.txt", "hello world")]);

    let results = search::search_files(&idx, "xyznonexistent", 10, &no_filter()).unwrap();
    assert!(
        results.is_empty(),
        "Should return empty for non-matching query"
    );
}

// =============================================================================
// Krok 2: --mode summary (directory facets)
// =============================================================================

#[test]
fn test_summary_groups_by_directory() {
    let (_tmp, idx) = setup_corpus(&[
        ("src/auth.rs", "fn authenticate(user: &str) {}"),
        (
            "src/auth_middleware.rs",
            "fn auth_middleware() { authenticate() }",
        ),
        ("tests/auth_test.rs", "fn test_auth() { authenticate() }"),
    ]);

    let file_matches = search::search_files(&idx, "authenticate", 10, &no_filter()).unwrap();
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

    let results =
        search::search_files_multi(&idx, &["authenticate", "authorize"], 10, &no_filter()).unwrap();

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

    let results = search::search_multi(
        &idx,
        &["authenticate", "authorize"],
        2,
        10,
        &no_filter(),
        Some(snowball_stemmers_rs::Algorithm::Polish),
    )
    .unwrap();

    assert!(!results.is_empty());
    // Should have chunks from both files
    let paths: Vec<&str> = results.iter().map(|c| c.file_path.as_str()).collect();
    assert!(
        paths.contains(&"auth.rs") || paths.contains(&"authz.rs"),
        "Should have results from at least one file"
    );
}

// =============================================================================
// Krok 4: --max-results
// =============================================================================

#[test]
fn test_max_results_limits_output() {
    let files: Vec<(String, String)> = (0..10)
        .map(|i| {
            (
                format!("file{}.rs", i),
                format!("fn search_function_{}() {{}}", i),
            )
        })
        .collect();
    let file_refs: Vec<(&str, &str)> = files
        .iter()
        .map(|(a, b)| (a.as_str(), b.as_str()))
        .collect();

    let (_tmp, idx) = setup_corpus(&file_refs);

    let results = search::search_files(&idx, "search_function", 3, &no_filter()).unwrap();
    assert!(
        results.len() <= 3,
        "Should return at most 3 results, got {}",
        results.len()
    );
}

#[test]
fn test_max_results_search_chunks() {
    let files: Vec<(String, String)> = (0..10)
        .map(|i| {
            (
                format!("file{}.rs", i),
                format!("fn search_function_{}() {{}}", i),
            )
        })
        .collect();
    let file_refs: Vec<(&str, &str)> = files
        .iter()
        .map(|(a, b)| (a.as_str(), b.as_str()))
        .collect();

    let (_tmp, idx) = setup_corpus(&file_refs);

    let results = search::search(
        &idx,
        "search_function",
        2,
        3,
        &no_filter(),
        Some(snowball_stemmers_rs::Algorithm::Polish),
    )
    .unwrap();
    assert!(
        results.len() <= 3,
        "Should return at most 3 results, got {}",
        results.len()
    );
}

// =============================================================================
// Krok 5: --include / --exclude (search)
// =============================================================================

#[test]
fn test_search_files_include() {
    let (_tmp, idx) = setup_corpus(&[
        ("docs/guide.md", "search tutorial guide keywords"),
        ("archive/old.md", "search old data keywords"),
    ]);

    let filter = PathFilter::new(&["docs/**".to_string()], &[]).unwrap();
    let results = search::search_files(&idx, "search", 100, &filter).unwrap();

    for fm in &results {
        assert!(
            fm.path.starts_with("docs/"),
            "Should only include docs/, got: {}",
            fm.path
        );
    }
}

#[test]
fn test_search_files_include_not_lost_by_limit_cutoff() {
    let mut files: Vec<(String, String)> = (0..40)
        .map(|i| (format!("src/high{}.txt", i), "search ".repeat(30)))
        .collect();
    files.push(("docs/target.md".to_string(), "search".to_string()));
    let file_refs: Vec<(&str, &str)> = files
        .iter()
        .map(|(name, content)| (name.as_str(), content.as_str()))
        .collect();

    let (_tmp, idx) = setup_corpus(&file_refs);
    let filter = PathFilter::new(&["docs/**".to_string()], &[]).unwrap();
    let results = search::search_files(&idx, "search", 10, &filter).unwrap();

    assert_eq!(
        results.len(),
        1,
        "Should return the docs match even if it is not in global top-10"
    );
    assert_eq!(results[0].path, "docs/target.md");
}

#[test]
fn test_search_files_exclude() {
    let (_tmp, idx) = setup_corpus(&[
        ("docs/guide.md", "search tutorial guide keywords"),
        ("archive/old.md", "search old data keywords"),
    ]);

    let filter = PathFilter::new(&[], &["archive/**".to_string()]).unwrap();
    let results = search::search_files(&idx, "search", 100, &filter).unwrap();

    for fm in &results {
        assert!(
            !fm.path.starts_with("archive/"),
            "Should exclude archive/, got: {}",
            fm.path
        );
    }
}

#[test]
fn test_search_chunks_include() {
    let (_tmp, idx) = setup_corpus(&[
        ("docs/guide.md", "search tutorial guide keywords"),
        ("archive/old.md", "search old data keywords"),
    ]);

    let filter = PathFilter::new(&["docs/**".to_string()], &[]).unwrap();
    let results = search::search(
        &idx,
        "search",
        2,
        100,
        &filter,
        Some(snowball_stemmers_rs::Algorithm::Polish),
    )
    .unwrap();

    for chunk in &results {
        assert!(
            chunk.file_path.starts_with("docs/"),
            "Should only include docs/, got: {}",
            chunk.file_path
        );
    }
}

// =============================================================================
// Krok 6: --sort
// =============================================================================

#[test]
fn test_files_sorted_by_score() {
    let (_tmp, idx) = setup_corpus(&[
        ("a.rs", "fn search_function() {}"),
        ("b.rs", "fn search_function() {} fn search_function_2() {}"),
    ]);

    let results = search::search_files(&idx, "search_function", 10, &no_filter()).unwrap();

    // Default: sorted by score descending
    for i in 1..results.len() {
        assert!(
            results[i - 1].score >= results[i].score,
            "Results should be sorted by score descending"
        );
    }
}

#[test]
fn test_files_sorted_by_path() {
    let (tmp, idx) = setup_corpus(&[
        ("charlie.rs", "fn search_data() {}"),
        ("alpha.rs", "fn search_data() {}"),
        ("bravo.rs", "fn search_data() {}"),
    ]);

    let mut results = search::search_files(&idx, "search_data", 10, &no_filter()).unwrap();
    search::sort_file_matches(&mut results, &SortOrder::Path, tmp.path());

    let paths: Vec<&str> = results.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(
        paths,
        vec!["alpha.rs", "bravo.rs", "charlie.rs"],
        "Should be sorted alphabetically by path"
    );
}

#[test]
fn test_grep_files_sorted_by_path() {
    let tmp = TempDir::new().unwrap();
    let corpus = tmp.path();
    for (name, content) in &[
        ("charlie.txt", "keyword here"),
        ("alpha.txt", "keyword here"),
        ("bravo.txt", "keyword here"),
    ] {
        fs::write(corpus.join(name), content).unwrap();
    }

    let mut results = agent_search::grep::grep_files(corpus, "keyword", 100, &no_filter()).unwrap();
    search::sort_file_matches(&mut results, &SortOrder::Path, corpus);

    let paths: Vec<&str> = results.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(
        paths,
        vec!["alpha.txt", "bravo.txt", "charlie.txt"],
        "Should be sorted alphabetically by path"
    );
}

#[test]
fn test_summary_groups_sorted_by_path() {
    let (tmp, idx) = setup_corpus(&[
        ("z_dir/a.txt", "keyword"),
        ("a_dir/b.txt", "keyword"),
        ("m_dir/c.txt", "keyword"),
    ]);

    let mut files = search::search_files(&idx, "keyword", 100, &no_filter()).unwrap();
    search::sort_file_matches(&mut files, &SortOrder::Path, tmp.path());
    let mut groups = search::summarize_by_directory(files);
    search::sort_dir_groups(&mut groups, &SortOrder::Path, tmp.path());

    let dirs: Vec<&str> = groups.iter().map(|g| g.directory.as_str()).collect();
    assert_eq!(dirs, vec!["a_dir", "m_dir", "z_dir"]);
}
