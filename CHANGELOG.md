# Changelog

## 0.1.0-dev0

First preview release on crates.io. The CLI is the `kashika` crate; the library crates (`kashika-diag`, `kashika-geom`, `kashika-text`, `kashika-syntax`, `kashika-model`, `kashika-check`, `kashika-route`, `kashika-render`) are published at the same version and have no stable API yet.

- The `.kash` language: explicit coordinates and frames, boxes, built-in and custom shapes, SVG shapes, images, bundled icons, groups, styles, anchors and anchor arithmetic, straight, orthogonal and curved line segments with markers and labels, imports, and a `kashika="x.y.z"` minimum-version check.
- Text with inline markup, sublabels and blocks, measured and drawn as glyph outlines with the bundled Inter, JetBrains Mono and Twemoji fonts, so every viewer and the checker agree.
- `kashika check`: errors plus geometry lints (overlaps, crossings, lines through boxes, near misses, label collisions, low contrast and more), with `allow=` suppression, metrics, and text or JSON output.
- `kashika render` to deterministic SVG, PNG, JPEG and WebP, with a `--debug` overlay.
- `kashika inspect` (anchors, free intervals, ports), `kashika route` (ranked path suggestions), `kashika measure`, `kashika fmt`, `kashika icons` and `kashika ref`, the reference guide for agents embedded in the binary.
