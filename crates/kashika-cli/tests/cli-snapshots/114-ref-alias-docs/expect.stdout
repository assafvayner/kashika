# Limits

## Raster size

PNG, JPEG and WebP output is 1 px per diagram unit, capped at 16,384 px per side and 100,000,000 px in total. A larger canvas exits 3 without writing; use `.svg` output or a smaller `size`.

Attached PNG and JPEG images (also those embedded in an SVG) are limited to 8,192 px per side and 32,000,000 px in total, read from the header; larger ones are a `file` error.

## Files

An image or SVG file (`image`, `icon=`, `shape ... from`) may be at most 16 MiB and an imported library at most 1 MiB. An SVG whose elements nest more than 256 deep, also one embedded in an SVG, is refused. Imports nest at most 32 deep, and one diagram registers at most 1,000 library instances (a file under one namespace; each file is still read once). Each of these is a `file` error.

## Not supported in this build

- Never in v1: auto-layout, `fix`, arithmetic in literals, named guides (use junctions without `dot`), remote URLs, themes, text wrapping (use `\n`).
