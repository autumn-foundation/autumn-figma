# ADR 0001 — Figma to Maud is a CLI code generator

- Status: accepted
- Date: 2026-10-26

## Context

We want Figma designs in Autumn apps as Maud markup with htmx behavior.
The other Autumn plugins (`autumn-plugin-discord`, `autumn-plugin-agent`)
are runtime `Plugin`s. A design is different: it changes when a designer
edits it, not when a request comes in.

Options:

1. A runtime `Plugin` that fetches the design and renders it.
2. An agent flow with the Figma MCP server (`get_design_context`), ported to
   Maud by hand.
3. A CLI that writes Maud code and CSS from a snapshot of the design.

## Decision

Option 3. The crate ships the `autumn-figma` binary and a library.

- `pull` is the only step with network access and a token.
- `generate` is offline and deterministic. Its output is plain Rust that
  depends on `maud` only.
- `generate --check` gives a drift gate for CI.
- Behavior (htmx) lives in `figma.toml` bindings that developers own. The
  design owns the look.

## Consequences

- No runtime dependency on Figma, its uptime, or its token.
- Design changes show as reviewable diffs in Rust and CSS.
- Typed component functions turn a renamed Figma property into a compile
  error at the call site.
- The output is not pixel perfect. Gradients, blend modes, and vector
  networks are partial or not supported.
- The Autumn CLI has no external-subcommand hook, so users run
  `autumn-figma`, not `autumn figma`. A hook is future work.
- The crate and the repository are `autumn-figma`, not
  `autumn-plugin-figma`. The `autumn-plugin-*` names are for runtime
  `Plugin`s.
- Option 2 stays useful for one-off screens. It is not repeatable, so it
  does not replace this tool.
