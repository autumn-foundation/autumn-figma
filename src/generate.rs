//! Generate, write, and check the output files.
//!
//! [`generate`] reads the snapshots and gives an [`Output`]. It does not
//! write. [`Output::write`] writes the files and deletes stale generated
//! files. [`Output::drift`] compares the files on disk with the output.

use std::path::PathBuf;

use crate::assets::Asset;
use crate::config::Config;
use crate::diagnostics::Diagnostic;
use crate::error::{Error, ErrorKind, Result};
use crate::model::{FileResponse, VariablesResponse};

/// The files of one generation run.
#[derive(Debug, Clone)]
pub struct Output {
    /// File contents by absolute path, in path order.
    pub files: Vec<(PathBuf, String)>,
    /// Generated Rust files in `out_dir` that this run does not produce.
    pub stale: Vec<PathBuf>,
    /// Assets that the output refers to.
    pub assets: Vec<Asset>,
    /// Notes for the user.
    pub diagnostics: Vec<Diagnostic>,
}

/// One difference between the disk and the output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Drift {
    /// The file on disk has other content.
    Changed(PathBuf),
    /// The file is not on disk.
    Missing(PathBuf),
    /// A generated file that the output does not have.
    Stale(PathBuf),
}

/// What [`Output::write`] did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WriteReport {
    /// Files written because they changed or were missing.
    pub written: Vec<PathBuf>,
    /// Files with no change.
    pub unchanged: Vec<PathBuf>,
    /// Stale generated files deleted.
    pub deleted: Vec<PathBuf>,
}

/// Read the snapshots of `config` and generate the output.
///
/// The variables snapshot is optional.
///
/// # Errors
///
/// [`ErrorKind::Io`] when the file snapshot is missing (run `pull`
/// first), [`ErrorKind::Json`] when a snapshot is not valid, and the
/// errors of [`crate::lower::lower`].
pub fn generate(config: &Config) -> Result<Output> {
    let _ = config;
    Err(Error::new(ErrorKind::Io, "not implemented"))
}

/// Generate the output from parsed snapshots. Stale files are not found
/// here; [`generate`] adds them.
///
/// # Errors
///
/// The errors of [`crate::lower::lower`].
pub fn generate_from(
    config: &Config,
    file: &FileResponse,
    variables: Option<&VariablesResponse>,
) -> Result<Output> {
    let _ = (config, file, variables);
    Err(Error::new(ErrorKind::Io, "not implemented"))
}

impl Output {
    /// Write changed and missing files. Delete stale generated files.
    ///
    /// # Errors
    ///
    /// [`ErrorKind::Io`] when a write or delete fails.
    pub fn write(&self) -> Result<WriteReport> {
        Err(Error::new(ErrorKind::Io, "not implemented"))
    }

    /// The differences between the disk and the output. Empty means in
    /// sync.
    ///
    /// # Errors
    ///
    /// [`ErrorKind::Io`] when a file exists but cannot be read.
    pub fn drift(&self) -> Result<Vec<Drift>> {
        Err(Error::new(ErrorKind::Io, "not implemented"))
    }

    /// Asset files in the asset directory that are not on disk.
    #[must_use]
    pub fn missing_assets(&self, config: &Config) -> Vec<PathBuf> {
        let _ = config;
        Vec::new()
    }
}

#[cfg(test)]
mod tests;
