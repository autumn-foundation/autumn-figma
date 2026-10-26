//! The todo fixture against the committed golden output.
//!
//! Run with `UPDATE_GOLDEN=1` to rewrite `tests/golden/` after an intended
//! change. Review the diff before you commit it.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::{Path, PathBuf};

use autumn_plugin_figma::config::Config;
use autumn_plugin_figma::generate;

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/todo")
}

fn golden() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden")
}

#[test]
fn todo_fixture_matches_golden_files() {
    let config = Config::load(&fixture().join("figma.toml")).unwrap();
    let output = generate::generate(&config).unwrap();
    let update = std::env::var_os("UPDATE_GOLDEN").is_some();
    let mut problems = Vec::new();
    for (path, text) in &output.files {
        let rel = path.strip_prefix(fixture()).unwrap();
        let golden_path = golden().join(rel);
        if update {
            fs::create_dir_all(golden_path.parent().unwrap()).unwrap();
            fs::write(&golden_path, text).unwrap();
            continue;
        }
        match fs::read_to_string(&golden_path) {
            Ok(expected) if &expected == text => {}
            Ok(_) => problems.push(format!("differs: {}", rel.display())),
            Err(_) => problems.push(format!("missing: {}", rel.display())),
        }
    }
    assert!(
        problems.is_empty(),
        "golden drift (rerun with UPDATE_GOLDEN=1 if intended): {problems:#?}"
    );
}

#[test]
fn output_is_byte_identical_across_runs() {
    let config = Config::load(&fixture().join("figma.toml")).unwrap();
    let a = generate::generate(&config).unwrap();
    let b = generate::generate(&config).unwrap();
    assert_eq!(a.files, b.files);
}

#[test]
fn output_has_no_inline_styles_or_pre_escaped() {
    let config = Config::load(&fixture().join("figma.toml")).unwrap();
    for (path, text) in generate::generate(&config).unwrap().files {
        if path.extension().is_some_and(|e| e == "rs") {
            assert!(!text.contains("style="), "{}", path.display());
            assert!(!text.contains("\"style\""), "{}", path.display());
            assert!(!text.contains("PreEscaped"), "{}", path.display());
        }
    }
}
