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

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use crate::assets::{self, Asset};
use crate::config::{Binding, Config, Selector};
use crate::diagnostics::Diagnostic;
use crate::error::{Error, ErrorKind, Result};
use crate::ir::{
    Arg, Arm, Body, Call, CssRule, Element, EnumDef, Function, Item, Module, Param, ParamType,
    ROOT_ATTRS, root_attr_field,
};
use crate::model::{FileResponse, LayoutMode, Node, NodeType, PropertyType, VariablesResponse};
use crate::names::{Allocator, camel_ident, slug, snake_ident};
use crate::style::{self, Parent, Role, StyleCx};
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
    let mut diagnostics = Vec::new();
    let tokens = TokenSet::new(variables, &config.tokens, &mut diagnostics);
    let pages = select_pages(file, config)?;
    let plan = Plan::build(&pages, config, &mut diagnostics);
    let mut lw = Lowerer {
        config,
        file,
        sigs: plan.sigs,
        by_component: plan.by_component,
        tokens,
        rules: Vec::new(),
        classes: Allocator::new("-"),
        assets: BTreeSet::new(),
        diagnostics,
        hits: vec![Vec::new(); config.binds.len()],
    };
    let mut modules = Vec::new();
    for planned in plan.modules {
        let mut functions = Vec::new();
        let mut enums = Vec::new();
        for def in &planned.defs {
            functions.push(lw.function(def)?);
            enums.extend(lw.sigs[def.sig].enums.iter().map(|e| EnumDef {
                name: e.name.clone(),
                doc: format!("Variant property \"{}\" of \"{}\".", e.key, def.node.name),
                variants: e.variants.clone(),
                default: e.default,
            }));
        }
        modules.push(Module {
            name: planned.name,
            page: planned.page,
            enums,
            functions,
        });
    }
    lw.check_bindings()?;
    for name in lw.tokens.autumn_clashes() {
        lw.diagnostics.push(Diagnostic::warning(
            "tokens",
            format!(
                "The token {name} has the name of an Autumn token. Autumn widgets take its value. To keep the Autumn value, rename the token in [tokens]."
            ),
        ));
    }
    Ok(Lowered {
        modules,
        rules: lw.rules,
        tokens: lw.tokens,
        assets: lw.assets.into_iter().collect(),
        diagnostics: lw.diagnostics,
    })
}

fn binding_err(message: String) -> Error {
    Error::new(ErrorKind::Binding, message)
}

/// The selected pages, in document order.
fn select_pages<'a>(file: &'a FileResponse, config: &Config) -> Result<Vec<&'a Node>> {
    let all: Vec<&Node> = file
        .document
        .children
        .iter()
        .filter(|n| n.kind == NodeType::Canvas)
        .collect();
    if config.pages.is_empty() {
        return Ok(all);
    }
    for wanted in &config.pages {
        if !all.iter().any(|p| p.name.trim() == wanted.trim()) {
            let names: Vec<&str> = all.iter().map(|p| p.name.as_str()).collect();
            return Err(Error::new(
                ErrorKind::Config,
                format!(
                    "page {wanted:?} is not in the file; its pages are: {}",
                    names.join(", ")
                ),
            ));
        }
    }
    Ok(all
        .into_iter()
        .filter(|p| config.pages.iter().any(|w| w.trim() == p.name.trim()))
        .collect())
}

/// True when `selector` selects `node` at `path`.
fn selects(selector: &Selector, node: &Node, path: &[String]) -> bool {
    match selector {
        Selector::Node(id) => node.id == *id,
        Selector::Path(parts) => {
            parts.len() == path.len() && parts.iter().zip(path).all(|(a, b)| a == b.trim())
        }
    }
}

fn child_path(path: &[String], child: &Node) -> Vec<String> {
    let mut p = path.to_vec();
    p.push(child.name.clone());
    p
}

/// What a function is made from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DefKind {
    Screen,
    Component,
    Set,
}

#[derive(Debug)]
struct Def<'a> {
    node: &'a Node,
    kind: DefKind,
    path: Vec<String>,
    sig: usize,
}

#[derive(Debug, Clone)]
struct EnumInfo {
    key: String,
    name: String,
    variants: Vec<(String, String)>,
    default: usize,
}

/// Where a parameter gets its value.
#[derive(Debug, Clone)]
enum Source {
    /// A variant property: the index in `Sig::enums`.
    Variant(usize),
    /// A component property by key.
    Prop(String, PropertyType),
    /// A text binding: the binding index, the definition node ids, and the
    /// default text.
    BindText(usize, Vec<String>, String),
    /// A slot binding: the binding index and the definition node ids.
    BindSlot(usize, Vec<String>),
    /// The root attributes.
    Attrs,
}

/// The signature of a generated function.
#[derive(Debug, Clone)]
struct Sig {
    module: String,
    name: String,
    params: Vec<(String, Source)>,
    enums: Vec<EnumInfo>,
    defaults: BTreeMap<String, Value>,
}

impl Sig {
    fn path(&self) -> String {
        format!("super::{}::{}", self.module, self.name)
    }
}

struct PlannedModule<'a> {
    name: String,
    page: String,
    defs: Vec<Def<'a>>,
}

/// Pass one: find every function, its name, and its signature.
struct Plan<'a> {
    modules: Vec<PlannedModule<'a>>,
    sigs: Vec<Sig>,
    /// Component (or set) node id to signature index and variant values.
    by_component: BTreeMap<String, (usize, BTreeMap<String, String>)>,
}

/// Parse `Size=Large, State=Hover`.
fn variant_values(name: &str) -> BTreeMap<String, String> {
    name.split(',')
        .filter_map(|part| part.split_once('='))
        .map(|(k, v)| (k.trim().to_owned(), v.trim().to_owned()))
        .collect()
}

fn sorted_ci<'k>(keys: impl Iterator<Item = &'k String>) -> Vec<&'k String> {
    let mut keys: Vec<&String> = keys.collect();
    keys.sort_by(|a, b| (a.to_lowercase(), *a).cmp(&(b.to_lowercase(), *b)));
    keys
}

fn prop_name(key: &str) -> &str {
    key.split('#').next().unwrap_or(key)
}

impl<'a> Plan<'a> {
    fn build(pages: &[&'a Node], config: &Config, diagnostics: &mut Vec<Diagnostic>) -> Self {
        let mut plan = Self {
            modules: Vec::new(),
            sigs: Vec::new(),
            by_component: BTreeMap::new(),
        };
        let mut module_names = Allocator::new("_");
        for reserved in RESERVED_MODULES {
            module_names.reserve(reserved);
        }
        let mut found: Vec<(String, String, Vec<Found<'a>>)> = Vec::new();
        for page in pages {
            let mut defs = Vec::new();
            let path = vec![page.name.clone()];
            for child in &page.children {
                top_level(child, &path, &mut defs, diagnostics);
            }
            if defs.is_empty() {
                continue;
            }
            let name = module_names.claim(&snake_ident(&page.name));
            found.push((name, page.name.clone(), defs));
        }
        // Register every component first, so a signature can find the
        // local components that it must not descend into.
        let mut locals: BTreeSet<String> = BTreeSet::new();
        for (_, _, defs) in &found {
            for (node, kind, _) in defs {
                if *kind != DefKind::Screen {
                    locals.insert(node.id.clone());
                    for v in &node.children {
                        locals.insert(v.id.clone());
                    }
                }
            }
        }
        for (module, page, defs) in found {
            let mut fn_names = Allocator::new("_");
            let mut enum_names = Allocator::new("");
            enum_names.reserve("Markup");
            enum_names.reserve("Attrs");
            let mut planned = PlannedModule {
                name: module.clone(),
                page,
                defs: Vec::new(),
            };
            for (node, kind, path) in defs {
                let name = fn_names.claim(&snake_ident(&node.name));
                let found = (node, kind, path);
                let sig = signature(
                    &found,
                    &module,
                    name,
                    config,
                    &locals,
                    &mut enum_names,
                    diagnostics,
                );
                let (node, kind, path) = found;
                let index = plan.sigs.len();
                match kind {
                    DefKind::Set => {
                        plan.by_component
                            .insert(node.id.clone(), (index, BTreeMap::new()));
                        for variant in &node.children {
                            plan.by_component
                                .insert(variant.id.clone(), (index, variant_values(&variant.name)));
                        }
                    }
                    DefKind::Component => {
                        plan.by_component
                            .insert(node.id.clone(), (index, BTreeMap::new()));
                    }
                    DefKind::Screen => {}
                }
                plan.sigs.push(sig);
                planned.defs.push(Def {
                    node,
                    kind,
                    path,
                    sig: index,
                });
            }
            plan.modules.push(planned);
        }
        plan
    }
}

/// A function found on a page: its node, kind, and layer path.
type Found<'a> = (&'a Node, DefKind, Vec<String>);

/// Module names that clash with Rust, Cargo, rustfmt, or Windows file names.
const RESERVED_MODULES: &[&str] = &[
    "main", "lib", "mod", "build", "rustfmt", "clippy", "con", "prn", "aux", "nul", "com1", "com2",
    "com3", "com4", "com5", "com6", "com7", "com8", "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5",
    "lpt6", "lpt7", "lpt8", "lpt9",
];

/// Collect the functions under one child of a page.
fn top_level<'a>(
    node: &'a Node,
    parent: &[String],
    defs: &mut Vec<Found<'a>>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let path = child_path(parent, node);
    match node.kind {
        NodeType::ComponentSet => defs.push((node, DefKind::Set, path)),
        NodeType::Component => defs.push((node, DefKind::Component, path)),
        NodeType::Frame => {
            defs.push((node, DefKind::Screen, path.clone()));
            nested_components(node, &path, defs);
        }
        NodeType::Section | NodeType::Group => {
            for child in &node.children {
                top_level(child, &path, defs, diagnostics);
            }
        }
        _ => diagnostics.push(Diagnostic::info(
            path.join(" > "),
            "The generator skips this top-level layer. Only frames, sections, groups, and components become functions.",
        )),
    }
}

/// Components that sit inside a screen frame.
fn nested_components<'a>(node: &'a Node, path: &[String], defs: &mut Vec<Found<'a>>) {
    for child in &node.children {
        let p = child_path(path, child);
        match child.kind {
            NodeType::ComponentSet => defs.push((child, DefKind::Set, p)),
            NodeType::Component => defs.push((child, DefKind::Component, p)),
            NodeType::Instance => {}
            _ => nested_components(child, &p, defs),
        }
    }
}

fn signature(
    found: &Found<'_>,
    module: &str,
    name: String,
    config: &Config,
    locals: &BTreeSet<String>,
    enum_names: &mut Allocator,
    diagnostics: &mut Vec<Diagnostic>,
) -> Sig {
    let (node, kind, path) = (found.0, found.1, found.2.as_slice());
    let mut params = Vec::new();
    let mut param_names = Allocator::new("_");
    param_names.reserve("attrs");
    let mut enums = Vec::new();
    let defs = &node.component_property_definitions;
    let defaults: BTreeMap<String, Value> = defs
        .iter()
        .map(|(k, d)| (k.clone(), d.default_value.clone()))
        .collect();
    if kind == DefKind::Set {
        let variant_keys = sorted_ci(
            defs.keys()
                .filter(|k| defs[*k].kind == PropertyType::Variant),
        );
        for key in variant_keys {
            let at = path.join(" > ");
            if let Some(e) = variant_enum(node, key, &name, &at, enum_names, diagnostics) {
                params.push((
                    param_names.claim(&snake_ident(key)),
                    Source::Variant(enums.len()),
                ));
                enums.push(e);
            }
        }
    }
    if kind != DefKind::Screen {
        for key in sorted_ci(defs.keys()) {
            let prop = defs[key].kind;
            if matches!(
                prop,
                PropertyType::Text
                    | PropertyType::Boolean
                    | PropertyType::InstanceSwap
                    | PropertyType::Slot
            ) {
                params.push((
                    param_names.claim(&snake_ident(prop_name(key))),
                    Source::Prop(key.clone(), prop),
                ));
            }
        }
    }
    let mut found: Vec<(usize, Vec<String>, String)> = Vec::new();
    let roots: Vec<(&Node, Vec<String>)> = if kind == DefKind::Set {
        node.children
            .iter()
            .map(|v| (v, child_path(path, v)))
            .collect()
    } else {
        vec![(node, path.to_vec())]
    };
    for (root, root_path) in roots {
        bound_params(root, &root_path, config, locals, &mut found, true);
    }
    for (index, def_ids, default) in found {
        let bind = &config.binds[index];
        if let Some(text) = &bind.text {
            params.push((
                param_names.claim(text),
                Source::BindText(index, def_ids, default),
            ));
        } else if let Some(slot) = &bind.slot {
            params.push((param_names.claim(slot), Source::BindSlot(index, def_ids)));
        }
    }
    if kind != DefKind::Screen {
        params.push(("attrs".to_owned(), Source::Attrs));
    }
    Sig {
        module: module.to_owned(),
        name,
        params,
        enums,
        defaults,
    }
}

/// The enum for the variant property `key` of the set `node`. `None`
/// (with a warning) when the property has no options.
fn variant_enum(
    node: &Node,
    key: &str,
    fn_name: &str,
    at: &str,
    enum_names: &mut Allocator,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<EnumInfo> {
    let def = node.component_property_definitions.get(key)?;
    let mut options = def.variant_options.clone();
    for child in &node.children {
        if let Some(v) = variant_values(&child.name).get(key)
            && !options.contains(v)
        {
            options.push(v.clone());
        }
    }
    if options.is_empty() {
        diagnostics.push(Diagnostic::warning(
            at,
            format!("The variant property {key:?} has no options. The generator skips it."),
        ));
        return None;
    }
    let default = def
        .default_value
        .as_str()
        .and_then(|d| options.iter().position(|o| o == d))
        .unwrap_or(0);
    let mut idents = Allocator::new("");
    let variants = options
        .iter()
        .map(|o| (idents.claim(&camel_ident(o)), o.clone()))
        .collect();
    Some(EnumInfo {
        key: key.to_owned(),
        name: enum_names.claim(&format!("{}{}", camel_ident(fn_name), camel_ident(key))),
        variants,
        default,
    })
}

/// Find `slot` and `text` bindings in a definition subtree.
fn bound_params(
    node: &Node,
    path: &[String],
    config: &Config,
    locals: &BTreeSet<String>,
    found: &mut Vec<(usize, Vec<String>, String)>,
    root: bool,
) {
    for (index, bind) in config.binds.iter().enumerate() {
        if (bind.slot.is_some() || bind.text.is_some()) && selects(&bind.selector, node, path) {
            let default = node.characters.clone().unwrap_or_default();
            if let Some(entry) = found.iter_mut().find(|(i, _, _)| *i == index) {
                entry.1.push(node.id.clone());
            } else {
                found.push((index, vec![node.id.clone()], default));
            }
        }
    }
    let local_instance = node.kind == NodeType::Instance
        && node
            .component_id
            .as_ref()
            .is_some_and(|c| locals.contains(c));
    let nested_def = !root && matches!(node.kind, NodeType::Component | NodeType::ComponentSet);
    if local_instance || nested_def {
        return;
    }
    for child in &node.children {
        bound_params(
            child,
            &child_path(path, child),
            config,
            locals,
            found,
            false,
        );
    }
}

/// The names in scope while one function body is lowered.
struct Scope {
    /// The class base, for example `button--size-large`.
    slug: String,
    /// Component properties: key to parameter name and type.
    props: BTreeMap<String, (String, PropertyType)>,
    /// Binding index to parameter name.
    binds: BTreeMap<usize, String>,
    /// True for components: the root takes `attrs`.
    root_attrs: bool,
    /// The name that picks the root tag (the set name for a variant).
    root_name: String,
}

/// Pass two: lower function bodies.
struct Lowerer<'a> {
    config: &'a Config,
    file: &'a FileResponse,
    sigs: Vec<Sig>,
    by_component: BTreeMap<String, (usize, BTreeMap<String, String>)>,
    tokens: TokenSet,
    rules: Vec<CssRule>,
    classes: Allocator,
    assets: BTreeSet<Asset>,
    diagnostics: Vec<Diagnostic>,
    /// Paths that each binding matched.
    hits: Vec<Vec<String>>,
}

/// Tags that an exact container layer name selects.
const BOX_NAME_TAGS: &[&str] = &[
    "a", "article", "aside", "button", "fieldset", "figure", "footer", "form", "header", "input",
    "li", "main", "nav", "ol", "section", "textarea", "ul",
];

/// Text properties that an `<input>` takes from the text layer it replaces.
const TYPE_PROPERTIES: &[&str] = &[
    "font",
    "font-family",
    "font-size",
    "font-style",
    "font-weight",
    "letter-spacing",
    "line-height",
    "text-align",
    "text-transform",
];

/// Tags that an exact text layer name selects.
const TEXT_NAME_TAGS: &[&str] = &[
    "em", "h1", "h2", "h3", "h4", "h5", "h6", "p", "small", "strong",
];

fn name_tag(name: &str, text: bool) -> Option<String> {
    let name = name.trim().to_ascii_lowercase();
    let tags = if text { TEXT_NAME_TAGS } else { BOX_NAME_TAGS };
    tags.contains(&name.as_str()).then_some(name)
}

fn child_parent(node: &Node, child: &Node) -> Parent {
    let origin = node.absolute_bounding_box.unwrap_or_default();
    if child.layout_positioning.as_deref() == Some("ABSOLUTE") {
        return Parent::Absolute { origin };
    }
    match node.layout_mode {
        Some(LayoutMode::Horizontal) => Parent::Flex { row: true },
        Some(LayoutMode::Vertical) => Parent::Flex { row: false },
        Some(LayoutMode::Grid) => Parent::Grid,
        _ => Parent::Absolute { origin },
    }
}

/// The first descendant of `node` (not `node`) that passes `pred`.
fn find<'n>(node: &'n Node, pred: &dyn Fn(&Node) -> bool) -> Option<&'n Node> {
    node.children
        .iter()
        .find_map(|c| if pred(c) { Some(c) } else { find(c, pred) })
}

impl Lowerer<'_> {
    fn function(&mut self, def: &Def<'_>) -> Result<Function> {
        let sig = self.sigs[def.sig].clone();
        let mut scope = Scope {
            slug: slug(&def.node.name),
            props: BTreeMap::new(),
            binds: BTreeMap::new(),
            root_attrs: def.kind != DefKind::Screen,
            root_name: def.node.name.clone(),
        };
        let mut params = Vec::new();
        for (name, source) in &sig.params {
            let ty = match source {
                Source::Variant(i) => ParamType::Enum(sig.enums[*i].name.clone()),
                Source::Prop(key, kind) => {
                    scope.props.insert(key.clone(), (name.clone(), *kind));
                    match kind {
                        PropertyType::Text => ParamType::Str,
                        PropertyType::Boolean => ParamType::Bool,
                        _ => ParamType::Markup,
                    }
                }
                Source::BindText(index, _, _) => {
                    scope.binds.insert(*index, name.clone());
                    ParamType::Str
                }
                Source::BindSlot(index, _) => {
                    scope.binds.insert(*index, name.clone());
                    ParamType::Markup
                }
                Source::Attrs => ParamType::Attrs,
            };
            params.push(Param {
                name: name.clone(),
                ty,
            });
        }
        let (body, doc) = match def.kind {
            DefKind::Screen | DefKind::Component => {
                let parent = Parent::Root {
                    screen: def.kind == DefKind::Screen,
                };
                let site = Site {
                    path: &def.path,
                    parent,
                    root: true,
                };
                let item = self.node(def.node, site, &scope)?;
                let what = if def.kind == DefKind::Screen {
                    "frame"
                } else {
                    "component"
                };
                (
                    Body::Single(item.into_iter().collect()),
                    format!("Figma {what} \"{}\" (node {}).", def.node.name, def.node.id),
                )
            }
            DefKind::Set => (
                self.variants(def, &sig, &mut scope)?,
                format!(
                    "Figma component set \"{}\" (node {}).",
                    def.node.name, def.node.id
                ),
            ),
        };
        Ok(Function {
            name: sig.name,
            doc,
            params,
            body,
        })
    }

    fn variants(&mut self, def: &Def<'_>, sig: &Sig, scope: &mut Scope) -> Result<Body> {
        let on: Vec<String> = sig
            .params
            .iter()
            .filter(|(_, s)| matches!(s, Source::Variant(_)))
            .map(|(n, _)| n.clone())
            .collect();
        let set_slug = slug(&def.node.name);
        let mut arms: Vec<Arm> = Vec::new();
        let mut seen: BTreeSet<Vec<String>> = BTreeSet::new();
        let mut default_arm = None;
        for variant in &def.node.children {
            if variant.kind != NodeType::Component {
                continue;
            }
            let values = variant_values(&variant.name);
            let mut pattern = Vec::new();
            let mut is_default = true;
            for e in &sig.enums {
                let index = values
                    .get(&e.key)
                    .and_then(|v| e.variants.iter().position(|(_, o)| o == v))
                    .unwrap_or(e.default);
                is_default &= index == e.default;
                pattern.push(format!("{}::{}", e.name, e.variants[index].0));
            }
            let path = child_path(&def.path, variant);
            if !seen.insert(pattern.clone()) {
                self.diagnostics.push(Diagnostic::warning(
                    path.join(" > "),
                    "This variant is a duplicate of an earlier variant. The generator skips it.",
                ));
                continue;
            }
            scope.slug = format!("{set_slug}--{}", slug(&variant.name));
            let site = Site {
                path: &path,
                parent: Parent::Root { screen: false },
                root: true,
            };
            let body: Vec<Item> = self.node(variant, site, scope)?.into_iter().collect();
            if is_default {
                default_arm = Some(arms.len());
            }
            arms.push(Arm { pattern, body });
        }
        if sig.enums.is_empty() || arms.is_empty() {
            // Without variant values a match has nothing to choose from.
            if arms.is_empty() {
                self.diagnostics.push(Diagnostic::warning(
                    def.path.join(" > "),
                    "The component set has no variants. The function renders nothing.",
                ));
            }
            let body = arms.into_iter().next().map(|a| a.body).unwrap_or_default();
            return Ok(Body::Single(body));
        }
        let combinations = sig
            .enums
            .iter()
            .fold(1_usize, |n, e| n.saturating_mul(e.variants.len()));
        let fallback = if arms.len() < combinations {
            arms.get(default_arm.unwrap_or(0)).map(|a| a.body.clone())
        } else {
            None
        };
        Ok(Body::Match { on, arms, fallback })
    }

    /// The binding for `node`, if any. Records the hit.
    fn binding(&mut self, node: &Node, path: &[String]) -> Result<Option<(usize, Binding)>> {
        let matched: Vec<usize> = self
            .config
            .binds
            .iter()
            .enumerate()
            .filter(|(_, b)| selects(&b.selector, node, path))
            .map(|(i, _)| i)
            .collect();
        match matched.as_slice() {
            [] => Ok(None),
            [index] => {
                self.hits[*index].push(path.join(" > "));
                Ok(Some((*index, self.config.binds[*index].clone())))
            }
            many => Err(binding_err(format!(
                "two bindings match {} (node {}): {}; merge them into one",
                path.join(" > "),
                node.id,
                many.iter()
                    .map(|i| self.config.binds[*i].selector.to_string())
                    .collect::<Vec<_>>()
                    .join(" and ")
            ))),
        }
    }

    fn check_bindings(&self) -> Result<()> {
        let mut problems = Vec::new();
        for (bind, hits) in self.config.binds.iter().zip(&self.hits) {
            if hits.is_empty() {
                problems.push(format!(
                    "[[bind]] {} matches no node; check the layer names, and note that layers inside an instance of a local component take no binding (bind the component definition)",
                    bind.selector
                ));
            } else if bind.id.is_some() && hits.len() > 1 {
                problems.push(format!(
                    "[[bind]] {} sets an id but matches {} nodes; an id must be unique",
                    bind.selector,
                    hits.len()
                ));
            }
        }
        if problems.is_empty() {
            Ok(())
        } else {
            Err(binding_err(problems.join("\n")))
        }
    }

    fn class(
        &mut self,
        scope: &Scope,
        node: &Node,
        root: bool,
        decls: style::Decls,
    ) -> Option<String> {
        if decls.is_empty() {
            return None;
        }
        let prefix = &self.config.class_prefix;
        let base = if root {
            format!("{prefix}-{}", scope.slug)
        } else {
            format!("{prefix}-{}__{}", scope.slug, slug(&node.name))
        };
        let class = self.classes.claim(&base);
        self.rules.push(CssRule {
            class: class.clone(),
            decls,
            boost: false,
        });
        Some(class)
    }

    fn decls(&mut self, node: &Node, parent: Parent, role: Role, path: &str) -> style::Decls {
        let mut cx = StyleCx {
            tokens: &mut self.tokens,
            styles: &self.file.styles,
            diagnostics: &mut self.diagnostics,
            path,
        };
        style::node_decls(node, parent, role, &mut cx)
    }

    /// Lower one node. `None` when it renders nothing.
    fn node(&mut self, node: &Node, site: Site<'_>, scope: &Scope) -> Result<Option<Item>> {
        let binding = self.binding(node, site.path)?;
        let shown_by = node
            .component_property_references
            .get("visible")
            .and_then(|key| scope.props.get(key))
            .filter(|(_, kind)| *kind == PropertyType::Boolean)
            .map(|(name, _)| name.clone());
        if !node.visible && shown_by.is_none() && !site.root {
            return Ok(None);
        }
        let item = self.visible_node(node, site, scope, binding.as_ref())?;
        Ok(item.map(|item| match shown_by {
            Some(cond) if !site.root => Item::If {
                cond,
                body: vec![item],
            },
            _ => item,
        }))
    }

    fn visible_node(
        &mut self,
        node: &Node,
        site: Site<'_>,
        scope: &Scope,
        binding: Option<&(usize, Binding)>,
    ) -> Result<Option<Item>> {
        for (field, key) in &node.component_property_references {
            if field == "visible" || field == "characters" {
                continue;
            }
            match scope.props.get(key) {
                Some((name, PropertyType::InstanceSwap)) => {
                    return Ok(Some(Item::Param(name.clone())));
                }
                Some((name, PropertyType::Slot)) => {
                    // A slot keeps its own box. Only its content changes.
                    let slot = name.clone();
                    return self
                        .container(node, site, scope, binding, Some(&slot))
                        .map(Some);
                }
                _ => {}
            }
        }
        if matches!(
            node.kind,
            NodeType::Other | NodeType::Document | NodeType::Canvas
        ) {
            self.diagnostics.push(Diagnostic::info(
                site.text(),
                "The generator does not support this layer type. It skips the layer.",
            ));
            return Ok(None);
        }
        if !site.root {
            let local = match node.kind {
                NodeType::Instance => node.component_id.as_ref(),
                NodeType::Component | NodeType::ComponentSet => Some(&node.id),
                _ => None,
            }
            .and_then(|id| self.by_component.get(id).cloned());
            if let Some((sig, values)) = local {
                let target = CallSite {
                    node,
                    instance: node.kind == NodeType::Instance,
                    values: &values,
                    path: site.path,
                    parent: site.parent,
                };
                return self
                    .call(&target, sig, scope, binding.map(|b| &b.1))
                    .map(Some);
            }
        }
        if let Some(asset) = assets::asset_of(node) {
            return self
                .image(node, asset, site, scope, binding.map(|b| &b.1))
                .map(Some);
        }
        if node.kind == NodeType::Text {
            return self.text(node, site, scope, binding).map(Some);
        }
        self.container(node, site, scope, binding, None).map(Some)
    }

    fn image(
        &mut self,
        node: &Node,
        asset: Asset,
        site: Site<'_>,
        scope: &Scope,
        bind: Option<&Binding>,
    ) -> Result<Item> {
        let path_text = site.text();
        if let Some(b) = bind {
            if b.slot.is_some() || b.text.is_some() {
                return Err(binding_err(format!(
                    "[[bind]] {path_text}: the layer exports as an image, so it takes no slot or text"
                )));
            }
            if b.tag.as_deref().is_some_and(|t| t != "img") {
                self.diagnostics.push(Diagnostic::warning(
                    &path_text,
                    "The layer exports as an image. The generator ignores the binding tag.",
                ));
            }
        }
        let decls = self.decls(node, site.parent, Role::Image, &path_text);
        let class = self.class(scope, node, site.root, decls);
        let mut attrs = vec![(
            "src".to_owned(),
            format!("{}/{}", self.config.assets_url, asset.file_name),
        )];
        let extra = bind.map(|b| b.attrs.clone()).unwrap_or_default();
        if !extra.contains_key("alt") {
            attrs.push(("alt".to_owned(), String::new()));
        }
        attrs.extend(extra);
        self.assets.insert(asset);
        Ok(Item::Element(Element {
            tag: "img".to_owned(),
            class,
            id: bind.and_then(|b| b.id.clone()),
            attrs,
            root_attrs: site.root && scope.root_attrs,
            children: Vec::new(),
        }))
    }

    fn text(
        &mut self,
        node: &Node,
        site: Site<'_>,
        scope: &Scope,
        binding: Option<&(usize, Binding)>,
    ) -> Result<Item> {
        let path_text = site.text();
        let bind = binding.map(|b| &b.1);
        if bind.is_some_and(|b| b.slot.is_some()) {
            return Err(binding_err(format!(
                "[[bind]] {path_text}: `slot` needs a layer with children, and this is a text layer"
            )));
        }
        let content = node
            .component_property_references
            .get("characters")
            .and_then(|key| scope.props.get(key))
            .filter(|(_, kind)| *kind == PropertyType::Text)
            .map(|(name, _)| Item::Param(name.clone()))
            .or_else(|| {
                binding
                    .and_then(|(i, _)| scope.binds.get(i))
                    .map(|name| Item::Param(name.clone()))
            })
            .unwrap_or_else(|| Item::Text(node.characters.clone().unwrap_or_default()));
        let tag = bind
            .and_then(|b| b.tag.clone())
            .or_else(|| name_tag(&node.name, true))
            .unwrap_or_else(|| "span".to_owned());
        let mut attrs: Vec<(String, String)> = Vec::new();
        let children = match (tag.as_str(), content) {
            ("input" | "textarea", Item::Text(text)) => {
                // A form field shows the design text as its placeholder.
                if !bind.is_some_and(|b| b.attrs.contains_key("placeholder")) {
                    attrs.push(("placeholder".to_owned(), text));
                }
                Vec::new()
            }
            ("input" | "textarea" | "img" | "hr", _) => {
                return Err(binding_err(format!(
                    "[[bind]] {path_text}: a <{tag}> cannot show this text; bind the frame around it, or drop `text`"
                )));
            }
            (_, content) => vec![content],
        };
        if let Some(b) = bind {
            attrs.extend(b.attrs.clone());
        }
        let decls = self.decls(node, site.parent, Role::Text, &path_text);
        let class = self.class(scope, node, false, decls);
        Ok(Item::Element(Element {
            tag,
            class,
            id: bind.and_then(|b| b.id.clone()),
            attrs,
            root_attrs: false,
            children,
        }))
    }

    fn container(
        &mut self,
        node: &Node,
        site: Site<'_>,
        scope: &Scope,
        binding: Option<&(usize, Binding)>,
        slot: Option<&str>,
    ) -> Result<Item> {
        let path_text = site.text();
        let bind = binding.map(|b| &b.1);
        if bind.is_some_and(|b| b.text.is_some()) {
            return Err(binding_err(format!(
                "[[bind]] {path_text}: `text` needs a text layer, and this layer is a {:?}",
                node.kind
            )));
        }
        let name_for_tag = if site.root {
            &scope.root_name
        } else {
            &node.name
        };
        let tag = bind
            .and_then(|b| b.tag.clone())
            .or_else(|| name_tag(name_for_tag, false))
            .unwrap_or_else(|| "div".to_owned());
        let mut decls = self.decls(node, site.parent, Role::Box, &path_text);
        let mut attrs = Vec::new();
        let mut children = Vec::new();
        let takes_children = !matches!(tag.as_str(), "input" | "textarea" | "img" | "hr");
        if !takes_children && node.children.iter().any(|c| c.visible) {
            let text = find(node, &|n| n.visible && n.kind == NodeType::Text);
            let has_own = bind.is_some_and(|b| b.attrs.contains_key("placeholder"));
            let message = match (text, tag.as_str()) {
                (Some(t), "input" | "textarea") => {
                    if !has_own {
                        let placeholder = t.characters.clone().unwrap_or_default();
                        attrs.push(("placeholder".to_owned(), placeholder));
                    }
                    // The typed text takes the type of the text layer. The
                    // color stays out: the layer shows placeholder text.
                    let text_decls = self.decls(t, site.parent, Role::Text, &path_text);
                    decls.0.extend(
                        text_decls
                            .0
                            .into_iter()
                            .filter(|(p, _)| TYPE_PROPERTIES.contains(&p.as_str())),
                    );
                    if has_own {
                        format!(
                            "The element <{tag}> takes no children. The generator removes them."
                        )
                    } else {
                        format!(
                            "The element <{tag}> takes no children. The generator removes them. The first text becomes the placeholder."
                        )
                    }
                }
                _ => format!("The element <{tag}> takes no children. The generator removes them."),
            };
            self.diagnostics.push(Diagnostic::info(&path_text, message));
        }
        let class = self.class(scope, node, site.root, decls);
        if let Some(param) = slot.or_else(|| {
            binding
                .and_then(|(i, _)| scope.binds.get(i))
                .map(String::as_str)
        }) {
            if !takes_children {
                return Err(binding_err(format!(
                    "[[bind]] {path_text}: a <{tag}> takes no children, so it takes no slot"
                )));
            }
            children.push(Item::Param(param.to_owned()));
        } else if takes_children {
            for child in &node.children {
                let path = child_path(site.path, child);
                let child_site = Site {
                    path: &path,
                    parent: child_parent(node, child),
                    root: false,
                };
                if let Some(item) = self.node(child, child_site, scope)? {
                    children.push(item);
                }
            }
        }
        if let Some(b) = bind {
            attrs.extend(b.attrs.clone());
        }
        if let Some(theme) = self.tokens.mode_attr(&node.explicit_variable_modes)
            && !attrs.iter().any(|(n, _)| n == "data-theme")
        {
            attrs.push(("data-theme".to_owned(), theme));
        }
        Ok(Item::Element(Element {
            tag,
            class,
            id: bind.and_then(|b| b.id.clone()),
            attrs,
            root_attrs: site.root && scope.root_attrs,
            children,
        }))
    }

    /// A call of a local component for an instance (or a definition placed
    /// in a frame, with default values).
    fn call(
        &mut self,
        target: &CallSite<'_>,
        sig_index: usize,
        scope: &Scope,
        bind: Option<&Binding>,
    ) -> Result<Item> {
        let sig = self.sigs[sig_index].clone();
        let path_text = target.path.join(" > ");
        let mut fields = root_fields(&path_text, bind)?;
        if let Some(class) = self.placement_class(target, scope) {
            let at = usize::from(fields.first().is_some_and(|(f, _)| f == "id"));
            fields.insert(at, ("class".to_owned(), class));
        }
        if target.instance
            && let Some((exposed, _, _)) = target.find(&|n| n.is_exposed_instance)
        {
            self.diagnostics.push(Diagnostic::warning(
                &path_text,
                format!(
                    "The nested instance {:?} is exposed. The generator does not pass its overrides. Add a property to the outer component, or bind the nested layer.",
                    exposed.name
                ),
            ));
        }
        let mut args = Vec::new();
        for (_, source) in &sig.params {
            let arg = match source {
                Source::Attrs => Arg::Attrs(fields.clone()),
                source => self.arg(target, &sig, source, scope)?,
            };
            args.push(arg);
        }
        Ok(Item::Call(Call {
            path: sig.path(),
            args,
        }))
    }

    /// A class that places an instance in its parent, or `None` when the
    /// instance needs no placement. The rule has a doubled selector, so it
    /// wins over the rule of the component root.
    fn placement_class(&mut self, target: &CallSite<'_>, scope: &Scope) -> Option<String> {
        let mut decls = style::placement_decls(target.node, target.parent);
        // The component root already sets its own size. Keep only what
        // differs for this instance.
        let main = if target.instance {
            target
                .node
                .component_id
                .as_deref()
                .and_then(|id| find_node(&self.file.document, id))
        } else {
            Some(target.node)
        };
        let own = main.map(|m| style::placement_decls(m, Parent::Root { screen: false }));
        decls.0.retain(|(p, v)| {
            PLACEMENT_PROPERTIES.contains(&p.as_str())
                && own.as_ref().is_none_or(|o| o.get(p) != Some(v.as_str()))
        });
        if decls.get("align-self") == Some("stretch")
            && let Parent::Flex { row } = target.parent
        {
            // The root has a fixed size. Stretch needs `auto` on that axis.
            decls.push(if row { "height" } else { "width" }, "auto");
        }
        if decls.is_empty() {
            return None;
        }
        let class = self.classes.claim(&format!(
            "{}-{}__{}",
            self.config.class_prefix,
            scope.slug,
            slug(&target.node.name)
        ));
        self.rules.push(CssRule {
            class: class.clone(),
            decls,
            boost: true,
        });
        Some(class)
    }

    /// One call argument for a property or a binding parameter.
    fn arg(
        &mut self,
        target: &CallSite<'_>,
        sig: &Sig,
        source: &Source,
        scope: &Scope,
    ) -> Result<Arg> {
        let value = |key: &str| -> Option<&Value> {
            target
                .instance
                .then(|| target.node.component_properties.get(key).map(|p| &p.value))
                .flatten()
                .or_else(|| sig.defaults.get(key))
        };
        Ok(match source {
            Source::Variant(i) => {
                let e = &sig.enums[*i];
                let wanted = target
                    .instance
                    .then(|| target.node.component_properties.get(&e.key))
                    .flatten()
                    .and_then(|p| p.value.as_str())
                    .or_else(|| target.values.get(&e.key).map(String::as_str));
                let index = wanted
                    .and_then(|w| e.variants.iter().position(|(_, o)| o == w))
                    .unwrap_or(e.default);
                Arg::Path(format!(
                    "super::{}::{}::{}",
                    sig.module, e.name, e.variants[index].0
                ))
            }
            Source::Prop(key, PropertyType::Text) => Arg::Str(
                value(key)
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned(),
            ),
            Source::Prop(key, PropertyType::Boolean) => {
                Arg::Bool(value(key).and_then(Value::as_bool).unwrap_or(false))
            }
            Source::Prop(key, kind) => {
                let found = target.find(&|n| {
                    n.component_property_references
                        .iter()
                        .any(|(f, k)| f != "visible" && f != "characters" && k == key)
                });
                let children = *kind == PropertyType::Slot;
                Arg::Markup(self.markup(found, children, scope)?)
            }
            Source::BindText(_, def_ids, default) => Arg::Str(
                target
                    .find(&|n| ends_with_any(&n.id, def_ids))
                    .and_then(|(t, _, _)| t.characters.clone())
                    .unwrap_or_else(|| default.clone()),
            ),
            Source::BindSlot(_, def_ids) => {
                let found = target.find(&|n| ends_with_any(&n.id, def_ids));
                Arg::Markup(self.markup(found, true, scope)?)
            }
            Source::Attrs => Arg::Attrs(Vec::new()),
        })
    }

    /// Lower a node found inside an instance as a call argument: the node,
    /// or its children when `children` is set. `None` gives no items.
    fn markup(
        &mut self,
        found: Option<(&Node, &Node, Vec<String>)>,
        children: bool,
        scope: &Scope,
    ) -> Result<Vec<Item>> {
        let Some((node, parent, path)) = found else {
            return Ok(Vec::new());
        };
        let nodes: Vec<(&Node, &Node, Vec<String>)> = if children {
            node.children
                .iter()
                .map(|c| (c, node, child_path(&path, c)))
                .collect()
        } else {
            vec![(node, parent, path)]
        };
        let mut items = Vec::new();
        for (node, parent, path) in nodes {
            let site = Site {
                path: &path,
                parent: child_parent(parent, node),
                root: false,
            };
            if let Some(item) = self.node(node, site, scope)? {
                items.push(item);
            }
        }
        Ok(items)
    }
}

/// Where a node sits while it is lowered.
#[derive(Debug, Clone, Copy)]
struct Site<'p> {
    /// Layer names from the page down.
    path: &'p [String],
    /// How the parent places the node.
    parent: Parent,
    /// True for the root of a function.
    root: bool,
}

impl Site<'_> {
    fn text(&self) -> String {
        self.path.join(" > ")
    }
}

/// The node that a call stands for.
struct CallSite<'n> {
    node: &'n Node,
    /// False for a definition placed in a frame: no instance values.
    instance: bool,
    /// Variant values from the main component name.
    values: &'n BTreeMap<String, String>,
    path: &'n [String],
    /// How the parent places the instance.
    parent: Parent,
}

impl<'n> CallSite<'n> {
    /// The first descendant that passes `pred`, with its parent and path.
    /// For a definition placed in a frame, this searches the definition.
    fn find(&self, pred: &dyn Fn(&Node) -> bool) -> Option<(&'n Node, &'n Node, Vec<String>)> {
        find_with_path(self.node, self.path, pred)
    }
}

/// CSS properties that place a node in its parent.
const PLACEMENT_PROPERTIES: &[&str] = &[
    "position",
    "left",
    "top",
    "width",
    "height",
    "max-width",
    "min-width",
    "min-height",
    "flex",
    "flex-shrink",
    "align-self",
];

/// The node with `id` in the tree under `node`.
fn find_node<'n>(node: &'n Node, id: &str) -> Option<&'n Node> {
    if node.id == id {
        return Some(node);
    }
    node.children.iter().find_map(|c| find_node(c, id))
}

/// The first descendant of `node` that passes `pred`, with its parent and
/// its layer path.
fn find_with_path<'n>(
    node: &'n Node,
    path: &[String],
    pred: &dyn Fn(&Node) -> bool,
) -> Option<(&'n Node, &'n Node, Vec<String>)> {
    node.children.iter().find_map(|child| {
        let child_path = child_path(path, child);
        if pred(child) {
            Some((child, node, child_path))
        } else {
            find_with_path(child, &child_path, pred)
        }
    })
}

/// The `Attrs` fields that a binding on an instance sets.
fn root_fields(path_text: &str, bind: Option<&Binding>) -> Result<Vec<(String, String)>> {
    let Some(b) = bind else {
        return Ok(Vec::new());
    };
    if b.tag.is_some() || b.slot.is_some() || b.text.is_some() {
        return Err(binding_err(format!(
            "[[bind]] {path_text}: a binding on a component instance sets only `id` and `attrs`; bind the component definition for tag, slot, or text"
        )));
    }
    if let Some(name) = b.attrs.keys().find(|n| root_attr_field(n).is_none()) {
        let allowed: Vec<&str> = ROOT_ATTRS.iter().map(|(h, _)| *h).collect();
        return Err(binding_err(format!(
            "[[bind]] {path_text}: {name:?} cannot pass to a component root; use one of {}",
            allowed.join(", ")
        )));
    }
    Ok(ROOT_ATTRS
        .iter()
        .filter_map(|(html, field)| {
            let value = if *html == "id" {
                b.id.clone()
            } else {
                b.attrs.get(*html).cloned()
            };
            value.map(|v| ((*field).to_owned(), v))
        })
        .collect())
}

/// True when `id` is one of `def_ids`, or an instance copy of one
/// (`I<instance>;<def id>`).
fn ends_with_any(id: &str, def_ids: &[String]) -> bool {
    def_ids.iter().any(|d| {
        // Figma drops the inner `I` when it nests an instance id:
        // `I5:3;4:2` becomes `I6:1;5:3;4:2`.
        let bare = d.trim_start_matches('I');
        id == d || id.ends_with(&format!(";{d}")) || id.ends_with(&format!(";{bare}"))
    })
}

#[cfg(test)]
mod tests;
