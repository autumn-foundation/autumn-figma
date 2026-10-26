#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;

use super::*;
use crate::emit::HEADER_MARK;

const SNAPSHOT: &str = include_str!("../../tests/fixtures/todo/figma/snapshot.json");
const VARIABLES: &str = include_str!("../../tests/fixtures/todo/figma/variables.json");
const CONFIG: &str = include_str!("../../tests/fixtures/todo/figma.toml");

/// A temp app with the fixture snapshots and config.
fn app() -> (tempfile::TempDir, Config) {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("figma")).unwrap();
    fs::write(dir.path().join("figma/snapshot.json"), SNAPSHOT).unwrap();
    fs::write(dir.path().join("figma/variables.json"), VARIABLES).unwrap();
    fs::write(dir.path().join("figma.toml"), CONFIG).unwrap();
    let config = Config::load(&dir.path().join("figma.toml")).unwrap();
    (dir, config)
}

fn rel(root: &Path, paths: &[PathBuf]) -> Vec<String> {
    paths
        .iter()
        .map(|p| p.strip_prefix(root).unwrap().display().to_string())
        .collect()
}

#[test]
fn files_and_paths() {
    let (dir, config) = app();
    let out = generate(&config).unwrap();
    let paths: Vec<PathBuf> = out.files.iter().map(|(p, _)| p.clone()).collect();
    assert_eq!(
        rel(dir.path(), &paths),
        [
            "src/figma/components.rs",
            "src/figma/mod.rs",
            "src/figma/screens.rs",
            "static/css/figma.css"
        ]
    );
    assert!(out.files.iter().all(|(_, text)| text.contains(HEADER_MARK)));
    assert_eq!(out.stale, Vec::<PathBuf>::new());
    assert_eq!(out.assets.len(), 2);
}

#[test]
fn missing_snapshot_says_run_pull() {
    let (dir, config) = app();
    fs::remove_file(dir.path().join("figma/snapshot.json")).unwrap();
    let err = generate(&config).unwrap_err();
    assert_eq!(err.kind(), ErrorKind::Io);
    assert!(err.message().contains("pull"), "{err}");
}

#[test]
fn variables_snapshot_is_optional() {
    let (dir, config) = app();
    fs::remove_file(dir.path().join("figma/variables.json")).unwrap();
    let out = generate(&config).unwrap();
    let css = &out
        .files
        .iter()
        .find(|(p, _)| p.ends_with("figma.css"))
        .unwrap()
        .1;
    assert!(!css.contains("--primary"), "no variables, no primary token");
    assert!(css.contains("--heading-h1"), "styles still give tokens");
}

#[test]
fn write_then_check_is_clean_and_idempotent() {
    let (dir, config) = app();
    let out = generate(&config).unwrap();
    let missing = out.drift().unwrap();
    assert_eq!(missing.len(), 4);
    assert!(missing.iter().all(|d| matches!(d, Drift::Missing(_))));

    let report = out.write().unwrap();
    assert_eq!(report.written.len(), 4);
    assert_eq!(out.drift().unwrap(), []);

    let again = generate(&config).unwrap();
    assert_eq!(again.files, out.files, "byte-identical");
    let report = again.write().unwrap();
    assert_eq!(report.written, Vec::<PathBuf>::new());
    assert_eq!(report.unchanged.len(), 4);
    assert_eq!(
        rel(dir.path(), &report.unchanged)[0],
        "src/figma/components.rs"
    );
}

#[test]
fn drift_finds_changed_and_stale_files() {
    let (dir, config) = app();
    generate(&config).unwrap().write().unwrap();
    let screens = dir.path().join("src/figma/screens.rs");
    let edited = fs::read_to_string(&screens).unwrap() + "// edited by hand\n";
    fs::write(&screens, edited).unwrap();
    let stale = dir.path().join("src/figma/old_page.rs");
    fs::write(
        &stale,
        format!("// {HEADER_MARK} from Figma file X. Do not edit.\n"),
    )
    .unwrap();
    let mine = dir.path().join("src/figma/helpers.rs");
    fs::write(&mine, "// my own code\n").unwrap();

    let out = generate(&config).unwrap();
    assert_eq!(out.stale, std::slice::from_ref(&stale));
    let drift = out.drift().unwrap();
    assert_eq!(
        drift,
        [Drift::Changed(screens.clone()), Drift::Stale(stale.clone())]
    );

    let report = out.write().unwrap();
    assert_eq!(report.written, [screens]);
    assert_eq!(report.deleted, std::slice::from_ref(&stale));
    assert!(!stale.exists());
    assert!(mine.exists(), "files without the header stay");
    assert!(
        out.drift().unwrap().is_empty(),
        "the stale file is gone from disk"
    );
}

#[test]
fn missing_assets_are_listed() {
    let (dir, config) = app();
    let out = generate(&config).unwrap();
    assert_eq!(out.missing_assets(&config).len(), 2);
    fs::create_dir_all(dir.path().join("static/figma")).unwrap();
    fs::write(
        dir.path().join("static/figma/icon-check--1-30.svg"),
        "<svg/>",
    )
    .unwrap();
    assert_eq!(
        rel(dir.path(), &out.missing_assets(&config)),
        ["static/figma/photo--2-11.png"]
    );
}

#[test]
fn generate_from_matches_generate() {
    let (_dir, config) = app();
    let file = FileResponse::from_json(SNAPSHOT).unwrap();
    let vars = VariablesResponse::from_json(VARIABLES).unwrap();
    let a = generate_from(&config, &file, Some(&vars)).unwrap();
    let b = generate(&config).unwrap();
    assert_eq!(a.files, b.files);
}

#[test]
fn never_overwrites_a_file_without_the_header() {
    // Regression: a page named "Helpers" replaced a hand-written helpers.rs.
    let (dir, config) = app();
    fs::create_dir_all(dir.path().join("src/figma")).unwrap();
    let mine = dir.path().join("src/figma/screens.rs");
    fs::write(&mine, "// my own code\n").unwrap();
    let err = generate(&config).unwrap_err();
    assert_eq!(err.kind(), ErrorKind::Io);
    assert!(err.message().contains("screens.rs"), "{err}");
    assert!(err.message().contains("header"), "{err}");
    assert_eq!(fs::read_to_string(&mine).unwrap(), "// my own code\n");

    fs::remove_file(&mine).unwrap();
    fs::create_dir_all(dir.path().join("static/css")).unwrap();
    fs::write(dir.path().join("static/css/figma.css"), "body {}\n").unwrap();
    let err = generate(&config).unwrap_err();
    assert!(err.message().contains("figma.css"), "{err}");
}
