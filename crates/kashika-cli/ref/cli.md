# CLI

```text
kashika check   <file|-> [--allow-path]
kashika render  <file|-> -o <out.svg|out.png|out.jpg|out.jpeg|out.webp> [--force] [--debug] [--allow-path]
kashika inspect <file|-> [id ...] [--ports] [--corridors x1,y1,x2,y2] [--allow-path]
kashika fmt     <file> [--check]
kashika measure "<text>" [--size N] [--weight bold]
kashika route   <file|-> <from> <to> [corner|arc|bezier] [--line <id>] [--radius N] [--max N] [--allow-path]
kashika icons   [dir...] [--search <term>] [--format text|json]
kashika ref     [topic] [--json]
kashika --version
```

- `check` and `render` take `--format text|json`, `--fail-on error|warn|never` (default `warn`), `--max N` (diagnostics shown), `--stdin-name <name>`, `--focus <id,...>` (only warnings naming these ids; errors always show) and `--show-suppressed` (list suppressed findings with their `reason`).
- `render` writes only when there are no errors, atomically. `--force` writes anyway, with red markers at each error. `--debug` adds rulers, ids, anchor dots and markers for every diagnostic.
- `inspect` prints resolved geometry (never fails on errors): boxes, groups and images with bbox and anchors (plus the file's natural size, aspect ratio and `fit` for images and `shape from` SVGs), junctions, texts, lines with vertices, authored segments (`line:col`), length and label positions, and the placement graph. Ids limit the output (unknown id: exit 3). Elements inside a group also show local coordinates. `--ports` lists each line endpoint on a shape side. `--corridors` lists the free `y` and `x` intervals in the rectangle. `--format json` writes `kashika.inspect/1`.
- `fmt` changes only whitespace: indentation, spacing and blank lines. Comments and the spelling of numbers and strings are kept. `--check` exits 1 when the file isn't formatted.
- `measure` prints the exact width and height of a label, with markup applied.
- `route` prints up to `--max` (default 3) ready-to-paste `line` statements from `<from>` to `<to>`, ranked by crossings, near-misses, segments and length. It never edits the file. Styles: `corner` (`x`/`y`/`hv`/`vh`), `arc` (the same plus `radius=N`, default 8), `bezier` (`curve`). `--line <id>` reroutes an existing line and keeps its attributes. Literal points are canvas coordinates. `--format json` is supported.
- Files (`import`, `shape from`, `image`, `icon=`) resolve from the input file's directory (the current directory for stdin), then the `icons=` directory, then each `KASHIKA_ICON_PATH` entry (`:`-separated), then the bundled icons. `check`, `render` and `route` take `--allow-path` to permit references outside the input's directory; without it they report `error[file]`. `KASHIKA_ICON_PATH` entries are always trusted.
- `icons` lists the bundled icons, then the images and `.kash` shapes under each `dir` (default: the `KASHIKA_ICON_PATH` entries), as `name path size library`. `--search` matches the name or path, case-insensitively. `--format json` returns `{"schema":"kashika.icons/1","icons":[...]}`.
- `ref` prints this reference; `--json` wraps a page as `{"schema":"kashika.ref/1",...}`.
- Exit codes: `0` clean, `1` warnings (also `fmt --check` with changes, and `route` with no path), `2` errors, `3` usage or I/O failure.
