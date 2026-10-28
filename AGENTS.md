# AGENTS.md

Guide for AI coding agents working on the Kashika codebase. Kashika is a text-to-diagram language (`.kash` files, no auto-layout) with a Rust CLI, `kashika`, that checks and renders to SVG. The normative spec is [`docs/spec/language.md`](docs/spec/language.md). Agents that *use* Kashika to draw diagrams should read [`skills/kashika/SKILL.md`](skills/kashika/SKILL.md) instead.

## Crates

In dependency order (`crates/`):

- `kashika-diag`: `Diagnostic`, severity, spans and the lint code list.
- `kashika-geom`: points, rects and path math (flattening, length, rounded corners), intersections, unit paths and convex hulls.
- `kashika-text`: bundled fonts (`fonts/`, OFL), inline markup, measurement, block layout and glyph outlines.
- `kashika-syntax`: lexer, parser, AST and the whitespace-only formatter.
- `kashika-model`: resolver; turns the AST into a `Scene` with anchors, attributes, styles and laid-out text (`compile`, `compile_with`). Also the built-in shape library (`shapes.rs`), outlines (`form.rs`), sandboxed file loading (`loader.rs`), imports (`library.rs`), SVG/PNG/JPEG assets (`assets.rs`) and the bundled icons (`icons/`).
- `kashika-check`: warning lints (one file each under `src/lints/`), suppression, metrics, free intervals and ports, built on a shared `Ctx` of shape polygons and flattened line pieces. `Ctx::boxes()` is boxes and images: the opaque shapes lines must avoid.
- `kashika-route`: `kashika route` path suggestions. Each candidate is compiled as a real `line` and scored with `kashika_check::check`, so it ranks by the `cross` and `near`/`shared-segment` findings `check` reports (`through` drops it), then segments and length.
- `kashika-render`: `Scene` to SVG and PNG (resvg), with images and icons embedded as `data:` URIs, and the `render --force`/`--debug` overlay.
- `kashika-cli`: the `kashika` binary (`check`, `render`, `inspect`, `route`, `measure`, `fmt`, `icons`, `ref`), text and JSON reports. `ref` pages are Markdown in `crates/kashika-cli/ref/`.

## Commands

```sh
cargo fmt --all        # stable rustfmt, never +nightly
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p kashika-cli -- check examples/upload/upload.kash
cargo run -p kashika-cli -- render examples/upload/upload.kash -o target/upload.svg
cargo run -p kashika-cli -- measure "**Payments** API" --size 14
cargo run -p kashika-cli -- inspect examples/upload/upload.kash --ports
cargo run -p kashika-cli -- icons --search server
cargo run -p kashika-cli -- fmt --check examples/upload/upload.kash
cargo run -p kashika-cli -- route examples/upload/upload.kash --line api1_queue arc
cargo run -p kashika-cli -- ref
```

## Adding an example

Create `examples/<name>/<name>.kash` (it must check with 0 errors and 0 warnings), then generate both renders from the repo root and add an entry to `examples/README.md`:

```sh
for ext in svg png; do cargo run -p kashika-cli -- render examples/<name>/<name>.kash -o examples/<name>/<name>.$ext; done
```

Commit the `.svg` and `.png` with the source. `crates/kashika-cli/tests/examples.rs` fails if a render is missing or stale; rerun the loop after any renderer or language change.

## Invariants

- SVG output is deterministic: the same input always yields the same bytes.
- Every diagnostic carries a code listed in section 7 of the spec and a source span.
- The parser and resolver recover and report diagnostics; they never panic on bad input.
- Spec features not yet implemented report the `unsupported` diagnostic.
- Every lint has a positive and a negative test in `crates/kashika-check/tests/`.
- A language or CLI change updates the spec, the tests, the `ref` pages and `skills/kashika/SKILL.md` together. Every ```` ```kash ```` block in `ref` is a fixture that must check clean and be in `fmt` form.
- `fmt` changes only whitespace; it never changes tokens.
- `route` never edits files.
- All file access in the model goes through `loader.rs`, which enforces local-only paths, the project root and the search path. The model never reads the environment; the CLI passes `KASHIKA_ICON_PATH` in `Options`.
- Embedded SVG is always the usvg re-serialization, never the input bytes.
- Text is measured and drawn only with the fonts in `crates/kashika-text/fonts/`. SVG text is glyph outlines (`<path aria-label=…>`), never `<text>`, so checks and every viewer agree.

## Conventions

- American spelling.
- No comments that restate the code.
- Lints take `&Ctx` and never mutate the scene. Outlines come from `kashika_check::outline::polygon` (which uses `Shape::form` for non-rect shapes) and text extents from `kashika_check::text_box` (including `below_box` for `text-area=below`), so new shapes plug in there.
- Put new tests next to similar ones: integration tests in `crates/*/tests/`, CLI fixtures in `crates/kashika-cli/tests/fixtures/`.
- Planning docs under `docs/superpowers/` and `.superpowers/` are local-only; do not commit them.
