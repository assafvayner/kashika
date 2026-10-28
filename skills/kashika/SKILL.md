---
name: kashika
description: Use when creating or editing a technical diagram (architecture, data flow, system boundaries) as a .kash file, or when asked for a diagram where exact placement and line routing matter. Covers the Kashika language and the write → check → fix → render loop.
---

# Writing Kashika diagrams

Kashika has **no auto-layout**. You decide every box position and every line path, and the `kashika` CLI checks the result. You can't see the image, so treat `kashika check` as your eyes.

## Loop

1. Sketch the layout as a grid on paper first: rows/columns, ~60–80px gaps, and lanes for the lines.
2. Write the `.kash` file.
3. `kashika check file.kash --format json` and fix every error, starting with the first one. The `help` entries name the fix: did-you-mean names, the required sizes.
4. `kashika render file.kash -o file.svg` when the check is clean.

Check snippets without a file: `kashika check - --stdin-name draft.kash < draft`.

## Rules people get wrong

1. **y grows down.** `(0,0)` is the top-left. `v -40` moves up.
2. **Literal coordinates are relative to the enclosing group.** Anchors (`db.left`) are always absolute.
3. **Ids are global** and referenced bare (`db.left`, never `vpc.db.left`).
4. **Prefer relationships to numbers.** `at api.right+(80,0) pin=left`, `-> hv db.top`, `-> y queue.left`. They survive later edits; literals don't.
5. **The last hop must line up.** Use `hv`/`vh` or `x`/`y` targets so the path stays orthogonal. Add `ortho` to a line to make any diagonal an error.
6. **The last segment sets the arrow direction.** `a.right -> hv b.left` ends with a vertical hop, so the arrow lands sideways on a left edge pointing up or down. To enter a `left`/`right` anchor horizontally, finish with a horizontal move: `a.right -> x 360 -> y b.left -> b.left`. Use `hv`/`vh` only when the final hop matches the anchor's side (`hv` into `top`/`bottom`, `vh` into `left`/`right`).
7. **A clean check is not a good diagram.** There are no crossing/overlap warnings yet, so two lines from one source that share a first segment pass silently. Give each its own lane (different `x`/`y`) or fan out from a `junction`.

## Cheat sheet

```
diagram "Title" size (900,500) kashika=1
style svc { fill=#e8f0fe stroke=#1a73e8 }                 // built-ins: service datastore external queue boundary note
box api "API" at (40,40) size (140,60) style=service       // shape=rect|rounded
box db "DB" at api.right+(120,0) size (140,60) pin=left    // pin: any plain anchor name (no `@t`); default `tl`
group vpc "VPC" at (20,140) size (600,300) { ... }         // children use vpc-relative coordinates
junction fan at (api.bottom.x, 130)                        // shared point for fan-out/fan-in
text "note" at (700,480) size=12 align=right
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

Comments: `// to end of line` and `/* block */` (no nesting; an unclosed `/*` is a `parse` error). `#` is not a comment.

Long lines can continue on the next line by starting it with `->`; attributes may follow on their own line.

## Common errors

| code | usual fix |
|---|---|
| `unknown-anchor` / `unknown-id` / `attr` | apply the did-you-mean suggestion |
| `placement-cycle` | give one box in the cycle literal coordinates |
| `not-ortho` | replace the diagonal hop with `hv`/`vh` or `x`/`y` |
| `degenerate-curve` / `zero-length` | the endpoints coincide; check your coordinates |
| `diagram` | the file must start with `diagram "<title>" size (w,h)` |
| `unsupported` | feature is in the spec but not built yet (`sublabel`, `icon`, `text-align`, shapes other than `rect`/`rounded`). Remove it; put the info in the `label` (use `\n` for extra lines) |
| `duplicate-id` / `unknown-style` / `size` / `parse` | fix as the message says; ids are unique across boxes, groups, junctions, lines and texts |

Exit codes: `0` clean, `1` warnings, `2` errors, `3` tool/usage failure. `render` writes nothing on errors unless `--force` is given.

## Not supported yet

Text measurement and overflow checks, all warnings (crossing, overlap, through-box, ...), `inspect`, `route`, `fmt`, shapes other than `rect`/`rounded`, `sublabel`/`icon`/text blocks, images, imports, and PNG output (`render` accepts `.svg` only). These arrive in later versions. Until then, leave about 20% spare room around labels and check crossings yourself by tracing each line's coordinates.
