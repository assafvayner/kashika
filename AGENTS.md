# AGENTS.md

Guide for AI coding agents working on the Kashika codebase. Kashika is a text-to-diagram language (`.kash` files, no auto-layout) with a Rust CLI, `kashika`, that checks and renders to SVG. The normative spec is [`docs/spec/language.md`](docs/spec/language.md). Agents that *use* Kashika to draw diagrams should read [`skills/kashika/SKILL.md`](skills/kashika/SKILL.md) instead.

## Crates

In dependency order (`crates/`):

- `kashika-diag`: `Diagnostic`, severity, spans and the lint code list.
- `kashika-geom`: points, rects and path math (flattening, length, rounded corners).
- `kashika-syntax`: lexer, parser and AST.
- `kashika-model`: resolver; turns the AST into a `Scene` with anchors, attributes and styles (`compile`).
- `kashika-render`: `Scene` to SVG.
- `kashika-cli`: the `kashika` binary (`check`, `render`), text and JSON reports.

## Commands

```sh
cargo fmt --all        # stable rustfmt, never +nightly
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p kashika-cli -- check examples/upload.kash
cargo run -p kashika-cli -- render examples/upload.kash -o target/upload.svg
```

## Invariants

- SVG output is deterministic: the same input always yields the same bytes.
- Every diagnostic carries a code listed in section 7 of the spec and a source span.
- The parser and resolver recover and report diagnostics; they never panic on bad input.
- Spec features not yet implemented report the `unsupported` diagnostic.
- A language change updates the spec, the tests and `skills/kashika/SKILL.md` together.

## Conventions

- American spelling.
- No comments that restate the code.
- Put new tests next to similar ones: integration tests in `crates/*/tests/`, CLI fixtures in `crates/kashika-cli/tests/fixtures/`.
- Planning docs under `docs/superpowers/` and `.superpowers/` are local-only; do not commit them.
