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

use std::fmt::Write as _;

use serde_json::Value;

use crate::diagnostics::Diagnostic;
use crate::model::{Color, Variable, VariableType, VariablesResponse, alias_id};
use crate::names::{Allocator, css_str, num, px, slug};

/// The CSS custom properties of Autumn's widgets (`autumn_web::ui::tokens`).
pub const AUTUMN_TOKENS: &[&str] = &[
    "--bg",
    "--surface",
    "--text",
    "--text-muted",
    "--border",
    "--primary",
    "--primary-hover",
    "--primary-light",
    "--danger",
    "--danger-hover",
    "--danger-light",
    "--success",
    "--success-light",
    "--warning",
    "--warning-light",
    "--radius",
    "--shadow",
    "--font-family",
];

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
    /// Mode slug by collection id and mode id.
    mode_slugs: BTreeMap<(String, String), String>,
    /// Names that the set made, not names from `[tokens]`.
    made: std::collections::BTreeSet<String>,
    /// Slugs of default modes in collections with more than one mode.
    default_slugs: std::collections::BTreeSet<String>,
}

impl TokenSet {
    /// Build the set from the variables snapshot, if any.
    #[must_use]
    pub fn new(
        variables: Option<&VariablesResponse>,
        renames: &BTreeMap<String, String>,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Self {
        let mut set = Self {
            names: Allocator::new("-"),
            renames: renames.clone(),
            ..Self::default()
        };
        for target in renames.values() {
            set.names.reserve(target);
        }
        if let Some(vars) = variables {
            set.add_variables(vars, diagnostics);
        }
        set
    }

    fn add_variables(&mut self, vars: &VariablesResponse, diagnostics: &mut Vec<Diagnostic>) {
        let collections = &vars.meta.variable_collections;
        for collection in collections.values() {
            for mode in &collection.modes {
                let mode_slug = slug(&mode.name);
                if mode.mode_id == collection.default_mode_id && collection.modes.len() > 1 {
                    self.default_slugs.insert(mode_slug.clone());
                }
                self.mode_slugs
                    .insert((collection.id.clone(), mode.mode_id.clone()), mode_slug);
            }
        }
        let mut sorted: Vec<&Variable> = vars
            .meta
            .variables
            .values()
            .filter(|v| {
                v.resolved_type != VariableType::Boolean && v.resolved_type != VariableType::Other
            })
            .collect();
        let collection_name = |v: &Variable| {
            collections
                .get(&v.variable_collection_id)
                .map_or("", |c| c.name.as_str())
        };
        sorted.sort_by(|a, b| {
            (collection_name(a), &a.name, &a.id).cmp(&(collection_name(b), &b.name, &b.id))
        });
        for var in &sorted {
            let name = self.renames.get(&var.name).cloned().unwrap_or_else(|| {
                let base = format!("--{}", slug(&var.name));
                let made = self.names.claim(&base);
                self.made.insert(made.clone());
                made
            });
            self.variables.insert(var.id.clone(), name);
        }
        for var in sorted {
            let name = self.variables[&var.id].clone();
            let collection = collections.get(&var.variable_collection_id);
            let default_mode = collection.map_or_else(
                || {
                    var.values_by_mode
                        .keys()
                        .next()
                        .cloned()
                        .unwrap_or_default()
                },
                |c| c.default_mode_id.clone(),
            );
            // The default mode goes first, so later modes can skip equal values.
            let mut modes: Vec<(String, String)> = collection.map_or_else(
                || vec![(default_mode.clone(), String::new())],
                |c| {
                    c.modes
                        .iter()
                        .map(|m| (m.mode_id.clone(), m.name.clone()))
                        .collect()
                },
            );
            modes.sort_by_key(|(id, _)| *id != default_mode);
            let mut default_value = None;
            for (mode_id, mode_name) in modes {
                let Some(raw) = var.values_by_mode.get(&mode_id) else {
                    continue;
                };
                let Some(value) = self.value(var, raw) else {
                    diagnostics.push(Diagnostic::warning(
                        format!("variables > {}", var.name),
                        format!(
                            "Mode {mode_name:?} has a value that CSS cannot use (for example an alias to {}). The generator skips the declaration.",
                            alias_id(raw).unwrap_or("an unknown variable")
                        ),
                    ));
                    continue;
                };
                if mode_id == default_mode {
                    default_value = Some(value.clone());
                    self.root.push((name.clone(), value));
                } else if default_value.as_ref() != Some(&value) {
                    self.modes
                        .entry(slug(&mode_name))
                        .or_default()
                        .push((name.clone(), value));
                }
            }
        }
    }

    /// The CSS value of one variable value, or `None` when it has none.
    fn value(&self, var: &Variable, raw: &Value) -> Option<String> {
        if let Some(target) = alias_id(raw) {
            return self.variables.get(target).map(|n| format!("var({n})"));
        }
        match var.resolved_type {
            VariableType::Color => {
                let color: Color = serde_json::from_value(raw.clone()).ok()?;
                Some(css_color(&color, 1.0))
            }
            VariableType::Float => {
                let n = raw.as_f64()?;
                let scoped = |s: &str| var.scopes.iter().any(|x| x == s);
                if scoped("OPACITY") {
                    Some(num(if n > 1.0 { n / 100.0 } else { n }))
                } else if scoped("FONT_WEIGHT") {
                    Some(num(n))
                } else {
                    Some(px(n))
                }
            }
            VariableType::String => raw.as_str().map(css_str),
            VariableType::Boolean | VariableType::Other => None,
        }
    }

    /// The `data-theme` value for a layer that sets variable modes
    /// (`explicitVariableModes`: collection id to mode id). `None` when no
    /// mode is known.
    #[must_use]
    pub fn mode_attr(&self, modes: &BTreeMap<String, String>) -> Option<String> {
        modes
            .iter()
            .find_map(|(collection, mode)| self.mode_slugs.get(&(collection.clone(), mode.clone())))
            .cloned()
    }

    /// `var(--name)` for a variable id, if the set has it.
    #[must_use]
    pub fn variable(&self, id: &str) -> Option<String> {
        self.variables.get(id).map(|n| format!("var({n})"))
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
        if let Some(name) = self.styles.get(style_id) {
            return Some(format!("var({name})"));
        }
        let value = value()?;
        let name = self.renames.get(style_name).cloned().unwrap_or_else(|| {
            let base = format!("--{}", slug(style_name));
            let made = self.names.claim(&base);
            self.made.insert(made.clone());
            made
        });
        self.root.push((name.clone(), value));
        self.styles.insert(style_id.to_owned(), name.clone());
        Some(format!("var({name})"))
    }

    /// Made names that are also Autumn tokens. Such a name changes the
    /// look of Autumn widgets.
    #[must_use]
    pub fn autumn_clashes(&self) -> Vec<&str> {
        self.made
            .iter()
            .filter(|n| AUTUMN_TOKENS.contains(&n.as_str()))
            .map(String::as_str)
            .collect()
    }

    /// True when the set has no declarations.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.root.is_empty() && self.modes.is_empty()
    }

    /// The CSS text: `:root`, then one block per mode.
    #[must_use]
    pub fn css(&self) -> String {
        if self.is_empty() {
            return String::new();
        }
        let mut out = String::new();
        // A layer can set the default mode inside another mode, so the
        // default values also go on `[data-theme="<default>"]`.
        let mut root = ":root".to_owned();
        for mode in &self.default_slugs {
            let _ = write!(root, ",\n[data-theme=\"{mode}\"]");
        }
        block(&mut out, &root, &self.root, "");
        for (mode, decls) in &self.modes {
            out.push('\n');
            block(&mut out, &format!("[data-theme=\"{mode}\"]"), decls, "");
        }
        for (mode, decls) in self.modes.iter().filter(|(m, _)| m.contains("dark")) {
            let _ = mode;
            out.push_str("\n@media (prefers-color-scheme: dark) {\n");
            block(&mut out, ":root:not([data-theme])", decls, "  ");
            out.push_str("}\n");
        }
        out
    }
}

/// Write one rule block with `indent` before each line.
fn block(out: &mut String, selector: &str, decls: &[(String, String)], indent: &str) {
    let _ = writeln!(out, "{indent}{selector} {{");
    for (name, value) in decls {
        let _ = writeln!(out, "{indent}  {name}: {value};");
    }
    let _ = writeln!(out, "{indent}}}");
}

/// A CSS color: `#rrggbb`, or `rgba(...)` when alpha × `opacity` < 1.
#[must_use]
pub fn css_color(color: &Color, opacity: f64) -> String {
    let channel = |v: f64| {
        // The clamp keeps the value in 0..=255, so the cast is exact.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let byte = (v.clamp(0.0, 1.0) * 255.0).round() as u8;
        byte
    };
    let (r, g, b) = (channel(color.r), channel(color.g), channel(color.b));
    let alpha = (color.a * opacity).clamp(0.0, 1.0);
    if alpha >= 0.995 {
        format!("#{r:02x}{g:02x}{b:02x}")
    } else {
        format!("rgba({r}, {g}, {b}, {})", num(alpha))
    }
}

#[cfg(test)]
mod tests;
