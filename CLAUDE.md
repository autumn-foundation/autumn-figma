# CLAUDE.md — autumn-plugin-figma

## Commands

```sh
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-targets --all-features
UPDATE_GOLDEN=1 cargo test --test golden   # after an intended output change; review the diff
```

All three gates must be green before push. `Cargo.lock` is committed.

## Architecture

| Module | Owns |
|---|---|
| `config` | `figma.toml`, `Binding`, `FIGMA_TOKEN` lookup, file key from a URL |
| `model` | Figma REST payloads; tolerant serde (`Other` for unknown values) |
| `client` | `FigmaClient`: files, variables, images, downloads |
| `pull` | Fetch snapshots (page-scoped) and assets |
| `names` | Slugs, identifiers, Rust and CSS literals, `Allocator` |
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
- The token travels only in `FIGMA_TOKEN` and only to the API base URL.
  Errors never hold a URL query (`reqwest::Error::without_url`).
- No `unwrap`/`expect`/`panic` in production code. Tests may use them.
- Tests never touch the process env: `cli::run` takes an env lookup.
- Unit tests live in `src/<module>/tests.rs`. The REST stub is
  `src/test_support.rs`. The fixture is `tests/fixtures/todo`.
- Docs and comments in ASD-STE100 style: short sentences, active voice,
  simple present tense.
