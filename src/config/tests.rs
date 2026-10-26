#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

fn root() -> PathBuf {
    PathBuf::from("/app")
}

fn parse(src: &str) -> Result<Config> {
    Config::from_toml_str(src, &root())
}

fn err(src: &str) -> String {
    let e = parse(src).unwrap_err();
    assert_eq!(e.kind(), ErrorKind::Config, "{e}");
    e.message().to_owned()
}

#[test]
fn defaults() {
    let c = parse(r#"file_key = "AbC123""#).unwrap();
    assert_eq!(c.file_key, "AbC123");
    assert_eq!(c.root, root());
    assert_eq!(c.snapshot, root().join("figma/snapshot.json"));
    assert_eq!(c.variables_snapshot, root().join("figma/variables.json"));
    assert_eq!(c.out_dir, root().join("src/figma"));
    assert_eq!(c.css_path, root().join("static/css/figma.css"));
    assert_eq!(c.assets_dir, root().join("static/figma"));
    assert_eq!(c.assets_url, "/static/figma");
    assert_eq!(c.class_prefix, "fg");
    assert!(c.pages.is_empty() && c.tokens.is_empty() && c.binds.is_empty());
}

#[test]
fn full_file() {
    let c = parse(
        r##"
file_key = "https://www.figma.com/design/AbC123/Todo?node-id=1-2"
pages = ["Components", "Screens"]
out_dir = "src/ui/figma"
css_path = "static/css/design.css"
assets_dir = "static/img"
assets_url = "/static/img"
class_prefix = "td"

[tokens]
"color/brand/primary" = "--primary"

[[bind]]
path = "Screens > Todos > New todo"
tag = "form"
attrs = { "hx-post" = "/todos", "hx-target" = "#todo-list" }

[[bind]]
node = "2:9"
id = "count"
text = "remaining"

[[bind]]
path = " Screens>Todos >List "
slot = "items"
"##,
    )
    .unwrap();
    assert_eq!(c.file_key, "AbC123");
    assert_eq!(c.out_dir, root().join("src/ui/figma"));
    assert_eq!(c.class_prefix, "td");
    assert_eq!(c.tokens["color/brand/primary"], "--primary");
    assert_eq!(c.binds.len(), 3);
    assert_eq!(
        c.binds[0].selector,
        Selector::Path(vec!["Screens".into(), "Todos".into(), "New todo".into()])
    );
    assert_eq!(c.binds[0].tag.as_deref(), Some("form"));
    assert_eq!(c.binds[0].attrs["hx-post"], "/todos");
    assert_eq!(c.binds[1].selector, Selector::Node("2:9".into()));
    assert_eq!(c.binds[1].text.as_deref(), Some("remaining"));
    assert_eq!(
        c.binds[2].selector,
        Selector::Path(vec!["Screens".into(), "Todos".into(), "List".into()])
    );
    assert_eq!(c.binds[0].selector.to_string(), "Screens > Todos > New todo");
}

#[test]
fn token_in_file_is_rejected_with_a_hint() {
    let msg = err("file_key = \"A1\"\ntoken = \"figd_secret\"");
    assert!(msg.contains("FIGMA_TOKEN"), "{msg}");
    assert!(!msg.contains("figd_secret"), "message leaks the token: {msg}");
}

#[test]
fn unknown_keys_are_rejected() {
    assert!(err("file_key = \"A1\"\nout_dri = \"x\"").contains("out_dri"));
    assert!(err("file_key = \"A1\"\n[[bind]]\nnode = \"1:2\"\ntga = \"p\"").contains("tga"));
}

#[test]
fn bad_values_are_rejected() {
    assert!(err("").contains("file_key"));
    assert!(err("file_key = \"has space\"").contains("file_key"));
    assert!(err("file_key = \"A1\"\nclass_prefix = \"9x\"").contains("class_prefix"));
    assert!(err("file_key = \"A1\"\nout_dir = \"/etc\"").contains("out_dir"));
    assert!(err("file_key = \"A1\"\nout_dir = \"../x\"").contains("out_dir"));
    assert!(err("file_key = \"A1\"\nassets_url = \"/a b\"").contains("assets_url"));
    assert!(err("file_key = \"A1\"\nassets_url = \"x\")\"").contains("assets_url"));
    assert!(err("file_key = \"A1\"\n[tokens]\n\"a\" = \"primary\"").contains("--"));
    assert!(err("file_key = \"A1\"\n[tokens]\n\"a\" = \"--a;b\"").contains("--a;b"));
}

#[test]
fn bad_bindings_are_rejected() {
    let base = "file_key = \"A1\"\n[[bind]]\n";
    assert!(err(&format!("{base}tag = \"div\"")).contains("path"));
    assert!(err(&format!("{base}path = \"A\"\nnode = \"1:2\"\ntag = \"p\"")).contains("one of"));
    assert!(err(&format!("{base}path = \"A\"")).contains("nothing"));
    assert!(err(&format!("{base}path = \"A > \"\ntag = \"p\"")).contains("path"));
    assert!(err(&format!("{base}path = \"A\"\ntag = \"script\"")).contains("script"));
    assert!(err(&format!("{base}path = \"A\"\nid = \"a b\"")).contains("id"));
    assert!(err(&format!("{base}path = \"A\"\nattrs = {{ style = \"x\" }}")).contains("style"));
    assert!(err(&format!("{base}path = \"A\"\nattrs = {{ class = \"x\" }}")).contains("class"));
    assert!(err(&format!("{base}path = \"A\"\nattrs = {{ id = \"x\" }}")).contains("id"));
    assert!(err(&format!("{base}path = \"A\"\nattrs = {{ \"a b\" = \"x\" }}")).contains("a b"));
    assert!(err(&format!("{base}path = \"A\"\nattrs = {{ \"x>\" = \"x\" }}")).contains("x>"));
    assert!(err(&format!("{base}path = \"A\"\nslot = \"Items\"")).contains("Items"));
    assert!(err(&format!("{base}path = \"A\"\nslot = \"type\"")).contains("type"));
    assert!(err(&format!("{base}path = \"A\"\nslot = \"a\"\ntext = \"b\"")).contains("slot"));
}

#[test]
fn htmx_attribute_names_are_allowed() {
    let c = parse(
        "file_key = \"A1\"\n[[bind]]\npath = \"A\"\nattrs = { \"hx-on:click\" = \"x\", \"data-id\" = \"1\", \"@click\" = \"y\", \"aria-label\" = \"z\" }",
    )
    .unwrap();
    assert_eq!(c.binds[0].attrs.len(), 4);
}

#[test]
fn load_reads_the_file_and_uses_its_directory() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("figma.toml");
    std::fs::write(&path, "file_key = \"A1\"").unwrap();
    let c = Config::load(&path).unwrap();
    assert_eq!(c.root, dir.path());
    assert_eq!(c.out_dir, dir.path().join("src/figma"));
    let missing = Config::load(&dir.path().join("nope.toml")).unwrap_err();
    assert_eq!(missing.kind(), ErrorKind::Io);
}

#[test]
fn file_keys_from_urls() {
    assert_eq!(parse_file_key("AbC123").unwrap(), "AbC123");
    assert_eq!(
        parse_file_key("https://www.figma.com/design/AbC123/My-File?node-id=1-2").unwrap(),
        "AbC123"
    );
    assert_eq!(parse_file_key("https://figma.com/file/XyZ9/Old").unwrap(), "XyZ9");
    assert_eq!(
        parse_file_key("https://www.figma.com/design/AbC123/branch/Br4nch/My-File").unwrap(),
        "Br4nch"
    );
    assert!(parse_file_key("").is_err());
    assert!(parse_file_key("https://example.com/x").is_err());
    assert!(parse_file_key("a/b").is_err());
}

#[test]
fn token_comes_from_the_lookup_and_is_redacted() {
    let token = token_from(&|k| (k == TOKEN_ENV).then(|| "figd_secret".to_owned())).unwrap();
    assert_eq!(token.expose(), "figd_secret");
    assert!(!format!("{token:?}").contains("figd_secret"));
    let missing = token_from(&|_| None).unwrap_err();
    assert_eq!(missing.kind(), ErrorKind::Auth);
    assert!(missing.message().contains(TOKEN_ENV));
    let blank = token_from(&|_| Some("  ".to_owned())).unwrap_err();
    assert_eq!(blank.kind(), ErrorKind::Auth);
}

#[test]
fn api_base() {
    assert_eq!(api_base_from(&|_| None).unwrap(), DEFAULT_API_BASE);
    assert_eq!(
        api_base_from(&|_| Some("http://127.0.0.1:9/".to_owned())).unwrap(),
        "http://127.0.0.1:9"
    );
    assert!(api_base_from(&|_| Some("ftp://x".to_owned())).is_err());
}

#[test]
fn starter_toml_parses() {
    let src = starter_toml("AbC123");
    let c = parse(&src).unwrap();
    assert_eq!(c.file_key, "AbC123");
    assert!(src.contains("FIGMA_TOKEN"));
    assert!(src.contains("[[bind]]"));
}
