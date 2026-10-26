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
    let limited = status_error(429, Some("120")).await;
    assert_eq!(limited.kind(), ErrorKind::RateLimited);
    assert_eq!(
        limited.retry_after(),
        Some(120),
        "too long to wait: no retry"
    );
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
        .images("K", &["1:1".into(), "1:2".into()], Format::Svg, Some("42"))
        .await
        .unwrap();
    assert_eq!(res.images["1:1"].as_deref(), Some("http://x/a.svg"));
    assert_eq!(res.images["1:2"], None);
    let seen = stub.seen();
    assert_eq!(seen[0].query["format"], "svg");
    assert_eq!(seen[0].query["version"], "42", "assets match the snapshot");
    assert_eq!(seen[0].query["use_absolute_bounds"], "true");
    assert!(!seen[0].query.contains_key("scale"));

    c.images("K", &["1:1".into()], Format::Png, None)
        .await
        .unwrap();
    assert_eq!(stub.seen()[1].query["scale"], "2");

    let bytes = c
        .download(&format!("{}/cdn/a.svg", stub.base))
        .await
        .unwrap();
    assert_eq!(bytes, b"bytes of a.svg");
    assert_eq!(stub.seen()[2].token, None, "no token to the CDN");
}

#[tokio::test]
async fn download_errors_hide_the_signed_query() {
    let err = client("http://127.0.0.1:1")
        .download("http://127.0.0.1:1/img.png?X-Amz-Signature=secret")
        .await
        .unwrap_err();
    assert_eq!(err.kind(), ErrorKind::Http);
    assert!(err.message().contains("/img.png"), "{err}");
    assert!(!err.message().contains("secret"), "{err}");
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

#[tokio::test]
async fn the_token_never_follows_a_redirect() {
    let stub = start(Script::default()).await;
    let err = client(&stub.base)
        .file("REDIRECT", &[], None)
        .await
        .unwrap_err();
    assert_eq!(err.kind(), ErrorKind::Http);
    assert!(err.message().contains("redirect"), "{err}");
    assert!(
        stub.seen().iter().all(|s| s.path != "/cdn/steal"),
        "the client must not follow: {:?}",
        stub.seen()
    );
}

#[test]
fn plain_http_is_only_for_loopback() {
    for base in [
        "http://127.0.0.1:9",
        "http://localhost:9",
        "http://[::1]:9",
        "https://api.figma.com",
    ] {
        assert!(
            FigmaClient::new(Token::new("t".into()), base).is_ok(),
            "{base}"
        );
    }
    for base in [
        "http://api.figma.com",
        "ftp://x",
        "http://127.0.0.1.evil.com",
        "http://localhost:9@evil.com",
    ] {
        let err = FigmaClient::new(Token::new("t".into()), base).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::Config, "{base}");
    }
}

#[tokio::test]
async fn downloads_are_capped_and_need_https_off_loopback() {
    let stub = start(Script::default()).await;
    let small = client(&stub.base).download_limit(4);
    let err = small
        .download(&format!("{}/cdn/a.svg", stub.base))
        .await
        .unwrap_err();
    assert!(err.message().contains("larger than"), "{err}");
    let err = client(&stub.base)
        .download("http://example.com/a.svg")
        .await
        .unwrap_err();
    assert!(err.message().contains("https"), "{err}");
    let err = client(&stub.base)
        .download("file:///etc/passwd")
        .await
        .unwrap_err();
    assert!(err.message().contains("https"), "{err}");
}

#[tokio::test]
async fn short_rate_limits_are_retried() {
    let stub = start(Script {
        file: "{}".into(),
        rate_limit_first: 2,
        ..Script::default()
    })
    .await;
    let body = client(&stub.base).file("K", &[], None).await.unwrap();
    assert_eq!(body, b"{}");
    assert_eq!(stub.seen().len(), 3, "two 429s, then the answer");

    let stub = start(Script {
        file: "{}".into(),
        rate_limit_first: 9,
        ..Script::default()
    })
    .await;
    let err = client(&stub.base).file("K", &[], None).await.unwrap_err();
    assert_eq!(err.kind(), ErrorKind::RateLimited);
    assert!(err.message().contains("low"), "names the limit type: {err}");
}

#[tokio::test]
async fn oauth_tokens_use_the_bearer_header() {
    let stub = start(Script {
        file: "{}".into(),
        ..Script::default()
    })
    .await;
    let c = FigmaClient::new(Token::oauth("oauth_secret".into()), &stub.base).unwrap();
    c.file("K", &[], None).await.unwrap();
    let seen = &stub.seen()[0];
    assert_eq!(seen.auth.as_deref(), Some("Bearer oauth_secret"));
    assert_eq!(seen.token, None);
}
