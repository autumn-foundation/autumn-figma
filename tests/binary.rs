//! The real `autumn-figma` binary: exit codes for CI.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;

fn copy_fixture(to: &Path) {
    let from = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/todo");
    fs::create_dir_all(to.join("figma")).unwrap();
    for rel in ["figma.toml", "figma/snapshot.json", "figma/variables.json"] {
        fs::copy(from.join(rel), to.join(rel)).unwrap();
    }
}

fn bin(dir: &Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_autumn-figma"))
        .args(args)
        .current_dir(dir)
        .env_remove("FIGMA_TOKEN")
        .output()
        .unwrap()
}

#[test]
fn check_exit_codes() {
    let dir = tempfile::tempdir().unwrap();
    copy_fixture(dir.path());
    assert_eq!(
        bin(dir.path(), &["generate", "--check"]).status.code(),
        Some(2)
    );
    assert_eq!(bin(dir.path(), &["generate"]).status.code(), Some(0));
    assert_eq!(
        bin(dir.path(), &["generate", "--check"]).status.code(),
        Some(0)
    );
    let pull = bin(dir.path(), &["pull"]);
    assert_eq!(pull.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&pull.stderr).contains("FIGMA_TOKEN"));
}

#[test]
fn version_flag() {
    let out = bin(Path::new("."), &["--version"]);
    assert_eq!(out.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&out.stdout).contains(env!("CARGO_PKG_VERSION")));
}
