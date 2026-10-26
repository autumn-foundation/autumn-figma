#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

#[test]
fn parses_a_minimal_file_and_ignores_unknown_fields() {
    let file = FileResponse::from_json(
        r#"{"name":"Demo","version":"7","newField":1,
            "document":{"id":"0:0","type":"DOCUMENT","children":[
              {"id":"1:0","type":"CANVAS","name":"Page","children":[
                {"id":"1:1","type":"SOME_FUTURE_NODE","name":"x"}]}]}}"#,
    )
    .unwrap();
    assert_eq!(file.name, "Demo");
    assert_eq!(file.version.as_deref(), Some("7"));
    let page = &file.document.children[0];
    assert_eq!(page.kind, NodeType::Canvas);
    assert_eq!(page.children[0].kind, NodeType::Other);
    assert!(page.children[0].visible);
}

#[test]
fn bad_json_is_an_error_not_a_panic() {
    let err = FileResponse::from_json("{").unwrap_err();
    assert_eq!(err.kind(), ErrorKind::Json);
    let err = FileResponse::from_json(r#"{"name":"no document"}"#).unwrap_err();
    assert_eq!(err.kind(), ErrorKind::Json);
    let err = VariablesResponse::from_json("[]").unwrap_err();
    assert_eq!(err.kind(), ErrorKind::Json);
}

#[test]
fn bound_variable_reads_scalar_and_list_aliases() {
    let file = FileResponse::from_json(
        r#"{"document":{"id":"0:0","type":"DOCUMENT","boundVariables":{
            "itemSpacing":{"type":"VARIABLE_ALIAS","id":"VariableID:1:4"},
            "fills":[{"type":"VARIABLE_ALIAS","id":"VariableID:1:1"}],
            "bad":{"type":"OTHER","id":"x"}}}}"#,
    )
    .unwrap();
    let node = &file.document;
    assert_eq!(node.bound_variable("itemSpacing"), Some("VariableID:1:4"));
    assert_eq!(node.bound_variable("fills"), Some("VariableID:1:1"));
    assert_eq!(node.bound_variable("bad"), None);
    assert_eq!(node.bound_variable("missing"), None);
}

#[test]
fn top_paint_is_the_last_visible_one() {
    let paints: Vec<Paint> = serde_json::from_str(
        r#"[{"type":"SOLID","color":{"r":1,"g":0,"b":0}},
            {"type":"SOLID","color":{"r":0,"g":1,"b":0}},
            {"type":"SOLID","visible":false,"color":{"r":0,"g":0,"b":1}}]"#,
    )
    .unwrap();
    let top = Node::top_paint(&paints).unwrap();
    assert!((top.color.unwrap().g - 1.0).abs() < f64::EPSILON);
    assert!((top.opacity - 1.0).abs() < f64::EPSILON);
    assert!(Node::top_paint(&[]).is_none());
}

#[test]
fn paint_color_variable() {
    let paint: Paint = serde_json::from_str(
        r#"{"type":"SOLID","color":{"r":0,"g":0,"b":0,"a":0.5},
            "boundVariables":{"color":{"type":"VARIABLE_ALIAS","id":"VariableID:9"}}}"#,
    )
    .unwrap();
    assert_eq!(paint.color_variable(), Some("VariableID:9"));
    assert!((paint.color.unwrap().a - 0.5).abs() < f64::EPSILON);
}

#[test]
fn node_type_classes() {
    assert!(NodeType::Vector.is_vector());
    assert!(NodeType::BooleanOperation.is_vector());
    assert!(!NodeType::Rectangle.is_vector());
    assert!(NodeType::Instance.is_container());
    assert!(!NodeType::Text.is_container());
}

#[test]
fn parses_variables() {
    let vars = VariablesResponse::from_json(
        r#"{"status":200,"error":false,"meta":{
          "variables":{"VariableID:1":{"id":"VariableID:1","name":"color/a",
            "variableCollectionId":"C:1","resolvedType":"COLOR",
            "valuesByMode":{"1:0":{"r":1,"g":1,"b":1,"a":1}},"scopes":["ALL_FILLS"]}},
          "variableCollections":{"C:1":{"id":"C:1","name":"Theme",
            "modes":[{"modeId":"1:0","name":"Light"}],"defaultModeId":"1:0"}}}}"#,
    )
    .unwrap();
    let var = &vars.meta.variables["VariableID:1"];
    assert_eq!(var.resolved_type, VariableType::Color);
    assert_eq!(vars.meta.variable_collections["C:1"].modes[0].name, "Light");
}

#[test]
fn normalize_json_sorts_keys_and_ends_with_newline() {
    let a = normalize_json(br#"{"b":1,"a":{"d":[3,2],"c":null}}"#).unwrap();
    let b = normalize_json(br#"{ "a" : { "c" : null, "d" : [3, 2] }, "b" : 1 }"#).unwrap();
    assert_eq!(a, b);
    assert!(a.ends_with('\n'));
    assert!(a.find("\"a\"").unwrap() < a.find("\"b\"").unwrap());
    assert!(a.contains("[\n"), "pretty printed: {a}");
    assert_eq!(normalize_json(b"nope").unwrap_err().kind(), ErrorKind::Json);
}
