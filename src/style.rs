//! CSS declarations for one node.
//!
//! Auto layout maps to flexbox, grid layout to CSS grid. A frame without
//! auto layout places its children with absolute positions, and gives a
//! warning. Tokens win over literal values.

use std::collections::BTreeMap;

use crate::diagnostics::Diagnostic;
use crate::model::{
    Effect, EffectType, LayoutMode, Node, NodeType, Paint, PaintType, Rect, Sizing, StyleMeta,
    TypeStyle, alias_id,
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
    if node.rotation.is_some_and(|r| r.abs() > 0.01) {
        cx.diagnostics.push(Diagnostic::warning(
            cx.path,
            "The generator does not support rotation. The layer renders without it.",
        ));
    }
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
        return match (node.text_resize(), horizontal) {
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
            // Figma ignores the item spacing for space-between.
            let between = node.primary_axis_align_items.as_deref() == Some("SPACE_BETWEEN");
            if node.item_spacing.is_some_and(|s| s < 0.0) {
                cx.diagnostics.push(Diagnostic::warning(
                    cx.path,
                    "The generator does not support negative item spacing. The items do not overlap.",
                ));
            }
            let item = spacing(node, "itemSpacing", node.item_spacing, cx)
                .filter(|_| !between && !node.item_spacing.is_some_and(|s| s < 0.0));
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
            if wrap {
                let content = if node.counter_axis_align_content.as_deref() == Some("SPACE_BETWEEN")
                {
                    "space-between"
                } else {
                    "flex-start"
                };
                d.push("align-content", content);
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
            let template = node
                .grid_columns_sizing
                .as_deref()
                .filter(|t| is_track_list(t))
                .map_or_else(
                    || format!("repeat({columns}, minmax(0, 1fr))"),
                    str::to_owned,
                );
            d.push("grid-template-columns", template);
            if let Some(rows) = node
                .grid_rows_sizing
                .as_deref()
                .filter(|t| is_track_list(t))
            {
                d.push("grid-template-rows", rows);
            }
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
            "The frame has no auto layout. Its children get absolute positions. Add auto layout in Figma for a responsive result.",
        ));
    }
}

/// True for a grid track list that is safe to copy into CSS: letters,
/// digits, spaces, and `( ) , . % -` only.
fn is_track_list(text: &str) -> bool {
    !text.trim().is_empty()
        && text
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || " (),.%-".contains(c))
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
        let explicit =
            node.layout_sizing_horizontal.is_some() || node.layout_sizing_vertical.is_some();
        placed.0.retain(|(p, v)| match p.as_str() {
            "flex" | "align-self" | "justify-self" | "grid-column" | "grid-row" => true,
            // From explicit hug sizing only. A guess must not pin the size.
            "flex-shrink" => explicit,
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
        Parent::Root { screen: false } => {
            if fixed_if(sizing(node, true)) == Sizing::Fixed {
                d.push("width", px(b.width));
            }
            if fixed_if(sizing(node, false)) == Sizing::Fixed {
                d.push("height", px(b.height));
            }
            // A component never shrinks below its size in a flex parent.
            d.push("flex-shrink", "0");
        }
        Parent::Grid => {
            let h = fixed_if(sizing(node, true));
            let v = fixed_if(sizing(node, false));
            if h == Sizing::Fixed {
                d.push("width", px(b.width));
            }
            if v == Sizing::Fixed {
                d.push("height", px(b.height));
            }
            grid_cell(node, h, v, d);
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
                // Figma never shrinks a hug child below its content.
                Sizing::Hug => d.push("flex-shrink", "0"),
                Sizing::Other => {}
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

/// The cell and the alignment of a grid child.
fn grid_cell(node: &Node, h: Sizing, v: Sizing, d: &mut Decls) {
    if let Some(column) = node.grid_column_anchor_index {
        let span = node.grid_column_span.unwrap_or(1).max(1);
        d.push("grid-column", format!("{} / span {span}", column + 1));
    }
    if let Some(row) = node.grid_row_anchor_index {
        let span = node.grid_row_span.unwrap_or(1).max(1);
        d.push("grid-row", format!("{} / span {span}", row + 1));
    }
    // A grid item stretches by default. A child that does not fill its
    // cell keeps its own size at the start.
    let align = |figma: Option<&str>, size: Sizing| match figma {
        Some("CENTER") => Some("center"),
        Some("MAX") => Some("end"),
        Some("MIN") => Some("start"),
        _ if size != Sizing::Fill => Some("start"),
        _ => None,
    };
    if let Some(a) = align(node.grid_child_horizontal_align.as_deref(), h) {
        d.push("justify-self", a);
    }
    if let Some(a) = align(node.grid_child_vertical_align.as_deref(), v) {
        d.push("align-self", a);
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
        if paint.opacity < 1.0 {
            let percent = num((paint.opacity * 100.0).clamp(0.0, 100.0));
            return Some(format!("color-mix(in srgb, {var} {percent}%, transparent)"));
        }
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
            "The generator does not support an image fill on a layer with children. It removes the fill.",
        )),
        kind => cx.diagnostics.push(Diagnostic::warning(
            cx.path,
            format!("The generator does not support a {kind:?} fill. The layer has no background."),
        )),
    }
}

/// The stroke. Figma draws it outside the layout, so the default is an
/// inset shadow (`INSIDE`) or an outline (`OUTSIDE`, `CENTER`, dashed). Only
/// a stroke that takes layout space becomes a `border`. An inset shadow goes
/// in `box-shadow`, and [`effects`] adds the effect shadows before it.
fn stroke(node: &Node, d: &mut Decls, cx: &mut StyleCx<'_>) {
    let weight = node.stroke_weight.unwrap_or(1.0);
    let Some(top) = Node::top_paint(&node.strokes) else {
        return;
    };
    if top.kind != PaintType::Solid || weight <= 0.0 {
        return;
    }
    let Some(color) = paint_color(node, top, "stroke", cx) else {
        return;
    };
    let line = if node.stroke_dashes.is_empty() {
        "solid"
    } else {
        "dashed"
    };
    let align = node.stroke_align.as_deref().unwrap_or("INSIDE");
    if let Some(sides) = node.individual_stroke_weights.filter(|w| {
        [w.top, w.right, w.bottom, w.left]
            .iter()
            .any(|v| (v - weight).abs() > f64::EPSILON)
    }) {
        let each = [
            ("top", sides.top, (0.0, 1.0)),
            ("right", sides.right, (-1.0, 0.0)),
            ("bottom", sides.bottom, (0.0, -1.0)),
            ("left", sides.left, (1.0, 0.0)),
        ];
        if node.strokes_included_in_layout {
            for (side, w, _) in each.iter().filter(|(_, w, _)| *w > 0.0) {
                d.push(
                    &format!("border-{side}"),
                    format!("{} {line} {color}", px(*w)),
                );
            }
            return;
        }
        let outside = align == "OUTSIDE";
        let shadows: Vec<String> = each
            .iter()
            .filter(|(_, w, _)| *w > 0.0)
            .map(|(_, w, (x, y))| {
                let sign = if outside { -1.0 } else { 1.0 };
                let inset = if outside { "" } else { "inset " };
                format!(
                    "{inset}{} {} 0 0 {color}",
                    px(x * w * sign),
                    px(y * w * sign)
                )
            })
            .collect();
        d.push("box-shadow", shadows.join(", "));
        return;
    }
    if node.strokes_included_in_layout {
        d.push("border", format!("{} {line} {color}", px(weight)));
    } else if align == "INSIDE" && line == "solid" {
        d.push("box-shadow", format!("inset 0 0 0 {} {color}", px(weight)));
    } else {
        d.push("outline", format!("{} {line} {color}", px(weight)));
        let offset = match align {
            "INSIDE" => -weight,
            "CENTER" => -weight / 2.0,
            _ => 0.0,
        };
        if offset != 0.0 {
            d.push("outline-offset", px(offset));
        }
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
    let nested = [
        "RECTANGLE_TOP_LEFT_CORNER_RADIUS",
        "RECTANGLE_TOP_RIGHT_CORNER_RADIUS",
        "RECTANGLE_BOTTOM_RIGHT_CORNER_RADIUS",
        "RECTANGLE_BOTTOM_LEFT_CORNER_RADIUS",
    ];
    let radii_map = node
        .bound_variables
        .get("rectangleCornerRadii")
        .and_then(serde_json::Value::as_object);
    let bound: Vec<Option<&str>> = corners
        .iter()
        .zip(nested)
        .map(|(flat, deep)| {
            node.bound_variable(flat)
                .or_else(|| radii_map.and_then(|m| m.get(deep)).and_then(alias_id))
        })
        .collect();
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
    // The stroke shadow, if any, goes after the effect shadows.
    let stroke =
        d.0.iter()
            .position(|(p, _)| p == "box-shadow")
            .map(|i| d.0.remove(i).1);
    let effect = (!shadows.is_empty()).then(|| {
        let literal = shadows.join(", ");
        let styled = node.styles.get("effect").and_then(|id| {
            let meta = cx.styles.get(id)?;
            let value = literal.clone();
            cx.tokens.style(id, &meta.name, || Some(value))
        });
        styled.unwrap_or(literal)
    });
    let all: Vec<String> = effect.into_iter().chain(stroke).collect();
    if !all.is_empty() {
        d.push("box-shadow", all.join(", "));
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
        // Figma leaves the percent out when it is 100. The px value stays.
        Some("FONT_SIZE_%") => style
            .line_height_percent_font_size
            .map(|p| num(p / 100.0))
            .or_else(|| style.line_height_px.map(px)),
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
    if node.text_resize() == Some("WIDTH_AND_HEIGHT") {
        d.push("white-space", "nowrap");
    }
}

#[cfg(test)]
mod tests;
