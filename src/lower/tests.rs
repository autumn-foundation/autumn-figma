#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;

use super::*;
use crate::assets::Format;
use crate::diagnostics::Level;
use crate::ir::{Arg, Body, Call, Element, Function, Item, ParamType};

const SNAPSHOT: &str = include_str!("../../tests/fixtures/todo/figma/snapshot.json");
const VARIABLES: &str = include_str!("../../tests/fixtures/todo/figma/variables.json");
const CONFIG: &str = include_str!("../../tests/fixtures/todo/figma.toml");

fn fixture() -> Lowered {
    let file = FileResponse::from_json(SNAPSHOT).unwrap();
    let vars = VariablesResponse::from_json(VARIABLES).unwrap();
    let config = Config::from_toml_str(CONFIG, Path::new("/app")).unwrap();
    lower(&file, Some(&vars), &config).unwrap()
}

fn func<'a>(l: &'a Lowered, module: &str, name: &str) -> &'a Function {
    l.modules
        .iter()
        .find(|m| m.name == module)
        .unwrap_or_else(|| panic!("no module {module}"))
        .functions
        .iter()
        .find(|f| f.name == name)
        .unwrap_or_else(|| panic!("no function {module}::{name}"))
}

fn single(f: &Function) -> &[Item] {
    match &f.body {
        Body::Single(items) => items,
        Body::Match { .. } => panic!("{} is a match", f.name),
    }
}

fn root(items: &[Item]) -> &Element {
    match items {
        [Item::Element(e)] => e,
        other => panic!("root is not one element: {other:?}"),
    }
}

fn child_el(e: &Element, i: usize) -> &Element {
    match &e.children[i] {
        Item::Element(c) => c,
        other => panic!("child {i} is not an element: {other:?}"),
    }
}

fn params(f: &Function) -> Vec<(&str, &ParamType)> {
    f.params.iter().map(|p| (p.name.as_str(), &p.ty)).collect()
}

/// A minimal file with one page and the given page children.
fn doc(children: &str) -> FileResponse {
    FileResponse::from_json(&format!(
        r#"{{"document":{{"id":"0:0","type":"DOCUMENT","children":[
            {{"id":"9:0","name":"Page","type":"CANVAS","children":[{children}]}}]}}}}"#
    ))
    .unwrap()
}

fn cfg(extra: &str) -> Config {
    Config::from_toml_str(&format!("file_key = \"A1\"\n{extra}"), Path::new("/app")).unwrap()
}

#[test]
fn modules_follow_selected_pages() {
    let l = fixture();
    let names: Vec<_> = l.modules.iter().map(|m| m.name.as_str()).collect();
    assert_eq!(names, ["components", "screens"], "Drafts is not selected");
    let comps: Vec<_> = l.modules[0]
        .functions
        .iter()
        .map(|f| f.name.as_str())
        .collect();
    assert_eq!(comps, ["button", "icon_check", "todo_row"]);
    let screens: Vec<_> = l.modules[1]
        .functions
        .iter()
        .map(|f| f.name.as_str())
        .collect();
    assert_eq!(screens, ["todos"]);
}

#[test]
fn component_set_becomes_enum_and_match() {
    let l = fixture();
    let e = &l.modules[0].enums[0];
    assert_eq!(e.name, "ButtonSize");
    assert_eq!(
        e.variants,
        [
            ("Large".into(), "Large".into()),
            ("Small".into(), "Small".into())
        ]
    );
    assert_eq!(e.default, 0);
    let f = func(&l, "components", "button");
    assert_eq!(
        params(f),
        [
            ("size", &ParamType::Enum("ButtonSize".into())),
            ("icon", &ParamType::Markup),
            ("label", &ParamType::Str),
            ("show_icon", &ParamType::Bool),
            ("attrs", &ParamType::Attrs),
        ]
    );
    let Body::Match { on, arms, fallback } = &f.body else {
        panic!("not a match")
    };
    assert_eq!(on, &["size"]);
    assert_eq!(arms.len(), 2);
    assert!(fallback.is_none(), "all variants exist");
    assert_eq!(arms[0].pattern, ["ButtonSize::Large"]);
    let large = root(&arms[0].body);
    assert_eq!(large.tag, "button", "tag from the set name");
    assert!(large.root_attrs);
    assert_eq!(large.class.as_deref(), Some("fg-button--size-large"));
    assert_eq!(
        large.children[0],
        Item::If {
            cond: "show_icon".into(),
            body: vec![Item::Param("icon".into())]
        }
    );
    let label = child_el(large, 1);
    assert_eq!(label.tag, "span");
    assert_eq!(label.children, [Item::Param("label".into())]);
    assert_eq!(label.class.as_deref(), Some("fg-button--size-large__label"));
}

#[test]
fn icon_component_is_an_image() {
    let l = fixture();
    let f = func(&l, "components", "icon_check");
    assert_eq!(params(f), [("attrs", &ParamType::Attrs)]);
    let img = root(single(f));
    assert_eq!(img.tag, "img");
    assert!(img.root_attrs);
    assert!(
        img.attrs
            .contains(&("src".into(), "/static/figma/icon-check-1-30.svg".into()))
    );
    assert!(img.attrs.contains(&("alt".into(), String::new())));
    assert!(img.children.is_empty());
}

#[test]
fn todo_row_uses_props_and_calls_the_icon() {
    let l = fixture();
    let f = func(&l, "components", "todo_row");
    assert_eq!(
        params(f),
        [
            ("done", &ParamType::Bool),
            ("title", &ParamType::Str),
            ("attrs", &ParamType::Attrs)
        ]
    );
    let li = root(single(f));
    assert_eq!(li.tag, "li", "binding tag");
    assert_eq!(child_el(li, 0).tag, "div", "the ellipse");
    assert_eq!(child_el(li, 1).children, [Item::Param("title".into())]);
    assert_eq!(
        li.children[2],
        Item::If {
            cond: "done".into(),
            body: vec![Item::Call(Call {
                path: "super::components::icon_check".into(),
                args: vec![Arg::Attrs(vec![])],
            })]
        }
    );
}

#[test]
fn screen_applies_bindings() {
    let l = fixture();
    let f = func(&l, "screens", "todos");
    assert_eq!(
        params(f),
        [
            ("items", &ParamType::Markup),
            ("remaining", &ParamType::Str)
        ]
    );
    let page = root(single(f));
    assert_eq!(page.tag, "div");
    assert!(!page.root_attrs, "screens take no attrs");
    assert_eq!(page.class.as_deref(), Some("fg-todos"));

    let title = child_el(page, 0);
    assert_eq!(title.tag, "h1");
    assert_eq!(title.children, [Item::Text("My todos".into())]);

    let form = child_el(page, 1);
    assert_eq!(form.tag, "form");
    assert_eq!(
        form.attrs,
        [
            ("hx-post".into(), "/todos".into()),
            ("hx-swap".into(), "beforeend".into()),
            ("hx-target".into(), "#todo-list".into()),
        ]
    );
    let input = child_el(form, 0);
    assert_eq!(input.tag, "input", "tag from the layer name");
    assert!(input.children.is_empty());
    assert_eq!(
        input.attrs,
        [
            ("placeholder".into(), "What needs doing?".into()),
            ("name".into(), "title".into()),
            ("required".into(), "required".into()),
        ]
    );
    assert_eq!(
        form.children[1],
        Item::Call(Call {
            path: "super::components::button".into(),
            args: vec![
                Arg::Path("super::components::ButtonSize::Small".into()),
                Arg::Markup(vec![Item::Call(Call {
                    path: "super::components::icon_check".into(),
                    args: vec![Arg::Attrs(vec![])],
                })]),
                Arg::Str("Add".into()),
                Arg::Bool(true),
                Arg::Attrs(vec![("type_".into(), "submit".into())]),
            ],
        })
    );

    let list = child_el(page, 2);
    assert_eq!(list.tag, "ul");
    assert_eq!(list.id.as_deref(), Some("todo-list"));
    assert_eq!(list.children, [Item::Param("items".into())]);

    let count = child_el(page, 3);
    assert_eq!(count.id.as_deref(), Some("count"));
    assert_eq!(count.children, [Item::Param("remaining".into())]);

    let hero = child_el(page, 4);
    let photo = child_el(hero, 0);
    assert_eq!(photo.tag, "img");
    assert!(
        photo
            .attrs
            .contains(&("src".into(), "/static/figma/photo-2-11.png".into()))
    );
    assert_eq!(
        child_el(hero, 1).children,
        [Item::Text("Ship it\nToday".into())]
    );

    assert_eq!(
        page.children.len(),
        5,
        "hidden and unsupported nodes are skipped"
    );
}

#[test]
fn assets_rules_tokens_and_notes() {
    let l = fixture();
    let assets: Vec<_> = l
        .assets
        .iter()
        .map(|a| (a.node_id.as_str(), a.file_name.as_str(), a.format))
        .collect();
    assert_eq!(
        assets,
        [
            ("1:30", "icon-check-1-30.svg", Format::Svg),
            ("2:11", "photo-2-11.png", Format::Png),
        ]
    );
    let classes: Vec<_> = l.rules.iter().map(|r| r.class.as_str()).collect();
    let mut unique = classes.clone();
    unique.sort_unstable();
    unique.dedup();
    assert_eq!(
        unique.len(),
        classes.len(),
        "classes are unique: {classes:?}"
    );
    assert!(classes.iter().all(|c| c.starts_with("fg-")));
    let rule = |c: &str| &l.rules.iter().find(|r| r.class == c).unwrap().decls;
    assert_eq!(
        rule("fg-button--size-large").get("background-color"),
        Some("var(--primary)")
    );
    assert_eq!(
        rule("fg-button--size-large").get("gap"),
        Some("var(--spacing-sm)")
    );
    assert_eq!(
        rule("fg-button--size-large").get("box-shadow"),
        Some("var(--shadow-card)")
    );
    assert_eq!(
        rule("fg-todos__title").get("font"),
        Some("var(--heading-h1)")
    );
    // An <input> keeps the type of the text layer it replaces, not its color.
    let input = rule("fg-todos__input");
    assert_eq!(input.get("font-size"), Some("16px"));
    assert_eq!(
        input.get("font-family"),
        Some("\"Inter\", system-ui, sans-serif")
    );
    assert_eq!(input.get("color"), None);
    let css = l.tokens.css();
    assert!(css.contains("--primary: #7c3aed;"), "{css}");
    assert!(
        css.contains("--heading-h1: 700 32px/40px \"Inter\", system-ui, sans-serif;"),
        "{css}"
    );

    let notes: Vec<String> = l.diagnostics.iter().map(ToString::to_string).collect();
    let has = |level: Level, path: &str, text: &str| {
        l.diagnostics
            .iter()
            .any(|d| d.level == level && d.path == path && d.message.contains(text))
    };
    assert!(
        has(Level::Warning, "Screens > Todos > Hero art", "auto layout"),
        "{notes:#?}"
    );
    assert!(
        has(Level::Info, "Screens > Todos > Map embed", "not supported"),
        "{notes:#?}"
    );
    assert!(
        has(Level::Info, "Screens > Note", "top-level"),
        "{notes:#?}"
    );
    assert!(
        has(
            Level::Info,
            "Screens > Todos > New todo > input",
            "placeholder"
        ),
        "{notes:#?}"
    );
}

#[test]
fn lowering_is_deterministic() {
    let a = fixture();
    let b = fixture();
    assert_eq!(a.modules, b.modules);
    assert_eq!(a.rules, b.rules);
    assert_eq!(a.tokens.css(), b.tokens.css());
}

#[test]
fn missing_page_is_a_config_error() {
    let err = lower(&doc(""), None, &cfg("pages = [\"Nope\"]")).unwrap_err();
    assert_eq!(err.kind(), ErrorKind::Config);
    assert!(err.message().contains("Nope"));
}

#[test]
fn unmatched_binding_is_an_error() {
    let file = doc(r#"{"id":"1","name":"Card","type":"FRAME"}"#);
    let err = lower(
        &file,
        None,
        &cfg("[[bind]]\npath = \"Page > Crad\"\ntag = \"p\""),
    )
    .unwrap_err();
    assert_eq!(err.kind(), ErrorKind::Binding);
    assert!(err.message().contains("Page > Crad"), "{err}");
}

#[test]
fn invalid_bindings_are_errors() {
    let file = doc(
        r#"{"id":"1","name":"Card","type":"FRAME","layoutMode":"VERTICAL","children":[
            {"id":"2","name":"Box","type":"FRAME"},{"id":"3","name":"Box","type":"FRAME"}]}"#,
    );
    let e = |bind: &str| lower(&file, None, &cfg(&format!("[[bind]]\n{bind}"))).unwrap_err();
    let err = e("path = \"Page > Card\"\ntext = \"t\"");
    assert!(err.message().contains("text"), "{err}");
    let err = e("path = \"Page > Card > Box\"\nid = \"dup\"");
    assert!(err.message().contains("2 nodes"), "{err}");
    let two = lower(
        &file,
        None,
        &cfg("[[bind]]\nnode = \"2\"\ntag = \"p\"\n[[bind]]\npath = \"Page > Card > Box\"\ntag = \"p\""),
    )
    .unwrap_err();
    assert!(two.message().contains("two bindings"), "{two}");
}

const LOCAL: &str = r#"
  {"id":"1","name":"Chip","type":"COMPONENT","componentPropertyDefinitions":{
     "Label#1":{"type":"TEXT","defaultValue":"Hi"},"label#2":{"type":"TEXT","defaultValue":"x"},
     "Type":{"type":"TEXT","defaultValue":"t"}},
   "children":[{"id":"1:1","name":"T","type":"TEXT","characters":"Hi","componentPropertyReferences":{"characters":"Label#1"}}]},
  {"id":"2","name":"Screen","type":"FRAME","layoutMode":"VERTICAL","children":[
     {"id":"3","name":"Chip","type":"INSTANCE","componentId":"1",
      "componentProperties":{"Label#1":{"type":"TEXT","value":"Yo"}}},
     {"id":"4","name":"Remote","type":"INSTANCE","componentId":"99:1","layoutMode":"HORIZONTAL","children":[
        {"id":"I4;5","name":"Text","type":"TEXT","characters":"remote"}]}]}"#;

#[test]
fn param_names_are_unique_and_defaults_fill_calls() {
    let l = lower(&doc(LOCAL), None, &cfg("")).unwrap();
    let chip = func(&l, "page", "chip");
    let names: Vec<_> = chip.params.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, ["label", "label_2", "type_", "attrs"]);
    let screen = root(single(func(&l, "page", "screen")));
    assert_eq!(
        screen.children[0],
        Item::Call(Call {
            path: "super::page::chip".into(),
            args: vec![
                Arg::Str("Yo".into()),
                Arg::Str("x".into()),
                Arg::Str("t".into()),
                Arg::Attrs(vec![]),
            ],
        })
    );
    let remote = child_el(screen, 1);
    assert_eq!(remote.tag, "div", "remote instance is inlined");
    assert_eq!(child_el(remote, 0).children, [Item::Text("remote".into())]);
}

#[test]
fn bindings_on_instances() {
    let file = doc(LOCAL);
    let ok = lower(
        &file,
        None,
        &cfg("[[bind]]\npath = \"Page > Screen > Chip\"\nid = \"c\"\nattrs = { hx-get = \"/c\" }"),
    )
    .unwrap();
    let screen = root(single(func(&ok, "page", "screen")));
    let Item::Call(call) = &screen.children[0] else {
        panic!("not a call")
    };
    assert_eq!(
        call.args.last().unwrap(),
        &Arg::Attrs(vec![
            ("id".into(), "c".into()),
            ("hx_get".into(), "/c".into())
        ])
    );
    for bad in ["tag = \"p\"", "slot = \"s\"", "attrs = { data-x = \"1\" }"] {
        let err = lower(
            &file,
            None,
            &cfg(&format!("[[bind]]\npath = \"Page > Screen > Chip\"\n{bad}")),
        )
        .unwrap_err();
        assert_eq!(err.kind(), ErrorKind::Binding, "{bad}");
    }
}

#[test]
fn partial_variant_sets_get_a_fallback() {
    let file = doc(
        r#"{"id":"1","name":"Tag","type":"COMPONENT_SET","componentPropertyDefinitions":{
             "Size":{"type":"VARIANT","defaultValue":"S","variantOptions":["S","M"]},
             "Tone":{"type":"VARIANT","defaultValue":"Calm","variantOptions":["Calm","Loud"]}},
           "children":[
             {"id":"2","name":"Size=S, Tone=Calm","type":"COMPONENT"},
             {"id":"3","name":"Size=M, Tone=Loud","type":"COMPONENT"},
             {"id":"4","name":"Size=M, Tone=Loud","type":"COMPONENT"}]}"#,
    );
    let l = lower(&file, None, &cfg("")).unwrap();
    let f = func(&l, "page", "tag");
    let Body::Match { on, arms, fallback } = &f.body else {
        panic!("not a match")
    };
    assert_eq!(on, &["size", "tone"]);
    assert_eq!(arms.len(), 2, "the duplicate variant is dropped");
    assert_eq!(arms[1].pattern, ["TagSize::M", "TagTone::Loud"]);
    assert!(fallback.is_some());
    assert!(
        l.diagnostics
            .iter()
            .any(|d| d.message.contains("duplicate"))
    );
}

#[test]
fn text_bindings_on_components_become_params_for_callers() {
    let file = doc(r#"{"id":"1","name":"Card","type":"COMPONENT","children":[
             {"id":"1:1","name":"Body","type":"TEXT","characters":"Default"},
             {"id":"1:2","name":"Items","type":"FRAME","children":[]}]},
           {"id":"2","name":"Screen","type":"FRAME","children":[
             {"id":"3","name":"Card","type":"INSTANCE","componentId":"1","children":[
               {"id":"I3;1:1","name":"Body","type":"TEXT","characters":"Override"},
               {"id":"I3;1:2","name":"Items","type":"FRAME","children":[
                 {"id":"I3;1:3","name":"Row","type":"TEXT","characters":"row"}]}]}]}"#);
    let l = lower(
        &file,
        None,
        &cfg("[[bind]]\npath = \"Page > Card > Body\"\ntext = \"body\"\n[[bind]]\npath = \"Page > Card > Items\"\nslot = \"items\""),
    )
    .unwrap();
    let card = func(&l, "page", "card");
    assert_eq!(
        params(card),
        [
            ("body", &ParamType::Str),
            ("items", &ParamType::Markup),
            ("attrs", &ParamType::Attrs)
        ]
    );
    let screen = root(single(func(&l, "page", "screen")));
    let Item::Call(call) = &screen.children[0] else {
        panic!("not a call: {:?}", screen.children)
    };
    assert_eq!(call.args[0], Arg::Str("Override".into()));
    let Arg::Markup(items) = &call.args[1] else {
        panic!("not markup")
    };
    assert_eq!(items.len(), 1);
}

#[test]
fn hidden_layers_and_name_tags() {
    let file = doc(
        r#"{"id":"1","name":"Screen","type":"FRAME","layoutMode":"VERTICAL","children":[
             {"id":"2","name":"Gone","type":"FRAME","visible":false},
             {"id":"3","name":" Button ","type":"FRAME","layoutMode":"HORIZONTAL","children":[
               {"id":"4","name":"h2","type":"TEXT","characters":"Go"}]},
             {"id":"5","name":"textarea","type":"FRAME","children":[
               {"id":"6","name":"t","type":"TEXT","characters":"Say hi"}]}]}"#,
    );
    let l = lower(&file, None, &cfg("")).unwrap();
    let screen = root(single(func(&l, "page", "screen")));
    assert_eq!(screen.children.len(), 2, "the hidden layer is skipped");
    let button = child_el(screen, 0);
    assert_eq!(button.tag, "button");
    assert_eq!(child_el(button, 0).tag, "h2");
    let textarea = child_el(screen, 1);
    assert_eq!(textarea.tag, "textarea");
    assert!(textarea.children.is_empty());
    assert_eq!(textarea.attrs, [("placeholder".into(), "Say hi".into())]);
}

#[test]
fn hostile_names_stay_safe() {
    let file = doc(
        r#"{"id":"1","name":"fn\"); drop(\"","type":"COMPONENT","componentPropertyDefinitions":{
             "}{ x#1":{"type":"TEXT","defaultValue":"\"quoted\" \\ </script>"}},
           "children":[{"id":"2","name":"<b>","type":"TEXT","characters":"<script>alert(1)</script>"}]}"#,
    );
    let l = lower(&file, None, &cfg("")).unwrap();
    let f = &l.modules[0].functions[0];
    assert_eq!(f.name, "fn_drop");
    assert_eq!(f.params[0].name, "x");
    let el = root(single(f));
    assert_eq!(
        child_el(el, 0).children,
        [Item::Text("<script>alert(1)</script>".into())]
    );
}

#[test]
fn a_swapped_instance_deep_in_an_instance_keeps_its_full_path() {
    // Regression: the argument used the instance as its parent and path,
    // so a binding on the real (deeper) path did not match.
    let file = doc(
        r#"{"id":"1","name":"Icon","type":"COMPONENT","children":[{"id":"1:1","type":"VECTOR"}]},
           {"id":"2","name":"Card","type":"COMPONENT","componentPropertyDefinitions":{
              "Glyph#1":{"type":"INSTANCE_SWAP","defaultValue":"1"}},
            "children":[{"id":"2:1","name":"Wrap","type":"FRAME","children":[
              {"id":"2:2","name":"Glyph","type":"INSTANCE","componentId":"1",
               "componentPropertyReferences":{"mainComponent":"Glyph#1"}}]}]},
           {"id":"3","name":"Screen","type":"FRAME","children":[
             {"id":"4","name":"Card","type":"INSTANCE","componentId":"2",
              "componentProperties":{"Glyph#1":{"type":"INSTANCE_SWAP","value":"1"}},
              "children":[{"id":"I4;2:1","name":"Wrap","type":"FRAME","children":[
                {"id":"I4;2:2","name":"Glyph","type":"INSTANCE","componentId":"1",
                 "componentPropertyReferences":{"mainComponent":"Glyph#1"}}]}]}]}"#,
    );
    let l = lower(
        &file,
        None,
        &cfg("[[bind]]\npath = \"Page > Screen > Card > Wrap > Glyph\"\nattrs = { aria-label = \"done\" }"),
    )
    .unwrap();
    let screen = root(single(func(&l, "page", "screen")));
    let Item::Call(card) = &screen.children[0] else {
        panic!("not a call")
    };
    assert_eq!(
        card.args[0],
        Arg::Markup(vec![Item::Call(Call {
            path: "super::page::icon".into(),
            args: vec![Arg::Attrs(vec![("aria_label".into(), "done".into())])],
        })])
    );
}
