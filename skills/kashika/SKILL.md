---
name: kashika
description: Use when creating or editing a technical diagram (architecture, data flow, system boundaries) as a .kash file, or when asked for a diagram where exact placement and line routing matter. Covers the Kashika language and the write → check → fix → render loop.
---

# Writing Kashika diagrams

Kashika has **no auto-layout**. You decide every box position and every line path, and the `kashika` CLI checks the result. You can't see the image, so treat `kashika check` as your eyes.

## Loop

1. Sketch the layout as a grid on paper first: rows/columns, ~60–80px gaps, and lanes for the lines.
2. Write the `.kash` file.
3. `kashika check file.kash --format json` and fix every error, starting with the first one. The `help` entries name the fix: did-you-mean names, the required sizes. Size boxes from the text: `kashika measure "Payments API"` prints the exact width and height; add 2 × padding (16px by default).
4. `kashika fmt file.kash` to normalize layout, then `kashika render file.kash -o file.svg` when the check is clean.

Check snippets without a file: `kashika check - --stdin-name draft.kash < draft`.

## Rules people get wrong

1. **y grows down.** `(0,0)` is the top-left. `v -40` moves up.
2. **Literal coordinates are relative to the enclosing group.** Anchors (`db.left`) are always absolute.
3. **Ids are global** and referenced bare (`db.left`, never `vpc.db.left`). The segment keywords `h v x y hv vh curve` can't be ids.
4. **Prefer relationships to numbers.** `at api.right+(80,0) pin=left`, `-> hv db.top`, `-> y queue.left`. They survive later edits; literals don't.
5. **The last hop must line up.** Use `hv`/`vh` or `x`/`y` targets so the path stays orthogonal. Add `ortho` to a line to make any diagonal an error.
6. **The last segment sets the arrow direction.** `a.right -> hv b.left` ends with a vertical hop, so the arrow lands sideways on a left edge pointing up or down. To enter a `left`/`right` anchor horizontally, finish with a horizontal move: `a.right -> x 360 -> y b.left -> b.left`. Use `hv`/`vh` only when the final hop matches the anchor's side (`hv` into `top`/`bottom`, `vh` into `left`/`right`).
7. **A clean check is not a good diagram.** There are no crossing/overlap warnings yet, so two lines from one source that share a first segment pass silently. Give each its own lane (different `x`/`y`) or fan out from a `junction`.

## Cheat sheet

```
diagram "Title" size (900,500) kashika=1
style svc { fill=#e8f0fe stroke=#1a73e8 }                 // built-ins: service datastore external queue boundary note
box api "API" at (40,40) size (140,60) style=service       // shape=rect|rounded
box pay "Payments API" at (40,140) size (180,60) sublabel="v2.4.1"   // muted second line, 0.8 × size
box db "DB" at api.right+(120,0) size (140,60) pin=left    // pin: any plain anchor name (no `@t`); default `tl`
group vpc "VPC" at (20,140) size (600,300) { ... }         // children use vpc-relative coordinates
junction fan at (api.bottom.x, 130)                        // shared point for fan-out/fan-in
text "note" at (700,480) size=12 align=right muted
line api_db: api.right -> db.left end=arrow label="SQL"
```

Anchors: `top bottom left right center tl tr bl br`, plus `side@t` (e.g. `db.left@0.25`) and offsets `+(dx,dy)`. Mixed points: `(db.left.x, lane.y)`.

Segments after `->`:

| segment | meaning |
|---|---|
| `(x,y)` / `db.left` | straight to the point |
| `h 40` / `v -20` | move by |
| `x 300` / `y db.top` | move to that x / y |
| `hv db.top` / `vh db.left` | one elbow: horizontal then vertical / vertical then horizontal |
| `curve left:30 db.top` | gentle curve; left/right as seen walking along the line |
| `curve via (c1) (c2) db.top` | explicit cubic |

Line attributes: `start=`/`end=` (`arrow open diamond dot none`), `stroke-style=dashed|dotted`, `radius=8` (rounded corners), `label="x"`, `label@0.25="x"`, `ortho`.

## Text

- Markup in any label or text (including a line's `label=`): `**bold**`, `` `code` ``. No nesting. `\*` and `` \` `` are literal. An unclosed marker is a `parse` error.
- `\n` breaks a line. There is no automatic wrapping.
- Placement in a box: `text-align=left|center|right` (default center), `text-valign=top|middle|bottom` (default middle), `gap=N` between lines (default 2). The text area is the box inset by `padding` (default 8).
- Groups: the label (and `sublabel`) sits top-left. `text-align` works, but `text-valign` doesn't.
- 3+ lines, bullets or mixed sizes: use a text block instead of `label`:
  ```
  box api at (100,80) size (200,120) style=service text-align=left {
    text "**Payments API**"
    text "v2.4.1" size=11 muted
    text "• validates cards" size=12
  }
  ```
  Items stack top to bottom. Each item takes `size`, `color`, `align` and `muted`. An item with `at (x,y)` is absolute, in box-local coordinates (`at` is its alignment point and vertical center), and leaves the stack. Don't combine `label`/`sublabel` with stacked items (`text-conflict`). Absolute items are fine.
- Line heights are `1.25 × size`: 17.5px at the default 14. A label needs `width + 16` × `17.5 + 16` px.

Comments: `// to end of line` and `/* block */` (no nesting; an unclosed `/*` is a `parse` error). `#` is not a comment.

Long lines can continue on the next line by starting it with `->`; attributes and `ortho` may follow on their own line.

## Common errors

| code | usual fix |
|---|---|
| `unknown-anchor` / `unknown-id` / `attr` | apply the did-you-mean suggestion |
| `placement-cycle` | give one box in the cycle literal coordinates |
| `not-ortho` | replace the diagonal hop with `hv`/`vh` or `x`/`y` |
| `label-overflow` | grow the box to the size in the help line, shorten the text, or break it with `\n` |
| `text-conflict` | move `label`/`sublabel` into the text block as its first items, or give every block item an `at` |
| `degenerate-curve` / `zero-length` | the endpoints coincide; check your coordinates |
| `diagram` | the file must start with `diagram "<title>" size (w,h)` |
| `unsupported` | feature is in the spec but not built yet (`icon`, shapes other than `rect`/`rounded`, `import`/`shape`/`image`, `icons=`, `pin=side@t`). Remove it; put the info in the label or a text block |
| `duplicate-id` / `unknown-style` / `size` / `parse` | fix as the message says; ids are unique across boxes, groups, junctions, lines and texts. Coordinates stay within ±1,000,000, sizes at most 1,000,000 |

Exit codes: `0` clean, `1` warnings, `2` errors, `3` tool/usage failure. `render` writes nothing on errors unless `--force` is given.

## Not supported yet

All warnings (crossing, overlap, through-box, ...), `inspect`, `route`, shapes other than `rect`/`rounded`, `icon=`, custom `shape`s, images, imports, `icons=`, and PNG output (`render` accepts `.svg` only). These arrive in later versions. Until then, check crossings and label collisions yourself by tracing each line's coordinates against the label rectangles.
