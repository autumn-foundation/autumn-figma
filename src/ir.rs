//! The intermediate representation between the Figma tree and the text.
//!
//! [`crate::lower`] builds it. [`crate::emit`] prints it. All names in it
//! are already safe Rust identifiers or CSS classes.

use crate::style::Decls;

/// One Rust module: one Figma page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Module {
    /// The module name.
    pub name: String,
    /// The Figma page name.
    pub page: String,
    /// Variant enums.
    pub enums: Vec<EnumDef>,
    /// Component and screen functions.
    pub functions: Vec<Function>,
}

/// A variant enum.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumDef {
    /// The enum name.
    pub name: String,
    /// The doc line.
    pub doc: String,
    /// Variant identifiers and their Figma values.
    pub variants: Vec<(String, String)>,
    /// The index of the default variant.
    pub default: usize,
}

/// A generated function.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Function {
    /// The function name.
    pub name: String,
    /// The doc line.
    pub doc: String,
    /// The parameters, in order.
    pub params: Vec<Param>,
    /// The body.
    pub body: Body,
}

/// A function parameter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Param {
    /// The name.
    pub name: String,
    /// The type.
    pub ty: ParamType,
}

/// The type of a parameter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParamType {
    /// `&str`.
    Str,
    /// `bool`.
    Bool,
    /// `Markup`.
    Markup,
    /// A variant enum of this module.
    Enum(String),
    /// `&super::Attrs<'_>`, the root attributes.
    Attrs,
}

/// A function body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Body {
    /// One template.
    Single(Vec<Item>),
    /// One template per variant.
    Match {
        /// The parameters to match on.
        on: Vec<String>,
        /// The arms.
        arms: Vec<Arm>,
        /// The template for combinations without an arm.
        fallback: Option<Vec<Item>>,
    },
}

/// One match arm.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Arm {
    /// One enum path per matched parameter, for example `ButtonSize::Large`.
    pub pattern: Vec<String>,
    /// The template.
    pub body: Vec<Item>,
}

/// One item of a template.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Item {
    /// An element.
    Element(Element),
    /// Literal text. A `\n` becomes `br`.
    Text(String),
    /// A parameter, rendered with escaping.
    Param(String),
    /// A call of a generated function.
    Call(Call),
    /// Items shown when a `bool` parameter is true.
    If {
        /// The parameter.
        cond: String,
        /// The items.
        body: Vec<Self>,
    },
}

/// An HTML element.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Element {
    /// The tag.
    pub tag: String,
    /// The CSS class.
    pub class: Option<String>,
    /// The `id` attribute.
    pub id: Option<String>,
    /// Other static attributes.
    pub attrs: Vec<(String, String)>,
    /// True on a component root: add the `attrs` parameter fields.
    pub root_attrs: bool,
    /// The children.
    pub children: Vec<Item>,
}

/// A call of a generated function.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Call {
    /// The function path, for example `super::components::button`.
    pub path: String,
    /// The arguments, in parameter order.
    pub args: Vec<Arg>,
}

/// A call argument.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Arg {
    /// A string literal.
    Str(String),
    /// A boolean literal.
    Bool(bool),
    /// A path, for example an enum variant.
    Path(String),
    /// A template.
    Markup(Vec<Item>),
    /// Root attributes: `Attrs` field names and values.
    Attrs(Vec<(String, String)>),
}

/// One CSS rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CssRule {
    /// The class, without the dot.
    pub class: String,
    /// The declarations.
    pub decls: Decls,
    /// True to double the class in the selector, so the rule wins over a
    /// component root rule (for the place of an instance).
    pub boost: bool,
}

/// The root attributes a component accepts: HTML name and `Attrs` field.
pub const ROOT_ATTRS: &[(&str, &str)] = &[
    ("id", "id"),
    ("name", "name"),
    ("value", "value"),
    ("type", "type_"),
    ("href", "href"),
    ("aria-label", "aria_label"),
    ("role", "role"),
    ("title", "title"),
    ("disabled", "disabled"),
    ("hx-get", "hx_get"),
    ("hx-post", "hx_post"),
    ("hx-put", "hx_put"),
    ("hx-patch", "hx_patch"),
    ("hx-delete", "hx_delete"),
    ("hx-target", "hx_target"),
    ("hx-swap", "hx_swap"),
    ("hx-trigger", "hx_trigger"),
    ("hx-vals", "hx_vals"),
    ("hx-include", "hx_include"),
    ("hx-confirm", "hx_confirm"),
    ("hx-push-url", "hx_push_url"),
    ("hx-select", "hx_select"),
    ("hx-indicator", "hx_indicator"),
    ("hx-boost", "hx_boost"),
    ("hx-swap-oob", "hx_swap_oob"),
    ("hx-headers", "hx_headers"),
    ("hx-ext", "hx_ext"),
    ("hx-sync", "hx_sync"),
    ("hx-disabled-elt", "hx_disabled_elt"),
];

/// The `Attrs` field for an HTML attribute name, if it has one.
#[must_use]
pub fn root_attr_field(html: &str) -> Option<&'static str> {
    ROOT_ATTRS
        .iter()
        .find(|(name, _)| *name == html)
        .map(|(_, field)| *field)
}

/// Tags without children or end tag.
pub const VOID_TAGS: &[&str] = &["br", "hr", "img", "input"];
