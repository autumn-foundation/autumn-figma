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

Run it:

```sh
cargo run                                # http://localhost:3000
cargo run --manifest-path ../../Cargo.toml -- --config figma.toml generate --check
```

Type a todo and press **Add**. htmx posts the form and appends the
`todo_row` fragment to `#todo-list`. Double-click a row to delete it. The
count changes out of band (`hx-swap-oob`).

Check it in a browser (CI runs this in the prod profile):

```sh
cd e2e && npm install && npx playwright install chromium
BASE_URL=http://127.0.0.1:3000 npm run smoke
```

Autumn serves htmx and `autumn-htmx-csrf.js`. The layout puts the CSRF
token in a `csrf-token` meta tag, so the htmx requests also pass in the
prod profile, where CSRF protection is on.
