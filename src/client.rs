//! A small Figma REST client.
//!
//! The client sends `X-Figma-Token` only to the API base URL. Image
//! downloads go to the URLs that Figma returns, without the token.

use std::fmt;

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
        let _ = (token, base);
        Err(Error::new(ErrorKind::Http, "not implemented"))
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
        let _ = (key, ids, depth);
        Err(Error::new(ErrorKind::Http, "not implemented"))
    }

    /// `GET /v1/files/:key/variables/local`. Returns `None` on 403 or 404:
    /// the plan or the token scope does not allow variables.
    ///
    /// # Errors
    ///
    /// As [`Self::file`], except 403 and 404.
    pub async fn variables(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let _ = key;
        Err(Error::new(ErrorKind::Http, "not implemented"))
    }

    /// `GET /v1/images/:key` for `ids` in `format`. PNG renders at 2x.
    ///
    /// # Errors
    ///
    /// As [`Self::file`], and [`ErrorKind::Json`] on a bad body.
    pub async fn images(&self, key: &str, ids: &[String], format: Format) -> Result<ImagesResponse> {
        let _ = (key, ids, format);
        Err(Error::new(ErrorKind::Http, "not implemented"))
    }

    /// Download a rendered image. The request has no token.
    ///
    /// # Errors
    ///
    /// [`ErrorKind::Http`] on any failure.
    pub async fn download(&self, url: &str) -> Result<Vec<u8>> {
        let _ = url;
        Err(Error::new(ErrorKind::Http, "not implemented"))
    }
}

#[cfg(test)]
mod tests;
