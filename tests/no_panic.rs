//! Random designs never make the generator panic (AC-19).
//!
//! Each case builds a random Figma tree: random node types, names (any
//! text), layout fields, component properties, property references, and
//! instances that point at random ids. Generation must return `Ok` or a
//! typed error. On `Ok`, the output must be the same on a second run.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;

use autumn_figma::config::Config;
use autumn_figma::generate::generate_from;
use autumn_figma::model::FileResponse;
use proptest::prelude::*;
use serde_json::{Value, json};

const TYPES: &[&str] = &[
    "FRAME",
    "GROUP",
    "SECTION",
    "COMPONENT",
    "COMPONENT_SET",
    "INSTANCE",
    "TEXT",
    "RECTANGLE",
    "ELLIPSE",
    "VECTOR",
    "SLOT",
    "EMBED",
];

fn name() -> impl Strategy<Value = String> {
    prop_oneof![
        "[A-Za-z ]{0,12}",
        ".{0,12}",
        Just("Size=Large, State=On".to_owned()),
        Just("button".to_owned()),
        Just("input".to_owned()),
    ]
}

fn leaf() -> impl Strategy<Value = Value> {
    (
        0..TYPES.len(),
        name(),
        0..40_u32,
        any::<bool>(),
        prop::option::of(prop_oneof![
            Just("HORIZONTAL"),
            Just("VERTICAL"),
            Just("GRID"),
            Just("NONE")
        ]),
        prop::option::of(prop_oneof![Just("FIXED"), Just("HUG"), Just("FILL")]),
        prop::option::of(-20.0..200.0_f64),
        prop::option::of(0..40_u32),
        ".{0,8}",
    )
        .prop_map(|(t, name, id, visible, mode, sizing, gap, target, chars)| {
            let mut node = json!({
                "id": format!("{id}:{}", id % 7),
                "name": name,
                "type": TYPES[t],
                "visible": visible,
                "characters": chars,
                "absoluteBoundingBox": {"x": 0, "y": 0, "width": 10, "height": 10},
            });
            let map = node.as_object_mut().unwrap();
            if let Some(mode) = mode {
                map.insert("layoutMode".into(), json!(mode));
            }
            if let Some(sizing) = sizing {
                map.insert("layoutSizingHorizontal".into(), json!(sizing));
            }
            if let Some(gap) = gap {
                map.insert("itemSpacing".into(), json!(gap));
            }
            if let Some(target) = target {
                let key = format!("P#{target}");
                map.insert("componentId".into(), json!(format!("{target}:{}", target % 7)));
                map.insert(
                    "componentPropertyReferences".into(),
                    json!({"characters": key, "visible": key, "mainComponent": key}),
                );
                map.insert(
                    "componentPropertyDefinitions".into(),
                    json!({
                        key.clone(): {"type": "TEXT", "defaultValue": "x"},
                        "Size": {"type": "VARIANT", "defaultValue": "Large", "variantOptions": ["Large", "Small"]},
                        "B#1": {"type": "BOOLEAN", "defaultValue": true},
                        "S#2": {"type": "SLOT"}
                    }),
                );
                map.insert(
                    "componentProperties".into(),
                    json!({key: {"type": "TEXT", "value": "y"}, "Size": {"type": "VARIANT", "value": "Small"}}),
                );
            }
            node
        })
}

fn tree() -> impl Strategy<Value = Value> {
    leaf().prop_recursive(4, 40, 5, |inner| {
        (leaf(), prop::collection::vec(inner, 0..5)).prop_map(|(mut node, children)| {
            node["children"] = Value::Array(children);
            node
        })
    })
}

fn file(pages: Vec<Value>) -> FileResponse {
    let pages: Vec<Value> = pages
        .into_iter()
        .enumerate()
        .map(|(i, children)| {
            json!({"id": format!("p{i}"), "name": format!("Page {i}"), "type": "CANVAS", "children": [children]})
        })
        .collect();
    let doc = json!({"name": "Random", "version": "1",
        "document": {"id": "0:0", "type": "DOCUMENT", "children": pages}});
    FileResponse::from_json(&doc.to_string()).unwrap()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(300))]

    #[test]
    fn random_designs_never_panic(pages in prop::collection::vec(tree(), 1..3)) {
        let config = Config::from_toml_str("file_key = \"R1\"", Path::new("/app")).unwrap();
        let file = file(pages);
        let first = generate_from(&config, &file, None);
        let second = generate_from(&config, &file, None);
        match (first, second) {
            (Ok(a), Ok(b)) => {
                prop_assert_eq!(&a.files, &b.files, "deterministic");
                for (path, text) in &a.files {
                    prop_assert!(!text.contains("PreEscaped"), "{}", path.display());
                    prop_assert!(!text.contains('\u{202e}'), "{}", path.display());
                }
            }
            (Err(a), Err(b)) => prop_assert_eq!(a.to_string(), b.to_string()),
            _ => prop_assert!(false, "the two runs disagree"),
        }
    }
}
