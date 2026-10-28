# CLI

```text
kashika check   <file|->
kashika render  <file|-> -o <out.svg|out.png> [--force] [--debug]
kashika fmt     <file> [--check]
kashika measure "<text>" [--size N] [--weight bold]
kashika route   <file|-> <from> <to> [corner|arc|bezier] [--line <id>] [--radius N] [--max N]
kashika ref     [topic] [--json]
kashika --version
```

- `check` and `render` take `--format text|json`, `--fail-on error|warn|never` (default `warn`), `--max N` (diagnostics shown) and `--stdin-name <name>`.
- `render` writes only when there are no errors, atomically. `--force` writes anyway, with red markers at each error. `--debug` adds rulers, ids, anchor dots and markers for every diagnostic.
- `fmt` changes only whitespace: indentation, spacing and blank lines. Comments and the spelling of numbers and strings are kept. `--check` exits 1 when the file isn't formatted.
- `measure` prints the exact width and height of a label, with markup applied.
- `route` prints up to `--max` (default 3) ready-to-paste `line` statements from `<from>` to `<to>`, ranked by crossings, near-misses, segments and length. It never edits the file. Styles: `corner` (`x`/`y`/`hv`/`vh`), `arc` (the same plus `radius=N`, default 8), `bezier` (`curve`). `--line <id>` reroutes an existing line and keeps its attributes. Literal points are canvas coordinates. `--format json` is supported.
- `ref` prints this reference; `--json` wraps a page as `{"schema":"kashika.ref/1",...}`.
- Exit codes: `0` clean, `1` warnings (also `fmt --check` with changes, and `route` with no path), `2` errors, `3` usage or I/O failure.
