# Changelog

## Unreleased

- Non-ASCII whitespace (NBSP, zero-width space, …) is a `parse` error, `unexpected non-ASCII whitespace`, instead of joining the identifiers around it.
- A second statement on the same line (`box a … box b …`) is a `parse` error, `expected end of statement`, with the help `each statement starts on its own line`; the second statement still parses.
- `fmt` keeps the file's line endings: CRLF when the first line ends in CRLF, else LF. A lone CR is whitespace.
- `route` on a file with errors reports them like `check` and exits 2 instead of suggesting paths; `--line` on a broken line now shows the real diagnostics.
- A closed stdout or stderr (`kashika check f.kash | head -1`) no longer panics; the exit code is unchanged.
- `inspect --ports` and the `port-crowding` help name the side a line leaves from when two sides' anchors land on the same outline point (such as on a `diamond`).
- An imported library that is not valid UTF-8 is a `file` error saying it is not valid UTF-8.
- `route`: lanes beside a contained SVG-file shape follow its anchor frame, and `arc --radius 0` counts its sharp corners as segments when ranking.

## 0.1.0-dev0

First preview release on crates.io. The CLI is the `kashika` crate; the library crates (`kashika-diag`, `kashika-geom`, `kashika-text`, `kashika-syntax`, `kashika-model`, `kashika-check`, `kashika-route`, `kashika-render`) are published at the same version and have no stable API yet.

- The `.kash` language: explicit coordinates and frames, boxes, built-in and custom shapes, SVG shapes, images, bundled icons, groups, styles, anchors and anchor arithmetic, straight, orthogonal and curved line segments with markers and labels, imports, and a `kashika="x.y.z"` minimum-version check.
- Text with inline markup, sublabels and blocks, measured and drawn as glyph outlines with the bundled Inter, JetBrains Mono and Twemoji fonts, so every viewer and the checker agree.
- `kashika check`: errors plus geometry lints (overlaps, crossings, lines through boxes, near misses, label collisions, low contrast and more), with `allow=` suppression, metrics, and text or JSON output.
- `kashika render` to deterministic SVG, PNG, JPEG and WebP, with a `--debug` overlay.
- `kashika inspect` (anchors, free intervals, ports), `kashika route` (ranked path suggestions), `kashika measure`, `kashika fmt`, `kashika icons` and `kashika ref`, the reference guide for agents embedded in the binary.
