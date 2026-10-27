# autumn-figma

Turn a Figma design into Maud components for [Autumn](https://autumn-web.app)
apps. Add htmx behavior in one config file. Keep the design and the code in
sync with one command.

`autumn-figma` is a CLI. The design is a build input, like a migration. Your
app does not call Figma at run time, and it does not depend on this crate.
The generated code depends on `maud` only. See
[ADR 0001](docs/adr/0001-cli-codegen.md).

This crate is a development tool, not a runtime `Plugin`. Install it with
`cargo install`. `autumn plugin add` and the Autumn plugin index do not
apply to it.

```text
Figma file ──pull──▶ figma/snapshot.json ──generate──▶ src/figma/*.rs       (Maud functions)
 (REST API)          figma/variables.json              static/css/figma.css (tokens + classes)
                     static/figma/*.svg|png
```

## Quickstart

```sh
cargo install --locked --git https://github.com/autumn-foundation/autumn-figma

cd my-autumn-app
autumn-figma init --file-key "https://www.figma.com/design/AbC123/Todo"
export FIGMA_TOKEN="figd_..."   # Figma > Settings > Security > Personal access tokens
autumn-figma sync               # pull, then generate
```

The token needs the `file_content:read` scope. For variables, it also needs
`file_variables:read` (Enterprise plans only).

Turn on Autumn's `htmx` feature. Then Autumn serves htmx and its CSRF
helper:

```toml
autumn-web = { version = "0.8", features = ["htmx"] }
maud = "0.27"
```

Mount the module, link the CSS, and call the components:

```rust
mod figma;

use autumn_web::prelude::*;
use autumn_web::security::CsrfToken;
use figma::Attrs;
use figma::components::todo_row;

fn layout(csrf: Option<&CsrfToken>, content: Markup) -> Markup {
    html! {
        (maud::DOCTYPE)
        html {
            head {
                @if let Some(csrf) = csrf {
                    meta name="csrf-token" content=(csrf.token());
                }
                // Load figma.css after Autumn's widget CSS, so its tokens win.
                link rel="stylesheet" href="/static/css/figma.css";
                script src=(asset_url("js/htmx.min.js")) defer {}
                script src=(asset_url("js/autumn-htmx-csrf.js")) defer {}
            }
            body { (content) }
        }
    }
}

#[get("/")]
#[public]
async fn index(csrf: Option<CsrfToken>) -> Markup {
    let rows = html! {
        (todo_row(false, "Buy milk", &Attrs::NONE))
    };
    layout(csrf.as_ref(), figma::screens::todos(rows, "1 item left"))
}

/// An htmx fragment: the new row for `hx-swap="beforeend"`.
#[post("/todos")]
#[public]
async fn create() -> Markup {
    todo_row(false, "New todo", &Attrs {
        hx_delete: Some("/todos/7"),
        hx_swap: Some("outerHTML"),
        ..Attrs::NONE
    })
}
```

The CSRF meta tag lets htmx requests pass in the prod profile, where Autumn
turns CSRF protection on. `examples/todo-app` is a full Autumn app made this
way.

## What maps to what

| Figma | Output |
|---|---|
| Page | A Rust module (`src/figma/<page>.rs`) |
| Component | `pub fn name(props..., attrs: &Attrs) -> Markup` |
| Component set (variants) | An `enum` per variant property, and a `match` in the function |
| TEXT property | `&str` parameter |
| BOOLEAN property | `bool` parameter with `@if` |
| INSTANCE_SWAP property | `Markup` parameter in place of the layer |
| SLOT property | `Markup` parameter inside the slot's own box |
| Instance of a local component | A call of its function, with the instance values and its place in the parent |
| Instance of a library component | Inlined markup |
| Top-level frame (also inside a section or group) | A screen function |
| Auto layout | Flexbox (direction, gap, padding, alignment, wrap, Fill/Hug/Fixed) |
| Grid layout | CSS grid (tracks, cells, spans, child alignment) |
| No auto layout | Absolute positions (with a warning) |
| Fills, strokes, radius, shadows, blur, opacity | CSS on a generated class |
| Strokes | Inset shadow (`INSIDE`), outline (`OUTSIDE`, `CENTER`, dashed), or border (when the stroke takes layout space) |
| Text style | `font`, `letter-spacing`, `text-align`, ... |
| Variables | CSS custom properties; modes go to `[data-theme="<mode>"]` |
| A layer with a variable mode | `data-theme="<mode>"` on its element |
| Styles (fill, text, effect) | CSS custom properties at first use |
| Vector icons, image fills | `static/figma/*.svg` / `*.png`, rendered as `img` |

The output uses classes in one CSS file. It has no inline styles and no
inline event handlers, so it works under Autumn's strict
Content-Security-Policy. Maud escapes all design text.

## Add htmx with bindings

The design owns the look. `figma.toml` owns the behavior. A `[[bind]]` entry
selects one layer by path (`Page > Frame > Layer`) or by node id:

```toml
[[bind]]
path = "Screens > Todos > New todo"
tag = "form"
attrs = { "hx-post" = "/todos", "hx-target" = "#todo-list", "hx-swap" = "beforeend" }

[[bind]]
path = "Screens > Todos > List"
tag = "ul"
id = "todo-list"
slot = "items"        # replace the children with a `Markup` parameter

[[bind]]
node = "2:9"
text = "remaining"    # replace the text with a `&str` parameter

[[bind]]
path = "Screens > Todos > New todo > Add"   # an instance of the Button component
attrs = { type = "submit" }                 # goes to the component root through `Attrs`
```

| Key | Effect |
|---|---|
| `tag` | The HTML tag. An exact layer name also sets it: `button`, `nav`, `header`, `footer`, `main`, `section`, `article`, `aside`, `form`, `ul`, `ol`, `li`, `input`, `textarea`, `fieldset`, `figure`, `a`, and for text `h1`–`h6`, `p`, `em`, `strong`, `small`. |
| `id` | The `id` attribute. It must match one node. |
| `attrs` | Other attributes. `style`, `class`, `id`, `on*`, and `hx-on*` are not allowed. |
| `slot` | Replace the children with a `Markup` parameter. |
| `text` | Replace the text with a `&str` parameter. A text layer with tag `input` or `textarea` shows its text as the placeholder. |

- A binding that matches no layer is an error, so a typo does not pass
  silently.
- A layer name with `>` cannot go in a path. Use `node = "<id>"`.
- Layers inside an instance of a local component take no binding. Bind the
  component definition.
- A binding on such an instance sets `id` and `attrs` only. The `attrs`
  names must be `Attrs` fields (see below).

Every component takes `&Attrs`. Set any field from the call site: `id`,
`class`, `name`, `value`, `type`, `href`, `aria-label`, `role`, `title`,
`disabled`, and 20 `hx-*` attributes (for example `hx-post`, `hx-target`,
`hx-swap`, `hx-swap-oob`, `hx-trigger`, `hx-vals`, `hx-headers`). For other
attributes, bind the component definition.

## Tokens and the Autumn theme

Figma variables become CSS custom properties. Rename one to an Autumn token,
and the stock Autumn widgets take your brand too:

```toml
[tokens]
"color/brand/primary" = "--primary"
```

Autumn's tokens are `--bg`, `--surface`, `--text`, `--text-muted`,
`--border`, `--primary`, `--primary-hover`, `--primary-light`, `--danger`,
`--danger-hover`, `--danger-light`, `--success`, `--success-light`,
`--warning`, `--warning-light`, `--radius`, `--shadow`, and
`--font-family`.

- A Figma variable that gets one of these names without a `[tokens]` entry
  gives a warning, because it changes Autumn widgets.
- A dark mode that maps only `--primary` leaves `--bg` and `--surface`
  light. Map them too.
- Load `figma.css` after Autumn's widget CSS.

The variables API needs a Figma Enterprise plan and the
`file_variables:read` scope. Without it, `pull` gives a note, and tokens
come from styles only.

## `figma.toml`

| Key | Default | Meaning |
|---|---|---|
| `file_key` | (none) | The file key, or the full Figma URL |
| `pages` | all pages | The pages to generate |
| `snapshot` | `figma/snapshot.json` | The file snapshot |
| `variables_snapshot` | `figma/variables.json` | The variables snapshot |
| `out_dir` | `src/figma` | The Rust module directory |
| `css_path` | `static/css/figma.css` | The CSS file |
| `assets_dir` | `static/figma` | The asset directory |
| `assets_url` | `/static/figma` | The URL of the asset directory. Autumn's CSP allows only same-origin images, so keep a `/` path. |
| `class_prefix` | `fg` | The CSS class prefix |
| `[tokens]` | | Token renames |
| `[[bind]]` | | Bindings |

The token never goes in this file. It comes only from `FIGMA_TOKEN` (or
`FIGMA_OAUTH_TOKEN` for an OAuth token).

## Commands

| Command | Does |
|---|---|
| `autumn-figma init [--file-key <key or URL>]` | Writes a starter `figma.toml`. Never overwrites. |
| `autumn-figma pull` | Fetches the snapshots and assets. Needs `FIGMA_TOKEN`. |
| `autumn-figma generate` | Writes Rust and CSS from the snapshot. Offline. |
| `autumn-figma generate --check` | Writes nothing. Exits 2 when the files differ. |
| `autumn-figma sync` | `pull`, then `generate`. |

All commands take `--config <path>` and `--quiet` (warnings and errors
only). Exit codes: 0 success, 1 error, 2 drift.

- `pull` writes the snapshots and the assets only when every request
  succeeds.
- `pull` waits for a short rate limit (up to 60 seconds, 3 times). Figma
  has few file requests on View and Collab seats, so pull only when the
  design changes.

Commit the snapshot and the generated files. Then a design change shows as
a diff in review, and CI can check it with no token:

```yaml
- run: cargo install --locked --git https://github.com/autumn-foundation/autumn-figma
- run: autumn-figma generate --check
```

Do not edit files in `out_dir`. `generate` overwrites them, and deletes
stale generated files there. It never overwrites or deletes a file without
the generated header. Put your own code outside `out_dir`.

## Limits

- The output is close to the design, not pixel perfect.
- The generator does not support gradients, blend modes, rotation, or mixed
  text styles in one layer. A warning names each one.
- The generator ignores a text override on an instance without a TEXT
  property. Add a TEXT property in Figma.
- The generator does not pass overrides of an exposed nested instance. A
  warning names it.

## One-off screens

For a single screen that you do not need to keep in sync, the Figma MCP
server and an agent work too: `get_design_context` gives a reference, and
the agent ports it to Maud. For a design that changes, use this tool.

## Start a Figma file from an app

To go the other way once, from an Autumn app to a new Figma file, see
[Import an Autumn app into Figma](docs/import-to-figma.md). It works on a
Professional plan.

## License

Apache-2.0
