## Diagnostics

Errors stop `render` unless `--force` is given. `kashika ref <code>` shows one row. This build reports only errors; geometry warnings such as `cross` and `through` aren't implemented yet, though `allow=` already accepts their names.

| code | severity | trigger | fix |
|---|---|---|---|
| `parse` | error | malformed statement, unknown escape, reserved id, unclosed markup or comment | follow the expected-token message |
| `unknown-id` | error | reference to an id that doesn't exist | apply the did-you-mean suggestion |
| `unknown-anchor` | error | bad anchor name or `@t` | use a listed anchor; `@t` only on edges, `0 <= t <= 1` |
| `duplicate-id` | error | two statements share an id | rename one; ids are global |
| `duplicate-style` | error | two `style` statements share a name | merge them or rename one; style names are separate from ids |
| `placement-cycle` | error | `at` references form a cycle | give one shape in the cycle literal coordinates |
| `label-overflow` | error | text doesn't fit the text area | grow the box to the size in the help line, shorten the text or break it with `\n` |
| `text-conflict` | error | `label` or `sublabel` plus stacked block items | move them into the block as its first items, or give every item an `at` |
| `not-ortho` | error | diagonal segment on an `ortho` line | use `hv`/`vh` or `x`/`y` |
| `degenerate-curve` | error | zero-size or negative curve, or a value beyond 1,000,000 | check the curve's endpoints and amount |
| `zero-length` | error | every segment has zero length | the endpoints coincide; fix them |
| `attr` | error | unknown, duplicate, mistyped or out-of-range attribute | apply the did-you-mean suggestion |
| `unknown-style` | error | `style=` names no style | define it or use a built-in |
| `size` | error | non-positive or oversized size | use a positive width and height |
| `diagram` | error | missing, misplaced or repeated `diagram` | start with one `diagram "<title>" size (w,h)` |
| `version` | error | `kashika=N` doesn't match the binary | update the file or the binary |
| `unsupported` | error | feature not built yet | remove it; see "Not supported" |
