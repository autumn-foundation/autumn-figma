# Todo app from a Figma design

An Autumn app whose UI comes from `figma/snapshot.json` through
`autumn-figma`. `src/main.rs` holds only behavior: routes and data.

| Path | Source |
|---|---|
| `figma.toml` | Hand-written: pages, token renames, htmx bindings |
| `figma/*.json` | `autumn-figma pull` (here: the test fixture) |
| `src/figma/` | `autumn-figma generate` |
| `static/css/figma.css` | `autumn-figma generate` |
| `static/figma/` | `autumn-figma pull` (here: hand-made stand-ins) |
| `static/js/htmx.min.js` | htmx 2.0.4, served from the same origin |

Run it:

```sh
cargo run                                # http://localhost:3000
cargo run --manifest-path ../../Cargo.toml -- --config figma.toml generate --check
```

Type a todo and press **Add**: htmx posts the form and appends the
`todo_row` fragment to `#todo-list`. Double-click a row to delete it.
