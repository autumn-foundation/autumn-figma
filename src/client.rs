//! A small Figma REST client.
//!
//! - The token goes only to the API base URL. The API client follows no
//!   redirect, so the token cannot move to another host.
//! - Image downloads use a second client without the token. They need
//!   `https` and have a size limit.
//! - A short rate limit (429 with `Retry-After` up to [`MAX_RETRY_WAIT`]
//!   seconds) gets up to [`MAX_RETRIES`] retries.

use std::fmt;
use std::time::Duration;

use crate::assets::Format;
use crate::config::{Token, check_base_url, is_loopback_http};
use crate::error::{Error, ErrorKind, Result};
use crate::model::ImagesResponse;

/// The largest download, in bytes. Figma renders are far smaller.
pub const MAX_DOWNLOAD: usize = 20 * 1024 * 1024;

/// The most retries after a 429.
pub const MAX_RETRIES: u32 = 3;

/// The longest `Retry-After`, in seconds, that a retry waits for.
pub const MAX_RETRY_WAIT: u64 = 60;

/// The Figma REST client.
#[derive(Clone)]
pub struct FigmaClient {
    api: reqwest::Client,
    cdn: reqwest::Client,
    base: String,
    token: Token,
    download_limit: usize,
}

impl fmt::Debug for FigmaClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FigmaClient")
            .field("base", &self.base)
            .field("token", &self.token)
            .finish_non_exhaustive()
    }
}

fn http_err(context: &str, e: reqwest::Error) -> Error {
    // A signed URL must not reach a log, so the error drops the URL.
    let e = e.without_url();
    Error::new(ErrorKind::Http, format!("{context}: {e}"))
}

impl FigmaClient {
    /// Make a client for `base` (for example `https://api.figma.com`).
    ///
    /// # Errors
    ///
    /// [`ErrorKind::Config`] when `base` is not `https` (or `http` to
    /// localhost), and [`ErrorKind::Http`] when the HTTP client cannot
    /// start.
    pub fn new(token: Token, base: &str) -> Result<Self> {
        let base = check_base_url(base)?;
        let builder = || {
            reqwest::Client::builder()
                .user_agent(concat!("autumn-figma/", env!("CARGO_PKG_VERSION")))
                .connect_timeout(Duration::from_secs(15))
                .timeout(Duration::from_secs(120))
        };
        let api = builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|e| http_err("HTTP client", e))?;
        let cdn = builder()
            .redirect(reqwest::redirect::Policy::limited(3))
            .build()
            .map_err(|e| http_err("HTTP client", e))?;
        Ok(Self {
            api,
            cdn,
            base,
            token,
            download_limit: MAX_DOWNLOAD,
        })
    }

    /// Set the largest download in bytes. Default [`MAX_DOWNLOAD`].
    #[must_use]
    pub const fn download_limit(mut self, bytes: usize) -> Self {
        self.download_limit = bytes;
        self
    }

    fn api(&self, path: &str) -> reqwest::RequestBuilder {
        let request = self.api.get(format!("{}{path}", self.base));
        if self.token.is_oauth() {
            request.bearer_auth(self.token.expose())
        } else {
            request.header("X-Figma-Token", self.token.expose())
        }
    }

    /// Send an API request. Retry a short rate limit. Map the status to
    /// an error kind.
    async fn send(request: reqwest::RequestBuilder) -> Result<Vec<u8>> {
        let mut attempt = 0;
        loop {
            let this_try = request
                .try_clone()
                .ok_or_else(|| Error::new(ErrorKind::Http, "the request cannot repeat"))?;
            match Self::send_once(this_try).await {
                Err(e)
                    if e.kind() == ErrorKind::RateLimited
                        && attempt < MAX_RETRIES
                        && e.retry_after().is_some_and(|s| s <= MAX_RETRY_WAIT) =>
                {
                    attempt += 1;
                    let wait = e.retry_after().unwrap_or_default();
                    tokio::time::sleep(Duration::from_secs(wait)).await;
                }
                other => return other,
            }
        }
    }

    async fn send_once(request: reqwest::RequestBuilder) -> Result<Vec<u8>> {
        let response = request
            .send()
            .await
            .map_err(|e| http_err("request failed", e))?;
        let status = response.status().as_u16();
        let header = |name: &str| {
            response
                .headers()
                .get(name)
                .and_then(|v| v.to_str().ok())
                .map(str::to_owned)
        };
        match status {
            200..=299 => response
                .bytes()
                .await
                .map(|b| b.to_vec())
                .map_err(|e| http_err("cannot read the response", e)),
            300..=399 => Err(Error::new(
                ErrorKind::Http,
                format!(
                    "Figma answered with a redirect (HTTP {status}). The client does not follow it, so the token stays with the API host."
                ),
            )),
            401 | 403 => Err(Error::new(
                ErrorKind::Auth,
                format!("Figma refused the token (HTTP {status}). Check the token and its scopes."),
            )),
            404 => Err(Error::new(
                ErrorKind::NotFound,
                "Figma has no such file or node (HTTP 404). Check file_key.",
            )),
            429 => {
                let retry = header("retry-after").and_then(|v| v.trim().parse().ok());
                let kind = header("x-figma-rate-limit-type")
                    .map(|k| format!(" Limit type: {k}."))
                    .unwrap_or_default();
                let link = header("x-figma-upgrade-link")
                    .map(|l| format!(" More requests: {l}."))
                    .unwrap_or_default();
                let detail = format!("{kind}{link}");
                Err(Error::rate_limited(retry, &detail))
            }
            _ => Err(Error::new(
                ErrorKind::Http,
                format!("Figma answered HTTP {status}."),
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
    /// [`ErrorKind::RateLimited`] on a 429 that stays after the retries, and
    /// [`ErrorKind::Http`] on a redirect or another failure.
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
    /// With `version`, Figma renders that version, so the assets match
    /// the snapshot. The render uses the layout box of each node.
    ///
    /// # Errors
    ///
    /// As [`Self::file`], and [`ErrorKind::Json`] on a bad body.
    pub async fn images(
        &self,
        key: &str,
        ids: &[String],
        format: Format,
        version: Option<&str>,
    ) -> Result<ImagesResponse> {
        Self::check_key(key)?;
        let mut request = self.api(&format!("/v1/images/{key}")).query(&[
            ("ids", ids.join(",")),
            ("format", format.as_str().to_owned()),
            ("use_absolute_bounds", "true".to_owned()),
        ]);
        if format == Format::Png {
            request = request.query(&[("scale", "2")]);
        }
        if let Some(version) = version {
            request = request.query(&[("version", version)]);
        }
        let body = Self::send(request).await?;
        let images: ImagesResponse = serde_json::from_slice(&body)
            .map_err(|e| Error::new(ErrorKind::Json, format!("Figma images: {e}")))?;
        if let Some(err) = images.err.as_deref().filter(|e| !e.is_empty()) {
            return Err(Error::new(
                ErrorKind::Http,
                format!("Figma cannot render images: {err}"),
            ));
        }
        Ok(images)
    }

    /// Download a rendered image. The request has no token. The URL must
    /// be `https` (or `http` to localhost). The body must not be larger
    /// than the download limit.
    ///
    /// # Errors
    ///
    /// [`ErrorKind::Http`] on any failure.
    pub async fn download(&self, url: &str) -> Result<Vec<u8>> {
        // A signed URL carries its signature in the query. Keep it out of logs.
        let shown = url.split('?').next().unwrap_or_default();
        let fail =
            |message: String| Error::new(ErrorKind::Http, format!("download {shown}: {message}"));
        if !url.starts_with("https://") && !is_loopback_http(url) {
            return Err(fail("the URL must be https".to_owned()));
        }
        let mut response = self
            .cdn
            .get(url)
            .send()
            .await
            .map_err(|e| fail(http_err("request failed", e).message().to_owned()))?;
        if !response.status().is_success() {
            return Err(fail(format!("HTTP {}", response.status().as_u16())));
        }
        let too_big = || {
            fail(format!(
                "the body is larger than {} bytes",
                self.download_limit
            ))
        };
        if response
            .content_length()
            .is_some_and(|n| n > self.download_limit as u64)
        {
            return Err(too_big());
        }
        let mut body = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|e| fail(http_err("cannot read the body", e).message().to_owned()))?
        {
            if body.len() + chunk.len() > self.download_limit {
                return Err(too_big());
            }
            body.extend_from_slice(&chunk);
        }
        Ok(body)
    }
}

#[cfg(test)]
mod tests;
