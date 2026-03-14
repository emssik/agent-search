use serde_json::Value;
use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn bin_path() -> &'static str {
    env!("CARGO_BIN_EXE_agent-search")
}

fn run_ok(args: &[&str]) -> String {
    let output = Command::new(bin_path())
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("Failed to run binary with args {:?}: {}", args, e));
    assert!(
        output.status.success(),
        "Command {:?} failed.\nstdout: {}\nstderr: {}",
        args,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("stdout should be valid UTF-8")
}

#[test]
fn test_cli_index_language_change_applies_without_force() {
    let tmp = TempDir::new().unwrap();
    let corpus = tmp.path();
    fs::write(corpus.join("doc.txt"), "running runners").unwrap();

    run_ok(&[
        "index",
        "-c",
        corpus.to_str().unwrap(),
        "--language",
        "none",
    ]);
    run_ok(&["index", "-c", corpus.to_str().unwrap(), "--language", "en"]);

    let manifest_path = corpus.join(".agent-search-index").join("manifest.json");
    let manifest: Value =
        serde_json::from_str(&fs::read_to_string(manifest_path).unwrap()).unwrap();
    assert_eq!(
        manifest.get("language").and_then(|v| v.as_str()),
        Some("en"),
        "Second index run with --language en should update index language"
    );
}

#[test]
fn test_cli_search_grep_not_lost_by_bm25_limit() {
    let tmp = TempDir::new().unwrap();
    let corpus = tmp.path();

    for i in 0..30 {
        fs::write(corpus.join(format!("high{}.txt", i)), "backup ".repeat(40)).unwrap();
    }
    fs::create_dir_all(corpus.join("docs")).unwrap();
    fs::write(corpus.join("docs/rsync_hit.txt"), "backup rsync").unwrap();

    run_ok(&["index", "-c", corpus.to_str().unwrap()]);
    let stdout = run_ok(&[
        "search",
        "-c",
        corpus.to_str().unwrap(),
        "-q",
        "backup",
        "--mode",
        "files",
        "--max-results",
        "10",
        "--grep",
        "rsync",
    ]);

    let json: Value = serde_json::from_str(&stdout).unwrap();
    let files = json
        .get("files")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    assert_eq!(
        files.len(),
        1,
        "Expected one grep-filtered file even if it has low BM25 score"
    );
    let path = files[0].get("path").and_then(|v| v.as_str()).unwrap_or("");
    assert_eq!(path, "docs/rsync_hit.txt");
}
