# Changelog

## [Unreleased]

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
