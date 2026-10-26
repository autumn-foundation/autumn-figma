//! Figma REST payloads.
//!
//! The types read only the fields that generation uses. Unknown fields are
//! ignored. Unknown enum values map to an `Other` variant, so a new Figma
//! feature does not break the parser.

use std::collections::BTreeMap;

use serde::Deserialize;
use serde_json::Value;

use crate::error::{Error, ErrorKind, Result};

/// `GET /v1/files/:key`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileResponse {
    /// The file name.
    #[serde(default)]
    pub name: String,
    /// The file version.
    #[serde(default)]
    pub version: Option<String>,
    /// The root node.
    pub document: Node,
    /// Component metadata by node id.
    #[serde(default)]
    pub components: BTreeMap<String, ComponentMeta>,
    /// Style metadata by style id.
    #[serde(default)]
    pub styles: BTreeMap<String, StyleMeta>,
}

impl FileResponse {
    /// Parse a file body.
    ///
    /// # Errors
    ///
    /// [`ErrorKind::Json`] when the body is not a Figma file.
    pub fn from_json(src: &str) -> Result<Self> {
        serde_json::from_str(src)
            .map_err(|e| Error::new(ErrorKind::Json, format!("Figma file: {e}")))
    }
}

/// Component metadata.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComponentMeta {
    /// The component name.
    #[serde(default)]
    pub name: String,
    /// The set id, for a variant.
    #[serde(default)]
    pub component_set_id: Option<String>,
    /// True when the component comes from a library.
    #[serde(default)]
    pub remote: bool,
}

/// Style metadata.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StyleMeta {
    /// The style name, for example `Heading/H1`.
    #[serde(default)]
    pub name: String,
    /// The style type.
    #[serde(default = "StyleType::other")]
    pub style_type: StyleType,
}

/// The type of a style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StyleType {
    /// A fill style.
    Fill,
    /// A text style.
    Text,
    /// An effect style.
    Effect,
    /// A grid style.
    Grid,
    /// Any other type.
    #[serde(other)]
    Other,
}

impl StyleType {
    const fn other() -> Self {
        Self::Other
    }
}

/// The type of a node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NodeType {
    /// The root.
    Document,
    /// A page.
    Canvas,
    /// A frame.
    Frame,
    /// A group.
    Group,
    /// A section.
    Section,
    /// A component.
    Component,
    /// A component set (variants).
    ComponentSet,
    /// An instance of a component.
    Instance,
    /// Text.
    Text,
    /// A rectangle.
    Rectangle,
    /// An ellipse.
    Ellipse,
    /// A vector.
    Vector,
    /// A line.
    Line,
    /// A star.
    Star,
    /// A polygon.
    RegularPolygon,
    /// A boolean operation.
    BooleanOperation,
    /// A slot.
    Slot,
    /// Any other type.
    #[serde(other)]
    Other,
}

impl NodeType {
    const fn other() -> Self {
        Self::Other
    }

    /// True for vector shapes that export as SVG.
    #[must_use]
    pub const fn is_vector(self) -> bool {
        matches!(
            self,
            Self::Vector | Self::Line | Self::Star | Self::RegularPolygon | Self::BooleanOperation
        )
    }

    /// True for nodes that hold children and render as a box.
    #[must_use]
    pub const fn is_container(self) -> bool {
        matches!(
            self,
            Self::Frame
                | Self::Group
                | Self::Section
                | Self::Component
                | Self::ComponentSet
                | Self::Instance
                | Self::Slot
        )
    }
}

/// A node in the document tree.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Node {
    /// The node id, for example `1:2`.
    pub id: String,
    /// The layer name.
    #[serde(default)]
    pub name: String,
    /// The node type.
    #[serde(rename = "type", default = "NodeType::other")]
    pub kind: NodeType,
    /// False when the layer is hidden.
    #[serde(default = "yes")]
    pub visible: bool,
    /// Child nodes, back to front.
    #[serde(default)]
    pub children: Vec<Self>,
    /// The box in canvas coordinates.
    #[serde(default)]
    pub absolute_bounding_box: Option<Rect>,
    /// Fills, bottom to top.
    #[serde(default)]
    pub fills: Vec<Paint>,
    /// Strokes.
    #[serde(default)]
    pub strokes: Vec<Paint>,
    /// The stroke width.
    #[serde(default)]
    pub stroke_weight: Option<f64>,
    /// Dash pattern of the stroke.
    #[serde(default)]
    pub stroke_dashes: Vec<f64>,
    /// One radius for all corners.
    #[serde(default)]
    pub corner_radius: Option<f64>,
    /// Radii: top left, top right, bottom right, bottom left.
    #[serde(default)]
    pub rectangle_corner_radii: Option<[f64; 4]>,
    /// Shadows and blurs.
    #[serde(default)]
    pub effects: Vec<Effect>,
    /// Layer opacity.
    #[serde(default)]
    pub opacity: Option<f64>,
    /// True when the frame clips its content.
    #[serde(default)]
    pub clips_content: bool,
    /// The auto-layout mode.
    #[serde(default)]
    pub layout_mode: Option<LayoutMode>,
    /// Wrap of auto layout.
    #[serde(default)]
    pub layout_wrap: Option<String>,
    /// Alignment on the main axis.
    #[serde(default)]
    pub primary_axis_align_items: Option<String>,
    /// Alignment on the cross axis.
    #[serde(default)]
    pub counter_axis_align_items: Option<String>,
    /// Sizing of the main axis: `FIXED` or `AUTO`.
    #[serde(default)]
    pub primary_axis_sizing_mode: Option<String>,
    /// Sizing of the cross axis: `FIXED` or `AUTO`.
    #[serde(default)]
    pub counter_axis_sizing_mode: Option<String>,
    /// Gap between items.
    #[serde(default)]
    pub item_spacing: Option<f64>,
    /// Gap between wrapped rows.
    #[serde(default)]
    pub counter_axis_spacing: Option<f64>,
    /// Left padding.
    #[serde(default)]
    pub padding_left: Option<f64>,
    /// Right padding.
    #[serde(default)]
    pub padding_right: Option<f64>,
    /// Top padding.
    #[serde(default)]
    pub padding_top: Option<f64>,
    /// Bottom padding.
    #[serde(default)]
    pub padding_bottom: Option<f64>,
    /// Horizontal sizing in the parent.
    #[serde(default)]
    pub layout_sizing_horizontal: Option<Sizing>,
    /// Vertical sizing in the parent.
    #[serde(default)]
    pub layout_sizing_vertical: Option<Sizing>,
    /// `ABSOLUTE` when the child ignores auto layout.
    #[serde(default)]
    pub layout_positioning: Option<String>,
    /// 1 when the child fills the main axis (older files).
    #[serde(default)]
    pub layout_grow: Option<f64>,
    /// `STRETCH` when the child fills the cross axis (older files).
    #[serde(default)]
    pub layout_align: Option<String>,
    /// Grid column count.
    #[serde(default)]
    pub grid_column_count: Option<u32>,
    /// Grid row gap.
    #[serde(default)]
    pub grid_row_gap: Option<f64>,
    /// Grid column gap.
    #[serde(default)]
    pub grid_column_gap: Option<f64>,
    /// Text content.
    #[serde(default)]
    pub characters: Option<String>,
    /// Text style.
    #[serde(default)]
    pub style: Option<TypeStyle>,
    /// How text resizes: `WIDTH_AND_HEIGHT`, `HEIGHT`, `NONE`, `TRUNCATE`.
    #[serde(default)]
    pub text_auto_resize: Option<String>,
    /// Style ids by kind (`fill`, `text`, `effect`, `stroke`).
    #[serde(default)]
    pub styles: BTreeMap<String, String>,
    /// Bound variables by field.
    #[serde(default)]
    pub bound_variables: BTreeMap<String, Value>,
    /// The main component, for an instance.
    #[serde(default)]
    pub component_id: Option<String>,
    /// Property values, for an instance.
    #[serde(default)]
    pub component_properties: BTreeMap<String, ComponentProperty>,
    /// Property definitions, for a component or set.
    #[serde(default)]
    pub component_property_definitions: BTreeMap<String, PropertyDefinition>,
    /// Property references by field (`characters`, `visible`, `mainComponent`).
    #[serde(default)]
    pub component_property_references: BTreeMap<String, String>,
}

const fn yes() -> bool {
    true
}

impl Node {
    /// The variable id bound to `field`, if any.
    ///
    /// Figma gives one alias for scalar fields and a list for paint
    /// fields. This returns the first alias in both cases.
    #[must_use]
    pub fn bound_variable(&self, field: &str) -> Option<&str> {
        alias_id(self.bound_variables.get(field)?)
    }

    /// The top visible paint of a paint list.
    #[must_use]
    pub fn top_paint(paints: &[Paint]) -> Option<&Paint> {
        paints.iter().rev().find(|p| p.visible)
    }
}

/// The id in a `VARIABLE_ALIAS` value, or the first one in a list.
#[must_use]
pub fn alias_id(value: &Value) -> Option<&str> {
    match value {
        Value::Object(map) => {
            if map.get("type").and_then(Value::as_str) == Some("VARIABLE_ALIAS") {
                map.get("id").and_then(Value::as_str)
            } else {
                None
            }
        }
        Value::Array(items) => items.iter().find_map(alias_id),
        _ => None,
    }
}

/// A rectangle in canvas coordinates.
#[derive(Debug, Clone, Copy, Default, PartialEq, Deserialize)]
pub struct Rect {
    /// Left.
    #[serde(default)]
    pub x: f64,
    /// Top.
    #[serde(default)]
    pub y: f64,
    /// Width.
    #[serde(default)]
    pub width: f64,
    /// Height.
    #[serde(default)]
    pub height: f64,
}

/// The auto-layout mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LayoutMode {
    /// No auto layout.
    None,
    /// A row.
    Horizontal,
    /// A column.
    Vertical,
    /// A grid.
    Grid,
    /// Any other mode.
    #[serde(other)]
    Other,
}

/// The sizing of a node on one axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Sizing {
    /// A fixed size.
    Fixed,
    /// The size of the content.
    Hug,
    /// The size of the parent.
    Fill,
    /// Any other value.
    #[serde(other)]
    Other,
}

/// A fill or a stroke.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Paint {
    /// The paint type.
    #[serde(rename = "type", default = "PaintType::other")]
    pub kind: PaintType,
    /// False when the paint is hidden.
    #[serde(default = "yes")]
    pub visible: bool,
    /// Paint opacity.
    #[serde(default = "one")]
    pub opacity: f64,
    /// The color, for a solid paint.
    #[serde(default)]
    pub color: Option<Color>,
    /// Bound variables by field (`color`).
    #[serde(default)]
    pub bound_variables: BTreeMap<String, Value>,
    /// The image reference, for an image paint.
    #[serde(default)]
    pub image_ref: Option<String>,
}

impl Paint {
    /// The variable id bound to the color, if any.
    #[must_use]
    pub fn color_variable(&self) -> Option<&str> {
        alias_id(self.bound_variables.get("color")?)
    }
}

const fn one() -> f64 {
    1.0
}

/// The type of a paint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PaintType {
    /// A solid color.
    Solid,
    /// A linear gradient.
    GradientLinear,
    /// A radial gradient.
    GradientRadial,
    /// An image.
    Image,
    /// Any other type.
    #[serde(other)]
    Other,
}

impl PaintType {
    const fn other() -> Self {
        Self::Other
    }
}

/// An RGBA color. Each channel is in `0..=1`.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct Color {
    /// Red.
    #[serde(default)]
    pub r: f64,
    /// Green.
    #[serde(default)]
    pub g: f64,
    /// Blue.
    #[serde(default)]
    pub b: f64,
    /// Alpha.
    #[serde(default = "one")]
    pub a: f64,
}

/// A shadow or a blur.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Effect {
    /// The effect type.
    #[serde(rename = "type", default = "EffectType::other")]
    pub kind: EffectType,
    /// False when the effect is hidden.
    #[serde(default = "yes")]
    pub visible: bool,
    /// Blur radius.
    #[serde(default)]
    pub radius: f64,
    /// Shadow color.
    #[serde(default)]
    pub color: Option<Color>,
    /// Shadow offset.
    #[serde(default)]
    pub offset: Option<Vector>,
    /// Shadow spread.
    #[serde(default)]
    pub spread: f64,
}

/// The type of an effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EffectType {
    /// An outer shadow.
    DropShadow,
    /// An inner shadow.
    InnerShadow,
    /// A blur of the layer.
    LayerBlur,
    /// A blur of the background.
    BackgroundBlur,
    /// Any other type.
    #[serde(other)]
    Other,
}

impl EffectType {
    const fn other() -> Self {
        Self::Other
    }
}

/// A 2D vector.
#[derive(Debug, Clone, Copy, Default, PartialEq, Deserialize)]
pub struct Vector {
    /// X.
    #[serde(default)]
    pub x: f64,
    /// Y.
    #[serde(default)]
    pub y: f64,
}

/// A text style.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TypeStyle {
    /// Font family.
    #[serde(default)]
    pub font_family: Option<String>,
    /// Font weight.
    #[serde(default)]
    pub font_weight: Option<f64>,
    /// Font size in px.
    #[serde(default)]
    pub font_size: Option<f64>,
    /// Line height in px.
    #[serde(default)]
    pub line_height_px: Option<f64>,
    /// Line height in percent of the font size.
    #[serde(default)]
    pub line_height_percent_font_size: Option<f64>,
    /// `PIXELS`, `FONT_SIZE_%`, or `INTRINSIC_%`.
    #[serde(default)]
    pub line_height_unit: Option<String>,
    /// Letter spacing in px.
    #[serde(default)]
    pub letter_spacing: Option<f64>,
    /// `LEFT`, `RIGHT`, `CENTER`, or `JUSTIFIED`.
    #[serde(default)]
    pub text_align_horizontal: Option<String>,
    /// True for italic text.
    #[serde(default)]
    pub italic: bool,
    /// `UNDERLINE` or `STRIKETHROUGH`.
    #[serde(default)]
    pub text_decoration: Option<String>,
    /// `UPPER`, `LOWER`, or `TITLE`.
    #[serde(default)]
    pub text_case: Option<String>,
}

/// The type of a component property.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PropertyType {
    /// A boolean.
    Boolean,
    /// Text.
    Text,
    /// A swappable instance.
    InstanceSwap,
    /// A variant.
    Variant,
    /// A slot for content.
    Slot,
    /// Any other type.
    #[serde(other)]
    Other,
}

impl PropertyType {
    const fn other() -> Self {
        Self::Other
    }
}

/// A component property definition.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PropertyDefinition {
    /// The property type.
    #[serde(rename = "type", default = "PropertyType::other")]
    pub kind: PropertyType,
    /// The default value.
    #[serde(default)]
    pub default_value: Value,
    /// The options, for a variant.
    #[serde(default)]
    pub variant_options: Vec<String>,
}

/// A property value on an instance.
#[derive(Debug, Clone, Deserialize)]
pub struct ComponentProperty {
    /// The property type.
    #[serde(rename = "type", default = "PropertyType::other")]
    pub kind: PropertyType,
    /// The value.
    #[serde(default)]
    pub value: Value,
}

/// `GET /v1/files/:key/variables/local`.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct VariablesResponse {
    /// The payload.
    #[serde(default)]
    pub meta: VariablesMeta,
}

impl VariablesResponse {
    /// Parse a variables body.
    ///
    /// # Errors
    ///
    /// [`ErrorKind::Json`] when the body is not a variables response.
    pub fn from_json(src: &str) -> Result<Self> {
        serde_json::from_str(src)
            .map_err(|e| Error::new(ErrorKind::Json, format!("Figma variables: {e}")))
    }
}

/// Variables and collections.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VariablesMeta {
    /// Variables by id.
    #[serde(default)]
    pub variables: BTreeMap<String, Variable>,
    /// Collections by id.
    #[serde(default)]
    pub variable_collections: BTreeMap<String, VariableCollection>,
}

/// A variable.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Variable {
    /// The id.
    pub id: String,
    /// The name, for example `color/brand/primary`.
    #[serde(default)]
    pub name: String,
    /// The collection id.
    #[serde(default)]
    pub variable_collection_id: String,
    /// The value type.
    #[serde(default = "VariableType::other")]
    pub resolved_type: VariableType,
    /// Values by mode id.
    #[serde(default)]
    pub values_by_mode: BTreeMap<String, Value>,
    /// Where the variable applies, for example `GAP` or `FONT_WEIGHT`.
    #[serde(default)]
    pub scopes: Vec<String>,
}

/// The value type of a variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VariableType {
    /// A boolean.
    Boolean,
    /// A number.
    Float,
    /// A string.
    String,
    /// A color.
    Color,
    /// Any other type.
    #[serde(other)]
    Other,
}

impl VariableType {
    const fn other() -> Self {
        Self::Other
    }
}

/// A variable collection.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VariableCollection {
    /// The id.
    pub id: String,
    /// The name.
    #[serde(default)]
    pub name: String,
    /// The modes.
    #[serde(default)]
    pub modes: Vec<Mode>,
    /// The default mode id.
    #[serde(default)]
    pub default_mode_id: String,
}

/// A mode of a collection.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Mode {
    /// The id.
    pub mode_id: String,
    /// The name, for example `Dark`.
    #[serde(default)]
    pub name: String,
}

/// `GET /v1/images/:key`.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ImagesResponse {
    /// An error message, if any.
    #[serde(default)]
    pub err: Option<String>,
    /// Image URLs by node id. `None` when Figma cannot render the node.
    #[serde(default)]
    pub images: BTreeMap<String, Option<String>>,
}

/// Parse any JSON body and print it with sorted keys and a final newline.
///
/// Snapshots use this form, so a re-pull of the same design gives the same
/// bytes.
///
/// # Errors
///
/// [`ErrorKind::Json`] when the body is not JSON.
pub fn normalize_json(body: &[u8]) -> Result<String> {
    let _ = body;
    Err(Error::new(ErrorKind::Json, "not implemented"))
}

#[cfg(test)]
mod tests;
