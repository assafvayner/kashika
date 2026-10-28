---
name: kashika
description: Use when creating or editing a technical diagram (architecture, data flow, system boundaries) as a .kash file, or when asked for a diagram where exact placement and line routing matter. Covers the Kashika language and the write → check → fix → render loop.
---

# Writing Kashika diagrams

Kashika has **no auto-layout**. You decide every box position and every line path, and the `kashika` CLI checks the result. You can't see the image, so treat `kashika check` as your eyes.

## Loop

1. Sketch the layout as a grid on paper first: rows/columns, ~60–80px gaps, and lanes for the lines.
2. Write the `.kash` file. `kashika ref` prints the full reference (rules, grammar, anchors, segments, every diagnostic, routing recipes, a worked example); look things up by topic (`kashika ref paths`, `kashika ref examples`, `kashika ref cli`) or by diagnostic code (`kashika ref through`).
3. `kashika check file.kash --format json`. Fix every error first, starting with the first one, then every warning. The `help` entries name the fix: did-you-mean names, required sizes. Size boxes from the text: `kashika measure "Payments API"` prints the exact width and height; add 2 × padding (16px by default). Warnings name their `elements` (with the authored `segment` index for lines): `free` lists clear lanes for `through`/`near`/`cross`, `shared-segment` names the nearest free lane, `label-collision` lists clear `label@t` positions, and `endpoint-off-edge` gives the exact anchor.
4. When a fix needs coordinates, `kashika inspect file.kash <ids> --ports` shows resolved boxes, anchors, line vertices and which side anchors are taken; `--corridors x1,y1,x2,y2` lists free lanes in a region. `kashika route file.kash api.right db.left` suggests ready-to-paste `line` statements, ranked by the `cross` and `near` findings `check` would report; add `arc` for rounded corners or `bezier` for curves, and `--line <id>` to reroute an existing line. It never edits the file.
5. If a warning is intentional, suppress it narrowly on the line or shape: `allow=cross:other_line reason="..."`. Prefer `code:id` over a bare code. Suppressed findings still count in the summary.
6. `kashika fmt file.kash` to normalize layout (whitespace only; comments are kept), then `kashika render file.kash -o file.svg` (or `-o file.png`) when the check is clean.

Check snippets without a file: `kashika check - --stdin-name draft.kash < draft`. Focus on one area with `--focus api,db` (warnings involving those ids only).

## Rules people get wrong

1. **y grows down.** `(0,0)` is the top-left. `v -40` moves up.
2. **Literal coordinates are relative to the enclosing group.** Anchors (`db.left`) are always absolute.
3. **Ids are global** and referenced bare (`db.left`, never `vpc.db.left`). The segment keywords `h v x y hv vh curve` can't be ids.
4. **Prefer relationships to numbers.** `at api.right+(80,0) pin=left`, `-> hv db.top`, `-> y queue.left`. They survive later edits; literals don't.
5. **The last hop must line up.** Use `hv`/`vh` or `x`/`y` targets so the path stays orthogonal. Add `ortho` to a line to make any diagonal an error.
6. **The last segment sets the arrow direction.** `a.right -> hv b.left` ends with a vertical hop, so the arrow lands sideways on a left edge pointing up or down. To enter a `left`/`right` anchor horizontally, finish with a horizontal move: `a.right -> x 360 -> y b.left -> b.left`. Use `hv`/`vh` only when the final hop matches the anchor's side (`hv` into `top`/`bottom`, `vh` into `left`/`right`).
7. **A clean check is not a good diagram.** It covers geometry only: crossings, overlaps, near-misses, boxes in the way, crowded ports, contrast, labels on things, the canvas and the grid. It doesn't prove that arrows point the right way or that the right boxes are connected.

## Cheat sheet

```
diagram "Title" size (900,500) kashika="0.1.0"
import "lib/aws.kash" as aws                               // shapes and styles from a library: shape=aws.s3 style=aws.service
shape tri { path "M 0.5 0 L 1 1 L 0 1 Z" }                 // unit coordinates 0..1, stretched to the box; M L H V C Q A Z
shape s3 from "icons/s3.svg" text-area=below               // an SVG file as a shape; outline=bbox|hull
style svc { fill=#e8f0fe stroke=#1a73e8 }                 // built-ins: service datastore external queue boundary note
box api "API" at (40,40) size (140,60) style=service       // shape=rect|rounded|cylinder|diamond|ellipse|hexagon|doc|cloud|person
box pay "Payments API" at (40,140) size (180,60) sublabel="v2.4.1"   // muted second line, 0.8 × size
box db "DB" at api.right+(120,0) size (140,60) pin=left    // pin: any anchor, `side@t` included (`pin=left@0.25`); default `tl`
group vpc "VPC" at (20,140) size (600,300) { ... }         // children use vpc-relative coordinates
junction fan at (api.bottom.x, 130)                        // shared point for fan-out/fan-in
text "note" at (700,480) size=12 align=right muted
line api_db: api.right -> db.left end=arrow label="SQL"
box badge at db.tr size (16,16) pin=center allow=overlap:db reason="corner badge"
image logo "assets/logo.png" at (700,20) size (80,80) fit=contain  // SVG/PNG/JPEG; fit=contain|cover|stretch
box q "Jobs" at (40,200) size (160,60) icon="queue"        // icon-pos=left|top icon-size=24; bundled: server browser queue lock cloud person
```

Anchors: `top bottom left right center tl tr bl br`, plus `side@t` (e.g. `db.left@0.25`) and offsets `+(dx,dy)`. Mixed points: `(db.left.x, lane.y)`.

On non-rectangular shapes, `top`/`bottom`/`left`/`right` (and `@t`) slide along their axis onto the outline, so a vertical line into `db.top` stays vertical. Corners (`tl` …) stay on the bounding box. `kashika ref shapes` lists each shape's outline and text area.

Files (imports, SVG shapes, images, icons) are local only and resolve from the `.kash` file's directory, then `icons="<dir>"` on the `diagram` line, then `KASHIKA_ICON_PATH`, then the bundled icons. Paths outside the file's directory need `--allow-path`. `kashika icons [dir] --search <term>` lists what's available, with natural sizes. A library file holds only `shape`, `style` and `import`; `import` goes right after `diagram`. Images are top-level only.

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
- Placement in a box: `text-align=left|center|right` (default center), `text-valign=top|middle|bottom` (default middle), `gap=N` between lines (default 2). The text area is the box inset by `padding` (default 8); other shapes use a smaller area (`kashika ref shapes`), and an icon takes `icon-size + padding` from its side. `text-area=below` on an SVG shape puts the label under it.
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
| `file` | a referenced file is missing, outside the project root, a URL, unreadable or the wrong type, or imports form a cycle. Fix the path (see the did-you-mean) or pass `--allow-path`. Errors inside an imported file are reported at the `import`, prefixed ``in `file` at L:C:`` |
| `unsupported` | feature is in the spec but not built into this binary. Remove it; put the info in the label or a text block |
| `duplicate-id` / `duplicate-style` / `unknown-style` / `size` / `parse` | fix as the message says; ids are unique across boxes, groups, images, junctions, lines and texts; style names are unique among styles and shape names among shapes (built-in names included). Coordinates stay within ±1,000,000, sizes at most 1,000,000 |

## Warnings

| code | usual fix |
|---|---|
| `cross` | move one line to a lane from the `free` help, or route around; `allow=cross:<other>` if the graph is non-planar |
| `shared-segment` | give the later line its own lane (the help names the nearest one) or fan out from a `junction` |
| `through` / `group-transit` | move the segment to a `free` interval or route around; images block lines like boxes |
| `near` | keep `clearance` (default 6px) from box edges and parallel lines |
| `overlap` / `spill` | move or resize; only a `group` may contain shapes; keep children below its title band |
| `diagonal-tail` | finish with `hv`/`vh` or `x`/`y` |
| `endpoint-off-edge` | use the anchor from the help instead of a literal |
| `port-crowding` | spread endpoints ≥8px apart (`side@t`) or meet at a `junction` |
| `label-collision` | move the label to a clear `label@t` from the help |
| `image-distort` | use `fit=contain`/`cover`, or the size in the help line |
| `low-contrast` / `out-of-canvas` / `off-grid` | as the message says |

Exit codes: `0` clean, `1` warnings, `2` errors, `3` tool/usage failure. `render` writes nothing on errors unless `--force` is given (it then marks each error in red). `render -o file.png` writes PNG; `--debug` overlays rulers, ids and anchor dots to help you locate problems.

## Not supported yet

The `block-reducible` and `text-outside-shape` warnings. They arrive in a later version.
