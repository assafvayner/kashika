# AGENTS.md

Guide for AI coding agents working on the Kashika codebase. Kashika is a text-to-diagram language (`.kash` files, no auto-layout) with a Rust CLI, `kashika`, that checks and renders to SVG. The normative spec is [`docs/spec/language.md`](docs/spec/language.md). Agents that *use* Kashika to draw diagrams should read [`skills/kashika/SKILL.md`](skills/kashika/SKILL.md) instead.

## Crates

In dependency order (`crates/`):

- `kashika-diag`: `Diagnostic`, severity, spans and the lint code list.
- `kashika-geom`: points, rects and path math (flattening, length, rounded corners).
- `kashika-text`: bundled fonts (`fonts/`, OFL), inline markup, measurement, block layout and glyph outlines.
- `kashika-syntax`: lexer, parser, AST and the whitespace-only formatter.
- `kashika-model`: resolver; turns the AST into a `Scene` with anchors, attributes, styles and laid-out text (`compile`).
- `kashika-check`: warning lints (one file each under `src/lints/`), suppression, metrics, free intervals and ports, built on a shared `Ctx` of shape polygons and flattened line pieces.
- `kashika-route`: `kashika route` path suggestions, ranked by crossings, near-misses, segments and length.
- `kashika-render`: `Scene` to SVG.
- `kashika-cli`: the `kashika` binary (`check`, `render`, `inspect`, `route`, `measure`, `fmt`, `ref`), text and JSON reports.

## Commands

```sh
cargo fmt --all        # stable rustfmt, never +nightly
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p kashika-cli -- check examples/upload.kash
cargo run -p kashika-cli -- render examples/upload.kash -o target/upload.svg
cargo run -p kashika-cli -- measure "**Payments** API" --size 14
cargo run -p kashika-cli -- inspect examples/upload.kash --ports
```

## Invariants

- SVG output is deterministic: the same input always yields the same bytes.
- Every diagnostic carries a code listed in section 7 of the spec and a source span.
- The parser and resolver recover and report diagnostics; they never panic on bad input.
- Spec features not yet implemented report the `unsupported` diagnostic.
- Every lint has a positive and a negative test in `crates/kashika-check/tests/`.
- A language change updates the spec, the tests and `skills/kashika/SKILL.md` together.
- Text is measured and drawn only with the fonts in `crates/kashika-text/fonts/`. SVG text is glyph outlines (`<path aria-label=…>`), never `<text>`, so checks and every viewer agree.

## Conventions

- American spelling.
- No comments that restate the code.
- Lints take `&Ctx` and never mutate the scene. Outlines come from `kashika_check::outline::polygon` and text extents from `kashika_check::text_box`, so new shapes plug in there.
- Put new tests next to similar ones: integration tests in `crates/*/tests/`, CLI fixtures in `crates/kashika-cli/tests/fixtures/`.
- Planning docs under `docs/superpowers/` and `.superpowers/` are local-only; do not commit them.
