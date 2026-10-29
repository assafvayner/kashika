## Diagnostics

Errors stop `render` unless `--force`; warnings exit 1 and `allow=code[:other]` suppresses them. `kashika ref <code>` shows one row.

| code | severity | trigger | fix |
|---|---|---|---|
| `parse` | error | malformed statement, unknown escape, reserved id, unclosed markup or comment | follow the expected-token message |
| `unknown-id` | error | reference to an id that doesn't exist (not to one whose statement failed to parse) | apply the did-you-mean suggestion |
| `unknown-anchor` | error | bad anchor name or `@t` | use a listed anchor; `@t` only on edges, `0 <= t <= 1` |
| `duplicate-id` | error | two statements share an id | rename one; ids are global |
| `duplicate-style` | error | two `style` or `shape` definitions share a name | merge or rename one |
| `placement-cycle` | error | `at` references form a cycle, or a chain of more than 256 | give one shape literal coordinates |
| `label-overflow` | error | text doesn't fit the text area | use the size in the help line or shorten the text |
| `text-conflict` | error | `label` or `sublabel` plus stacked block items | move them into the block, or give every item an `at` |
| `not-ortho` | error | diagonal segment on an `ortho` line | use `hv`/`vh` or `x`/`y` |
| `degenerate-curve` | error | zero-size curve or negative amount | fix its points or amount |
| `zero-length` | error | every segment has zero length | fix the endpoints |
| `attr` | error | unknown, duplicate, mistyped or out-of-range attribute | apply the did-you-mean suggestion |
| `unknown-style` | error | `style=` names no style | define it or use a built-in |
| `unknown-shape` | error | `shape=` names no shape | apply the did-you-mean suggestion, or define or import it |
| `size` | error | non-positive or oversized size | use a positive width and height |
| `range` | error | a coordinate or curve amount beyond ±1,000,000 | keep coordinates within -1000000..1000000 |
| `diagram` | error | missing, misplaced or repeated `diagram` | start with one `diagram "<title>" size (w,h)` |
| `version` | error | `kashika=` is malformed or newer than the binary | fix the file or the binary |
| `file` | error | missing, oversized or unusable file, an import cycle, or too many imports | fix the path, or pass `--include <file>` or `--allow-path` |
| `cross` | warn | two lines intersect away from a shared endpoint | reroute via a free interval |
| `shared-segment` | warn | two lines overlap collinearly over 1px | move one to the free lane |
| `through` | warn | a line enters a box or image (not `layer=under`/`over`), including its own endpoint box: reaching an edge anchor from inside the box is flagged; only the run from an interior anchor like `center` to the outline is exempt | route around it; for its own endpoint box, keep the last corner outside so a `left`/`right` end is reached horizontally (`vh`, `x …`) and a `top`/`bottom` end vertically (`hv`, `y …`) |
| `near` | warn | a line within `clearance` of a box, a parallel line or along a group border | move it `clearance` away |
| `overlap` | warn | two shapes overlap, unless a group contains the other | move one, or use a group |
| `spill` | warn | a child leaves its group or enters its title band | grow the group |
| `diagonal-tail` | warn | the last segment is diagonal and every other straight segment is horizontal or vertical (at least one) | end with `hv`/`vh` |
| `near-axis` | warn | a straight segment (no `ortho`) is off horizontal or vertical by more than 0.5px but at most 5°, so it shows a slant | use the level anchor in the help line, or `hv`/`vh` |
| `endpoint-off-edge` | warn | a literal endpoint near but off an outline | use the anchor in the help line |
| `port-crowding` | warn | two endpoints on a shape within 8px (12px for two crow's feet) | spread them with `side@t` |
| `group-transit` | warn | a line crosses a group it doesn't connect to (a junction on it that taps into the group counts as connecting) | route around the group |
| `low-contrast` | warn | text contrast below 3:1 | change the color or fill |
| `label-collision` | warn | a label or text overlaps another element, a group's title or border, or a label its own line's marker (or its own line, with `label-side`); or a line crosses a group's title | move it; the help lists clear `t` or the title's extent |
| `out-of-canvas` | warn | an element extends past `size` (children of a group that itself extends past it aren't reported again) | move it or grow `size` |
| `off-grid` | warn | a literal is not a multiple of `grid` | round it |
| `block-reducible` | warn | a block of 1–2 items in one style | use the rewrite in the help line |
| `text-outside-shape` | warn | an absolute block item outside the text area | move it inside the range in the help line |
| `image-distort` | warn | `fit=stretch` distorts by over 5% | use `fit=contain` |
| `missing-glyph` | warn | text has a character the bundled fonts cannot draw | use Latin, Greek or Cyrillic, or an `icon=`/`image` |
