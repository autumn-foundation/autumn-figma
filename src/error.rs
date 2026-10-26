//! The crate error type.

use std::fmt;

/// The result type of this crate.
pub type Result<T> = std::result::Result<T, Error>;

/// The class of an [`Error`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ErrorKind {
    /// `figma.toml` is missing or not valid.
    Config,
    /// A file read or write failed.
    Io,
    /// A snapshot or API body is not valid JSON for its shape.
    Json,
    /// The token is missing, or Figma refused it (401 or 403).
    Auth,
    /// Figma has no such file or node (404).
    NotFound,
    /// Figma limits the request rate (429).
    RateLimited,
    /// Any other HTTP or transport failure.
    Http,
    /// A `[[bind]]` entry does not match a node or is not valid there.
    Binding,
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Config => "config error",
            Self::Io => "io error",
            Self::Json => "json error",
            Self::Auth => "auth error",
            Self::NotFound => "not found",
            Self::RateLimited => "rate limited",
            Self::Http => "http error",
            Self::Binding => "binding error",
        })
    }
}

/// An error from this crate.
#[derive(Debug, thiserror::Error)]
#[error("{kind}: {message}")]
pub struct Error {
    kind: ErrorKind,
    message: String,
    retry_after: Option<u64>,
}

impl Error {
    /// Make an error of `kind` with `message`.
    #[must_use]
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            retry_after: None,
        }
    }

    /// Make a rate-limit error with the server's retry hint in seconds.
    #[must_use]
    pub fn rate_limited(retry_after: Option<u64>) -> Self {
        let hint = retry_after.map_or_else(String::new, |s| format!(" Retry after {s}s."));
        Self {
            kind: ErrorKind::RateLimited,
            message: format!("Figma limits the request rate.{hint}"),
            retry_after,
        }
    }

    /// The error class.
    #[must_use]
    pub const fn kind(&self) -> ErrorKind {
        self.kind
    }

    /// The message, without the class prefix.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// The retry hint in seconds, for [`ErrorKind::RateLimited`].
    #[must_use]
    pub const fn retry_after(&self) -> Option<u64> {
        self.retry_after
    }

    pub(crate) fn io(path: &std::path::Path, err: &std::io::Error) -> Self {
        Self::new(ErrorKind::Io, format!("{}: {err}", path.display()))
    }
}

#[cfg(test)]
mod tests;
