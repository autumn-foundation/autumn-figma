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

use serde::Deserialize;

use crate::error::{Error, ErrorKind, Result};
use crate::names::is_keyword;

/// The environment variable that holds the Figma personal access token.
pub const TOKEN_ENV: &str = "FIGMA_TOKEN";

/// The environment variable that holds a Figma OAuth access token. The
/// client reads it when `FIGMA_TOKEN` is not set.
pub const OAUTH_TOKEN_ENV: &str = "FIGMA_OAUTH_TOKEN";

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
        let src = std::fs::read_to_string(path).map_err(|e| Error::io(path, &e))?;
        let root = match path.parent() {
            Some(dir) if !dir.as_os_str().is_empty() => dir,
            _ => Path::new("."),
        };
        Self::from_toml_str(&src, root)
            .map_err(|e| Error::new(e.kind(), format!("{}: {}", path.display(), e.message())))
    }

    /// Parse and check `figma.toml` text. `root` is its directory.
    ///
    /// # Errors
    ///
    /// [`ErrorKind::Config`] when the text is not valid.
    pub fn from_toml_str(src: &str, root: &Path) -> Result<Self> {
        let table: toml::Table = toml::from_str(src).map_err(|e| config_err(e.message()))?;
        if let Some(key) = ["token", "figma_token", "access_token"]
            .into_iter()
            .find(|k| table.contains_key(*k))
        {
            return Err(config_err(&format!(
                "remove `{key}`: the token goes only in the {TOKEN_ENV} environment variable"
            )));
        }
        let raw: RawConfig = toml::from_str(src).map_err(|e| config_err(e.message()))?;
        let file_key = parse_file_key(raw.file_key.as_deref().unwrap_or_default())
            .map_err(|e| config_err(&format!("file_key: {}", e.message())))?;
        let path = |name: &str, value: Option<String>, default: &str| -> Result<PathBuf> {
            let value = value.unwrap_or_else(|| default.to_owned());
            check_relative(name, &value)?;
            Ok(root.join(value))
        };
        let class_prefix = raw.class_prefix.unwrap_or_else(|| "fg".to_owned());
        if !is_name(
            &class_prefix,
            |c| c.is_ascii_lowercase(),
            |c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-',
        ) {
            return Err(config_err(&format!(
                "class_prefix {class_prefix:?}: use lowercase letters, digits, and `-`, and start with a letter"
            )));
        }
        let assets_url = raw.assets_url.unwrap_or_else(|| "/static/figma".to_owned());
        check_assets_url(&assets_url)?;
        for (name, property) in &raw.tokens {
            if !is_custom_property(property) {
                return Err(config_err(&format!(
                    "token {name:?} maps to {property:?}: use a CSS custom property name such as --primary"
                )));
            }
        }
        let binds = raw
            .bind
            .into_iter()
            .map(RawBinding::check)
            .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            file_key,
            root: root.to_path_buf(),
            snapshot: path("snapshot", raw.snapshot, "figma/snapshot.json")?,
            variables_snapshot: path(
                "variables_snapshot",
                raw.variables_snapshot,
                "figma/variables.json",
            )?,
            out_dir: path("out_dir", raw.out_dir, "src/figma")?,
            css_path: path("css_path", raw.css_path, "static/css/figma.css")?,
            assets_dir: path("assets_dir", raw.assets_dir, "static/figma")?,
            assets_url: assets_url.trim_end_matches('/').to_owned(),
            pages: raw.pages,
            class_prefix,
            tokens: raw.tokens,
            binds,
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawConfig {
    file_key: Option<String>,
    snapshot: Option<String>,
    variables_snapshot: Option<String>,
    out_dir: Option<String>,
    css_path: Option<String>,
    assets_dir: Option<String>,
    assets_url: Option<String>,
    #[serde(default)]
    pages: Vec<String>,
    class_prefix: Option<String>,
    #[serde(default)]
    tokens: BTreeMap<String, String>,
    #[serde(default)]
    bind: Vec<RawBinding>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawBinding {
    path: Option<String>,
    node: Option<String>,
    tag: Option<String>,
    id: Option<String>,
    #[serde(default)]
    attrs: BTreeMap<String, String>,
    slot: Option<String>,
    text: Option<String>,
}

impl RawBinding {
    fn check(self) -> Result<Binding> {
        let selector = match (self.path, self.node) {
            (Some(path), None) => {
                let parts: Vec<String> = path.split('>').map(|p| p.trim().to_owned()).collect();
                if parts.iter().any(String::is_empty) {
                    return Err(config_err(&format!(
                        "[[bind]] path {path:?} has an empty part; write `Page > Frame > Layer`"
                    )));
                }
                Selector::Path(parts)
            }
            (None, Some(node)) => {
                if node.is_empty() || node.chars().any(char::is_whitespace) {
                    return Err(config_err(&format!(
                        "[[bind]] node {node:?} is not a node id"
                    )));
                }
                Selector::Node(node)
            }
            (None, None) => return Err(config_err("a [[bind]] needs `path` or `node`")),
            (Some(_), Some(_)) => {
                return Err(config_err("a [[bind]] takes only one of `path` and `node`"));
            }
        };
        let at = selector.to_string();
        if self.tag.is_none()
            && self.id.is_none()
            && self.attrs.is_empty()
            && self.slot.is_none()
            && self.text.is_none()
        {
            return Err(config_err(&format!(
                "[[bind]] {at} does nothing; set tag, id, attrs, slot, or text"
            )));
        }
        if let Some(tag) = &self.tag
            && !TAGS.contains(&tag.as_str())
        {
            return Err(config_err(&format!(
                "[[bind]] {at}: tag {tag:?} is not allowed; use one of {}",
                TAGS.join(", ")
            )));
        }
        if let Some(id) = &self.id
            && !is_name(
                id,
                |c| c.is_ascii_alphabetic(),
                |c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | ':' | '.'),
            )
        {
            return Err(config_err(&format!(
                "[[bind]] {at}: id {id:?} is not a valid HTML id"
            )));
        }
        for name in self.attrs.keys() {
            check_attr_name(&at, name)?;
        }
        if self.slot.is_some() && self.text.is_some() {
            return Err(config_err(&format!(
                "[[bind]] {at}: set slot or text, not both"
            )));
        }
        for (key, param) in [("slot", &self.slot), ("text", &self.text)] {
            if let Some(param) = param
                && !is_param(param)
            {
                return Err(config_err(&format!(
                    "[[bind]] {at}: {key} {param:?} must be a snake_case Rust name that is not a keyword or `attrs`"
                )));
            }
        }
        Ok(Binding {
            selector,
            tag: self.tag,
            id: self.id,
            attrs: self.attrs,
            slot: self.slot,
            text: self.text,
        })
    }
}

fn config_err(message: &str) -> Error {
    Error::new(ErrorKind::Config, message.trim())
}

/// True when `s` is not empty, its first char passes `first`, and all
/// chars pass `rest`.
fn is_name(s: &str, first: impl Fn(char) -> bool, rest: impl Fn(char) -> bool) -> bool {
    s.chars().next().is_some_and(&first) && s.chars().all(|c| first(c) || rest(c))
}

fn is_param(name: &str) -> bool {
    is_name(
        name,
        |c| c.is_ascii_lowercase() || c == '_',
        |c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_',
    ) && !is_keyword(name)
        && name != "attrs"
}

fn is_custom_property(name: &str) -> bool {
    name.len() > 2
        && name.starts_with("--")
        && name[2..]
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

fn check_attr_name(at: &str, name: &str) -> Result<()> {
    let valid = is_name(
        name,
        |c| c.is_ascii_lowercase() || matches!(c, '_' | ':' | '@'),
        |c| {
            c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '-' | '_' | ':' | '.' | '@')
        },
    );
    if !valid {
        return Err(config_err(&format!(
            "[[bind]] {at}: attribute name {name:?} is not valid; use lowercase letters, digits, and - _ : . @"
        )));
    }
    match name {
        "style" => Err(config_err(&format!(
            "[[bind]] {at}: attribute \"style\" is not allowed; inline styles break a strict Content-Security-Policy"
        ))),
        "class" => Err(config_err(&format!(
            "[[bind]] {at}: attribute \"class\" is not allowed; the generator owns classes"
        ))),
        "id" => Err(config_err(&format!(
            "[[bind]] {at}: attribute \"id\" is not allowed in attrs; use the `id` key"
        ))),
        _ => Ok(()),
    }
}

fn check_relative(name: &str, value: &str) -> Result<()> {
    let path = Path::new(value);
    let ok = !value.is_empty()
        && path.components().all(|c| {
            matches!(
                c,
                std::path::Component::Normal(_) | std::path::Component::CurDir
            )
        });
    if ok {
        Ok(())
    } else {
        Err(config_err(&format!(
            "{name} {value:?} must be a relative path inside the app, without `..`"
        )))
    }
}

fn check_assets_url(url: &str) -> Result<()> {
    let prefix = url.starts_with('/') || url.starts_with("https://") || url.starts_with("http://");
    let bad = url
        .chars()
        .any(|c| c.is_whitespace() || c.is_control() || "\"'()<>\\`".contains(c));
    if prefix && !bad {
        Ok(())
    } else {
        Err(config_err(&format!(
            "assets_url {url:?} must start with `/` or `http(s)://` and have no spaces, quotes, or brackets"
        )))
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
    let input = input.trim();
    let key = if input.contains("figma.com/") {
        let path = input.split("figma.com/").nth(1).unwrap_or_default();
        let path = path.split(['?', '#']).next().unwrap_or_default();
        let parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();
        match parts.as_slice() {
            [kind, _, "branch", branch, ..] if is_file_kind(kind) => *branch,
            [kind, key, ..] if is_file_kind(kind) => *key,
            _ => "",
        }
    } else {
        input
    };
    if !key.is_empty() && key.chars().all(|c| c.is_ascii_alphanumeric()) {
        Ok(key.to_owned())
    } else if input.is_empty() {
        Err(Error::new(
            ErrorKind::Config,
            "the file key is missing; copy it from the Figma URL (figma.com/design/<key>/...)",
        ))
    } else {
        Err(Error::new(
            ErrorKind::Config,
            format!("{input:?} is not a Figma file key or file URL"),
        ))
    }
}

fn is_file_kind(segment: &str) -> bool {
    matches!(segment, "design" | "file" | "proto" | "board" | "make")
}

/// A Figma access token. `Debug` does not show it.
#[derive(Clone, PartialEq, Eq)]
pub struct Token {
    text: String,
    oauth: bool,
}

impl Token {
    /// Wrap a personal access token. It goes in `X-Figma-Token`.
    #[must_use]
    pub const fn new(token: String) -> Self {
        Self {
            text: token,
            oauth: false,
        }
    }

    /// Wrap an OAuth access token. It goes in `Authorization: Bearer`.
    #[must_use]
    pub const fn oauth(token: String) -> Self {
        Self {
            text: token,
            oauth: true,
        }
    }

    /// True for an OAuth token.
    #[must_use]
    pub const fn is_oauth(&self) -> bool {
        self.oauth
    }

    /// The token text. Send it only to the Figma API.
    #[must_use]
    pub fn expose(&self) -> &str {
        &self.text
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
/// `FIGMA_TOKEN` (a personal access token) wins over `FIGMA_OAUTH_TOKEN`.
///
/// # Errors
///
/// [`ErrorKind::Auth`] when neither variable has a token.
pub fn token_from(lookup: &dyn Fn(&str) -> Option<String>) -> Result<Token> {
    let read = |name: &str| {
        lookup(name)
            .map(|t| t.trim().to_owned())
            .filter(|t| !t.is_empty())
    };
    if let Some(token) = read(TOKEN_ENV) {
        return Ok(Token::new(token));
    }
    if let Some(token) = read(OAUTH_TOKEN_ENV) {
        return Ok(Token::oauth(token));
    }
    Err(Error::new(
        ErrorKind::Auth,
        format!(
            "{TOKEN_ENV} is not set. Make a personal access token in Figma (Settings > Security) with the file_content:read scope. Then export {TOKEN_ENV}."
        ),
    ))
}

/// Read the API base URL through `lookup`. Default [`DEFAULT_API_BASE`].
///
/// # Errors
///
/// [`ErrorKind::Config`] when the value is not a URL that
/// [`check_base_url`] accepts.
pub fn api_base_from(lookup: &dyn Fn(&str) -> Option<String>) -> Result<String> {
    let base = lookup(API_BASE_ENV).unwrap_or_else(|| DEFAULT_API_BASE.to_owned());
    check_base_url(&base).map_err(|e| {
        Error::new(
            ErrorKind::Config,
            format!("{API_BASE_ENV}: {}", e.message()),
        )
    })
}

/// Check a URL that receives the token. It must be `https`, or `http` to a
/// loopback host (for tests and local proxies). Returns it without a final
/// `/`.
///
/// # Errors
///
/// [`ErrorKind::Config`] for any other URL.
pub fn check_base_url(url: &str) -> Result<String> {
    let url = url.trim().trim_end_matches('/');
    if url.starts_with("https://") || is_loopback_http(url) {
        Ok(url.to_owned())
    } else {
        Err(Error::new(
            ErrorKind::Config,
            format!("{url:?} must be an https URL (plain http only for localhost)"),
        ))
    }
}

/// True for `http://localhost`, `http://127.0.0.1`, or `http://[::1]`,
/// with an optional port and path.
#[must_use]
pub fn is_loopback_http(url: &str) -> bool {
    let Some(rest) = url.strip_prefix("http://") else {
        return false;
    };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    if authority.contains('@') {
        return false;
    }
    let host = authority.strip_prefix('[').map_or_else(
        || authority.split(':').next().unwrap_or_default().to_owned(),
        |v6| format!("[{}]", v6.split(']').next().unwrap_or_default()),
    );
    matches!(host.as_str(), "localhost" | "127.0.0.1" | "[::1]")
}

/// A starter `figma.toml` for `file_key`.
#[must_use]
pub fn starter_toml(file_key: &str) -> String {
    format!(
        r##"# autumn-figma: Figma design to Maud components.
# The token never goes here. Export FIGMA_TOKEN before `autumn-figma pull`.

# The file key, or the full Figma URL of the file.
file_key = "{file_key}"

# The pages to generate. Remove the line to generate all pages.
# pages = ["Components", "Screens"]

# Output paths, relative to this file. These are the defaults.
# out_dir = "src/figma"
# css_path = "static/css/figma.css"
# assets_dir = "static/figma"
# assets_url = "/static/figma"
# class_prefix = "fg"

# Rename Figma variables or styles to CSS custom properties.
# Map the brand color to Autumn's token, so Autumn widgets match too.
[tokens]
# "color/brand/primary" = "--primary"

# Add behavior to a layer. Path is `Page > Frame > Layer`, or use `node = "1:2"`.
# [[bind]]
# path = "Screens > Todos > New todo"
# tag = "form"
# attrs = {{ "hx-post" = "/todos", "hx-target" = "#todo-list", "hx-swap" = "beforeend" }}
#
# [[bind]]
# path = "Screens > Todos > List"
# id = "todo-list"
# slot = "items"   # replace the children with a `Markup` parameter
"##
    )
}

#[cfg(test)]
mod tests;
