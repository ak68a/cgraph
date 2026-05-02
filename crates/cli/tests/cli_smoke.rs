use assert_cmd::Command;
use predicates::prelude::*;
use std::path::PathBuf;

/// Return the workspace root (parent of crates/cli).
fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is the directory containing Cargo.toml for this crate (crates/cli).
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // Go up two levels: crates/cli -> crates -> workspace root
    manifest_dir
        .parent()
        .expect("crates/cli has no parent")
        .parent()
        .expect("crates has no parent")
        .to_path_buf()
}

/// Test: `cg --version` prints version string
#[test]
fn version_flag() {
    Command::cargo_bin("cg")
        .unwrap()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("0.1.0"));
}

/// Test: `cg --help` prints usage info
#[test]
fn help_flag() {
    Command::cargo_bin("cg")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Code graph visualization"))
        .stdout(predicate::str::contains("<PATH>").or(predicate::str::contains("path")));
}

/// Test: `cg <nonexistent-path>` exits with error
#[test]
fn nonexistent_path_errors() {
    Command::cargo_bin("cg")
        .unwrap()
        .arg("/tmp/definitely-does-not-exist-cgraph-test")
        .assert()
        .failure()
        .stderr(predicate::str::contains("does not exist"));
}

/// Test: `cg <file-not-dir>` exits with error (path must be a directory)
#[test]
fn file_path_errors() {
    // Use workspace root Cargo.toml as a file that exists but is not a directory
    let cargo_toml = workspace_root().join("Cargo.toml");
    Command::cargo_bin("cg")
        .unwrap()
        .arg(cargo_toml)
        .assert()
        .failure()
        .stderr(predicate::str::contains("not a directory"));
}

/// Test: `cg <valid-path>` runs indexer and prints scan statistics + analysis summary
#[test]
fn scan_fixture_directory() {
    let fixtures = workspace_root().join("crates/core/tests/fixtures");
    Command::cargo_bin("cg")
        .unwrap()
        .arg(fixtures)
        .assert()
        .success()
        .stdout(predicate::str::contains("cgraph scan:"))
        .stdout(predicate::str::contains("files"))
        .stdout(predicate::str::contains("symbols"))
        .stdout(predicate::str::contains("edges"));
}

/// Test: `cg <valid-path>` prints analysis summary with dead code and cycle counts
#[test]
fn scan_prints_analysis_summary() {
    let fixtures = workspace_root().join("crates/core/tests/fixtures");
    Command::cargo_bin("cg")
        .unwrap()
        .arg(fixtures)
        .assert()
        .success()
        .stdout(predicate::str::contains("analysis:"))
        .stdout(predicate::str::contains("dead code:"))
        .stdout(predicate::str::contains("circular dependencies:"));
}

/// Test: `cg <path> --dead-code` prints dead code report
#[test]
fn dead_code_flag() {
    let fixtures = workspace_root().join("crates/core/tests/fixtures");
    Command::cargo_bin("cg")
        .unwrap()
        .args([
            fixtures.to_str().expect("fixtures path is valid UTF-8"),
            "--dead-code",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("dead code"));
}

/// Test: `cg <path> --cycles` prints cycles report
#[test]
fn cycles_flag() {
    let fixtures = workspace_root().join("crates/core/tests/fixtures");
    Command::cargo_bin("cg")
        .unwrap()
        .args([
            fixtures.to_str().expect("fixtures path is valid UTF-8"),
            "--cycles",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("circular dependencies"));
}
