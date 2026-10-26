#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;

use super::*;
use crate::test_support::{Script, start};

const SECRET: &str = "figd_test_secret";

fn client(base: &str) -> FigmaClient {
    FigmaClient::new(Token::new(SECRET.into()), base).unwrap()
}

#[tokio::test]
async fn file_sends_the_token_and_the_query() {
    let stub = start(Script {
        file: "{\"ok\":1}".into(),
        ..Script::default()
    })
    .await;
    let body = client(&stub.base)
        .file("KEY1", &["1:0".into(), "2:0".into()], Some(1))
        .await
        .unwrap();
    assert_eq!(body, b"{\"ok\":1}");
    let seen = stub.seen();
    assert_eq!(seen[0].path, "/v1/files/KEY1");
    assert_eq!(seen[0].token.as_deref(), Some(SECRET));
    assert_eq!(seen[0].query["ids"], "1:0,2:0");
    assert_eq!(seen[0].query["depth"], "1");

    client(&stub.base).file("KEY1", &[], None).await.unwrap();
    assert!(stub.seen()[1].query.is_empty());
}

async fn status_error(status: u16, retry_after: Option<&'static str>) -> Error {
    let stub = start(Script {
        file_status: Some(status),
        retry_after,
        ..Script::default()
    })
    .await;
    client(&stub.base).file("K", &[], None).await.unwrap_err()
}

#[tokio::test]
async fn status_codes_map_to_kinds() {
    assert_eq!(status_error(401, None).await.kind(), ErrorKind::Auth);
    assert_eq!(status_error(403, None).await.kind(), ErrorKind::Auth);
    assert_eq!(status_error(404, None).await.kind(), ErrorKind::NotFound);
    let limited = status_error(429, Some("7")).await;
    assert_eq!(limited.kind(), ErrorKind::RateLimited);
    assert_eq!(limited.retry_after(), Some(7));
    let server = status_error(500, None).await;
    assert_eq!(server.kind(), ErrorKind::Http);
    assert!(server.message().contains("500"), "{server}");
    for err in [status_error(401, None).await, server] {
        assert!(!err.to_string().contains(SECRET));
    }
}

#[tokio::test]
async fn variables_403_is_none() {
    let stub = start(Script::default()).await;
    assert_eq!(client(&stub.base).variables("K").await.unwrap(), None);
    let stub = start(Script {
        variables: Some("{\"meta\":{}}".into()),
        ..Script::default()
    })
    .await;
    assert_eq!(
        client(&stub.base).variables("K").await.unwrap().as_deref(),
        Some(&b"{\"meta\":{}}"[..])
    );
    assert_eq!(stub.seen()[0].path, "/v1/files/K/variables/local");
}

#[tokio::test]
async fn images_and_downloads() {
    let stub = start(Script {
        images: BTreeMap::from([
            ("1:1".into(), Some("http://x/a.svg".into())),
            ("1:2".into(), None),
        ]),
        ..Script::default()
    })
    .await;
    let c = client(&stub.base);
    let res = c
        .images("K", &["1:1".into(), "1:2".into()], Format::Svg)
        .await
        .unwrap();
    assert_eq!(res.images["1:1"].as_deref(), Some("http://x/a.svg"));
    assert_eq!(res.images["1:2"], None);
    let seen = stub.seen();
    assert_eq!(seen[0].query["format"], "svg");
    assert!(!seen[0].query.contains_key("scale"));

    c.images("K", &["1:1".into()], Format::Png).await.unwrap();
    assert_eq!(stub.seen()[1].query["scale"], "2");

    let bytes = c
        .download(&format!("{}/cdn/a.svg", stub.base))
        .await
        .unwrap();
    assert_eq!(bytes, b"bytes of a.svg");
    assert_eq!(stub.seen()[2].token, None, "no token to the CDN");
}

#[tokio::test]
async fn transport_errors_are_http_errors() {
    let err = client("http://127.0.0.1:1")
        .file("K", &[], None)
        .await
        .unwrap_err();
    assert_eq!(err.kind(), ErrorKind::Http);
}

#[test]
fn debug_hides_the_token() {
    let c = client("http://127.0.0.1:1");
    let text = format!("{c:?}");
    assert!(!text.contains(SECRET), "{text}");
    assert!(text.contains("redacted"));
}
