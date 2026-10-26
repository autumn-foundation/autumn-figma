#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

const VARS: &str = r#"{"meta":{
  "variableCollections":{
    "C:1":{"id":"C:1","name":"Theme","defaultModeId":"m:light",
      "modes":[{"modeId":"m:light","name":"Light"},{"modeId":"m:dark","name":"Dark"},
               {"modeId":"m:hc","name":"High Contrast"}]},
    "C:2":{"id":"C:2","name":"Base","defaultModeId":"m:base","modes":[{"modeId":"m:base","name":"Value"}]}},
  "variables":{
    "V:1":{"id":"V:1","name":"color/brand/primary","variableCollectionId":"C:1","resolvedType":"COLOR",
      "valuesByMode":{"m:light":{"r":0.486,"g":0.227,"b":0.929,"a":1},
                      "m:dark":{"r":0.655,"g":0.545,"b":0.98,"a":1},
                      "m:hc":{"r":0,"g":0,"b":0,"a":1}}},
    "V:2":{"id":"V:2","name":"color/surface","variableCollectionId":"C:1","resolvedType":"COLOR",
      "valuesByMode":{"m:light":{"r":1,"g":1,"b":1,"a":1},
                      "m:dark":{"r":0,"g":0,"b":0,"a":0.5},
                      "m:hc":{"r":1,"g":1,"b":1,"a":1}}},
    "V:3":{"id":"V:3","name":"color/on-primary","variableCollectionId":"C:1","resolvedType":"COLOR",
      "valuesByMode":{"m:light":{"type":"VARIABLE_ALIAS","id":"V:2"},
                      "m:dark":{"type":"VARIABLE_ALIAS","id":"V:remote"},
                      "m:hc":{"r":1,"g":1,"b":1,"a":1}}},
    "V:4":{"id":"V:4","name":"spacing/sm","variableCollectionId":"C:2","resolvedType":"FLOAT",
      "valuesByMode":{"m:base":8},"scopes":["GAP"]},
    "V:5":{"id":"V:5","name":"font/weight/bold","variableCollectionId":"C:2","resolvedType":"FLOAT",
      "valuesByMode":{"m:base":700},"scopes":["FONT_WEIGHT"]},
    "V:6":{"id":"V:6","name":"font/family/sans","variableCollectionId":"C:2","resolvedType":"STRING",
      "valuesByMode":{"m:base":"Inter"}},
    "V:7":{"id":"V:7","name":"flag/beta","variableCollectionId":"C:2","resolvedType":"BOOLEAN",
      "valuesByMode":{"m:base":true}},
    "V:8":{"id":"V:8","name":"Spacing/SM","variableCollectionId":"C:1","resolvedType":"FLOAT",
      "valuesByMode":{"m:light":4,"m:dark":4,"m:hc":4}}
  }}}"#;

fn set(renames: &[(&str, &str)]) -> (TokenSet, Vec<Diagnostic>) {
    let vars = VariablesResponse::from_json(VARS).unwrap();
    let renames = renames
        .iter()
        .map(|(a, b)| ((*a).to_owned(), (*b).to_owned()))
        .collect();
    let mut diags = Vec::new();
    let set = TokenSet::new(Some(&vars), &renames, &mut diags);
    (set, diags)
}

#[test]
fn colors() {
    let c = |r, g, b, a| Color { r, g, b, a };
    assert_eq!(css_color(&c(1.0, 1.0, 1.0, 1.0), 1.0), "#ffffff");
    assert_eq!(css_color(&c(0.486, 0.227, 0.929, 1.0), 1.0), "#7c3aed");
    assert_eq!(css_color(&c(0.0, 0.0, 0.0, 0.5), 1.0), "rgba(0, 0, 0, 0.5)");
    assert_eq!(
        css_color(&c(0.0, 0.0, 0.0, 1.0), 0.25),
        "rgba(0, 0, 0, 0.25)"
    );
    assert_eq!(css_color(&c(2.0, -1.0, 0.0, 1.0), 1.0), "#ff0000");
}

#[test]
fn variable_values_and_references() {
    let (set, _) = set(&[]);
    assert_eq!(
        set.variable("V:1").as_deref(),
        Some("var(--color-brand-primary)")
    );
    assert_eq!(set.variable("V:7"), None, "booleans have no CSS value");
    assert_eq!(set.variable("V:nope"), None);
    let css = set.css();
    assert!(css.contains("  --color-brand-primary: #7c3aed;\n"), "{css}");
    assert!(css.contains("  --color-surface: #ffffff;\n"), "{css}");
    assert!(
        css.contains("  --color-on-primary: var(--color-surface);\n"),
        "{css}"
    );
    assert!(css.contains("  --spacing-sm: 8px;\n"), "{css}");
    assert!(css.contains("  --font-weight-bold: 700;\n"), "{css}");
    assert!(css.contains("  --font-family-sans: \"Inter\";\n"), "{css}");
    assert!(!css.contains("flag"), "{css}");
}

#[test]
fn name_collisions_get_a_suffix() {
    let (set, _) = set(&[]);
    // Collections sort by name, so "Base" claims the plain name first.
    assert_eq!(set.variable("V:4").as_deref(), Some("var(--spacing-sm)"));
    assert_eq!(set.variable("V:8").as_deref(), Some("var(--spacing-sm-2)"));
}

#[test]
fn modes() {
    let (set, diags) = set(&[]);
    let css = set.css();
    let dark = css.find("[data-theme=\"dark\"] {").expect(&css);
    let hc = css.find("[data-theme=\"high-contrast\"] {").expect(&css);
    assert!(hc > dark, "modes sort by slug");
    assert!(
        css.contains("  --color-surface: rgba(0, 0, 0, 0.5);\n"),
        "{css}"
    );
    assert!(
        css.contains("@media (prefers-color-scheme: dark) {\n  :root:not([data-theme]) {\n    --color-brand-primary: #a78bfa;"),
        "{css}"
    );
    assert_eq!(
        css.matches("@media").count(),
        1,
        "only the dark mode: {css}"
    );
    assert!(
        diags.iter().any(|d| d.message.contains("V:remote")),
        "unknown alias gives a diagnostic: {diags:?}"
    );
    assert!(!css.contains("V:remote"));
}

#[test]
fn renames_win_and_reserve_the_name() {
    let (set, _) = set(&[("color/brand/primary", "--primary")]);
    assert_eq!(set.variable("V:1").as_deref(), Some("var(--primary)"));
    assert!(set.css().contains("  --primary: #7c3aed;\n"));
}

#[test]
fn styles_take_their_first_value() {
    let mut set = TokenSet::new(None, &BTreeMap::new(), &mut Vec::new());
    assert!(set.is_empty());
    assert_eq!(set.css(), "");
    let first = set.style("S:1", "Heading/H1", || {
        Some("700 32px/40px \"Inter\"".to_owned())
    });
    assert_eq!(first.as_deref(), Some("var(--heading-h1)"));
    let again = set.style("S:1", "Heading/H1", || Some("other".to_owned()));
    assert_eq!(again, first);
    assert_eq!(set.style("S:2", "Empty", || None), None);
    let css = set.css();
    assert!(
        css.contains("  --heading-h1: 700 32px/40px \"Inter\";\n"),
        "{css}"
    );
    assert!(!css.contains("other"));
    assert!(!set.is_empty());
}

#[test]
fn style_renames_apply() {
    let renames = BTreeMap::from([("Shadow/Card".to_owned(), "--shadow".to_owned())]);
    let mut set = TokenSet::new(None, &renames, &mut Vec::new());
    let v = set.style("S:9", "Shadow/Card", || Some("0 1px 2px #000".to_owned()));
    assert_eq!(v.as_deref(), Some("var(--shadow)"));
}

#[test]
fn output_is_stable() {
    assert_eq!(set(&[]).0.css(), set(&[]).0.css());
}

#[test]
fn explicit_modes_map_to_data_theme() {
    let (set, _) = set(&[]);
    let modes = BTreeMap::from([("C:1".to_owned(), "m:dark".to_owned())]);
    assert_eq!(set.mode_attr(&modes).as_deref(), Some("dark"));
    let light = BTreeMap::from([("C:1".to_owned(), "m:light".to_owned())]);
    assert_eq!(set.mode_attr(&light).as_deref(), Some("light"));
    assert_eq!(set.mode_attr(&BTreeMap::new()), None);
    // A frame set to Light inside a dark area needs the light values back.
    let css = set.css();
    assert!(
        css.starts_with(":root,\n[data-theme=\"light\"] {\n"),
        "{css}"
    );
}
