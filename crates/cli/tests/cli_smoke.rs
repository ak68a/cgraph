use assert_cmd::Command;
use predicates::prelude::*;
use std::path::PathBuf;
use std::time::Duration;

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

/// Test: `cg --help` shows --no-open flag
#[test]
fn help_shows_no_open_flag() {
    Command::cargo_bin("cg")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--no-open"));
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

/// Test: `cg <valid-path> --no-open` runs indexer and prints scan statistics + analysis summary.
/// Uses --no-open to suppress browser and a timeout since the server blocks on Ctrl-C.
/// The process is killed after timeout; we verify stdout up to that point.
#[test]
fn scan_fixture_directory() {
    let fixtures = workspace_root().join("crates/core/tests/fixtures");
    Command::cargo_bin("cg")
        .unwrap()
        .arg(fixtures)
        .arg("--no-open")
        .timeout(Duration::from_secs(10))
        .assert()
        .stdout(predicate::str::contains("cgraph scan:"))
        .stdout(predicate::str::contains("files"))
        .stdout(predicate::str::contains("symbols"))
        .stdout(predicate::str::contains("edges"));
}

/// Test: `cg <valid-path> --no-open` prints analysis summary with dead code and cycle counts.
#[test]
fn scan_prints_analysis_summary() {
    let fixtures = workspace_root().join("crates/core/tests/fixtures");
    Command::cargo_bin("cg")
        .unwrap()
        .arg(fixtures)
        .arg("--no-open")
        .timeout(Duration::from_secs(10))
        .assert()
        .stdout(predicate::str::contains("analysis:"))
        .stdout(predicate::str::contains("dead code:"))
        .stdout(predicate::str::contains("circular dependencies:"));
}

/// Test: `cg <valid-path> --no-open` prints server URL message.
#[test]
fn scan_prints_server_url() {
    let fixtures = workspace_root().join("crates/core/tests/fixtures");
    Command::cargo_bin("cg")
        .unwrap()
        .arg(fixtures)
        .arg("--no-open")
        .timeout(Duration::from_secs(10))
        .assert()
        .stdout(predicate::str::contains("cgraph listening on"));
}

/// Test: `cg <path> --dead-code --no-open` prints dead code report
#[test]
fn dead_code_flag() {
    let fixtures = workspace_root().join("crates/core/tests/fixtures");
    Command::cargo_bin("cg")
        .unwrap()
        .args([
            fixtures.to_str().expect("fixtures path is valid UTF-8"),
            "--dead-code",
            "--no-open",
        ])
        .timeout(Duration::from_secs(10))
        .assert()
        .stdout(predicate::str::contains("dead code"));
}

/// Test: `cg <path> --cycles --no-open` prints cycles report
#[test]
fn cycles_flag() {
    let fixtures = workspace_root().join("crates/core/tests/fixtures");
    Command::cargo_bin("cg")
        .unwrap()
        .args([
            fixtures.to_str().expect("fixtures path is valid UTF-8"),
            "--cycles",
            "--no-open",
        ])
        .timeout(Duration::from_secs(10))
        .assert()
        .stdout(predicate::str::contains("circular dependencies"));
}
