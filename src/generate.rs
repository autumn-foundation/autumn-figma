//! Generate, write, and check the output files.
//!
//! [`generate`] reads the snapshots and gives an [`Output`]. It does not
//! write. [`Output::write`] writes the files and deletes stale generated
//! files. [`Output::drift`] compares the files on disk with the output.

use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use crate::assets::Asset;
use crate::config::Config;
use crate::diagnostics::Diagnostic;
use crate::emit::{self, HEADER_MARK, Header};
use crate::error::{Error, ErrorKind, Result};
use crate::lower::lower;
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
    let src = fs::read_to_string(&config.snapshot).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            Error::new(
                ErrorKind::Io,
                format!(
                    "{} is missing; run `autumn-figma pull` first",
                    config.snapshot.display()
                ),
            )
        } else {
            Error::io(&config.snapshot, &e)
        }
    })?;
    let file = FileResponse::from_json(&src)?;
    let variables = match fs::read_to_string(&config.variables_snapshot) {
        Ok(text) => Some(VariablesResponse::from_json(&text)?),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(Error::io(&config.variables_snapshot, &e)),
    };
    let mut output = generate_from(config, &file, variables.as_ref())?;
    output.stale = stale_files(&config.out_dir, &output)?;
    Ok(output)
}

/// Generated Rust files in `dir` that `output` does not have.
fn stale_files(dir: &Path, output: &Output) -> Result<Vec<PathBuf>> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(Error::io(dir, &e)),
    };
    let mut stale = Vec::new();
    for entry in entries {
        let path = entry.map_err(|e| Error::io(dir, &e))?.path();
        let is_rs = path.extension().is_some_and(|e| e == "rs");
        if is_rs && !output.files.iter().any(|(p, _)| *p == path) && is_generated(&path) {
            stale.push(path);
        }
    }
    stale.sort();
    Ok(stale)
}

/// True when the first line of the file has [`HEADER_MARK`].
fn is_generated(path: &Path) -> bool {
    let Ok(file) = fs::File::open(path) else {
        return false;
    };
    let mut first = String::new();
    BufReader::new(file).read_line(&mut first).is_ok() && first.contains(HEADER_MARK)
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
    let lowered = lower(file, variables, config)?;
    let header = Header {
        file_key: config.file_key.clone(),
        version: file.version.clone(),
        file_name: file.name.clone(),
    };
    let mut files: Vec<(PathBuf, String)> = lowered
        .modules
        .iter()
        .map(|m| {
            (
                config.out_dir.join(format!("{}.rs", m.name)),
                emit::module(m, &header),
            )
        })
        .collect();
    files.push((
        config.out_dir.join("mod.rs"),
        emit::mod_rs(&lowered.modules, &header),
    ));
    files.push((
        config.css_path.clone(),
        emit::css(
            &lowered.tokens,
            &config.class_prefix,
            &lowered.rules,
            &header,
        ),
    ));
    files.sort();
    Ok(Output {
        files,
        stale: Vec::new(),
        assets: lowered.assets,
        diagnostics: lowered.diagnostics,
    })
}

impl Output {
    /// Write changed and missing files. Delete stale generated files.
    ///
    /// # Errors
    ///
    /// [`ErrorKind::Io`] when a write or delete fails.
    pub fn write(&self) -> Result<WriteReport> {
        let mut report = WriteReport::default();
        for (path, text) in &self.files {
            if fs::read_to_string(path).is_ok_and(|old| old == *text) {
                report.unchanged.push(path.clone());
                continue;
            }
            if let Some(dir) = path.parent() {
                fs::create_dir_all(dir).map_err(|e| Error::io(dir, &e))?;
            }
            fs::write(path, text).map_err(|e| Error::io(path, &e))?;
            report.written.push(path.clone());
        }
        for path in &self.stale {
            if path.exists() {
                fs::remove_file(path).map_err(|e| Error::io(path, &e))?;
                report.deleted.push(path.clone());
            }
        }
        Ok(report)
    }

    /// The differences between the disk and the output. Empty means in
    /// sync.
    ///
    /// # Errors
    ///
    /// [`ErrorKind::Io`] when a file exists but cannot be read.
    pub fn drift(&self) -> Result<Vec<Drift>> {
        let mut drift = Vec::new();
        for (path, text) in &self.files {
            match fs::read_to_string(path) {
                Ok(old) if old == *text => {}
                Ok(_) => drift.push(Drift::Changed(path.clone())),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    drift.push(Drift::Missing(path.clone()));
                }
                Err(e) => return Err(Error::io(path, &e)),
            }
        }
        drift.extend(
            self.stale
                .iter()
                .filter(|p| p.exists())
                .map(|p| Drift::Stale(p.clone())),
        );
        Ok(drift)
    }

    /// Asset files in the asset directory that are not on disk.
    #[must_use]
    pub fn missing_assets(&self, config: &Config) -> Vec<PathBuf> {
        self.assets
            .iter()
            .map(|a| config.assets_dir.join(&a.file_name))
            .filter(|p| !p.exists())
            .collect()
    }
}

#[cfg(test)]
mod tests;
