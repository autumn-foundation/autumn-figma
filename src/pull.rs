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
    let ids = page_ids(config, client).await?;
    let body = client.file(key, &ids, None).await?;
    let text = normalize_json(&body)?;
    let file = FileResponse::from_json(&text)?;
    write(&config.snapshot, text.as_bytes())?;

    let variables = if let Some(body) = client.variables(key).await? {
        let text = normalize_json(&body)?;
        let vars = VariablesResponse::from_json(&text)?;
        write(&config.variables_snapshot, text.as_bytes())?;
        report.variables = Some(config.variables_snapshot.clone());
        Some(vars)
    } else {
        if config.variables_snapshot.exists() {
            fs::remove_file(&config.variables_snapshot)
                .map_err(|e| Error::io(&config.variables_snapshot, &e))?;
        }
        report.notices.push(format!(
            "Figma gave no variables: the variables API needs an Enterprise plan and the file_variables:read scope on {TOKEN_ENV}; tokens come from styles only"
        ));
        None
    };

    let lowered = lower(&file, variables.as_ref(), config)?;
    for format in [Format::Svg, Format::Png] {
        let assets: Vec<_> = lowered
            .assets
            .iter()
            .filter(|a| a.format == format)
            .collect();
        for batch in assets.chunks(IMAGE_BATCH) {
            let ids: Vec<String> = batch.iter().map(|a| a.node_id.clone()).collect();
            let images = client.images(key, &ids, format).await?;
            for asset in batch {
                let url = images.images.get(&asset.node_id).cloned().flatten();
                let Some(url) = url else {
                    report.notices.push(format!(
                        "Figma could not render node {} ({}); the asset is missing",
                        asset.node_id, asset.file_name
                    ));
                    continue;
                };
                let bytes = client.download(&url).await?;
                let path = config.assets_dir.join(&asset.file_name);
                write(&path, &bytes)?;
                report.assets.push(path);
            }
        }
    }
    Ok(report)
}

/// The ids of the selected pages. Empty when all pages are selected.
async fn page_ids(config: &Config, client: &FigmaClient) -> Result<Vec<String>> {
    if config.pages.is_empty() {
        return Ok(Vec::new());
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

fn write(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| Error::io(dir, &e))?;
    }
    fs::write(path, bytes).map_err(|e| Error::io(path, &e))
}

#[cfg(test)]
mod tests;
