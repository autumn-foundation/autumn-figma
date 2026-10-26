#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

fn node(json: &str) -> Node {
    serde_json::from_str(json).unwrap()
}

#[test]
fn vectors_are_svg() {
    let n = node(r#"{"id":"1:31","name":"Vector","type":"VECTOR"}"#);
    assert_eq!(format_of(&n), Some(Format::Svg));
    assert_eq!(
        asset_of(&n).unwrap(),
        Asset {
            node_id: "1:31".into(),
            file_name: "vector--1-31.svg".into(),
            format: Format::Svg,
        }
    );
}

#[test]
fn icon_like_containers_are_svg() {
    let icon = node(
        r#"{"id":"1:30","name":"Icon/Check","type":"COMPONENT","children":[
            {"id":"a","type":"GROUP","children":[{"id":"b","type":"VECTOR"},{"id":"c","type":"ELLIPSE"}]},
            {"id":"d","type":"BOOLEAN_OPERATION"}]}"#,
    );
    assert_eq!(format_of(&icon), Some(Format::Svg));
    assert_eq!(asset_of(&icon).unwrap().file_name, "icon-check--1-30.svg");
}

#[test]
fn containers_with_text_or_only_shapes_are_not_assets() {
    let text = node(
        r#"{"id":"1","type":"FRAME","children":[{"id":"a","type":"VECTOR"},{"id":"b","type":"TEXT"}]}"#,
    );
    assert_eq!(format_of(&text), None);
    let shapes = node(r#"{"id":"1","type":"FRAME","children":[{"id":"a","type":"RECTANGLE"}]}"#);
    assert_eq!(format_of(&shapes), None);
    let empty = node(r#"{"id":"1","type":"FRAME","children":[]}"#);
    assert_eq!(format_of(&empty), None);
    let hidden_vector_only = node(
        r#"{"id":"1","type":"FRAME","children":[{"id":"a","type":"VECTOR","visible":false}]}"#,
    );
    assert_eq!(format_of(&hidden_vector_only), None);
}

#[test]
fn image_fills_are_png() {
    let photo = node(
        r#"{"id":"2:11","name":"Photo","type":"RECTANGLE","fills":[{"type":"IMAGE","imageRef":"x"}]}"#,
    );
    assert_eq!(format_of(&photo), Some(Format::Png));
    assert_eq!(asset_of(&photo).unwrap().file_name, "photo--2-11.png");
    let hidden =
        node(r#"{"id":"1","type":"RECTANGLE","fills":[{"type":"IMAGE","visible":false}]}"#);
    assert_eq!(format_of(&hidden), None);
    let container = node(
        r#"{"id":"1","type":"FRAME","fills":[{"type":"IMAGE"}],"children":[{"id":"t","type":"TEXT"}]}"#,
    );
    assert_eq!(
        format_of(&container),
        None,
        "a container keeps its children"
    );
}

#[test]
fn text_is_never_an_asset() {
    assert_eq!(format_of(&node(r#"{"id":"1","type":"TEXT"}"#)), None);
}

#[test]
fn format_strings() {
    assert_eq!(Format::Svg.as_str(), "svg");
    assert_eq!(Format::Png.as_str(), "png");
}

#[test]
fn file_names_cannot_collide() {
    // Regression: `check-i1-2` + `3:4` and `check` + `I1:2;3:4` gave one name.
    let a = node(r#"{"id":"3:4","name":"check-i1-2","type":"VECTOR"}"#);
    let b = node(r#"{"id":"I1:2;3:4","name":"check","type":"VECTOR"}"#);
    assert_ne!(
        asset_of(&a).unwrap().file_name,
        asset_of(&b).unwrap().file_name
    );
}
