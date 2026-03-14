use agent_search::dedup::merge_chunks;
use agent_search::filter::PathFilter;
use agent_search::index;
use agent_search::types::Chunk;
use std::fs;
use tempfile::TempDir;

fn no_filter() -> PathFilter {
    PathFilter::default()
}

// =============================================================================
// Bug 1: scan_corpus indexes its own index directory
// =============================================================================

#[test]
fn bug1_scan_corpus_indexes_own_index_dir_default_hidden() {
    // Default index dir is .agent-search-index (hidden) — currently "works"
    // because hidden(true) filters it out, but there's no explicit exclusion.
    // This test verifies the behavior is correct (should pass even before fix).
    let tmp = TempDir::new().unwrap();
    let corpus = tmp.path();

    // Create a real file in corpus
    fs::write(corpus.join("hello.txt"), "hello world").unwrap();

    // Create the default index dir with a file inside it
    let idx_dir = corpus.join(".agent-search-index");
    fs::create_dir_all(&idx_dir).unwrap();
    fs::write(idx_dir.join("manifest.json"), r#"{"files":{}}"#).unwrap();
    fs::write(idx_dir.join("some_index_file.txt"), "index data").unwrap();

    let idx = index::build_index(corpus, &idx_dir, "pl").unwrap();
    let reader = idx.reader().unwrap();
    let searcher = reader.searcher();

    // Index should contain only hello.txt, NOT files from .agent-search-index
    assert_eq!(
        searcher.num_docs(),
        1,
        "Should index only corpus files, not index dir files"
    );
}

#[test]
fn bug1_scan_corpus_indexes_non_hidden_index_dir() {
    // When --index-dir is a non-hidden directory inside the corpus,
    // scan_corpus will index its contents too — THIS IS THE BUG.
    let tmp = TempDir::new().unwrap();
    let corpus = tmp.path();

    // Create a real file in corpus
    fs::write(corpus.join("hello.txt"), "hello world").unwrap();

    // Create a non-hidden index dir inside corpus (simulating --index-dir my-index)
    let idx_dir = corpus.join("my-search-index");
    fs::create_dir_all(&idx_dir).unwrap();

    let idx = index::build_index(corpus, &idx_dir, "pl").unwrap();
    let reader = idx.reader().unwrap();
    let searcher = reader.searcher();

    // BUG: This will fail before fix because manifest.json gets indexed
    assert_eq!(
        searcher.num_docs(),
        1,
        "Should index only hello.txt, not files from the index directory itself"
    );
}

// =============================================================================
// Bug 2: merge_chunks produces inconsistent line range vs content when gap exists
// =============================================================================

#[test]
fn bug2_merge_chunks_gap_stays_separate() {
    // Two chunks from the same file with a 2-line gap.
    // Chunk A: lines 1-5, Chunk B: lines 8-12 (gap at lines 6-7)
    // After fix: chunks with gaps should NOT be merged (we don't have gap content).
    let chunk_a = Chunk {
        source_id: String::new(),
        file_path: "test.rs".to_string(),
        start_line: 1,
        end_line: 5,
        content: "line1\nline2\nline3\nline4\nline5".to_string(),
        score: 1.0,
    };
    let chunk_b = Chunk {
        source_id: String::new(),
        file_path: "test.rs".to_string(),
        start_line: 8,
        end_line: 12,
        content: "line8\nline9\nline10\nline11\nline12".to_string(),
        score: 0.9,
    };

    let merged = merge_chunks(vec![chunk_a, chunk_b]);
    assert_eq!(merged.len(), 2, "Chunks with gap should NOT be merged");

    // Each chunk's content should match its declared line range
    for chunk in &merged {
        let content_lines: Vec<&str> = chunk.content.lines().collect();
        let expected = chunk.end_line - chunk.start_line + 1;
        assert_eq!(
            content_lines.len(),
            expected,
            "Content line count must match range for {}:{}-{}",
            chunk.file_path,
            chunk.start_line,
            chunk.end_line,
        );
    }
}

#[test]
fn bug2_merge_chunks_overlapping_still_merges() {
    // Overlapping chunks SHOULD still be merged correctly.
    let chunk_a = Chunk {
        source_id: String::new(),
        file_path: "test.rs".to_string(),
        start_line: 1,
        end_line: 6,
        content: "line1\nline2\nline3\nline4\nline5\nline6".to_string(),
        score: 1.0,
    };
    let chunk_b = Chunk {
        source_id: String::new(),
        file_path: "test.rs".to_string(),
        start_line: 5,
        end_line: 10,
        content: "line5\nline6\nline7\nline8\nline9\nline10".to_string(),
        score: 0.9,
    };

    let merged = merge_chunks(vec![chunk_a, chunk_b]);
    assert_eq!(merged.len(), 1, "Overlapping chunks should merge");

    let chunk = &merged[0];
    assert_eq!(chunk.start_line, 1);
    assert_eq!(chunk.end_line, 10);

    let content_lines: Vec<&str> = chunk.content.lines().collect();
    assert_eq!(
        content_lines.len(),
        10,
        "Merged content should have 10 lines"
    );
    assert_eq!(content_lines[0], "line1");
    assert_eq!(content_lines[9], "line10");
}

// =============================================================================
// Bug 3: mtime detection misses sub-second changes
// =============================================================================

#[test]
fn bug3_mtime_subsecond_change_not_detected() {
    // Write a file, index it, then rewrite with same size but different content.
    // If the change happens within the same second, the old code won't detect it.
    let tmp = TempDir::new().unwrap();
    let corpus = tmp.path();
    let idx_dir = tmp.path().join(".agent-search-index");

    let file_path = corpus.join("data.txt");
    fs::write(&file_path, "aaaa").unwrap(); // 4 bytes

    // Build initial index
    index::build_index(corpus, &idx_dir, "pl").unwrap();

    // Immediately overwrite with same-size, different content
    fs::write(&file_path, "bbbb").unwrap(); // still 4 bytes, same second likely

    // Update index — should detect the change
    let (_idx, changed) = index::update_index(corpus, &idx_dir).unwrap();

    // BUG: With second-precision mtime, this often returns false
    // (change not detected) when both writes happen in the same second.
    // After fix (sub-second precision), this should reliably detect the change.
    // NOTE: This test is timing-dependent. On fast systems, writes happen
    // in the same second, exposing the bug. We assert changed==true as the
    // desired behavior.
    assert!(
        changed,
        "Index should detect the file change even when it happens within the same second"
    );
}

// =============================================================================
// Bug 4: line_matches_any doesn't use stemming, misses BM25-matched terms
// =============================================================================

#[test]
fn bug4_line_matches_stemmed_terms() {
    // Build a small index with Polish text and search for a stemmed form.
    // The BM25 ranker (with Polish stemmer) will find the document,
    // but line_matches_any (simple contains) won't highlight the right lines.
    let tmp = TempDir::new().unwrap();
    let corpus = tmp.path();
    let idx_dir = tmp.path().join(".agent-search-index");

    // "programowanie" stems to "programowa" in Polish stemmer
    // Query "programowania" also stems to "programowa"
    // But raw contains("programowania") won't match "programowanie"
    // File must be long enough so the fallback (first context_lines*2 lines)
    // does NOT reach the hit line.
    let mut lines: Vec<String> = (1..=50)
        .map(|i| format!("zwykla linia numer {}", i))
        .collect();
    lines[40] = "programowanie w rust jest wydajne".to_string(); // line 41
    let content = lines.join("\n");
    fs::write(corpus.join("test.txt"), content).unwrap();

    let idx = index::build_index(corpus, &idx_dir, "pl").unwrap();

    // Search with a different inflected form (context_lines=2 → fallback shows first 4 lines)
    let chunks = agent_search::search::search(
        &idx,
        "programowania",
        2,
        100,
        &no_filter(),
        Some(snowball_stemmers_rs::Algorithm::Polish),
    )
    .unwrap();

    // BM25 should find the document (stemmer normalizes both forms)
    assert!(
        !chunks.is_empty(),
        "BM25 should find the document via stemming"
    );

    // The chunk should contain the actual hit line "programowanie w rust"
    // BUG: Before fix, line_matches_any("programowanie w rust", ["programowania"])
    // returns false (no substring match), so hit_set is empty and we get
    // fallback to beginning of file instead of the actual hit line.
    let chunk = &chunks[0];
    assert!(
        chunk.content.contains("programowanie w rust"),
        "Chunk should contain the actual matching line, not just fallback to file start. \
         Got content: {:?}",
        chunk.content
    );
}

// =============================================================================
// Language / stemmer tests
// =============================================================================

#[test]
fn test_english_stemmer_indexes_and_searches() {
    let tmp = TempDir::new().unwrap();
    let corpus = tmp.path();
    let idx_dir = corpus.join(".agent-search-index");

    fs::write(
        corpus.join("doc.txt"),
        "The runners were running quickly through the streets",
    )
    .unwrap();

    let idx = index::build_index(corpus, &idx_dir, "en").unwrap();

    // "running" and "run" should both stem to "run" with English stemmer
    let results = agent_search::search::search_files(&idx, "run", 10, &no_filter()).unwrap();
    assert!(
        !results.is_empty(),
        "English stemmer should match 'run' → 'runners'/'running'"
    );
}

#[test]
fn test_no_stemmer_option() {
    let tmp = TempDir::new().unwrap();
    let corpus = tmp.path();
    let idx_dir = corpus.join(".agent-search-index");

    fs::write(corpus.join("doc.txt"), "The runners were running quickly").unwrap();

    let idx = index::build_index(corpus, &idx_dir, "none").unwrap();

    // Without stemming, "run" should NOT match "runners" or "running"
    let results = agent_search::search::search_files(&idx, "run", 10, &no_filter()).unwrap();
    assert!(
        results.is_empty(),
        "Without stemming, 'run' should not match 'runners'/'running'"
    );

    // But exact token should match
    let results = agent_search::search::search_files(&idx, "running", 10, &no_filter()).unwrap();
    assert!(!results.is_empty(), "Exact token 'running' should match");
}

#[test]
fn test_polish_default_backward_compat() {
    // An index built with "pl" should work the same as before
    let tmp = TempDir::new().unwrap();
    let corpus = tmp.path();
    let idx_dir = corpus.join(".agent-search-index");

    let mut lines: Vec<String> = (1..=20).map(|i| format!("zwykla linia {}", i)).collect();
    lines[10] = "programowanie w rust jest wydajne".to_string();
    let content = lines.join("\n");
    fs::write(corpus.join("test.txt"), content).unwrap();

    let idx = index::build_index(corpus, &idx_dir, "pl").unwrap();

    // Polish stemmer should match inflected forms
    let results =
        agent_search::search::search_files(&idx, "programowania", 10, &no_filter()).unwrap();
    assert!(
        !results.is_empty(),
        "Polish stemmer should match inflected forms"
    );
}

#[test]
fn test_index_language_persisted_in_manifest() {
    let tmp = TempDir::new().unwrap();
    let corpus = tmp.path();
    let idx_dir = corpus.join(".agent-search-index");

    fs::write(corpus.join("doc.txt"), "hello world").unwrap();

    index::build_index(corpus, &idx_dir, "en").unwrap();

    // Read language back from manifest
    let lang = index::read_index_language(&idx_dir);
    assert_eq!(lang, "en", "Language should be persisted in manifest");
}

#[test]
fn test_unsupported_language_errors() {
    let result = index::resolve_language("klingon");
    assert!(result.is_err(), "Unsupported language should return error");
}

#[test]
fn test_update_index_preserves_language() {
    let tmp = TempDir::new().unwrap();
    let corpus = tmp.path();
    let idx_dir = corpus.join(".agent-search-index");

    fs::write(corpus.join("doc.txt"), "hello world").unwrap();
    index::build_index(corpus, &idx_dir, "en").unwrap();

    // Add a new file and update
    fs::write(corpus.join("doc2.txt"), "goodbye world").unwrap();
    let (_idx, changed) = index::update_index(corpus, &idx_dir).unwrap();
    assert!(changed);

    // Language should still be "en"
    let lang = index::read_index_language(&idx_dir);
    assert_eq!(lang, "en", "Language should be preserved after update");
}
