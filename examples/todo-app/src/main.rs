//! A todo app made from a Figma design with `autumn-figma`.
//!
//! `autumn-figma generate` writes `src/figma/` and `static/css/figma.css`.
//! Run it after a design change. This file holds only the behavior: the
//! routes and the data.
//!
//! Autumn serves htmx and its CSRF helper. The helper reads the token from
//! the `csrf-token` meta tag, so htmx requests also work in the prod
//! profile, where CSRF protection is on.

mod figma;

use std::sync::Mutex;

use autumn_web::prelude::*;
use autumn_web::security::CsrfToken;
use figma::Attrs;
use figma::components::todo_row;
use serde::Deserialize;

/// One todo: id, title, done.
type Todo = (u32, String, bool);

static TODOS: Mutex<Vec<Todo>> = Mutex::new(Vec::new());

fn todos() -> Vec<Todo> {
    TODOS.lock().map(|t| t.clone()).unwrap_or_default()
}

fn layout(csrf: Option<&CsrfToken>, content: Markup) -> Markup {
    html! {
        (maud::DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { "Todos" }
                @if let Some(csrf) = csrf {
                    meta name="csrf-token" content=(csrf.token());
                }
                link rel="stylesheet" href="/static/css/figma.css";
                script src=(asset_url("js/htmx.min.js")) defer {}
                script src=(asset_url("js/autumn-htmx-csrf.js")) defer {}
            }
            body { (content) }
        }
    }
}

/// One row. The root gets an id and an htmx delete action.
fn row(todo: &Todo) -> Markup {
    let (id, title, done) = todo;
    let dom_id = format!("todo-{id}");
    let delete = format!("/todos/{id}");
    todo_row(
        *done,
        title,
        &Attrs {
            id: Some(&dom_id),
            hx_delete: Some(&delete),
            hx_swap: Some("outerHTML"),
            hx_trigger: Some("dblclick"),
            ..Attrs::NONE
        },
    )
}

fn remaining(todos: &[Todo]) -> String {
    match todos.iter().filter(|t| !t.2).count() {
        1 => "1 item left".to_owned(),
        open => format!("{open} items left"),
    }
}

/// The count, swapped out of band, so it changes with each add or delete.
fn count_oob() -> Markup {
    html! {
        span id="count" class="fg-todos__count" hx-swap-oob="true" { (remaining(&todos())) }
    }
}

#[get("/")]
#[public]
async fn index(csrf: Option<CsrfToken>) -> Markup {
    let todos = todos();
    let rows = html! {
        @for todo in &todos {
            (row(todo))
        }
    };
    layout(
        csrf.as_ref(),
        figma::screens::todos(rows, &remaining(&todos)),
    )
}

#[derive(Deserialize)]
struct NewTodo {
    title: String,
}

/// The new row, for `hx-swap="beforeend"` into `#todo-list`.
#[post("/todos")]
#[public]
async fn create(Form(form): Form<NewTodo>) -> Markup {
    let todo = TODOS.lock().map_or_else(
        |_| (0, form.title.clone(), false),
        |mut list| {
            let id = list.iter().map(|t| t.0).max().unwrap_or(0) + 1;
            let todo = (id, form.title.clone(), false);
            list.push(todo.clone());
            todo
        },
    );
    html! {
        (row(&todo))
        (count_oob())
    }
}

/// No row: htmx swaps the row out. The count changes out of band.
#[delete("/todos/{id}")]
#[public]
async fn remove(id: Path<u32>) -> Markup {
    if let Ok(mut list) = TODOS.lock() {
        list.retain(|t| t.0 != *id);
    }
    count_oob()
}

#[autumn_web::main]
async fn main() {
    if let Ok(mut list) = TODOS.lock() {
        list.push((1, "Buy milk".to_owned(), true));
        list.push((2, "Walk the dog".to_owned(), false));
    }
    autumn_web::app()
        .routes(routes![index, create, remove])
        .run()
        .await;
}
