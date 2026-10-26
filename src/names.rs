//! Safe names: slugs, Rust identifiers, and literals.
//!
//! Layer names come from designers. They can hold any text. These functions
//! turn them into names that cannot break the generated Rust or CSS.

use std::collections::BTreeSet;

/// A lowercase ASCII slug: runs of letters and digits joined by `-`.
///
/// An empty result becomes `node`.
#[must_use]
pub fn slug(name: &str) -> String {
    let _ = name;
    String::new()
}

/// A `snake_case` Rust identifier. Keywords get a `_` suffix.
#[must_use]
pub fn snake_ident(name: &str) -> String {
    let _ = name;
    String::new()
}

/// An `UpperCamelCase` Rust identifier.
#[must_use]
pub fn camel_ident(name: &str) -> String {
    let _ = name;
    String::new()
}

/// True for a Rust keyword (strict or reserved).
#[must_use]
pub fn is_keyword(word: &str) -> bool {
    let _ = word;
    false
}

/// A Rust string literal for `text`, with quotes.
#[must_use]
pub fn rust_str(text: &str) -> String {
    let _ = text;
    String::new()
}

/// A CSS string literal for `text`, with double quotes.
#[must_use]
pub fn css_str(text: &str) -> String {
    let _ = text;
    String::new()
}

/// A number with at most two decimals and no trailing zeros.
#[must_use]
pub fn num(value: f64) -> String {
    let _ = value;
    String::new()
}

/// A CSS length in px. Zero gives `0`.
#[must_use]
pub fn px(value: f64) -> String {
    let _ = value;
    String::new()
}

/// Gives each caller a name that no earlier caller has.
#[derive(Debug, Clone, Default)]
pub struct Allocator {
    used: BTreeSet<String>,
    separator: &'static str,
}

impl Allocator {
    /// Make an allocator that adds `<separator><n>` to a used name.
    #[must_use]
    pub const fn new(separator: &'static str) -> Self {
        Self {
            used: BTreeSet::new(),
            separator,
        }
    }

    /// Mark `name` as used without a claim.
    pub fn reserve(&mut self, name: &str) {
        self.used.insert(name.to_owned());
    }

    /// Return `base`, or `base<sep>2`, `base<sep>3`, ... if it is used.
    pub fn claim(&mut self, base: &str) -> String {
        let _ = (base, self.separator);
        String::new()
    }
}

#[cfg(test)]
mod tests;
