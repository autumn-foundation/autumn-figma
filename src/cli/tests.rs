#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;
use std::fs;

use super::*;
use crate::config::{API_BASE_ENV, TOKEN_ENV};
use crate::test_support::{Script, start};

const SNAPSHOT: &str = include_str!("../../tests/fixtures/todo/figma/snapshot.json");
const VARIABLES: &str = include_str!("../../tests/fixtures/todo/figma/variables.json");
const CONFIG: &str = include_str!("../../tests/fixtures/todo/figma.toml");

struct Run {
    code: i32,
    out: String,
    err: String,
}

fn cli(cwd: &Path, args: &[&str], env: &BTreeMap<&str, String>) -> Run {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let words = std::iter::once("autumn-figma")
        .chain(args.iter().copied())
        .map(OsString::from);
    let lookup = |k: &str| env.get(k).cloned();
    let code = run(words, &lookup, cwd, &mut out, &mut err);
    Run {
        code,
        out: String::from_utf8(out).unwrap(),
        err: String::from_utf8(err).unwrap(),
    }
}

fn no_env() -> BTreeMap<&'static str, String> {
    BTreeMap::new()
}

fn fixture_app() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("figma")).unwrap();
    fs::write(dir.path().join("figma/snapshot.json"), SNAPSHOT).unwrap();
    fs::write(dir.path().join("figma/variables.json"), VARIABLES).unwrap();
    fs::write(dir.path().join("figma.toml"), CONFIG).unwrap();
    dir
}

#[test]
fn init_writes_a_starter_and_never_overwrites() {
    let dir = tempfile::tempdir().unwrap();
    let r = cli(
        dir.path(),
        &[
            "init",
            "--file-key",
            "https://www.figma.com/design/AbC123/Todo",
        ],
        &no_env(),
    );
    assert_eq!(r.code, EXIT_OK, "{}", r.err);
    let text = fs::read_to_string(dir.path().join("figma.toml")).unwrap();
    assert!(text.contains("file_key = \"AbC123\""), "{text}");
    assert!(r.out.contains("figma.toml"));

    let again = cli(dir.path(), &["init", "--file-key", "Other1"], &no_env());
    assert_eq!(again.code, EXIT_ERROR);
    assert!(again.err.contains("exists"), "{}", again.err);
    assert_eq!(
        fs::read_to_string(dir.path().join("figma.toml")).unwrap(),
        text
    );
}

#[test]
fn init_without_a_key_writes_a_placeholder() {
    let dir = tempfile::tempdir().unwrap();
    let r = cli(dir.path(), &["init"], &no_env());
    assert_eq!(r.code, EXIT_OK, "{}", r.err);
    assert!(r.out.contains("file_key"), "{}", r.out);
}

#[test]
fn generate_then_check() {
    let dir = fixture_app();
    let check = cli(dir.path(), &["generate", "--check"], &no_env());
    assert_eq!(check.code, EXIT_DRIFT, "{}", check.err);
    assert!(
        check.err.contains("missing: src/figma/mod.rs"),
        "{}",
        check.err
    );
    assert!(
        !dir.path().join("src/figma/mod.rs").exists(),
        "check writes nothing"
    );

    let gen_run = cli(dir.path(), &["generate"], &no_env());
    assert_eq!(gen_run.code, EXIT_OK, "{}", gen_run.err);
    assert!(
        gen_run.out.contains("wrote src/figma/mod.rs"),
        "{}",
        gen_run.out
    );
    assert!(
        gen_run.err.contains("warning: Screens > Todos > Hero art"),
        "{}",
        gen_run.err
    );
    assert!(gen_run.err.contains("missing asset"), "{}", gen_run.err);

    let clean = cli(dir.path(), &["generate", "--check"], &no_env());
    assert_eq!(clean.code, EXIT_OK, "{}", clean.err);
    assert!(clean.out.contains("in sync"), "{}", clean.out);

    fs::write(dir.path().join("src/figma/screens.rs"), "// hand edit\n").unwrap();
    let changed = cli(dir.path(), &["generate", "--check"], &no_env());
    assert_eq!(changed.code, EXIT_DRIFT);
    assert!(
        changed.err.contains("changed: src/figma/screens.rs"),
        "{}",
        changed.err
    );
}

#[test]
fn config_flag_and_errors() {
    let dir = fixture_app();
    fs::rename(
        dir.path().join("figma.toml"),
        dir.path().join("design.toml"),
    )
    .unwrap();
    let missing = cli(dir.path(), &["generate"], &no_env());
    assert_eq!(missing.code, EXIT_ERROR);
    assert!(missing.err.contains("figma.toml"), "{}", missing.err);
    assert!(missing.err.contains("init"), "hint: {}", missing.err);
    let ok = cli(
        dir.path(),
        &["--config", "design.toml", "generate"],
        &no_env(),
    );
    assert_eq!(ok.code, EXIT_OK, "{}", ok.err);

    let bad = cli(dir.path(), &["frobnicate"], &no_env());
    assert_eq!(bad.code, EXIT_ERROR);
    assert!(!bad.err.is_empty());
    let help = cli(dir.path(), &["--help"], &no_env());
    assert_eq!(help.code, EXIT_OK);
    assert!(help.out.contains("generate"), "{}", help.out);
}

#[test]
fn pull_needs_the_token() {
    let dir = fixture_app();
    let r = cli(dir.path(), &["pull"], &no_env());
    assert_eq!(r.code, EXIT_ERROR);
    assert!(r.err.contains(TOKEN_ENV), "{}", r.err);
}

#[test]
fn sync_pulls_and_generates_against_the_stub() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let stub = rt.block_on(start(Script {
        file: SNAPSHOT.into(),
        file_depth1: Some(
            r#"{"document":{"id":"0:0","type":"DOCUMENT","children":[
                {"id":"1:0","name":"Components","type":"CANVAS"},
                {"id":"2:0","name":"Screens","type":"CANVAS"}]}}"#
                .into(),
        ),
        variables: None,
        images: BTreeMap::from([
            ("1:30".to_owned(), Some("{base}/cdn/icon.svg".to_owned())),
            ("2:11".to_owned(), Some("{base}/cdn/photo.png".to_owned())),
        ]),
        ..Script::default()
    }));
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("figma.toml"), CONFIG).unwrap();
    let env = BTreeMap::from([
        (TOKEN_ENV, "figd_cli_secret".to_owned()),
        (API_BASE_ENV, stub.base.clone()),
    ]);
    let r = cli(dir.path(), &["sync"], &env);
    assert_eq!(r.code, EXIT_OK, "{}", r.err);
    assert!(dir.path().join("figma/snapshot.json").exists());
    assert!(dir.path().join("static/figma/photo-2-11.png").exists());
    assert!(dir.path().join("src/figma/screens.rs").exists());
    assert!(r.err.contains("Enterprise"), "variables notice: {}", r.err);
    assert!(!r.err.contains("missing asset"), "{}", r.err);
    assert!(!r.out.contains("figd_cli_secret") && !r.err.contains("figd_cli_secret"));
    assert!(
        stub.seen()
            .iter()
            .all(|s| s.path.starts_with("/cdn") || s.token.is_some())
    );
}
