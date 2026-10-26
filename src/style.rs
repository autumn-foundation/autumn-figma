//! CSS declarations for one node.
//!
//! Auto layout maps to flexbox, grid layout to CSS grid. A frame without
//! auto layout places its children with absolute positions, and gives a
//! warning. Tokens win over literal values.

use std::collections::BTreeMap;

use crate::diagnostics::Diagnostic;
use crate::model::{
    Effect, EffectType, LayoutMode, Node, NodeType, Paint, PaintType, Rect, Sizing, StyleMeta,
    TypeStyle,
};
use crate::names::{css_str, num, px};
use crate::tokens::{TokenSet, css_color};

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
    pub const fn is_empty(&self) -> bool {
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
    let mut d = Decls::default();
    let absolute_box = role == Role::Box && places_children_absolutely(node);
    match role {
        Role::Image => d.push("display", "block"),
        Role::Box => layout(node, absolute_box, &mut d, cx),
        Role::Text => {}
    }
    placement(node, parent, role, absolute_box, &mut d);
    if role == Role::Box {
        padding(node, &mut d, cx);
    }
    paint(node, role, &mut d, cx);
    if role != Role::Text {
        stroke(node, &mut d, cx);
        radius(node, &mut d, cx);
    }
    effects(node, role, &mut d, cx);
    if let Some(opacity) = node.opacity.filter(|o| *o < 1.0) {
        d.push("opacity", num(opacity.max(0.0)));
    }
    if node.clips_content {
        d.push("overflow", "hidden");
    }
    if role == Role::Text {
        typography(node, &mut d, cx);
    }
    d
}

/// Only the declarations that place `node` in its parent: position,
/// offsets, size, and flex sizing.
#[must_use]
pub fn placement_decls(node: &Node, parent: Parent) -> Decls {
    let mut d = Decls::default();
    placement(
        node,
        parent,
        Role::Box,
        places_children_absolutely(node),
        &mut d,
    );
    d
}

/// True when `node` is a box without auto layout that has visible children.
#[must_use]
pub fn places_children_absolutely(node: &Node) -> bool {
    let auto = matches!(
        node.layout_mode,
        Some(LayoutMode::Horizontal | LayoutMode::Vertical | LayoutMode::Grid)
    );
    !auto && node.children.iter().any(|c| c.visible)
}

/// The sizing of `node` on one axis, with fallbacks for older files.
fn sizing(node: &Node, horizontal: bool) -> Sizing {
    let explicit = if horizontal {
        node.layout_sizing_horizontal
    } else {
        node.layout_sizing_vertical
    };
    if let Some(s @ (Sizing::Fixed | Sizing::Hug | Sizing::Fill)) = explicit {
        return s;
    }
    if node.kind == NodeType::Text {
        return match (node.text_auto_resize.as_deref(), horizontal) {
            (Some("WIDTH_AND_HEIGHT"), _) | (Some("HEIGHT"), false) => Sizing::Hug,
            _ => Sizing::Fixed,
        };
    }
    let primary_horizontal = match node.layout_mode {
        Some(LayoutMode::Horizontal) => true,
        Some(LayoutMode::Vertical) => false,
        _ => return Sizing::Fixed,
    };
    let mode = if horizontal == primary_horizontal {
        node.primary_axis_sizing_mode.as_deref()
    } else {
        node.counter_axis_sizing_mode.as_deref()
    };
    if mode == Some("FIXED") {
        Sizing::Fixed
    } else {
        Sizing::Hug
    }
}

fn bbox(node: &Node) -> Rect {
    node.absolute_bounding_box.unwrap_or_default()
}

fn layout(node: &Node, absolute_box: bool, d: &mut Decls, cx: &mut StyleCx<'_>) {
    match node.layout_mode {
        Some(mode @ (LayoutMode::Horizontal | LayoutMode::Vertical)) => {
            let row = mode == LayoutMode::Horizontal;
            d.push("display", "flex");
            d.push("flex-direction", if row { "row" } else { "column" });
            let wrap = node.layout_wrap.as_deref() == Some("WRAP");
            if wrap {
                d.push("flex-wrap", "wrap");
            }
            let item = spacing(node, "itemSpacing", node.item_spacing, cx);
            let counter =
                spacing(node, "counterAxisSpacing", node.counter_axis_spacing, cx).filter(|_| wrap);
            match (item, counter) {
                (Some(item), Some(counter)) if row => d.push("gap", format!("{counter} {item}")),
                (Some(item), Some(counter)) => d.push("gap", format!("{item} {counter}")),
                (Some(gap), None) => d.push("gap", gap),
                (None, Some(counter)) if row => d.push("gap", format!("{counter} 0")),
                (None, Some(counter)) => d.push("gap", format!("0 {counter}")),
                (None, None) => {}
            }
            match node.primary_axis_align_items.as_deref() {
                Some("CENTER") => d.push("justify-content", "center"),
                Some("MAX") => d.push("justify-content", "flex-end"),
                Some("SPACE_BETWEEN") => d.push("justify-content", "space-between"),
                _ => {}
            }
            d.push(
                "align-items",
                match node.counter_axis_align_items.as_deref() {
                    Some("CENTER") => "center",
                    Some("MAX") => "flex-end",
                    Some("BASELINE") => "baseline",
                    _ => "flex-start",
                },
            );
        }
        Some(LayoutMode::Grid) => {
            d.push("display", "grid");
            let columns = node.grid_column_count.unwrap_or(1).max(1);
            d.push(
                "grid-template-columns",
                format!("repeat({columns}, minmax(0, 1fr))"),
            );
            let row_gap = node.grid_row_gap.unwrap_or(0.0);
            let column_gap = node.grid_column_gap.unwrap_or(0.0);
            if row_gap > 0.0 || column_gap > 0.0 {
                d.push("gap", format!("{} {}", px(row_gap), px(column_gap)));
            }
        }
        _ => {}
    }
    let absolute_child = node
        .children
        .iter()
        .any(|c| c.visible && c.layout_positioning.as_deref() == Some("ABSOLUTE"));
    if absolute_box || absolute_child {
        d.push("position", "relative");
    }
    if absolute_box {
        cx.diagnostics.push(Diagnostic::warning(
            cx.path,
            "the frame has no auto layout, so its children use absolute positions; add auto layout in Figma for responsive output",
        ));
    }
}

/// A spacing value: a bound variable, or px. `None` for zero.
fn spacing(node: &Node, field: &str, value: Option<f64>, cx: &StyleCx<'_>) -> Option<String> {
    if let Some(var) = node
        .bound_variable(field)
        .and_then(|id| cx.tokens.variable(id))
    {
        return Some(var);
    }
    value.filter(|v| *v != 0.0).map(px)
}

fn placement(node: &Node, parent: Parent, role: Role, absolute_box: bool, d: &mut Decls) {
    let mut placed = Decls::default();
    placement_from_box(node, parent, role, absolute_box, &mut placed);
    if node.absolute_bounding_box.is_none() {
        // Without a box, a size or an offset would be 0. Leave them out.
        placed.0.retain(|(p, v)| match p.as_str() {
            "flex" | "align-self" => true,
            "min-width" | "min-height" => v == "0",
            "width" => v == "100%",
            _ => false,
        });
    }
    d.0.extend(placed.0);
}

fn placement_from_box(node: &Node, parent: Parent, role: Role, absolute_box: bool, d: &mut Decls) {
    let b = bbox(node);
    let fixed_if = |s: Sizing| {
        if (absolute_box || role == Role::Image) && s == Sizing::Hug {
            Sizing::Fixed
        } else {
            s
        }
    };
    match parent {
        Parent::Root { screen: true } => {
            d.push("width", "100%");
            d.push("max-width", px(b.width));
            if absolute_box {
                d.push("min-height", px(b.height));
            }
        }
        Parent::Root { screen: false } | Parent::Grid => {
            if fixed_if(sizing(node, true)) == Sizing::Fixed {
                d.push("width", px(b.width));
            }
            if fixed_if(sizing(node, false)) == Sizing::Fixed {
                d.push("height", px(b.height));
            }
        }
        Parent::Flex { row } => {
            let old_fill = |explicit: Option<Sizing>, yes: bool| explicit.is_none() && yes;
            let grow = node.layout_grow == Some(1.0);
            let stretch = node.layout_align.as_deref() == Some("STRETCH");
            let (h_explicit, v_explicit) =
                (node.layout_sizing_horizontal, node.layout_sizing_vertical);
            let mut h = fixed_if(sizing(node, true));
            let mut v = fixed_if(sizing(node, false));
            if old_fill(if row { h_explicit } else { v_explicit }, grow) {
                if row {
                    h = Sizing::Fill;
                } else {
                    v = Sizing::Fill;
                }
            }
            if old_fill(if row { v_explicit } else { h_explicit }, stretch) {
                if row {
                    v = Sizing::Fill;
                } else {
                    h = Sizing::Fill;
                }
            }
            let (main, cross) = if row { (h, v) } else { (v, h) };
            let (main_size, cross_size) = if row {
                (("width", b.width), ("height", b.height))
            } else {
                (("height", b.height), ("width", b.width))
            };
            match main {
                Sizing::Fill => {
                    d.push("flex", "1 1 0");
                    d.push(if row { "min-width" } else { "min-height" }, "0");
                }
                Sizing::Fixed => {
                    d.push(main_size.0, px(main_size.1));
                    d.push("flex-shrink", "0");
                }
                Sizing::Hug | Sizing::Other => {}
            }
            match cross {
                Sizing::Fill => d.push("align-self", "stretch"),
                Sizing::Fixed => d.push(cross_size.0, px(cross_size.1)),
                Sizing::Hug | Sizing::Other => {}
            }
        }
        Parent::Absolute { origin } => {
            d.push("position", "absolute");
            d.push("left", px(b.x - origin.x));
            d.push("top", px(b.y - origin.y));
            d.push("width", px(b.width));
            d.push("height", px(b.height));
        }
    }
}

fn padding(node: &Node, d: &mut Decls, cx: &StyleCx<'_>) {
    let side = |field: &str, value: Option<f64>| {
        node.bound_variable(field)
            .and_then(|id| cx.tokens.variable(id))
            .unwrap_or_else(|| px(value.unwrap_or(0.0)))
    };
    let top = side("paddingTop", node.padding_top);
    let right = side("paddingRight", node.padding_right);
    let bottom = side("paddingBottom", node.padding_bottom);
    let left = side("paddingLeft", node.padding_left);
    if [&top, &right, &bottom, &left].iter().all(|v| *v == "0") {
        return;
    }
    let value = if top == right && right == bottom && bottom == left {
        top
    } else if top == bottom && left == right {
        format!("{top} {right}")
    } else {
        format!("{top} {right} {bottom} {left}")
    };
    d.push("padding", value);
}

/// The CSS color of a solid paint: a bound variable, a style, or a literal.
fn paint_color(
    node: &Node,
    paint: &Paint,
    style_key: &str,
    cx: &mut StyleCx<'_>,
) -> Option<String> {
    if let Some(var) = paint.color_variable().and_then(|id| cx.tokens.variable(id)) {
        return Some(var);
    }
    let literal = paint.color.map(|c| css_color(&c, paint.opacity));
    if let Some(style_id) = node.styles.get(style_key)
        && let Some(meta) = cx.styles.get(style_id)
    {
        let value = literal.clone();
        if let Some(var) = cx.tokens.style(style_id, &meta.name, || value) {
            return Some(var);
        }
    }
    literal
}

fn paint(node: &Node, role: Role, d: &mut Decls, cx: &mut StyleCx<'_>) {
    if role == Role::Image {
        return;
    }
    let Some(top) = Node::top_paint(&node.fills) else {
        return;
    };
    match top.kind {
        PaintType::Solid => {
            if let Some(color) = paint_color(node, top, "fill", cx) {
                let property = if role == Role::Text {
                    "color"
                } else {
                    "background-color"
                };
                d.push(property, color);
            }
        }
        PaintType::Image => cx.diagnostics.push(Diagnostic::warning(
            cx.path,
            "an image fill on a layer with children is not supported; the fill is dropped",
        )),
        kind => cx.diagnostics.push(Diagnostic::warning(
            cx.path,
            format!("{kind:?} fill is not supported; the layer has no background"),
        )),
    }
}

fn stroke(node: &Node, d: &mut Decls, cx: &mut StyleCx<'_>) {
    let weight = node.stroke_weight.unwrap_or(1.0);
    let Some(top) = Node::top_paint(&node.strokes) else {
        return;
    };
    if top.kind != PaintType::Solid || weight <= 0.0 {
        return;
    }
    if let Some(color) = paint_color(node, top, "stroke", cx) {
        let line = if node.stroke_dashes.is_empty() {
            "solid"
        } else {
            "dashed"
        };
        d.push("border", format!("{} {line} {color}", px(weight)));
    }
}

fn radius(node: &Node, d: &mut Decls, cx: &StyleCx<'_>) {
    if node.kind == NodeType::Ellipse {
        d.push("border-radius", "50%");
        return;
    }
    let corners = [
        "topLeftRadius",
        "topRightRadius",
        "bottomRightRadius",
        "bottomLeftRadius",
    ];
    let bound: Vec<Option<&str>> = corners.iter().map(|c| node.bound_variable(c)).collect();
    if let Some(Some(first)) = bound.first()
        && bound.iter().all(|b| *b == Some(*first))
        && let Some(var) = cx.tokens.variable(first)
    {
        d.push("border-radius", var);
        return;
    }
    if let Some(radii) = node.rectangle_corner_radii {
        if radii.iter().any(|r| (r - radii[0]).abs() > f64::EPSILON) {
            let parts: Vec<String> = radii.iter().map(|r| px(*r)).collect();
            d.push("border-radius", parts.join(" "));
            return;
        }
        if radii[0] > 0.0 {
            d.push("border-radius", px(radii[0]));
        }
        return;
    }
    if let Some(r) = node.corner_radius.filter(|r| *r > 0.0) {
        d.push("border-radius", px(r));
    }
}

fn effects(node: &Node, role: Role, d: &mut Decls, cx: &mut StyleCx<'_>) {
    let visible: Vec<&Effect> = node.effects.iter().filter(|e| e.visible).collect();
    let shadow = |e: &Effect, text: bool| {
        let offset = e.offset.unwrap_or_default();
        let color = e
            .color
            .map_or_else(|| "#000000".to_owned(), |c| css_color(&c, 1.0));
        if text {
            format!("{} {} {} {color}", px(offset.x), px(offset.y), px(e.radius))
        } else {
            let inset = if e.kind == EffectType::InnerShadow {
                "inset "
            } else {
                ""
            };
            format!(
                "{inset}{} {} {} {} {color}",
                px(offset.x),
                px(offset.y),
                px(e.radius),
                px(e.spread)
            )
        }
    };
    if role == Role::Text {
        let shadows: Vec<String> = visible
            .iter()
            .filter(|e| e.kind == EffectType::DropShadow)
            .map(|e| shadow(e, true))
            .collect();
        if !shadows.is_empty() {
            d.push("text-shadow", shadows.join(", "));
        }
        return;
    }
    let shadows: Vec<String> = visible
        .iter()
        .filter(|e| matches!(e.kind, EffectType::DropShadow | EffectType::InnerShadow))
        .map(|e| shadow(e, false))
        .collect();
    if !shadows.is_empty() {
        let literal = shadows.join(", ");
        let styled = node.styles.get("effect").and_then(|id| {
            let meta = cx.styles.get(id)?;
            let value = literal.clone();
            cx.tokens.style(id, &meta.name, || Some(value))
        });
        d.push("box-shadow", styled.unwrap_or(literal));
    }
    for e in &visible {
        match e.kind {
            EffectType::LayerBlur => d.push("filter", format!("blur({})", px(e.radius / 2.0))),
            EffectType::BackgroundBlur => {
                d.push("backdrop-filter", format!("blur({})", px(e.radius / 2.0)));
            }
            _ => {}
        }
    }
}

const FONT_FALLBACK: &str = "system-ui, sans-serif";

/// The `font` shorthand of a text style, or `None` without a font size.
fn font_shorthand(style: &TypeStyle) -> Option<String> {
    let size = style.font_size?;
    let family = style.font_family.as_deref()?;
    let italic = if style.italic { "italic " } else { "" };
    let weight = num(style.font_weight.unwrap_or(400.0));
    let line = line_height(style).unwrap_or_else(|| "normal".to_owned());
    Some(format!(
        "{italic}{weight} {}/{line} {}, {FONT_FALLBACK}",
        px(size),
        css_str(family)
    ))
}

fn line_height(style: &TypeStyle) -> Option<String> {
    match style.line_height_unit.as_deref() {
        Some("FONT_SIZE_%") => style.line_height_percent_font_size.map(|p| num(p / 100.0)),
        Some("INTRINSIC_%") => None,
        _ => style.line_height_px.map(px),
    }
}

fn typography(node: &Node, d: &mut Decls, cx: &mut StyleCx<'_>) {
    let Some(style) = &node.style else {
        return;
    };
    let token = node.styles.get("text").and_then(|id| {
        let meta = cx.styles.get(id)?;
        cx.tokens.style(id, &meta.name, || font_shorthand(style))
    });
    if let Some(var) = token {
        d.push("font", var);
    } else {
        if let Some(family) = &style.font_family {
            d.push(
                "font-family",
                format!("{}, {FONT_FALLBACK}", css_str(family)),
            );
        }
        if let Some(weight) = style.font_weight {
            d.push("font-weight", num(weight));
        }
        if let Some(size) = style.font_size {
            d.push("font-size", px(size));
        }
        if let Some(line) = line_height(style) {
            d.push("line-height", line);
        }
        if style.italic {
            d.push("font-style", "italic");
        }
    }
    if let Some(spacing) = style.letter_spacing.filter(|s| *s != 0.0) {
        d.push("letter-spacing", px(spacing));
    }
    match style.text_align_horizontal.as_deref() {
        Some("CENTER") => d.push("text-align", "center"),
        Some("RIGHT") => d.push("text-align", "right"),
        Some("JUSTIFIED") => d.push("text-align", "justify"),
        _ => {}
    }
    match style.text_decoration.as_deref() {
        Some("UNDERLINE") => d.push("text-decoration", "underline"),
        Some("STRIKETHROUGH") => d.push("text-decoration", "line-through"),
        _ => {}
    }
    match style.text_case.as_deref() {
        Some("UPPER") => d.push("text-transform", "uppercase"),
        Some("LOWER") => d.push("text-transform", "lowercase"),
        Some("TITLE") => d.push("text-transform", "capitalize"),
        _ => {}
    }
    if node.text_auto_resize.as_deref() == Some("WIDTH_AND_HEIGHT") {
        d.push("white-space", "nowrap");
    }
}

#[cfg(test)]
mod tests;
