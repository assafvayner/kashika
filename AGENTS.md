# AGENTS.md

Guide for AI coding agents working on the Kashika codebase. Kashika is a text-to-diagram language (`.kash` files, no auto-layout) with a Rust CLI, `kashika`, that checks and renders to SVG. The normative spec is [`docs/spec/language.md`](docs/spec/language.md). Agents that *use* Kashika to draw diagrams should read [`skills/kashika/SKILL.md`](skills/kashika/SKILL.md) instead.

## Crates

In dependency order (`crates/`):

- `kashika-diag`: `Diagnostic`, severity, spans and the lint code list.
- `kashika-geom`: points, rects and path math (flattening, length, rounded corners), intersections, unit paths and convex hulls.
- `kashika-text`: bundled fonts (`fonts/`, OFL, and Twemoji under CC BY 4.0), inline markup, font fallback, measurement, emoji shaping (rustybuzz), block layout, glyph outlines and COLR color layers.
- `kashika-syntax`: lexer, parser, AST and the whitespace-only formatter.
- `kashika-model`: resolver; turns the AST into a `Scene` with anchors, attributes, styles and laid-out text (`compile`, `compile_with`). Also the built-in shape library (`shapes.rs`), outlines (`form.rs`), sandboxed file loading (`loader.rs`), imports (`library.rs`), SVG/PNG/JPEG assets (`assets.rs`) and the bundled icons (`icons/`).
- `kashika-check`: warning lints (one file each under `src/lints/`), suppression, metrics, free intervals and ports, built on a shared `Ctx` of shape polygons and flattened line pieces. `Ctx::boxes()` is boxes and images: the opaque shapes lines must avoid.
- `kashika-route`: `kashika route` path suggestions. Each candidate is compiled as a real `line` and scored with `kashika_check::check`, so it ranks by the `cross` plus `group-transit` findings (each counted alike, listed apart), then the `near`/`shared-segment` findings `check` reports (`through` drops it), then segments and length.
- `kashika-render`: `Scene` to SVG, PNG, JPEG and WebP (resvg, jpeg-encoder, image-webp), with images and icons embedded as `data:` URIs, and the `render --force`/`--debug` overlay.
- `kashika-cli`: the `kashika` package and binary (`check`, `render`, `inspect`, `route`, `measure`, `fmt`, `icons`, `ref`), text and JSON reports. `ref` pages are Markdown in `crates/kashika-cli/ref/`.

## Commands

```sh
cargo fmt --all        # stable rustfmt, never +nightly
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo test -p kashika --test golden -- parse/lexical   # golden cases, filtered by path
KASHIKA_BLESS=1 cargo test -p kashika --test golden -- <case path>   # write expected files
cargo run -p kashika -- check examples/upload/upload.kash
cargo run -p kashika -- render examples/upload/upload.kash -o target/upload.svg
cargo run -p kashika -- measure "**Payments** API" --size 14
cargo run -p kashika -- inspect examples/upload/upload.kash --ports
cargo run -p kashika -- icons --search server
cargo run -p kashika -- fmt --check examples/upload/upload.kash
cargo run -p kashika -- route examples/upload/upload.kash --line api1_queue arc
cargo run -p kashika -- ref
```


## Adding a golden case

`tests/golden/` holds data-driven cases, run by `cargo test -p kashika --test golden` (harness in `crates/kashika-cli/tests/golden/`). Every directory with an `input.kash` is one test, named by its path:

- `parse/<area>/<case>/` compiles and checks the input like `kashika check` and keeps `diagnostics.txt`, plus `ast.txt` and/or `scene.json` when the header says `// dump: ast, scene`. Use `ast` when the parse tree is under test, `scene` when resolution is, and neither for diagnostic-only cases.
- `render/<area>/<case>/` runs `kashika render` in the case directory and keeps `out.svg` and `diagnostics.txt`, plus `out.png`/`out.jpg`/`out.webp` for `// raster: png jpg webp`.

`input.kash` starts with directive comments, then a one-line description:

```kash
// spec: 5, 7
// A diagonal segment on an `ortho` line is an error.
diagram "not ortho" size (300,200)
```

`spec` (required) names the spec sections covered. `flags` takes `force`, `debug` (render only) and `allow-path`; `icon-path: <dir>` sets the icon search path to a directory in the case, and `include: <file>` passes `--include` for a file relative to the case directory. Imports, images and icons go next to `input.kash`, because the case directory is the project root.

To add a case, create the directory and `input.kash`, then bless it and the coverage map:

```sh
KASHIKA_BLESS=1 cargo test -p kashika --test golden -- parse/lines/not-ortho
KASHIKA_BLESS=1 cargo test -p kashika --test golden -- coverage/map
```

Review the new files (open `out.svg` in a browser) and commit them with `tests/golden/COVERAGE.md`. After an intended behavior change, bless the affected cases with a path filter and review `git diff tests/golden`. A mismatch prints a unified diff and saves the actual file under the cargo target directory's `tmp/golden-actual/`. When a render case mismatches or is blessed with a new `out.svg`, the run also writes `tmp/golden-review.html` there, showing each such case's expected and actual SVG side by side; open it to review. The `coverage/*` tests fail when a diagnostic code, spec section, built-in shape, bundled icon, attribute key (per statement), line end, `label-side` value or segment kind has no case, and when a code in `NOT_REPORTED` (`coverage.rs`) starts being reported.

## Adding a built-in asset

Each kind has one registry. The `ref` pages print their name lists from it through placeholders in `crates/kashika-cli/src/ref_cmd.rs` (`{shapes}`, `{icons}`, `{styles}`, `{markers}`, and the `{builtins}` table of unit paths), so the pages need no edit. `spec_and_ref_name_every_builtin_asset` (`crates/kashika-cli/tests/ref.rs`) fails until `docs/spec/language.md` and the `ref` pages name the new asset in code (`` `name` ``).

- **Shape:** add a `BuiltinShape` to `SHAPES` in `crates/kashika-model/src/shapes.rs`: name, search keywords and a `source` fn that returns the unit path, the outline subpath (1-based) and the unit text area for a `w`×`h` box. Measure features that should not stretch (caps, waves, heads) in pixels with a cap, like `CAP`. The fitted size is solved numerically, so a shape needs no inverse; its text-area width must grow with `w` and its height with `h`. `crates/kashika-model/tests/shapes.rs` checks it on every shape: the outline stays in the box, the text area stays inside the outline, and fitted sizes hold their text and are minimal. `tests/registry.rs` wants keywords. Add a `render/` golden that draws `shape=<name>` (`coverage/builtin-shapes-and-icons`) and list the shape in the spec's §3 shape list and §6 text-area table.
- **Icon:** add `crates/kashika-model/icons/<name>.svg`, neutral and license-clean, painting with `currentColor`, and a `<name> = ["keyword", ...]` line to `icons/keywords.toml`, whose order is the listing order. `build.rs` generates `BUNDLED_ICONS` and `ICON_KEYWORDS` from the two. `tests/registry.rs` fails on an SVG without keywords or keywords without an SVG. Add a `render/` golden with `icon="<name>"` (`coverage/builtin-shapes-and-icons`) and list it in the spec's licensing rule (§3a).
- **Style:** add a `BuiltinStyle` to `STYLES` in `crates/kashika-model/src/style.rs`; `ref styles` prints its colors. List it in the spec's built-in styles (§3).
- **Line end:** add a `Marker` variant in `crates/kashika-model/src/scene.rs` to `ALL` and `name()` (which make `NAMES`), with its `length()` and `half_width()`, then draw it in `kashika-render/src/svg.rs` and give it an outline in `kashika-check/src/lints/collision.rs`. Describe it in the spec (§5 `start=`/`end=`) and `ref paths`, and add a golden that draws it.
- **Font:** add the file and its license to `crates/kashika-text/fonts/`, load it in `font.rs` as a `FaceId` (in `font_files()` order), put it in the fallback `chain`, and say in the spec (§6) which text uses it. A face ahead of others in a chain changes metrics for every label it covers, so re-bless all goldens and example renders and review the diff; a face added last changes only text that was `missing-glyph`. Credit its license in the README.

## Invariants

- SVG output is deterministic: the same input always yields the same bytes.
- Every diagnostic carries a code listed in section 7 of the spec (`ERROR_CODES`/`LINT_CODES` in `kashika-diag`) and a source span, and every code has a golden case.
- The parser and resolver recover and report diagnostics; they never panic on bad input.
- Every lint has a positive and a negative test in `crates/kashika-check/tests/`.
- A language or CLI change updates the spec, the tests (including golden cases under `tests/golden/`), the `ref` pages and, when it is affected, `skills/kashika/SKILL.md` together. The skill is a short entry point; the detail lives in the `ref` topics (`ref` with no topic is the core guide an agent reads first). Every ```` ```kash ```` block in `ref` and the skill is a fixture that must check clean and be in `fmt` form.
- `fmt` changes only whitespace; it never changes tokens.
- `route` never edits files.
- All file access in the model goes through `loader.rs`, which enforces local-only paths, the project root and the search path. The model never reads the environment; the CLI passes `KASHIKA_ICON_PATH` in `Options`.
- Embedded SVG is always the usvg re-serialization, never the input bytes.
- Text is measured and drawn only with the fonts in `crates/kashika-text/fonts/` (Inter, JetBrains Mono and the Twemoji emoji font), each character by the same fallback face in both. SVG text is glyph outlines (`<path aria-label=…>`; a line with emoji is a `<g aria-label=…>` of the text path and one path per COLR color layer), never `<text>`, so checks and every viewer agree.
- Nothing paints the canvas unless `background=` is set. Line labels hide their own line with a per-line SVG `<mask>` of their rects, never with a painted rect, so the fill beneath shows through.

## Conventions

- American spelling.
- No comments that restate the code.
- Lints take `&Ctx` and never mutate the scene. Outlines come from `kashika_check::outline::polygon` (which uses `Shape::form` for non-rect shapes) and text extents from `kashika_check::text_box` (including `below_box` for `text-area=below`), so new shapes plug in there.
- Put new tests next to similar ones: integration tests in `crates/*/tests/`, CLI fixtures in `crates/kashika-cli/tests/fixtures/`, golden cases in `tests/golden/<suite>/<area>/`.
- Planning docs under `docs/superpowers/` and `.superpowers/` are local-only; do not commit them.
