## Limits of a clean check

A clean check covers only the geometry in the diagnostics table. It doesn't prove that arrows point the right way, that the right boxes are connected, or that the labels say what you mean.

## Not supported in this build

- The `block-reducible`, `text-outside-shape` and `image-distort` warnings; `allow=` accepts them.
- Shapes other than `rect` and `rounded`, custom `shape`, `image`, `icon`, `import`, `icons`.
- Never in v1: auto-layout, `fix`, arithmetic in literals, named guides, remote URLs, themes, text wrapping (use `\n`).
