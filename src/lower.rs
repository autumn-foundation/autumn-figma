//! Lower the Figma tree to the intermediate representation.
//!
//! - Each selected page becomes a module.
//! - Each component set and each component outside a set becomes a
//!   function. Its properties become parameters.
//! - Each top-level frame of a page becomes a screen function.
//! - An instance of a component of this run becomes a call. An instance of
//!   another component is inlined.
//! - `[[bind]]` entries set tags, ids, attributes, slots, and text
//!   parameters.

use crate::assets::Asset;
use crate::config::Config;
use crate::diagnostics::Diagnostic;
use crate::error::{Error, ErrorKind, Result};
use crate::ir::{CssRule, Module};
use crate::model::{FileResponse, VariablesResponse};
use crate::tokens::TokenSet;

/// The result of [`lower`].
#[derive(Debug, Clone)]
pub struct Lowered {
    /// One module per selected page with at least one function, in page
    /// order.
    pub modules: Vec<Module>,
    /// CSS rules in output order.
    pub rules: Vec<CssRule>,
    /// Tokens from variables and from the styles in use.
    pub tokens: TokenSet,
    /// Assets that the output refers to, sorted and without duplicates.
    pub assets: Vec<Asset>,
    /// Notes for the user.
    pub diagnostics: Vec<Diagnostic>,
}

/// Lower `file` with `config`.
///
/// # Errors
///
/// - [`ErrorKind::Config`] when a page in `pages` is not in the file.
/// - [`ErrorKind::Binding`] when a binding matches no node, matches a node
///   that cannot take it, or two bindings match one node.
pub fn lower(
    file: &FileResponse,
    variables: Option<&VariablesResponse>,
    config: &Config,
) -> Result<Lowered> {
    let _ = (file, variables, config);
    Err(Error::new(ErrorKind::Config, "not implemented"))
}

#[cfg(test)]
mod tests;
