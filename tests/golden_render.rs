//! Compile the golden output and render it with Maud.
//!
//! This test target builds `tests/golden/src/figma` as a module, so the
//! crate lints (pedantic, nursery) also run on the generated code.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

#[path = "golden/src/figma/mod.rs"]
mod figma;

use figma::Attrs;
use figma::components::{ButtonSize, button, icon_check, todo_row};
use figma::screens::todos;
use maud::html;

#[test]
fn todos_screen_renders_with_htmx_wiring() {
    let rows = html! {
        (todo_row(true, "Buy milk", &Attrs::NONE))
        (todo_row(false, "Walk <the> dog", &Attrs::NONE))
    };
    let page = todos(rows, "2 items left").into_string();
    assert!(page.starts_with("<div class=\"fg-todos\">"), "{page}");
    assert!(
        page.contains("<h1 class=\"fg-todos__title\">My todos</h1>"),
        "{page}"
    );
    assert!(
        page.contains("<form class=\"fg-todos__new-todo\" hx-post=\"/todos\" hx-swap=\"beforeend\" hx-target=\"#todo-list\">"),
        "{page}"
    );
    assert!(
        page.contains("<input class=\"fg-todos__input\" placeholder=\"What needs doing?\" name=\"title\" required=\"required\">"),
        "{page}"
    );
    assert!(
        page.contains("<button class=\"fg-button--size-small\" type=\"submit\">"),
        "{page}"
    );
    assert!(
        page.contains("<ul class=\"fg-todos__list\" id=\"todo-list\"><li"),
        "{page}"
    );
    assert!(page.contains("Walk &lt;the&gt; dog"), "escaped: {page}");
    assert!(page.contains("id=\"count\">2 items left</span>"), "{page}");
    assert!(
        page.contains("src=\"/static/figma/photo--2-11.png\""),
        "{page}"
    );
    assert!(page.contains("Ship it<br>Today"), "{page}");
    assert!(!page.contains("style="), "no inline styles: {page}");
}

#[test]
fn a_fragment_for_an_htmx_swap() {
    let row = todo_row(
        false,
        "New",
        &Attrs {
            id: Some("todo-7"),
            hx_delete: Some("/todos/7"),
            hx_swap: Some("outerHTML"),
            ..Attrs::NONE
        },
    )
    .into_string();
    assert!(
        row.starts_with(
            "<li class=\"fg-todo-row\" id=\"todo-7\" hx-delete=\"/todos/7\" hx-swap=\"outerHTML\">"
        ),
        "{row}"
    );
    assert!(!row.contains("fg-icon-check"), "done is false: {row}");
}

#[test]
fn variants_and_props() {
    let large = button(ButtonSize::Large, html! {}, "Save", false, &Attrs::NONE).into_string();
    assert!(large.contains("fg-button--size-large"), "{large}");
    assert!(large.contains(">Save</span>"), "{large}");
    let icon = icon_check(&Attrs::NONE);
    let small = button(ButtonSize::Small, icon, "Go", true, &Attrs::NONE).into_string();
    assert!(small.contains("fg-button--size-small"), "{small}");
    assert!(
        small.contains("src=\"/static/figma/icon-check--1-30.svg\""),
        "{small}"
    );
    assert_eq!(ButtonSize::default(), ButtonSize::Large);
}
