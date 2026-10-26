//! Fetch the design: the file snapshot, the variables snapshot, and the
//! assets.

use std::fs;
use std::path::{Path, PathBuf};

use crate::assets::Format;
use crate::client::FigmaClient;
use crate::config::{Config, TOKEN_ENV};
use crate::error::{Error, ErrorKind, Result};
use crate::lower::lower;
use crate::model::{FileResponse, NodeType, VariablesResponse, normalize_json};

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
    let key = &config.file_key;
    let mut report = PullReport {
        snapshot: config.snapshot.clone(),
        ..PullReport::default()
    };
    // Fetch everything first. Write only when all fetches succeed, so the
    // snapshot and the assets always agree.
    let ids = page_ids(config, client).await?;
    let body = client.file(key, &ids, None).await?;
    let snapshot = normalize_json(&body)?;
    let file = FileResponse::from_json(&snapshot)?;

    let variables_text = if let Some(body) = client.variables(key).await? {
        Some(normalize_json(&body)?)
    } else {
        report.notices.push(format!(
            "Figma gave no variables: the variables API needs an Enterprise plan and the file_variables:read scope on {TOKEN_ENV}; tokens come from styles only"
        ));
        None
    };
    let variables = variables_text
        .as_deref()
        .map(VariablesResponse::from_json)
        .transpose()?;

    let lowered = lower(&file, variables.as_ref(), config)?;
    let mut downloads = Vec::new();
    for format in [Format::Svg, Format::Png] {
        let assets: Vec<_> = lowered
            .assets
            .iter()
            .filter(|a| a.format == format)
            .collect();
        for batch in assets.chunks(IMAGE_BATCH) {
            let ids: Vec<String> = batch.iter().map(|a| a.node_id.clone()).collect();
            let images = client
                .images(key, &ids, format, file.version.as_deref())
                .await?;
            for asset in batch {
                let url = images.images.get(&asset.node_id).cloned().flatten();
                let Some(url) = url else {
                    report.notices.push(format!(
                        "Figma cannot render node {} ({}); the asset is missing",
                        asset.node_id, asset.file_name
                    ));
                    continue;
                };
                let bytes = client.download(&url).await?;
                downloads.push((config.assets_dir.join(&asset.file_name), bytes));
            }
        }
    }

    write(&config.snapshot, snapshot.as_bytes())?;
    match &variables_text {
        Some(text) => {
            write(&config.variables_snapshot, text.as_bytes())?;
            report.variables = Some(config.variables_snapshot.clone());
        }
        None if config.variables_snapshot.exists() => {
            fs::remove_file(&config.variables_snapshot)
                .map_err(|e| Error::io(&config.variables_snapshot, &e))?;
        }
        None => {}
    }
    for (path, bytes) in downloads {
        write(&path, &bytes)?;
        report.assets.push(path);
    }
    Ok(report)
}

/// The ids of the selected pages. Empty when all pages are selected.
async fn page_ids(config: &Config, client: &FigmaClient) -> Result<Vec<String>> {
    if config.pages.is_empty() {
        return Ok(Vec::new());
    }
    // Page ids do not change. The last snapshot saves a depth=1 request.
    if let Some(ids) = known_page_ids(config) {
        return Ok(ids);
    }
    let body = client.file(&config.file_key, &[], Some(1)).await?;
    let text = std::str::from_utf8(&body)
        .map_err(|e| Error::new(ErrorKind::Json, format!("Figma file: {e}")))?;
    let shallow = FileResponse::from_json(text)?;
    let pages: Vec<_> = shallow
        .document
        .children
        .iter()
        .filter(|n| n.kind == NodeType::Canvas)
        .collect();
    config
        .pages
        .iter()
        .map(|wanted| {
            pages
                .iter()
                .find(|p| p.name.trim() == wanted.trim())
                .map(|p| p.id.clone())
                .ok_or_else(|| {
                    let names: Vec<&str> = pages.iter().map(|p| p.name.as_str()).collect();
                    Error::new(
                        ErrorKind::Config,
                        format!(
                            "page {wanted:?} is not in the file; its pages are: {}",
                            names.join(", ")
                        ),
                    )
                })
        })
        .collect()
}

/// The ids of the selected pages from the last snapshot, when it has all
/// of them.
fn known_page_ids(config: &Config) -> Option<Vec<String>> {
    let text = fs::read_to_string(&config.snapshot).ok()?;
    let file = FileResponse::from_json(&text).ok()?;
    config
        .pages
        .iter()
        .map(|wanted| {
            file.document
                .children
                .iter()
                .find(|p| p.kind == NodeType::Canvas && p.name.trim() == wanted.trim())
                .map(|p| p.id.clone())
        })
        .collect()
}

fn write(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| Error::io(dir, &e))?;
    }
    fs::write(path, bytes).map_err(|e| Error::io(path, &e))
}

#[cfg(test)]
mod tests;
