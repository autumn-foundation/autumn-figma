#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use crate::diagnostics::Level;
use crate::model::VariablesResponse;

fn node(json: &str) -> Node {
    serde_json::from_str(json).unwrap()
}

struct Run {
    decls: Decls,
    diags: Vec<Diagnostic>,
    tokens: TokenSet,
}

fn run_with(json: &str, parent: Parent, role: Role, tokens: TokenSet) -> Run {
    let n = node(json);
    let styles: BTreeMap<String, StyleMeta> = serde_json::from_str(
        r#"{"S:text":{"name":"Body/Strong","styleType":"TEXT"},
            "S:fill":{"name":"Brand/Accent","styleType":"FILL"},
            "S:fx":{"name":"Shadow/Card","styleType":"EFFECT"}}"#,
    )
    .unwrap();
    let mut tokens = tokens;
    let mut diags = Vec::new();
    let mut cx = StyleCx {
        tokens: &mut tokens,
        styles: &styles,
        diagnostics: &mut diags,
        path: "Page > Node",
    };
    let decls = node_decls(&n, parent, role, &mut cx);
    Run {
        decls,
        diags,
        tokens,
    }
}

fn run(json: &str, parent: Parent, role: Role) -> Run {
    run_with(json, parent, role, TokenSet::default())
}

fn decls(json: &str, parent: Parent) -> Decls {
    run(json, parent, Role::Box).decls
}

const FLEX_ROW: Parent = Parent::Flex { row: true };
const FLEX_COL: Parent = Parent::Flex { row: false };
const COMPONENT: Parent = Parent::Root { screen: false };

#[test]
fn horizontal_auto_layout_is_a_flex_row() {
    let d = decls(
        r#"{"id":"1","type":"FRAME","layoutMode":"HORIZONTAL","itemSpacing":8,
            "paddingLeft":20,"paddingRight":20,"paddingTop":12,"paddingBottom":12,
            "primaryAxisAlignItems":"CENTER","counterAxisAlignItems":"CENTER",
            "layoutSizingHorizontal":"HUG","layoutSizingVertical":"HUG"}"#,
        COMPONENT,
    );
    assert_eq!(d.get("display"), Some("flex"));
    assert_eq!(d.get("flex-direction"), Some("row"));
    assert_eq!(d.get("gap"), Some("8px"));
    assert_eq!(d.get("padding"), Some("12px 20px"));
    assert_eq!(d.get("justify-content"), Some("center"));
    assert_eq!(d.get("align-items"), Some("center"));
    assert_eq!(d.get("width"), None, "hug has no width");
}

#[test]
fn vertical_wrap_and_alignment() {
    let d = decls(
        r#"{"id":"1","type":"FRAME","layoutMode":"VERTICAL","itemSpacing":4,
            "counterAxisSpacing":10,"layoutWrap":"WRAP",
            "paddingLeft":1,"paddingRight":2,"paddingTop":3,"paddingBottom":4,
            "primaryAxisAlignItems":"SPACE_BETWEEN","counterAxisAlignItems":"MAX"}"#,
        COMPONENT,
    );
    assert_eq!(d.get("flex-direction"), Some("column"));
    assert_eq!(d.get("flex-wrap"), Some("wrap"));
    assert_eq!(d.get("gap"), Some("4px 10px"));
    assert_eq!(d.get("padding"), Some("3px 2px 4px 1px"));
    assert_eq!(d.get("justify-content"), Some("space-between"));
    assert_eq!(d.get("align-items"), Some("flex-end"));
}

#[test]
fn row_wrap_gap_order() {
    let d = decls(
        r#"{"id":"1","type":"FRAME","layoutMode":"HORIZONTAL","itemSpacing":4,
            "counterAxisSpacing":10,"layoutWrap":"WRAP"}"#,
        COMPONENT,
    );
    assert_eq!(d.get("gap"), Some("10px 4px"));
}

#[test]
fn grid_layout() {
    let d = decls(
        r#"{"id":"1","type":"FRAME","layoutMode":"GRID","gridColumnCount":3,
            "gridRowGap":16,"gridColumnGap":8}"#,
        COMPONENT,
    );
    assert_eq!(d.get("display"), Some("grid"));
    assert_eq!(
        d.get("grid-template-columns"),
        Some("repeat(3, minmax(0, 1fr))")
    );
    assert_eq!(d.get("gap"), Some("16px 8px"));
}

#[test]
fn bound_spacing_uses_tokens() {
    let vars = VariablesResponse::from_json(
        r#"{"meta":{"variableCollections":{"C":{"id":"C","name":"T","defaultModeId":"m","modes":[{"modeId":"m","name":"V"}]}},
            "variables":{"V:gap":{"id":"V:gap","name":"space/sm","variableCollectionId":"C","resolvedType":"FLOAT","valuesByMode":{"m":8}},
                         "V:pad":{"id":"V:pad","name":"space/md","variableCollectionId":"C","resolvedType":"FLOAT","valuesByMode":{"m":16}},
                         "V:r":{"id":"V:r","name":"radius/md","variableCollectionId":"C","resolvedType":"FLOAT","valuesByMode":{"m":6}},
                         "V:c":{"id":"V:c","name":"brand","variableCollectionId":"C","resolvedType":"COLOR","valuesByMode":{"m":{"r":1,"g":0,"b":0,"a":1}}}}}}"#,
    )
    .unwrap();
    let tokens = TokenSet::new(Some(&vars), &BTreeMap::new(), &mut Vec::new());
    let r = run_with(
        r#"{"id":"1","type":"FRAME","layoutMode":"HORIZONTAL","itemSpacing":8,
            "paddingLeft":16,"paddingRight":16,"paddingTop":16,"paddingBottom":16,"cornerRadius":6,
            "fills":[{"type":"SOLID","color":{"r":1,"g":0,"b":0},"boundVariables":{"color":{"type":"VARIABLE_ALIAS","id":"V:c"}}}],
            "boundVariables":{"itemSpacing":{"type":"VARIABLE_ALIAS","id":"V:gap"},
              "paddingLeft":{"type":"VARIABLE_ALIAS","id":"V:pad"},"paddingRight":{"type":"VARIABLE_ALIAS","id":"V:pad"},
              "paddingTop":{"type":"VARIABLE_ALIAS","id":"V:pad"},"paddingBottom":{"type":"VARIABLE_ALIAS","id":"V:pad"},
              "topLeftRadius":{"type":"VARIABLE_ALIAS","id":"V:r"},"topRightRadius":{"type":"VARIABLE_ALIAS","id":"V:r"},
              "bottomLeftRadius":{"type":"VARIABLE_ALIAS","id":"V:r"},"bottomRightRadius":{"type":"VARIABLE_ALIAS","id":"V:r"}}}"#,
        COMPONENT,
        Role::Box,
        tokens,
    );
    assert_eq!(r.decls.get("gap"), Some("var(--space-sm)"));
    assert_eq!(r.decls.get("padding"), Some("var(--space-md)"));
    assert_eq!(r.decls.get("border-radius"), Some("var(--radius-md)"));
    assert_eq!(r.decls.get("background-color"), Some("var(--brand)"));
}

#[test]
fn child_sizing_in_a_row() {
    let fill = decls(
        r#"{"id":"1","type":"FRAME","layoutSizingHorizontal":"FILL","layoutSizingVertical":"FILL",
            "absoluteBoundingBox":{"x":0,"y":0,"width":100,"height":40}}"#,
        FLEX_ROW,
    );
    assert_eq!(fill.get("flex"), Some("1 1 0"));
    assert_eq!(fill.get("min-width"), Some("0"));
    assert_eq!(fill.get("align-self"), Some("stretch"));
    assert_eq!(fill.get("width"), None);
    let fixed = decls(
        r#"{"id":"1","type":"FRAME","layoutSizingHorizontal":"FIXED","layoutSizingVertical":"FIXED",
            "absoluteBoundingBox":{"x":0,"y":0,"width":100,"height":40}}"#,
        FLEX_ROW,
    );
    assert_eq!(fixed.get("width"), Some("100px"));
    assert_eq!(fixed.get("height"), Some("40px"));
    assert_eq!(fixed.get("flex-shrink"), Some("0"));
}

#[test]
fn child_sizing_in_a_column_and_old_fields() {
    let d = decls(
        r#"{"id":"1","type":"FRAME","layoutSizingHorizontal":"FILL","layoutSizingVertical":"FILL"}"#,
        FLEX_COL,
    );
    assert_eq!(d.get("flex"), Some("1 1 0"));
    assert_eq!(d.get("min-height"), Some("0"));
    assert_eq!(d.get("align-self"), Some("stretch"));
    let old = decls(
        r#"{"id":"1","type":"FRAME","layoutGrow":1,"layoutAlign":"STRETCH",
            "absoluteBoundingBox":{"x":0,"y":0,"width":10,"height":10}}"#,
        FLEX_COL,
    );
    assert_eq!(old.get("flex"), Some("1 1 0"));
    assert_eq!(old.get("align-self"), Some("stretch"));
}

#[test]
fn absolute_children_and_parents() {
    let child = decls(
        r#"{"id":"1","type":"RECTANGLE","absoluteBoundingBox":{"x":110,"y":220,"width":30,"height":40}}"#,
        Parent::Absolute {
            origin: Rect {
                x: 100.0,
                y: 200.0,
                width: 300.0,
                height: 300.0,
            },
        },
    );
    assert_eq!(child.get("position"), Some("absolute"));
    assert_eq!(child.get("left"), Some("10px"));
    assert_eq!(child.get("top"), Some("20px"));
    assert_eq!(child.get("width"), Some("30px"));
    assert_eq!(child.get("height"), Some("40px"));

    let r = run(
        r#"{"id":"1","type":"FRAME","absoluteBoundingBox":{"x":0,"y":0,"width":300,"height":200},
            "children":[{"id":"2","type":"RECTANGLE"}]}"#,
        FLEX_ROW,
        Role::Box,
    );
    assert_eq!(r.decls.get("position"), Some("relative"));
    assert_eq!(r.decls.get("width"), Some("300px"));
    assert_eq!(r.decls.get("height"), Some("200px"));
    assert!(
        r.diags
            .iter()
            .any(|d| d.level == Level::Warning && d.message.contains("auto layout")),
        "{:?}",
        r.diags
    );
}

#[test]
fn absolute_child_in_auto_layout() {
    let parent = decls(
        r#"{"id":"1","type":"FRAME","layoutMode":"VERTICAL",
            "children":[{"id":"2","type":"RECTANGLE","layoutPositioning":"ABSOLUTE"}]}"#,
        COMPONENT,
    );
    assert_eq!(parent.get("position"), Some("relative"));
}

#[test]
fn roots() {
    let screen = decls(
        r#"{"id":"1","type":"FRAME","layoutMode":"VERTICAL",
            "absoluteBoundingBox":{"x":0,"y":0,"width":1440,"height":900}}"#,
        Parent::Root { screen: true },
    );
    assert_eq!(screen.get("width"), Some("100%"));
    assert_eq!(screen.get("max-width"), Some("1440px"));
    assert_eq!(screen.get("height"), None);
    let comp = decls(
        r#"{"id":"1","type":"COMPONENT","layoutMode":"HORIZONTAL","layoutSizingHorizontal":"FIXED",
            "layoutSizingVertical":"HUG","absoluteBoundingBox":{"x":0,"y":0,"width":360,"height":48}}"#,
        COMPONENT,
    );
    assert_eq!(comp.get("width"), Some("360px"));
    assert_eq!(comp.get("height"), None);
}

#[test]
fn fills_strokes_radius() {
    let d = decls(
        r#"{"id":"1","type":"RECTANGLE","fills":[{"type":"SOLID","color":{"r":1,"g":1,"b":1},"opacity":0.5}],
            "strokes":[{"type":"SOLID","color":{"r":0,"g":0,"b":0}}],"strokeWeight":2,"strokeDashes":[4,2],
            "rectangleCornerRadii":[4,4,0,0],"opacity":0.8,"clipsContent":true}"#,
        COMPONENT,
    );
    assert_eq!(d.get("background-color"), Some("rgba(255, 255, 255, 0.5)"));
    assert_eq!(d.get("border"), Some("2px dashed #000000"));
    assert_eq!(d.get("border-radius"), Some("4px 4px 0 0"));
    assert_eq!(d.get("opacity"), Some("0.8"));
    assert_eq!(d.get("overflow"), Some("hidden"));
    let ellipse = decls(r#"{"id":"1","type":"ELLIPSE"}"#, COMPONENT);
    assert_eq!(ellipse.get("border-radius"), Some("50%"));
}

#[test]
fn fill_style_becomes_a_token() {
    let r = run(
        r#"{"id":"1","type":"RECTANGLE","styles":{"fill":"S:fill"},
            "fills":[{"type":"SOLID","color":{"r":0,"g":0,"b":1}}]}"#,
        COMPONENT,
        Role::Box,
    );
    assert_eq!(r.decls.get("background-color"), Some("var(--brand-accent)"));
    assert!(r.tokens.css().contains("--brand-accent: #0000ff;"));
}

#[test]
fn gradients_warn() {
    let r = run(
        r#"{"id":"1","type":"RECTANGLE","fills":[{"type":"GRADIENT_LINEAR"}]}"#,
        COMPONENT,
        Role::Box,
    );
    assert_eq!(r.decls.get("background-color"), None);
    assert!(
        r.diags.iter().any(|d| d.message.contains("GradientLinear")),
        "{:?}",
        r.diags
    );
}

#[test]
fn effects() {
    let d = decls(
        r#"{"id":"1","type":"FRAME","effects":[
            {"type":"DROP_SHADOW","color":{"r":0,"g":0,"b":0,"a":0.25},"offset":{"x":0,"y":4},"radius":8,"spread":0},
            {"type":"INNER_SHADOW","color":{"r":0,"g":0,"b":0,"a":1},"offset":{"x":1,"y":1},"radius":0,"spread":1},
            {"type":"DROP_SHADOW","visible":false,"radius":99},
            {"type":"LAYER_BLUR","radius":8},{"type":"BACKGROUND_BLUR","radius":4}]}"#,
        COMPONENT,
    );
    assert_eq!(
        d.get("box-shadow"),
        Some("0 4px 8px 0 rgba(0, 0, 0, 0.25), inset 1px 1px 0 1px #000000")
    );
    assert_eq!(d.get("filter"), Some("blur(4px)"));
    assert_eq!(d.get("backdrop-filter"), Some("blur(2px)"));
}

#[test]
fn effect_style_becomes_a_token() {
    let r = run(
        r#"{"id":"1","type":"FRAME","styles":{"effect":"S:fx"},"effects":[
            {"type":"DROP_SHADOW","color":{"r":0,"g":0,"b":0,"a":1},"offset":{"x":0,"y":1},"radius":2}]}"#,
        COMPONENT,
        Role::Box,
    );
    assert_eq!(r.decls.get("box-shadow"), Some("var(--shadow-card)"));
}

#[test]
fn text() {
    let r = run(
        r#"{"id":"1","type":"TEXT","characters":"Hi","textAutoResize":"WIDTH_AND_HEIGHT",
            "fills":[{"type":"SOLID","color":{"r":1,"g":0,"b":0}}],
            "effects":[{"type":"DROP_SHADOW","color":{"r":0,"g":0,"b":0,"a":1},"offset":{"x":0,"y":1},"radius":2}],
            "style":{"fontFamily":"Inter","fontWeight":600,"fontSize":16,"lineHeightPx":24,"lineHeightUnit":"PIXELS",
              "letterSpacing":0.5,"textAlignHorizontal":"CENTER","italic":true,
              "textDecoration":"STRIKETHROUGH","textCase":"UPPER"}}"#,
        FLEX_ROW,
        Role::Text,
    );
    let d = r.decls;
    assert_eq!(d.get("color"), Some("#ff0000"));
    assert_eq!(d.get("background-color"), None);
    assert_eq!(
        d.get("font-family"),
        Some("\"Inter\", system-ui, sans-serif")
    );
    assert_eq!(d.get("font-weight"), Some("600"));
    assert_eq!(d.get("font-size"), Some("16px"));
    assert_eq!(d.get("line-height"), Some("24px"));
    assert_eq!(d.get("letter-spacing"), Some("0.5px"));
    assert_eq!(d.get("text-align"), Some("center"));
    assert_eq!(d.get("font-style"), Some("italic"));
    assert_eq!(d.get("text-decoration"), Some("line-through"));
    assert_eq!(d.get("text-transform"), Some("uppercase"));
    assert_eq!(d.get("white-space"), Some("nowrap"));
    assert_eq!(d.get("text-shadow"), Some("0 1px 2px #000000"));
    assert_eq!(d.get("box-shadow"), None);
}

#[test]
fn text_style_token_and_percent_line_height() {
    let r = run(
        r#"{"id":"1","type":"TEXT","styles":{"text":"S:text"},
            "style":{"fontFamily":"Inter","fontWeight":700,"fontSize":20,
              "lineHeightPercentFontSize":150,"lineHeightUnit":"FONT_SIZE_%"}}"#,
        COMPONENT,
        Role::Text,
    );
    assert_eq!(r.decls.get("font"), Some("var(--body-strong)"));
    assert_eq!(r.decls.get("font-size"), None);
    assert!(
        r.tokens
            .css()
            .contains("--body-strong: 700 20px/1.5 \"Inter\", system-ui, sans-serif;"),
        "{}",
        r.tokens.css()
    );
}

#[test]
fn images_are_blocks() {
    let d = run(
        r#"{"id":"1","type":"VECTOR","layoutSizingHorizontal":"FIXED","layoutSizingVertical":"FIXED",
            "fills":[{"type":"SOLID","color":{"r":0,"g":0,"b":0}}],
            "absoluteBoundingBox":{"x":0,"y":0,"width":16,"height":16}}"#,
        FLEX_ROW,
        Role::Image,
    )
    .decls;
    assert_eq!(d.get("display"), Some("block"));
    assert_eq!(d.get("width"), Some("16px"));
    assert_eq!(d.get("background-color"), None, "the SVG has the fill");
}
