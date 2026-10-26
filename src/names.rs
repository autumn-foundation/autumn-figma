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
    let words = words(name);
    if words.is_empty() {
        return "node".to_owned();
    }
    words.join("-").to_ascii_lowercase()
}

/// Runs of ASCII letters and digits.
fn words(name: &str) -> Vec<&str> {
    name.split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect()
}

/// A `snake_case` Rust identifier. Keywords get a `_` suffix.
#[must_use]
pub fn snake_ident(name: &str) -> String {
    let words = words(name);
    if words.is_empty() {
        return "node".to_owned();
    }
    let mut ident = words.join("_").to_ascii_lowercase();
    if ident.starts_with(|c: char| c.is_ascii_digit()) {
        ident.insert(0, 'n');
    }
    if is_keyword(&ident) {
        ident.push('_');
    }
    ident
}

/// An `UpperCamelCase` Rust identifier.
#[must_use]
pub fn camel_ident(name: &str) -> String {
    let mut ident: String = words(name)
        .iter()
        .map(|w| {
            let mut chars = w.chars();
            chars.next().map_or_else(String::new, |first| {
                first.to_ascii_uppercase().to_string() + chars.as_str()
            })
        })
        .collect();
    if ident.is_empty() {
        return "Node".to_owned();
    }
    if ident.starts_with(|c: char| c.is_ascii_digit()) {
        ident.insert(0, 'V');
    }
    if is_keyword(&ident) {
        ident.push('_');
    }
    ident
}

/// True for a Rust keyword (strict or reserved).
#[must_use]
pub fn is_keyword(word: &str) -> bool {
    const KEYWORDS: &[&str] = &[
        "Self", "abstract", "as", "async", "await", "become", "box", "break", "const", "continue",
        "crate", "do", "dyn", "else", "enum", "extern", "false", "final", "fn", "for", "gen", "if",
        "impl", "in", "let", "loop", "macro", "match", "mod", "move", "mut", "override", "priv",
        "pub", "ref", "return", "self", "static", "struct", "super", "trait", "true", "try",
        "type", "typeof", "unsafe", "unsized", "use", "virtual", "where", "while", "yield",
    ];
    KEYWORDS.contains(&word)
}

/// A Rust string literal for `text`, with quotes.
#[must_use]
pub fn rust_str(text: &str) -> String {
    format!("{text:?}")
}

/// A CSS string literal for `text`, with double quotes.
#[must_use]
pub fn css_str(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '<' => out.push_str("\\3c "),
            c if c.is_control() => {
                use std::fmt::Write as _;
                let _ = write!(out, "\\{:x} ", u32::from(c));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// A number with at most two decimals and no trailing zeros.
#[must_use]
pub fn num(value: f64) -> String {
    let rounded = (value * 100.0).round() / 100.0;
    if rounded == 0.0 || !rounded.is_finite() {
        return "0".to_owned();
    }
    let text = format!("{rounded:.2}");
    text.trim_end_matches('0').trim_end_matches('.').to_owned()
}

/// A CSS length in px. Zero gives `0`.
#[must_use]
pub fn px(value: f64) -> String {
    let n = num(value);
    if n == "0" { n } else { format!("{n}px") }
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
        let mut name = base.to_owned();
        let mut n = 2;
        while self.used.contains(&name) {
            name = format!("{base}{}{n}", self.separator);
            n += 1;
        }
        self.used.insert(name.clone());
        name
    }
}

#[cfg(test)]
mod tests;
