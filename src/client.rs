//! A small Figma REST client.
//!
//! The client sends `X-Figma-Token` only to the API base URL. Image
//! downloads go to the URLs that Figma returns, without the token.

use std::fmt;
use std::time::Duration;

use crate::assets::Format;
use crate::config::Token;
use crate::error::{Error, ErrorKind, Result};
use crate::model::ImagesResponse;

/// The Figma REST client.
#[derive(Clone)]
pub struct FigmaClient {
    http: reqwest::Client,
    base: String,
    token: Token,
}

impl fmt::Debug for FigmaClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FigmaClient")
            .field("base", &self.base)
            .field("token", &self.token)
            .finish_non_exhaustive()
    }
}

impl FigmaClient {
    /// Make a client for `base` (for example `https://api.figma.com`).
    ///
    /// # Errors
    ///
    /// [`ErrorKind::Http`] when the HTTP client cannot start.
    pub fn new(token: Token, base: &str) -> Result<Self> {
        let http = reqwest::Client::builder()
            .user_agent(concat!("autumn-figma/", env!("CARGO_PKG_VERSION")))
            .connect_timeout(Duration::from_secs(15))
            .timeout(Duration::from_secs(120))
            .build()
            .map_err(|e| Error::new(ErrorKind::Http, format!("HTTP client: {e}")))?;
        Ok(Self {
            http,
            base: base.trim_end_matches('/').to_owned(),
            token,
        })
    }

    fn api(&self, path: &str) -> reqwest::RequestBuilder {
        self.http
            .get(format!("{}{path}", self.base))
            .header("X-Figma-Token", self.token.expose())
    }

    /// Send a request. Map the status to an error kind.
    async fn send(request: reqwest::RequestBuilder) -> Result<Vec<u8>> {
        let response = request.send().await.map_err(|e| {
            // The caller names the URL. A signed URL must not reach a log.
            let e = e.without_url();
            Error::new(ErrorKind::Http, format!("request failed: {e}"))
        })?;
        let status = response.status().as_u16();
        match status {
            200..=299 => response.bytes().await.map(|b| b.to_vec()).map_err(|e| {
                let e = e.without_url();
                Error::new(ErrorKind::Http, format!("reading the response: {e}"))
            }),
            401 | 403 => Err(Error::new(
                ErrorKind::Auth,
                format!(
                    "Figma refused the token (HTTP {status}); check FIGMA_TOKEN and its scopes"
                ),
            )),
            404 => Err(Error::new(
                ErrorKind::NotFound,
                "Figma has no such file or node (HTTP 404); check file_key",
            )),
            429 => {
                let retry = response
                    .headers()
                    .get("retry-after")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.trim().parse().ok());
                Err(Error::rate_limited(retry))
            }
            _ => Err(Error::new(
                ErrorKind::Http,
                format!("Figma answered HTTP {status}"),
            )),
        }
    }

    fn check_key(key: &str) -> Result<()> {
        if !key.is_empty() && key.chars().all(|c| c.is_ascii_alphanumeric()) {
            Ok(())
        } else {
            Err(Error::new(
                ErrorKind::Config,
                format!("{key:?} is not a Figma file key"),
            ))
        }
    }

    /// `GET /v1/files/:key`, with optional `ids` and `depth`. Returns the
    /// raw body.
    ///
    /// # Errors
    ///
    /// [`ErrorKind::Auth`] on 401 or 403, [`ErrorKind::NotFound`] on 404,
    /// [`ErrorKind::RateLimited`] on 429, and [`ErrorKind::Http`] on other
    /// failures.
    pub async fn file(&self, key: &str, ids: &[String], depth: Option<u32>) -> Result<Vec<u8>> {
        Self::check_key(key)?;
        let mut request = self.api(&format!("/v1/files/{key}"));
        if !ids.is_empty() {
            request = request.query(&[("ids", ids.join(","))]);
        }
        if let Some(depth) = depth {
            request = request.query(&[("depth", depth)]);
        }
        Self::send(request).await
    }

    /// `GET /v1/files/:key/variables/local`. Returns `None` on 403 or 404:
    /// the plan or the token scope does not allow variables.
    ///
    /// # Errors
    ///
    /// As [`Self::file`], except 403 and 404.
    pub async fn variables(&self, key: &str) -> Result<Option<Vec<u8>>> {
        Self::check_key(key)?;
        match Self::send(self.api(&format!("/v1/files/{key}/variables/local"))).await {
            Ok(body) => Ok(Some(body)),
            Err(e) if matches!(e.kind(), ErrorKind::Auth | ErrorKind::NotFound) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// `GET /v1/images/:key` for `ids` in `format`. PNG renders at 2x.
    ///
    /// # Errors
    ///
    /// As [`Self::file`], and [`ErrorKind::Json`] on a bad body.
    pub async fn images(
        &self,
        key: &str,
        ids: &[String],
        format: Format,
    ) -> Result<ImagesResponse> {
        Self::check_key(key)?;
        let mut request = self.api(&format!("/v1/images/{key}")).query(&[
            ("ids", ids.join(",")),
            ("format", format.as_str().to_owned()),
        ]);
        if format == Format::Png {
            request = request.query(&[("scale", "2")]);
        }
        let body = Self::send(request).await?;
        let images: ImagesResponse = serde_json::from_slice(&body)
            .map_err(|e| Error::new(ErrorKind::Json, format!("Figma images: {e}")))?;
        if let Some(err) = images.err.as_deref().filter(|e| !e.is_empty()) {
            return Err(Error::new(
                ErrorKind::Http,
                format!("Figma could not render images: {err}"),
            ));
        }
        Ok(images)
    }

    /// Download a rendered image. The request has no token.
    ///
    /// # Errors
    ///
    /// [`ErrorKind::Http`] on any failure.
    pub async fn download(&self, url: &str) -> Result<Vec<u8>> {
        Self::send(self.http.get(url)).await.map_err(|e| {
            // A signed URL carries its signature in the query. Keep it out of logs.
            let shown = url.split('?').next().unwrap_or_default();
            Error::new(
                ErrorKind::Http,
                format!("download {shown}: {}", e.message()),
            )
        })
    }
}

#[cfg(test)]
mod tests;
