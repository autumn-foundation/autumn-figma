# Changelog

## [Unreleased]

### Changed

- The crate and the repository are now `autumn-figma`. The library is
  `autumn_figma`. Install from
  `https://github.com/autumn-foundation/autumn-figma`.

### Added

- `autumn-figma` CLI: `init`, `pull`, `generate`, `generate --check`, `sync`.
- Figma components to typed Maud functions. TEXT, BOOLEAN, INSTANCE_SWAP,
  SLOT, and VARIANT properties become parameters.
- Instances of local components become calls.
- Auto layout to flexbox, grid layout to CSS grid, absolute fallback.
- Variables and styles to CSS custom properties, with modes and renames onto
  Autumn tokens.
- `[[bind]]` entries for tags, ids, htmx attributes, slots, and text
  parameters. Every component takes `&Attrs` for root attributes.
- SVG and PNG asset export.
- Drift check with exit code 2 for CI.
- Instances keep their place in the parent (Fill, fixed, absolute) through
  `Attrs.class`.
- Strokes follow `strokeAlign`, `strokesIncludedInLayout`, and per-side
  weights. Grid tracks, cells, and spans map to CSS grid.
- `data-theme` on layers that set a variable mode.
- `--quiet`, `FIGMA_OAUTH_TOKEN`, and TOML errors with line and column.
- `docs/import-to-figma.md`: start a Figma file from an Autumn app, with a
  `[tokens]` table that maps it back on a Professional plan.

### Security

- The token never follows a redirect. The API base must be `https`.
- Downloads need `https` and have a size limit.
- The generator never overwrites a file without its header.
- Design text in comments and terminal output loses control and
  direction characters.
- Bindings reject inline event handlers (`on*`, `hx-on*`).
