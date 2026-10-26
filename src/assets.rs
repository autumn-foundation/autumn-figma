//! Nodes that render as image files.
//!
//! An icon-like node (vectors only, no text) exports as SVG. A node with an
//! image fill and no children exports as PNG. The generated code shows both
//! with `img`.

use crate::model::Node;

/// The file format of an asset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Format {
    /// SVG, for vectors.
    Svg,
    /// PNG at 2x, for image fills.
    Png,
}

impl Format {
    /// The value of the `format` query parameter and the file extension.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Svg => "svg",
            Self::Png => "png",
        }
    }
}

/// One asset to export.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Asset {
    /// The node to render.
    pub node_id: String,
    /// The file name in the asset directory.
    pub file_name: String,
    /// The format.
    pub format: Format,
}

/// The asset format for `node`, or `None` when it is not an asset.
#[must_use]
pub fn format_of(node: &Node) -> Option<Format> {
    let _ = node;
    None
}

/// The asset for `node`, or `None` when it is not an asset.
#[must_use]
pub fn asset_of(node: &Node) -> Option<Asset> {
    let _ = node;
    None
}

#[cfg(test)]
mod tests;
