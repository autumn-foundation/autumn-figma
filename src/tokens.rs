//! Design tokens: Figma variables and styles as CSS custom properties.
//!
//! - Each variable becomes `--<slug of its name>`. The `[tokens]` table can
//!   rename it, for example to Autumn's `--primary`.
//! - The default mode goes on `:root`. Other modes go on
//!   `[data-theme="<mode>"]`. A mode with "dark" in its name also goes under
//!   `@media (prefers-color-scheme: dark)`.
//! - An alias becomes `var(--other)`.
//! - A style becomes a token at its first use, with the value of that use.

use std::collections::BTreeMap;

use crate::diagnostics::Diagnostic;
use crate::model::{Color, VariablesResponse};
use crate::names::Allocator;

/// The token set of one generation run.
#[derive(Debug, Clone, Default)]
pub struct TokenSet {
    names: Allocator,
    /// CSS name by variable id.
    variables: BTreeMap<String, String>,
    /// CSS name by style id.
    styles: BTreeMap<String, String>,
    /// Declarations on `:root`, in output order.
    root: Vec<(String, String)>,
    /// Declarations by mode slug.
    modes: BTreeMap<String, Vec<(String, String)>>,
    renames: BTreeMap<String, String>,
}

impl TokenSet {
    /// Build the set from the variables snapshot, if any.
    #[must_use]
    pub fn new(
        variables: Option<&VariablesResponse>,
        renames: &BTreeMap<String, String>,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Self {
        let _ = (variables, diagnostics);
        Self {
            names: Allocator::new("-"),
            renames: renames.clone(),
            ..Self::default()
        }
    }

    /// `var(--name)` for a variable id, if the set has it.
    #[must_use]
    pub fn variable(&self, id: &str) -> Option<String> {
        let _ = id;
        None
    }

    /// `var(--name)` for a style. The first call for a style id sets its
    /// value from `value`. Returns `None` when `value` gives `None` on the
    /// first call.
    pub fn style(
        &mut self,
        style_id: &str,
        style_name: &str,
        value: impl FnOnce() -> Option<String>,
    ) -> Option<String> {
        let _ = (style_id, style_name, value);
        None
    }

    /// True when the set has no declarations.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.root.is_empty() && self.modes.is_empty()
    }

    /// The CSS text: `:root`, then one block per mode.
    #[must_use]
    pub fn css(&self) -> String {
        String::new()
    }
}

/// A CSS color: `#rrggbb`, or `rgba(...)` when alpha × `opacity` < 1.
#[must_use]
pub fn css_color(color: &Color, opacity: f64) -> String {
    let _ = (color, opacity);
    String::new()
}

#[cfg(test)]
mod tests;
