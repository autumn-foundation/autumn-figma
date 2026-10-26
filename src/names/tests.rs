#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use proptest::prelude::*;

use super::*;

#[test]
fn slug_examples() {
    assert_eq!(slug("Button/Primary"), "button-primary");
    assert_eq!(slug("  Size=Large, State=Hover "), "size-large-state-hover");
    assert_eq!(slug("Ünïcode ✨"), "n-code");
    assert_eq!(slug("***"), "node");
    assert_eq!(slug(""), "node");
    assert_eq!(slug("I2:5;1:12"), "i2-5-1-12");
}

#[test]
fn snake_ident_examples() {
    assert_eq!(snake_ident("Todo row"), "todo_row");
    assert_eq!(snake_ident("Show icon"), "show_icon");
    assert_eq!(snake_ident("Button/Primary"), "button_primary");
    assert_eq!(snake_ident("type"), "type_");
    assert_eq!(snake_ident("Self"), "self_");
    assert_eq!(snake_ident("3 columns"), "n3_columns");
    assert_eq!(snake_ident(""), "node");
    assert_eq!(snake_ident("camelCase"), "camelcase");
}

#[test]
fn camel_ident_examples() {
    assert_eq!(camel_ident("size"), "Size");
    assert_eq!(camel_ident("extra large"), "ExtraLarge");
    assert_eq!(camel_ident("Button/Primary"), "ButtonPrimary");
    assert_eq!(camel_ident("2xl"), "V2xl");
    assert_eq!(camel_ident("Self"), "Self_");
    assert_eq!(camel_ident(""), "Node");
}

#[test]
fn keywords() {
    for word in [
        "fn", "type", "match", "async", "self", "Self", "crate", "abstract", "gen",
    ] {
        assert!(is_keyword(word), "{word}");
    }
    assert!(!is_keyword("button"));
}

#[test]
fn rust_str_escapes() {
    assert_eq!(rust_str("hi"), "\"hi\"");
    assert_eq!(rust_str("a\"b\\c\nd"), "\"a\\\"b\\\\c\\nd\"");
    assert_eq!(rust_str("}{ ); drop"), "\"}{ ); drop\"");
}

#[test]
fn css_str_escapes() {
    assert_eq!(css_str("Inter"), "\"Inter\"");
    assert_eq!(css_str("a\"b\\c\nd"), "\"a\\\"b\\\\c\\a d\"");
    assert_eq!(css_str("</style>"), "\"\\3c /style>\"");
}

#[test]
fn numbers() {
    assert_eq!(num(12.0), "12");
    assert_eq!(num(12.5), "12.5");
    assert_eq!(num(0.333_333), "0.33");
    assert_eq!(num(-0.0), "0");
    assert_eq!(num(-0.001), "0");
    assert_eq!(px(0.0), "0");
    assert_eq!(px(16.0), "16px");
    assert_eq!(px(1.25), "1.25px");
}

#[test]
fn allocator_dedupes() {
    let mut names = Allocator::new("_");
    assert_eq!(names.claim("row"), "row");
    assert_eq!(names.claim("row"), "row_2");
    assert_eq!(names.claim("row"), "row_3");
    names.reserve("item");
    assert_eq!(names.claim("item"), "item_2");
    let mut classes = Allocator::new("-");
    assert_eq!(classes.claim("fg-a"), "fg-a");
    assert_eq!(classes.claim("fg-a"), "fg-a-2");
}

fn is_snake(s: &str) -> bool {
    let mut chars = s.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_lowercase() || c == '_')
        && s.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

proptest! {
    #[test]
    fn snake_ident_is_always_a_valid_identifier(name in ".{0,40}") {
        let id = snake_ident(&name);
        prop_assert!(is_snake(&id), "{id:?}");
        prop_assert!(!is_keyword(&id), "{id:?}");
    }

    #[test]
    fn camel_ident_is_always_a_valid_identifier(name in ".{0,40}") {
        let id = camel_ident(&name);
        let first = id.chars().next().unwrap();
        prop_assert!(first.is_ascii_uppercase(), "{id:?}");
        prop_assert!(id.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'), "{id:?}");
        prop_assert!(!is_keyword(&id), "{id:?}");
    }

    #[test]
    fn slug_is_safe_for_css_and_paths(name in ".{0,40}") {
        let s = slug(&name);
        prop_assert!(!s.is_empty());
        prop_assert!(s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'));
        prop_assert!(!s.starts_with('-') && !s.ends_with('-') && !s.contains("--"));
    }

    #[test]
    fn rust_str_round_trips_through_debug(text in ".{0,40}") {
        let lit = rust_str(&text);
        prop_assert!(lit.starts_with('"') && lit.ends_with('"'));
        prop_assert_eq!(lit, format!("{text:?}"));
    }

    #[test]
    fn css_str_never_closes_early(text in ".{0,40}") {
        let lit = css_str(&text);
        let inner = &lit[1..lit.len() - 1];
        let mut escaped = false;
        for c in inner.chars() {
            if escaped { escaped = false; continue; }
            prop_assert!(c != '"' && c != '\n' && c != '<', "{lit:?}");
            if c == '\\' { escaped = true; }
        }
    }
}
