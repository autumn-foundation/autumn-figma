#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;
use std::fs;

use super::*;
use crate::config::Token;
use crate::model::normalize_json;
use crate::test_support::{Script, Stub, start};

const SNAPSHOT: &str = include_str!("../../tests/fixtures/todo/figma/snapshot.json");
const VARIABLES: &str = include_str!("../../tests/fixtures/todo/figma/variables.json");
const CONFIG: &str = include_str!("../../tests/fixtures/todo/figma.toml");

const DEPTH1: &str = r#"{"name":"Todo","document":{"id":"0:0","type":"DOCUMENT","children":[
  {"id":"1:0","name":"Components","type":"CANVAS"},
  {"id":"2:0","name":"Screens","type":"CANVAS"},
  {"id":"3:0","name":"Drafts","type":"CANVAS"}]}}"#;

fn app(config: &str) -> (tempfile::TempDir, Config) {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("figma.toml"), config).unwrap();
    let config = Config::load(&dir.path().join("figma.toml")).unwrap();
    (dir, config)
}

async fn stub(variables: bool) -> Stub {
    start(Script {
        file: SNAPSHOT.into(),
        file_depth1: Some(DEPTH1.into()),
        variables: variables.then(|| VARIABLES.into()),
        images: BTreeMap::from([
            ("1:30".to_owned(), Some("{base}/cdn/icon.svg".to_owned())),
            ("2:11".to_owned(), None),
        ]),
        ..Script::default()
    })
    .await
}

fn client(stub: &Stub) -> FigmaClient {
    FigmaClient::new(Token::new("figd_x".into()), &stub.base).unwrap()
}

#[tokio::test]
async fn pull_writes_normalized_snapshots_for_selected_pages() {
    let (dir, config) = app(CONFIG);
    let stub = stub(true).await;
    let report = pull(&config, &client(&stub)).await.unwrap();
    assert_eq!(report.snapshot, dir.path().join("figma/snapshot.json"));
    assert_eq!(
        report.variables,
        Some(dir.path().join("figma/variables.json"))
    );
    let written = fs::read_to_string(&report.snapshot).unwrap();
    assert_eq!(written, normalize_json(SNAPSHOT.as_bytes()).unwrap());
    let seen = stub.seen();
    assert_eq!(seen[0].query["depth"], "1");
    assert_eq!(seen[1].query["ids"], "1:0,2:0", "only the selected pages");
}

#[tokio::test]
async fn pull_without_pages_fetches_the_whole_file_once() {
    let (_dir, config) = app("file_key = \"AbC123xyz\"");
    let stub = stub(true).await;
    pull(&config, &client(&stub)).await.unwrap();
    let files: Vec<_> = stub
        .seen()
        .into_iter()
        .filter(|s| s.path == "/v1/files/AbC123xyz")
        .collect();
    assert_eq!(files.len(), 1);
    assert!(files[0].query.is_empty());
}

#[tokio::test]
async fn missing_page_is_a_config_error() {
    let (_dir, config) = app("file_key = \"AbC123xyz\"\npages = [\"Nope\"]");
    let stub = stub(true).await;
    let err = pull(&config, &client(&stub)).await.unwrap_err();
    assert_eq!(err.kind(), ErrorKind::Config);
    assert!(err.message().contains("Nope"));
}

#[tokio::test]
async fn forbidden_variables_remove_the_old_snapshot_with_a_notice() {
    let (dir, config) = app(CONFIG);
    fs::create_dir_all(dir.path().join("figma")).unwrap();
    fs::write(dir.path().join("figma/variables.json"), "{}").unwrap();
    let stub = stub(false).await;
    let report = pull(&config, &client(&stub)).await.unwrap();
    assert_eq!(report.variables, None);
    assert!(!dir.path().join("figma/variables.json").exists());
    assert!(
        report.notices.iter().any(|n| n.contains("Enterprise")),
        "{:?}",
        report.notices
    );
}

#[tokio::test]
async fn assets_are_downloaded_in_batches_without_the_token() {
    let (dir, config) = app(CONFIG);
    let stub = stub(true).await;
    let report = pull(&config, &client(&stub)).await.unwrap();
    assert_eq!(
        report.assets,
        [dir.path().join("static/figma/icon-check--1-30.svg")]
    );
    assert!(
        report.notices.iter().any(|n| n.contains("2:11")),
        "{:?}",
        report.notices
    );
    let seen = stub.seen();
    let cdn = seen.iter().find(|s| s.path == "/cdn/icon.svg").unwrap();
    assert_eq!(cdn.token, None, "no token to the CDN");
    assert_eq!(fs::read(&report.assets[0]).unwrap(), b"bytes of icon.svg");
    let svg = seen
        .iter()
        .find(|s| s.path.starts_with("/v1/images") && s.query["format"] == "svg")
        .unwrap();
    assert_eq!(svg.query["ids"], "1:30");
    let png = seen
        .iter()
        .find(|s| s.path.starts_with("/v1/images") && s.query["format"] == "png")
        .unwrap();
    assert_eq!(png.query["ids"], "2:11");
}
