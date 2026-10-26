//! A todo app made from a Figma design with `autumn-figma`.
//!
//! `src/figma/` and `static/css/figma.css` are generated. Run
//! `autumn-figma generate` after a design change. This file holds only the
//! behavior: the routes and the data.

mod figma;

use std::sync::Mutex;

use autumn_web::prelude::*;
use figma::Attrs;
use figma::components::todo_row;
use serde::Deserialize;

/// One todo: id, title, done.
type Todo = (u32, String, bool);

static TODOS: Mutex<Vec<Todo>> = Mutex::new(Vec::new());

fn todos() -> Vec<Todo> {
    TODOS.lock().map(|t| t.clone()).unwrap_or_default()
}

fn layout(content: Markup) -> Markup {
    html! {
        (maud::DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { "Todos" }
                link rel="stylesheet" href="/static/css/figma.css";
                script src="/static/js/htmx.min.js" defer {}
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
    let open = todos.iter().filter(|t| !t.2).count();
    format!("{open} items left")
}

#[get("/")]
#[public]
async fn index() -> Markup {
    let todos = todos();
    let rows = html! {
        @for todo in &todos {
            (row(todo))
        }
    };
    layout(figma::screens::todos(rows, &remaining(&todos)))
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
    row(&todo)
}

/// An empty body: htmx swaps the row out.
#[delete("/todos/{id}")]
#[public]
async fn remove(id: Path<u32>) -> Markup {
    if let Ok(mut list) = TODOS.lock() {
        list.retain(|t| t.0 != *id);
    }
    html! {}
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
