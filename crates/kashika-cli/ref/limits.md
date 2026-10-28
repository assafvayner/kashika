## Limits of a clean check

A clean check covers only the geometry in the diagnostics table. It doesn't prove that arrows point the right way, that the right boxes are connected, or that the labels say what you mean.

## Raster size

PNG, JPEG and WebP output is 1 px per diagram unit, capped at 16,384 px per side and 100,000,000 px in total. A larger canvas exits 3 without writing; use `.svg` output or a smaller `size`.

Attached PNG and JPEG images (also those embedded in an SVG) are limited to 8,192 px per side and 32,000,000 px in total, read from the header; larger ones are a `file` error.

## Not supported in this build

- Never in v1: auto-layout, `fix`, arithmetic in literals, named guides, remote URLs, themes, text wrapping (use `\n`).
