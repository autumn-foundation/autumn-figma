# Import an Autumn app into Figma

`autumn-figma` reads a design and writes code. This guide goes the other
way, one time: it starts a Figma file from an Autumn app. After that, the
design is the source, and `pull` and `generate` keep the code in sync.

The Figma MCP server (`use_figma`) does the work through the Plugin API.
It works on a Professional plan. It does not need Enterprise.

Example file: Autumn widgets and the `examples/todo-app` list screen, made
with these steps.

## Steps

1. **Tokens.** Make one variable collection per token source. Name each
   variable `color/<token>`. Set its WEB code syntax to the CSS name, for
   example `var(--primary)`.
2. **Styles.** Make one paint style per color variable, with the same
   name. Bind the style paint to the variable. Make an effect style for
   each shadow and a text style for each type role.
3. **Components.** Build each widget from its CSS. Apply the styles, not
   raw colors. Bind the corner radius to the `radius` variable. Make a
   variant for each state that the CSS has (`:hover`, `:focus`,
   `--invalid`, `:disabled`).
4. **Screens.** Run the app with seed data. Take a screenshot as the
   reference. Build the screen from instances. Compare the two
   screenshots.

## Why the styles have the variable names

The variables API (`file_variables:read`) needs Enterprise. Without it,
`pull` cannot get variable names, so a bound variable gives no token. The
paint style still gives one: `color/primary` becomes `--color-primary`.
With Enterprise, the variable gives the same name. Both paths give the
same CSS.

## `figma.toml`

```toml
file_key = "<file key or URL>"
pages = ["Components", "Screens"]

[tokens]
"color/bg" = "--bg"
"color/surface" = "--surface"
"color/text" = "--text"
"color/text-muted" = "--text-muted"
"color/border" = "--border"
"color/primary" = "--primary"
"color/primary-hover" = "--primary-hover"
"color/primary-light" = "--primary-light"
"color/danger" = "--danger"
"color/danger-hover" = "--danger-hover"
"color/danger-light" = "--danger-light"
"color/success" = "--success"
"color/success-light" = "--success-light"
"color/warning" = "--warning"
"color/warning-light" = "--warning-light"
"color/surface-muted" = "--surface-muted"
"color/success-text" = "--success-text"
"color/warning-text" = "--warning-text"
"color/info" = "--info"
"color/info-light" = "--info-light"
"radius" = "--radius"
"font-family" = "--font-family"
"shadow" = "--shadow"
```

Tailwind colors need no entry. The style `color/stone-50` becomes
`--color-stone-50`, which is the Tailwind v4 name.

## Limits

- A radius bound to a variable becomes a pixel value without Enterprise.
  No radius style exists in Figma.
- Figma has no `system-ui`. Inter stands in for it. The `font-family`
  variable keeps the CSS value in its description.
- htmx states (a swapped row, a validation error) are separate variants
  or frames. Capture each state on its own.
- A screen needs seed data. An empty database gives an empty screen.
- Code that picks a value at run time (the avatar palette) shows one
  value in Figma.
