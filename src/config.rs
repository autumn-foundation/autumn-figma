//! `figma.toml`: where the design is, where the output goes, and the
//! bindings that add htmx behavior.
//!
//! ```toml
//! file_key = "AbC123xyz"           # or the full Figma URL
//! pages = ["Components", "Screens"] # default: all pages
//!
//! [tokens]
//! "color/brand/primary" = "--primary"
//!
//! [[bind]]
//! path = "Screens > Todos > New todo"
//! tag = "form"
//! attrs = { "hx-post" = "/todos", "hx-target" = "#todo-list" }
//! ```
//!
//! The token never goes in this file. It comes only from `FIGMA_TOKEN`.

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

use crate::error::{Error, ErrorKind, Result};

/// The environment variable that holds the Figma personal access token.
pub const TOKEN_ENV: &str = "FIGMA_TOKEN";

/// The environment variable that overrides the API base URL (for tests and
/// proxies).
pub const API_BASE_ENV: &str = "FIGMA_API_BASE";

/// The Figma REST API base URL.
pub const DEFAULT_API_BASE: &str = "https://api.figma.com";

/// The default config file name.
pub const CONFIG_FILE: &str = "figma.toml";

/// Tags that a binding or a layer name can select.
pub const TAGS: &[&str] = &[
    "a",
    "article",
    "aside",
    "blockquote",
    "button",
    "dd",
    "details",
    "dialog",
    "div",
    "dl",
    "dt",
    "em",
    "fieldset",
    "figcaption",
    "figure",
    "footer",
    "form",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "header",
    "hr",
    "img",
    "input",
    "label",
    "legend",
    "li",
    "main",
    "nav",
    "ol",
    "option",
    "p",
    "section",
    "select",
    "small",
    "span",
    "strong",
    "summary",
    "table",
    "tbody",
    "td",
    "textarea",
    "th",
    "thead",
    "tr",
    "ul",
];

/// The resolved `figma.toml`.
///
/// All paths are absolute: the directory of `figma.toml` joined with the
/// relative path in the file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    /// The Figma file key.
    pub file_key: String,
    /// The directory that holds `figma.toml`.
    pub root: PathBuf,
    /// The file snapshot. Default `figma/snapshot.json`.
    pub snapshot: PathBuf,
    /// The variables snapshot. Default `figma/variables.json`.
    pub variables_snapshot: PathBuf,
    /// The Rust module directory. Default `src/figma`.
    pub out_dir: PathBuf,
    /// The CSS file. Default `static/css/figma.css`.
    pub css_path: PathBuf,
    /// The asset directory. Default `static/figma`.
    pub assets_dir: PathBuf,
    /// The URL of the asset directory. Default `/static/figma`.
    pub assets_url: String,
    /// The pages to generate, by name. Empty means all pages.
    pub pages: Vec<String>,
    /// The CSS class prefix. Default `fg`.
    pub class_prefix: String,
    /// Token renames: Figma variable or style name to CSS custom property.
    pub tokens: BTreeMap<String, String>,
    /// The bindings, in file order.
    pub binds: Vec<Binding>,
}

/// Which node a binding selects.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Selector {
    /// Layer names from the page down, for example `["Screens", "Todos"]`.
    Path(Vec<String>),
    /// A node id, for example `2:9`.
    Node(String),
}

impl fmt::Display for Selector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Path(parts) => f.write_str(&parts.join(" > ")),
            Self::Node(id) => write!(f, "node {id}"),
        }
    }
}

/// One `[[bind]]` entry: behavior for one node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binding {
    /// The node.
    pub selector: Selector,
    /// The HTML tag.
    pub tag: Option<String>,
    /// The `id` attribute.
    pub id: Option<String>,
    /// Other attributes, for example `hx-post`.
    pub attrs: BTreeMap<String, String>,
    /// Replace the children with a `Markup` parameter of this name.
    pub slot: Option<String>,
    /// Replace the text with a `&str` parameter of this name.
    pub text: Option<String>,
}

impl Config {
    /// Read and check `figma.toml` at `path`.
    ///
    /// # Errors
    ///
    /// [`ErrorKind::Io`] when the file cannot be read, and
    /// [`ErrorKind::Config`] when it is not valid.
    pub fn load(path: &Path) -> Result<Self> {
        let _ = path;
        Err(Error::new(ErrorKind::Config, "not implemented"))
    }

    /// Parse and check `figma.toml` text. `root` is its directory.
    ///
    /// # Errors
    ///
    /// [`ErrorKind::Config`] when the text is not valid.
    pub fn from_toml_str(src: &str, root: &Path) -> Result<Self> {
        let _ = (src, root);
        Err(Error::new(ErrorKind::Config, "not implemented"))
    }
}

/// Get the file key from a key or a Figma URL.
///
/// Accepts `/design/<key>/`, `/file/<key>/`, and branch URLs
/// (`/design/<key>/branch/<branch key>/`, which give the branch key).
///
/// # Errors
///
/// [`ErrorKind::Config`] when no key is found or the key has characters
/// other than ASCII letters and digits.
pub fn parse_file_key(input: &str) -> Result<String> {
    let _ = input;
    Err(Error::new(ErrorKind::Config, "not implemented"))
}

/// A Figma access token. `Debug` does not show it.
#[derive(Clone, PartialEq, Eq)]
pub struct Token(String);

impl Token {
    /// Wrap a token.
    #[must_use]
    pub const fn new(token: String) -> Self {
        Self(token)
    }

    /// The token text. Send it only to the Figma API.
    #[must_use]
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Token(<redacted>)")
    }
}

/// Read the token through `lookup` (normally the process environment).
///
/// # Errors
///
/// [`ErrorKind::Auth`] when `FIGMA_TOKEN` is not set or is blank.
pub fn token_from(lookup: &dyn Fn(&str) -> Option<String>) -> Result<Token> {
    let _ = lookup;
    Err(Error::new(ErrorKind::Auth, "not implemented"))
}

/// Read the API base URL through `lookup`. Default [`DEFAULT_API_BASE`].
///
/// # Errors
///
/// [`ErrorKind::Config`] when the value is not an `http` or `https` URL.
pub fn api_base_from(lookup: &dyn Fn(&str) -> Option<String>) -> Result<String> {
    let _ = lookup;
    Err(Error::new(ErrorKind::Config, "not implemented"))
}

/// A starter `figma.toml` for `file_key`.
#[must_use]
pub fn starter_toml(file_key: &str) -> String {
    let _ = file_key;
    String::new()
}

#[cfg(test)]
mod tests;
