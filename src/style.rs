//! CSS declarations for one node.
//!
//! Auto layout maps to flexbox, grid layout to CSS grid. A frame without
//! auto layout places its children with absolute positions, and gives a
//! warning. Tokens win over literal values.

use std::collections::BTreeMap;

use crate::diagnostics::Diagnostic;
use crate::model::{Node, Rect, StyleMeta};
use crate::tokens::TokenSet;

/// CSS declarations in output order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Decls(pub Vec<(String, String)>);

impl Decls {
    /// Add a declaration.
    pub fn push(&mut self, property: &str, value: impl Into<String>) {
        self.0.push((property.to_owned(), value.into()));
    }

    /// The value of `property`, if set.
    #[must_use]
    pub fn get(&self, property: &str) -> Option<&str> {
        self.0
            .iter()
            .find(|(p, _)| p == property)
            .map(|(_, v)| v.as_str())
    }

    /// True when there are no declarations.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// How the parent places this node.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Parent {
    /// The node is the root of a generated function.
    Root {
        /// True for a screen (a top-level frame), false for a component.
        screen: bool,
    },
    /// A flex parent.
    Flex {
        /// True for a row, false for a column.
        row: bool,
    },
    /// A grid parent.
    Grid,
    /// A parent without auto layout. `origin` is the parent box.
    Absolute {
        /// The parent box in canvas coordinates.
        origin: Rect,
    },
}

/// What the node renders as.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// A box (`div` and friends).
    Box,
    /// Text.
    Text,
    /// An exported image (`img`).
    Image,
}

/// The context of one style call.
#[derive(Debug)]
pub struct StyleCx<'a> {
    /// The token set. Style tokens are added at first use.
    pub tokens: &'a mut TokenSet,
    /// Style metadata of the file.
    pub styles: &'a BTreeMap<String, StyleMeta>,
    /// Notes for the user.
    pub diagnostics: &'a mut Vec<Diagnostic>,
    /// The layer path of the node, for notes.
    pub path: &'a str,
}

/// The CSS declarations for `node`.
#[must_use]
pub fn node_decls(node: &Node, parent: Parent, role: Role, cx: &mut StyleCx<'_>) -> Decls {
    let _ = (node, parent, role, cx);
    Decls::default()
}

#[cfg(test)]
mod tests;
