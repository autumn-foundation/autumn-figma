# Planning — autumn-figma

## Goal

Put a Figma design into an Autumn app as Maud components with htmx
behavior. Make the result repeatable: when the design changes, run one
command and review the diff.

## The question: a runtime plugin or a CLI?

A CLI. The design is a build input, like a migration or a schema. It is not
a request-time dependency. See [ADR 0001](adr/0001-cli-codegen.md).

The tool is `autumn-figma`. It has two steps and a check:

1. `pull` reads the Figma REST API and writes a JSON snapshot. Only this step
   needs the network and `FIGMA_TOKEN`.
2. `generate` reads the snapshot and `figma.toml`. It writes Rust (Maud)
   modules, one CSS file, and nothing else. It is offline and deterministic.
3. `generate --check` writes nothing. It fails when the committed code does
   not agree with the snapshot. CI runs it.

`sync` runs `pull` then `generate`. `init` writes a starter `figma.toml`.

The generated code depends on `maud` only. The app does not depend on this
crate at run time.

## Brainstorming

Ideas, not filtered:

- Runtime plugin that fetches Figma at boot and renders frames.
- CLI codegen: Figma JSON → Maud functions + CSS.
- Figma MCP + an agent: `get_design_context` gives React/Tailwind; the agent
  ports it to Maud by hand.
- A Figma-side plugin that exports a custom JSON with htmx annotations.
- Tokens only: Figma variables → CSS custom properties. Leave markup to
  people.
- Map Figma variables onto Autumn's own tokens (`--primary`, `--radius`) so
  Autumn widgets take the brand too.
- Components with properties → Rust functions with typed parameters.
  TEXT → `&str`, BOOLEAN → `bool`, VARIANT → `enum`, INSTANCE_SWAP / SLOT →
  `Markup`.
- Instances of local components → function calls, not copied markup.
- Auto layout → flexbox. Grid layout → CSS grid.
- htmx from layer names (`Save [hx-post=/todos]`).
- htmx from a sidecar file that the developer owns (`[[bind]]` in
  `figma.toml`).
- Each component is a fragment, so an htmx handler can return it for a swap.
- Story gallery entries for each component.
- Tailwind classes in place of a CSS file.
- Drift gate in CI.
- Icons → SVG assets.

## Reverse brainstorming

"How do we make this fail?" Each answer gives a rule.

| How to fail | Rule |
|---|---|
| Fetch Figma at request time; the site goes down with Figma. | Codegen at build time. No runtime dependency. |
| Put the token in `figma.toml`; it goes to git. | Token only from `FIGMA_TOKEN`. Reject a `token` key. |
| Send the token to the image CDN. | Send `X-Figma-Token` only to the API base. |
| Output changes on each run; every PR has noise. | Sorted maps, stable names, no timestamps. Golden test. |
| Hand edits in generated files are lost. | Header marks the file. Users write code outside `out_dir`. |
| Delete a user file in `out_dir`. | Delete only files with the generated header. |
| Inline `style=""`; strict CSP breaks the page. | Classes in one CSS file. No inline styles. |
| A layer name with `"` or `}` breaks the Rust build or injects code. | Escape strings with Rust literal escapes. Sanitize identifiers. Allow-list tags and attribute names. |
| `PreEscaped` with design text; XSS. | Never emit `PreEscaped`. Maud escapes all text. |
| Designers must type htmx in layer names. | htmx lives in `figma.toml` `[[bind]]`, owned by developers. |
| A typo in a binding path silently does nothing. | An unmatched binding is an error. |
| A designer re-orders layers; bindings break. | Match by layer path (names) or by node id. |
| Duplicate markup for each instance. | Local instances become calls. |
| A screen is fixed at 1440px. | Root frames get `width: 100%` and `max-width`. |
| No auto layout; output is not responsive. | Absolute fallback works, with a warning. |
| Enterprise-only variables API returns 403; the pull fails. | 403 on variables is a notice. Fall back to styles. |
| Unknown node types or new API fields crash the parser. | Tolerant serde: defaults, unknown enum values map to `Other`. Skip with a diagnostic. |
| Huge files are slow to pull. | `pages` in config limits the fetch with `ids=`. |
| A component root needs `hx-post` from the caller. | Each component takes `&Attrs` for root attributes. |

## Six thinking hats

- **White (facts).**
  - The Figma REST API has `GET /v1/files/:key` (with `ids`),
    `GET /v1/files/:key/variables/local`, and `GET /v1/images/:key`.
  - The variables API needs an Enterprise plan and the
    `file_variables:read` scope.
  - Nodes carry auto-layout fields, component properties, property
    references, bound variables, and style ids.
  - Autumn apps use `maud` 0.27 directly. `autumn new` adds it.
  - Autumn widgets take their colors from `:root` tokens.
  - Autumn serves pages under a strict CSP.
  - The Autumn CLI has no external-subcommand hook.
- **Red (feelings).** Developers distrust "design to code" output that is a
  pile of absolute `div`s. Designers do not want to learn htmx. Both want
  the design to stay the source of truth for look, and code to stay the
  source of truth for behavior.
- **Black (risks).** Figma's model is richer than CSS (vector networks,
  blend modes, gradients). Output will not be pixel perfect. The API needs a
  token we cannot use in tests. Variables need Enterprise. Large designs
  give large modules.
- **Yellow (benefits).** Tokens alone remove most drift. Components with
  typed parameters give compile-time checks when the design changes a
  property. Fragments fit htmx swaps. CI drift check makes the design
  reviewable in PRs.
- **Green (creative).** Map Figma tokens to Autumn tokens, so stock Autumn
  widgets match the brand. SLOT properties map to `Markup` parameters. A
  future `autumn figma` subcommand through an external-subcommand hook.
  Generate story-gallery entries.
- **Blue (process).** Plan → RED tests from the ACs → GREEN → refactor →
  multi-angle review → AC evidence. Fixtures stand in for the API. Golden
  files prove determinism. A compile test proves the output builds and
  renders.

## Decisions

- Library + binary in one crate. Binary name `autumn-figma`.
- Snapshot is normalized JSON (sorted keys, pretty) for clean diffs.
- One Rust module per Figma page, plus `mod.rs` with the `Attrs` type.
- One CSS file: tokens, a base rule, then one rule per styled node.
- Class names: `<prefix>-<component>__<layer>`. Default prefix `fg`.
- Tokens: variables first, styles second, literal values last. The
  `[tokens]` table renames a token (for example to Autumn's `--primary`).
- Non-default variable modes go under `[data-theme="<mode>"]`. A mode named
  "dark" also goes under `prefers-color-scheme: dark`.
- Tags come from a binding, then from an exact layer-name match
  (`button`, `nav`, `h1`, ...), then `div` (containers) or `span` (text).
- Bindings: `tag`, `id`, `attrs`, `slot` (replace children with a `Markup`
  parameter), `text` (replace text with a `&str` parameter). Match by
  `path` (`Page > Frame > Layer`) or `node` id.
- A binding on a local instance sets `Attrs` fields on the call.
- Icon-like nodes (only vector descendants) export as SVG. Nodes with an
  image fill export as PNG. Both render as `img`.
- Exit codes: 0 success, 1 error, 2 drift (`--check`).
- No Verus here: the crate has no unsafe or concurrent state machine. The
  invariants (determinism, escaping, identifier validity) have property tests.

## Acceptance criteria

| ID | Criterion |
|---|---|
| AC-01 | `pull` fetches the file (and variables when allowed) and writes normalized snapshots. |
| AC-02 | The token comes only from `FIGMA_TOKEN`. A `token` key in `figma.toml` is an error. The token is not in logs, `Debug`, or requests to other hosts. |
| AC-03 | A 403 on variables is a notice, not a failure. Other HTTP errors are typed errors (401/403, 404, 429 with retry hint). |
| AC-04 | `generate` runs offline from the snapshot and writes Rust modules and one CSS file. |
| AC-05 | Output is byte-identical across runs (golden test + property test). |
| AC-06 | `generate --check` writes nothing, exits 2 on drift (changed, missing, or stale files), and exits 0 when in sync. |
| AC-07 | Generation deletes stale generated files and never touches files without the generated header. |
| AC-08 | Variables become CSS custom properties: colors, numbers (px or unitless by scope), strings, aliases as `var()`, modes as `[data-theme]` (+ dark media query). |
| AC-09 | Styles (fill, text, effect) become tokens when variables are absent. `[tokens]` renames a token. |
| AC-10 | Auto layout becomes flexbox (direction, gap, padding, alignment, wrap, child sizing). Grid becomes CSS grid. No auto layout falls back to absolute positions with a warning. |
| AC-11 | Fills, strokes, radius, shadows, opacity, clipping, and typography become CSS. Bound variables and styles give `var(--token)`. |
| AC-12 | Components become `pub fn ... -> Markup`. TEXT → `&str`, BOOLEAN → `bool` with `@if`, INSTANCE_SWAP/SLOT → `Markup`, VARIANT → `enum` + `match`. |
| AC-13 | Instances of local components become calls with arguments from the instance. Remote instances are inlined. |
| AC-14 | Bindings apply `tag`, `id`, `attrs` (htmx), `slot`, and `text` by path or node id. An unmatched or invalid binding is an error. |
| AC-15 | Each component takes `&Attrs`, so a caller (or a binding on an instance) sets root attributes such as `hx-post`. |
| AC-16 | Safety: no inline styles or inline event handlers, no `PreEscaped`, escaped literals, sanitized identifiers and classes, allow-listed tags and attribute names. |
| AC-17 | The generated code compiles and renders the expected HTML (compile + render test). |
| AC-18 | Icon-like nodes and image fills export as assets during `pull`, and render as `img`. |
| AC-19 | Hidden nodes are skipped (unless a BOOLEAN property shows them). Unsupported nodes give a diagnostic. Bad JSON is an error, never a panic. |
| AC-20 | `init` writes a starter `figma.toml` and does not overwrite one. |
| AC-21 | The generated code works in a real Autumn app: a page renders, and the htmx attributes are present. |
| AC-22 | Docs: README quickstart, ADR, this plan, CLAUDE.md, CHANGELOG. ASD-STE100 style. |
| AC-23 | Gates: `fmt`, `clippy` pedantic + nursery with `-D warnings`, tests, CI workflow. No `unwrap`/`expect`/`panic` in production code. |

## Test strategy

- Unit tests per module in `src/<module>/tests.rs`.
- Fixtures in `tests/fixtures/`: a todo design that uses each feature.
- `tests/golden.rs`: generate from the fixture and compare to
  `tests/golden/`. `UPDATE_GOLDEN=1` rewrites the files.
- `tests/golden_render.rs`: includes the golden Rust modules, renders them
  with `maud`, and checks the HTML.
- `src/cli/tests.rs`: runs the CLI in-process for `init`, `generate`,
  `--check`, `pull`, and `sync` against a local stub API.
- `tests/binary.rs`: runs the real binary for the exit codes.
- `tests/fixtures/edge`: unusual and hostile designs. Its golden output
  also builds under pedantic clippy.
- Property tests: names are valid identifiers; any layer name gives code
  that is stable and escaped.

## Review

Four agents reviewed the first version: generator correctness, security,
Figma API fidelity, and docs with Autumn fit. Each confirmed finding has a
fix and a regression test. The commits "fix: security review findings",
"fix: generator correctness review findings", "fix: Figma API fidelity
review findings", and the docs and UX commit hold them.

## Future work

- An external-subcommand hook in the Autumn CLI, so `autumn figma sync` works.
- Overrides of exposed nested instances (now a warning).
- Rotation, constraints, and min/max sizes (rotation is now a warning).
- Gradients, blend modes, and rich text runs.
- Story-gallery entries for each component.
- A Tailwind output mode.
