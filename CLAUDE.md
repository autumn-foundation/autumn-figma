# CLAUDE.md — autumn-figma

## Commands

```sh
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_BUILD_JOBS=2
export CARGO_TARGET_DIR=~/workspace/autumn-arena/target   # shared arena dir; never cargo clean
export TMPDIR=~/workspace/.tmp-cargo                      # /tmp is a 512MB tmpfs

cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-targets --all-features
UPDATE_GOLDEN=1 cargo test --test golden   # after an intended output change; review the diff
cargo run -- --config examples/todo-app/figma.toml generate --check
```

All gates must be green before push. CI runs the latest stable clippy, so
also run `rustup run stable cargo clippy ...` when it is newer than the
local toolchain. `Cargo.lock` is committed.

The fixtures come from scripts: `tests/fixtures/gen_todo_fixture.py` and
`tests/fixtures/gen_edge_fixture.py`. Change a script, run it, then update
the golden files.

## Architecture

| Module | Owns |
|---|---|
| `config` | `figma.toml`, `Binding`, `FIGMA_TOKEN` lookup, file key from a URL |
| `model` | Figma REST payloads; tolerant serde (`Other` for unknown values) |
| `client` | `FigmaClient`: files, variables, images, downloads |
| `pull` | Fetch snapshots (page-scoped) and assets |
| `names` | Slugs, identifiers, Rust and CSS literals, `printable`, `comment_text`, `Allocator` |
| `diagnostics` | `Diagnostic` notes; `Display` escapes terminal controls |
| `tokens` | Variables and styles to CSS custom properties |
| `style` | One node to CSS declarations |
| `assets` | Which nodes export as SVG or PNG |
| `ir` | The intermediate representation |
| `lower` | Pass one: plan functions and signatures. Pass two: lower bodies, apply bindings |
| `emit` | IR to Rust (Maud) and CSS text |
| `generate` | Orchestration, write, drift, stale files |
| `cli` | `init`, `pull`, `generate [--check]`, `sync` |

Data flow: `pull` → snapshot → `lower` → `emit` → files. Only `pull` uses
the network.

## Rules

- Output must be deterministic: `BTreeMap` only, no time, no `HashMap`
  iteration. `tests/golden.rs` and the property tests guard this.
- Generated code must compile under pedantic + nursery clippy.
  `tests/golden_render.rs` builds the golden files with this crate's lints.
- Generated `mod.rs` must be rustfmt-clean. Page modules have
  `#[rustfmt::skip]`.
- Never emit inline styles or `PreEscaped`. All design text goes through
  `names::rust_str`; all CSS strings through `names::css_str`.
- Delete only files whose first line has `emit::HEADER_MARK`.
- The token travels only in `FIGMA_TOKEN` (or `FIGMA_OAUTH_TOKEN`) and
  only to the API base URL. The API client follows no redirect. The base
  URL is `https` (plain `http` only for localhost). Errors never hold a URL
  query (`reqwest::Error::without_url`).
- Design text is untrusted. Comments go through `names::comment_text`,
  terminal output through `names::printable`.
- Never overwrite a file without `emit::HEADER_MARK`.
- `tests/golden_render.rs` builds both golden trees (`golden`,
  `golden_edge`), so generated code must pass pedantic clippy.
- No `unwrap`/`expect`/`panic` in production code. Tests may use them.
- Tests never touch the process env: `cli::run` takes an env lookup.
- Unit tests live in `src/<module>/tests.rs`. The REST stub is
  `src/test_support.rs`. The fixture is `tests/fixtures/todo`.
- Docs and comments in ASD-STE100 style: short sentences, active voice,
  simple present tense.
