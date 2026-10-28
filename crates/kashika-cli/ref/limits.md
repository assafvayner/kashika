## Limits of a clean check

A clean check covers only the geometry in the diagnostics table. It doesn't prove that arrows point the right way, that the right boxes are connected, or that the labels say what you mean.

## Not supported in this build

- Warnings: crossings, overlaps and near-misses aren't reported yet. Trace each line's coordinates yourself; `kashika route` scores its suggestions against them.
- `inspect`, `--focus`, `--show-suppressed`.
- Shapes other than `rect` and `rounded`, custom `shape`, `image`, `icon`, `import`, `icons`.
- Never in v1: auto-layout, `fix`, arithmetic in literals, named guides, remote URLs, themes, text wrapping (use `\n`).
