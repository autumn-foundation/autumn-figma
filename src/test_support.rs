//! A local stand-in for the Figma REST API.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use axum::Router;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;

/// One recorded request.
#[derive(Debug, Clone)]
pub struct Seen {
    pub path: String,
    pub query: BTreeMap<String, String>,
    pub token: Option<String>,
}

/// What the stub answers.
#[derive(Debug, Clone, Default)]
pub struct Script {
    /// Body of `GET /v1/files/:key` without `depth`.
    pub file: String,
    /// Body with `depth=1`.
    pub file_depth1: Option<String>,
    /// Status of `GET /v1/files/:key` (default 200).
    pub file_status: Option<u16>,
    /// `Retry-After` on a 429.
    pub retry_after: Option<&'static str>,
    /// Variables body, or `None` for a 403.
    pub variables: Option<String>,
    /// Image URL by node id. `None` means Figma cannot render it.
    /// `{base}` in a URL becomes the stub base URL.
    pub images: BTreeMap<String, Option<String>>,
}

#[derive(Clone)]
struct Shared {
    script: Arc<Script>,
    seen: Arc<Mutex<Vec<Seen>>>,
    base: String,
}

pub struct Stub {
    pub base: String,
    seen: Arc<Mutex<Vec<Seen>>>,
}

impl Stub {
    pub fn seen(&self) -> Vec<Seen> {
        self.seen.lock().unwrap().clone()
    }
}

fn record(shared: &Shared, path: String, query: BTreeMap<String, String>, headers: &HeaderMap) {
    let token = headers
        .get("x-figma-token")
        .map(|v| v.to_str().unwrap().to_owned());
    shared.seen.lock().unwrap().push(Seen { path, query, token });
}

async fn file(
    State(s): State<Shared>,
    Path(key): Path<String>,
    Query(q): Query<BTreeMap<String, String>>,
    headers: HeaderMap,
) -> Response {
    record(&s, format!("/v1/files/{key}"), q.clone(), &headers);
    if let Some(code) = s.script.file_status {
        let mut res = (StatusCode::from_u16(code).unwrap(), "{\"status\":0,\"err\":\"no\"}").into_response();
        if let Some(after) = s.script.retry_after {
            res.headers_mut().insert("retry-after", after.parse().unwrap());
        }
        return res;
    }
    let body = if q.get("depth").map(String::as_str) == Some("1") {
        s.script.file_depth1.clone().unwrap_or_else(|| s.script.file.clone())
    } else {
        s.script.file.clone()
    };
    (StatusCode::OK, body).into_response()
}

async fn variables(State(s): State<Shared>, Path(key): Path<String>, headers: HeaderMap) -> Response {
    record(&s, format!("/v1/files/{key}/variables/local"), BTreeMap::new(), &headers);
    s.script.variables.clone().map_or_else(
        || (StatusCode::FORBIDDEN, "{\"status\":403,\"error\":true,\"message\":\"Limited by Figma plan\"}").into_response(),
        |body| (StatusCode::OK, body).into_response(),
    )
}

async fn images(
    State(s): State<Shared>,
    Path(key): Path<String>,
    Query(q): Query<BTreeMap<String, String>>,
    headers: HeaderMap,
) -> Response {
    record(&s, format!("/v1/images/{key}"), q.clone(), &headers);
    let ids = q.get("ids").cloned().unwrap_or_default();
    let map: BTreeMap<String, Option<String>> = ids
        .split(',')
        .map(|id| {
            let url = s.script.images.get(id).cloned().flatten();
            (id.to_owned(), url.map(|u| u.replace("{base}", &s.base)))
        })
        .collect();
    let body = serde_json::json!({ "err": null, "images": map });
    (StatusCode::OK, body.to_string()).into_response()
}

async fn cdn(State(s): State<Shared>, Path(name): Path<String>, headers: HeaderMap) -> Response {
    record(&s, format!("/cdn/{name}"), BTreeMap::new(), &headers);
    (StatusCode::OK, format!("bytes of {name}")).into_response()
}

/// Start the stub on a free local port.
pub async fn start(script: Script) -> Stub {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let seen = Arc::new(Mutex::new(Vec::new()));
    let shared = Shared {
        script: Arc::new(script),
        seen: Arc::clone(&seen),
        base: base.clone(),
    };
    let app = Router::new()
        .route("/v1/files/{key}", get(file))
        .route("/v1/files/{key}/variables/local", get(variables))
        .route("/v1/images/{key}", get(images))
        .route("/cdn/{name}", get(cdn))
        .with_state(shared);
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    Stub { base, seen }
}
