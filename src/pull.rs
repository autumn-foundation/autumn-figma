//! Fetch the design: the file snapshot, the variables snapshot, and the
//! assets.

use std::path::PathBuf;

use crate::client::FigmaClient;
use crate::config::Config;
use crate::error::{Error, ErrorKind, Result};

/// The most node ids in one images request.
pub const IMAGE_BATCH: usize = 50;

/// What [`pull`] did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PullReport {
    /// The file snapshot path.
    pub snapshot: PathBuf,
    /// The variables snapshot path, when Figma gave variables.
    pub variables: Option<PathBuf>,
    /// Asset files written.
    pub assets: Vec<PathBuf>,
    /// Notes for the user, for example "variables need an Enterprise plan".
    pub notices: Vec<String>,
}

/// Fetch the snapshots and the assets of `config`.
///
/// With `pages` set, the fetch asks only for those pages.
///
/// # Errors
///
/// - The errors of [`FigmaClient`].
/// - [`ErrorKind::Config`] when a page in `pages` is not in the file.
/// - The errors of [`crate::lower::lower`], which finds the assets. The
///   snapshots are already written then.
/// - [`ErrorKind::Io`] when a write fails.
pub async fn pull(config: &Config, client: &FigmaClient) -> Result<PullReport> {
    let _ = (config, client);
    Err(Error::new(ErrorKind::Http, "not implemented"))
}

#[cfg(test)]
mod tests;
