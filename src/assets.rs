//! Nodes that render as image files.
//!
//! An icon-like node (vectors only, no text) exports as SVG. A node with an
//! image fill and no children exports as PNG. The generated code shows both
//! with `img`.

use crate::model::{Node, NodeType, PaintType};
use crate::names::slug;

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
    if node.kind == NodeType::Text {
        return None;
    }
    if node.kind.is_vector() {
        return Some(Format::Svg);
    }
    let visible_children = node.children.iter().any(|c| c.visible);
    if !visible_children && has_image_fill(node) {
        return Some(Format::Png);
    }
    if node.kind.is_container() && visible_children {
        let mut vector = false;
        if only_shapes(node, &mut vector) && vector {
            return Some(Format::Svg);
        }
    }
    None
}

fn has_image_fill(node: &Node) -> bool {
    node.fills
        .iter()
        .any(|p| p.visible && p.kind == PaintType::Image)
}

/// True when all visible descendants are vectors, plain shapes, or groups
/// of them. Sets `vector` when one is a vector.
fn only_shapes(node: &Node, vector: &mut bool) -> bool {
    node.children.iter().filter(|c| c.visible).all(|c| {
        if c.kind.is_vector() {
            *vector = true;
            true
        } else if matches!(c.kind, NodeType::Rectangle | NodeType::Ellipse) {
            !has_image_fill(c)
        } else if c.kind.is_container() {
            only_shapes(c, vector)
        } else {
            false
        }
    })
}

/// The asset for `node`, or `None` when it is not an asset.
#[must_use]
pub fn asset_of(node: &Node) -> Option<Asset> {
    let format = format_of(node)?;
    Some(Asset {
        node_id: node.id.clone(),
        // A slug never holds `--`, so the separator keeps two names apart.
        file_name: format!(
            "{}--{}.{}",
            slug(&node.name),
            slug(&node.id),
            format.as_str()
        ),
        format,
    })
}

#[cfg(test)]
mod tests;
