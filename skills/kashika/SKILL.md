---
name: kashika
description: Use when creating or editing a technical diagram (architecture, data flow, system boundaries) as a .kash file, or when asked for a diagram where exact placement and line routing matter. Covers the Kashika language and the write → check → fix → render loop.
---

# Writing Kashika diagrams

Kashika has **no auto-layout**. You decide every box position and every line path, and the `kashika` CLI checks the result. You may not be able to see the image: treat `kashika check` as your eyes, and if you can view the PNG, look at it too. A clean check is not a good diagram.

## Loop

1. Sketch the layout as a grid on paper first: rows/columns, ~60–80px gaps, and lanes for the lines.
2. Write the `.kash` file and size every box from its text (see Sizing). `kashika ref` prints the full reference; `kashika ref <topic>` takes `syntax`, `anchors`, `paths`, `shapes`, `styles`, `metrics`, `lints`, `examples` or `cli`, and `kashika ref <code>` explains any diagnostic (`kashika ref through`).
3. `kashika check file.kash --format json`. Fix every error first, starting with the first one, then every warning. The `help` entries name the fix: did-you-mean names, required sizes. Warnings name their `elements` (with the authored `segment` index for lines): `free` lists clear lanes for `through`/`near`/`cross`, `shared-segment` names the nearest free lane, `label-collision` lists clear `label@t` positions, and `endpoint-off-edge` gives the exact anchor. `--focus api,db` limits warnings to those ids; `kashika check - --stdin-name draft.kash < draft` checks stdin.
4. When a fix needs coordinates, `kashika inspect file.kash <ids> --ports` shows resolved (absolute) boxes, anchors, line vertices and which side anchors are taken; `--corridors x1,y1,x2,y2` lists free lanes in a region. `kashika route file.kash api.right db.left@0.3` suggests ready-to-paste `line` statements, ranked by the `cross` and `near` findings `check` would report; add `arc` for rounded corners or `bezier` for curves, and `--line <id>` to reroute an existing line. It never edits the file, and only tries paths with at most 2 elbows: write longer detours by hand along a lane from `--corridors`.
5. If a warning is intentional, suppress it narrowly on the line or shape: `allow=cross:other_line reason="..."`. Prefer `code:id` over a bare code; for a crossing, put it on either line, naming the other. Suppressed findings still count in the summary.
6. `kashika fmt file.kash` to normalize layout (whitespace only; comments are kept), then `kashika render file.kash -o file.svg` (or `-o file.png`) when the check is clean.

## Rules people get wrong

1. **y grows down.** `(0,0)` is the top-left. `v -40` moves up.
2. **Literal coordinates are relative to the enclosing group.** Anchors (`db.left`) are always absolute. `kashika inspect file.kash <id>` prints a child's absolute box.
3. **Ids are global** and referenced bare (`db.left`, never `vpc.db.left`). The segment keywords `h v x y hv vh curve` can't be ids.
4. **Prefer relationships to numbers.** `at api.right+(80,0) pin=left`, `-> hv db.top`, `-> y queue.left`, `junction j at stock.bottom+(0,50)`. They survive later edits; literals don't.
5. **The last hop must line up.** Use `hv`/`vh` or `x`/`y` targets so the path stays orthogonal. Add `ortho` to a line to make any diagonal an error.
6. **The last segment sets the arrow direction.** `hv` goes horizontal first, then vertical; `vh` vertical first, then horizontal. `a.right -> hv b.left` ends with a vertical hop, so the arrow lands sideways on a left edge pointing up or down. To enter a `left`/`right` anchor horizontally, finish with a horizontal move: `a.right -> x 360 -> y b.left -> b.left`. Use `hv` into `top`/`bottom` and `vh` into `left`/`right`.
7. **`check` covers geometry only:** crossings, overlaps, near-misses, boxes in the way, crowded ports, contrast, labels on things, the canvas and the grid. It doesn't prove that arrows point the right way or that the right boxes are connected.

## Cheat sheet

```kash
diagram "Title" size (900,500) kashika="0.1.0"  // optional: grid=10 clearance=6 icons="dir" background=#rrggbb (default transparent)
import "lib/aws.kash" as aws  // library shapes and styles: shape=aws.s3 style=aws.service
shape tri { path "M 0.5 0 L 1 1 L 0 1 Z" }  // unit coordinates 0..1, stretched to the box; M L H V C Q A Z
shape s3 from "icons/s3.svg" text-area=below  // an SVG file as a shape; outline=bbox|hull
style svc { fill=#e8f0fe stroke=#1a73e8 }  // built-ins: service datastore external queue boundary note; colors are hex or CSS names (steelblue), case-insensitive
box api "API" at (40,40) size (140,60) style=service  // shape=rect|rounded|cylinder|diamond|ellipse|hexagon|doc|cloud|person
box db "DB" at api.right+(120,0) size (140,60) pin=left  // pin: any anchor, `side@t` included (`pin=left@0.25`); default `tl`
box badge at db.tr size (16,16) pin=center allow=overlap:db reason="corner badge"
line api_db: api.right -> db.left end=arrow label="SQL"
group vpc "VPC" at (20,180) size (600,280) {  // children use vpc-relative coordinates
  box pay "Payments API" at (20,60) size (140,60) sublabel="v2.4.1"  // muted second line at 0.8 × size
  box q "Jobs" at (300,60) size (160,60) icon="queue"  // icon-pos=left|top icon-size=24; bundled: server browser queue lock cloud person
}
junction fan at api.bottom+(0,40) dot  // a point lines meet at; draws nothing without `dot`
text "note" at (880,480) size=12 align=right muted  // centered on `at` unless align=left|right
image logo "assets/logo.png" at (780,20) size (80,80) fit=contain  // SVG/PNG/JPEG; fit=contain|cover|stretch
```

Anchors: `top bottom left right center tl tr bl br`, plus `side@t` (e.g. `db.left@0.25`) and offsets `+(dx,dy)`. Mixed points take each axis from a reference: `(db.left.x, lane.y)`, `(b.top.x, fan.y)` with a junction. `.x`/`.y` only work inside such a parenthesized point.

On non-rectangular shapes, `top`/`bottom`/`left`/`right` (and `@t`) slide along their axis onto the outline, so a vertical line into `db.top` stays vertical. Corners (`tl` …) stay on the bounding box. `kashika ref shapes` lists each shape's outline and text area.

Files (imports, SVG shapes, images, icons) are local only and resolve from the `.kash` file's directory, then `icons="<dir>"` on the `diagram` line, then `KASHIKA_ICON_PATH`, then the bundled icons. Paths outside the file's directory need `--allow-path`. `kashika icons [dir] --search <term>` lists what's available, with natural sizes. `import` goes right after `diagram`. A library file has no `diagram` line and holds only `shape`, `style` and `import`; it can't be checked on its own, so check it through a diagram that imports it. Images are top-level only.

`grid=N` makes `off-grid` check every number you write for a position or distance: point coordinates (also inside mixed points), `+(dx,dy)` offsets, sizes, `h`/`v`, numeric `x`/`y` and `curve via` points. Anchors, `@t`, `curve left:N` and attribute values aren't checked. Round measured sizes up to the grid.

Segments after `->`:

| segment | meaning |
|---|---|
| `(x,y)` / `db.left` | straight to the point |
| `h 40` / `v -20` | move by |
| `x 300` / `y db.top` | move to that x / y: a number or a point (`x bi.bottom`, not `x bi.bottom.x`) |
| `hv db.top` / `vh db.left` | one elbow: horizontal then vertical / vertical then horizontal |
| `curve left:30 db.top` | the curve's midpoint bulges 30px off the straight chord, left/right as seen walking along the line |
| `curve via (c1) (c2) db.top` | explicit cubic |

Line attributes: `start=`/`end=` (`arrow open diamond dot none`), `stroke-style=dashed|dotted`, `radius=8` (rounded corners), `label="x"`, `label@0.25="x"` (`t` is the fraction of the whole path length, across all segments; a label masks the line under it), `ortho`. A line may start or end at a junction, with or without an arrow.

## Line recipes

- **Fan-out without `shared-segment`:** one line into a junction, then one line per child into its `top` (`hv` when it's off to the side). When two branches leave the junction the same way, run the trunk to the farthest child and put a second junction on it for the nearer one (`mid` below).
- **Two lines on one side** of a box: spread them with `side@t` (`core.right@0.3`, `core.right@0.7`) to avoid `port-crowding`/`shared-segment`.
- **Self-loop:** `curve via` from `top@0.8` back to `top@0.2`, with control points above the box. `out-of-canvas` measures a curve's true extent, labels included, so leave room.
- **Short lines:** `check` doesn't yet flag a label pressed against an arrowhead or box edge. Leave the label's width + ~30px (≥ 70px) between boxes joined by a labeled line, or move the label with `label@t`.

```kash
diagram "Recipes" size (500,260) kashika="0.1.0"
box src "Source" at (20,60) size (100,40)
box a "A" at (20,200) size (100,40)
box b "B" at (200,200) size (100,40)
box c "C" at (380,200) size (100,40)
junction fan at src.bottom+(0,50) dot
junction mid at (b.top.x, fan.y) dot
line src_fan: src.bottom -> fan
line fan_a: fan -> a.top end=arrow
line fan_c: fan -> hv c.top end=arrow
line mid_b: mid -> b.top end=arrow
line again: src.top@0.8 -> curve via src.top@0.8+(0,-50) src.top@0.2+(0,-50) src.top@0.2 end=arrow label="retry"
```

## Text

- Markup in any label or text (including a line's `label=`): `**bold**`, `` `code` ``. No nesting. `\*` and `` \` `` are literal. An unclosed marker is a `parse` error.
- `\n` breaks a line. There is no automatic wrapping.
- Placement in a box: `text-align=left|center|right` (default center), `text-valign=top|middle|bottom` (default middle), `gap=N` between lines (default 2). `text-area=below` on an SVG shape puts the label under it; lines leaving that side collide with it, so route them from another side.
- Groups: the label (and `sublabel`) sits top-left. `text-align` works, but `text-valign` doesn't.
- 3+ lines, bullets or mixed sizes: use a text block instead of `label`:
  ```kash
  box api at (100,80) size (200,120) style=service text-align=left {
    text "**Payments API**"
    text "v2.4.1" size=11 muted
    text "• validates cards" size=12
  }
  ```
  Items stack top to bottom. Each item takes `size`, `color`, `align` and `muted`. An item with `at (x,y)` is absolute, in box-local coordinates (`at` is its alignment point and vertical center), and leaves the stack. Don't combine `label`/`sublabel` with stacked items (`text-conflict`). Absolute items are fine.

Comments: `// to end of line` and `/* block */` (no nesting; an unclosed `/*` is a `parse` error). `#` is not a comment. Long lines can continue on the next line by starting it with `->`; attributes and `ortho` may follow on their own line.

## Sizing

- `kashika measure "Payments API"` prints the exact width and height of one text argument (quote it; `\n` for more lines). Add `--size N`, and `--weight bold` for text in a bold style; `**` markup is applied as in a label.
- A line is `1.25 × size` tall (17.5px at the default 14). A sublabel adds a `0.8 × size` line (`--size 11.2`) after `gap`. A text block is as wide as its widest item.
- `rect`/`rounded`: box = text + 2 × padding (16px by default). Other shapes put the text in a fraction of the box (`diamond` 0.5 × 0.5, `cloud` 0.6 × 0.4; `kashika ref shapes`): add the padding, then divide by the fraction. `icon=` takes `icon-size + padding` (32px) from the icon's side. Easiest for any shape: write a guess, then use the size from the `label-overflow` help, which accounts for shape, icon and sublabel.

## Common errors

| code | usual fix |
|---|---|
| `unknown-anchor` / `unknown-id` / `attr` | apply the did-you-mean suggestion |
| `placement-cycle` | give one box in the cycle literal coordinates |
| `not-ortho` | replace the diagonal hop with `hv`/`vh` or `x`/`y` |
| `label-overflow` | grow the box to the size in the help line, shorten the text, or break it with `\n` |
| `text-conflict` | move `label`/`sublabel` into the text block as its first items, or give every block item an `at` |
| `degenerate-curve` / `zero-length` | the endpoints coincide; check your coordinates (loops need `curve via`) |
| `diagram` | the file must start with `diagram "<title>" size (w,h)` |
| `file` | a referenced file is missing, outside the project root, a URL, unreadable or the wrong type, or imports form a cycle. Fix the path (see the did-you-mean) or pass `--allow-path`. Errors inside an imported file are reported at the `import`, prefixed ``in `file` at L:C:`` |
| `unsupported` | feature is in the spec but not built into this binary. Remove it; put the info in the label or a text block |
| `duplicate-id` / `duplicate-style` / `unknown-style` / `size` / `parse` | fix as the message says; ids are unique across boxes, groups, images, junctions, lines and texts; style names are unique among styles and shape names among shapes (built-in names included). Coordinates stay within ±1,000,000, sizes at most 1,000,000 |

## Warnings

| code | usual fix |
|---|---|
| `cross` | move one line to a lane from the `free` help, or route around; `allow=cross:<other>` if the graph is non-planar |
| `shared-segment` | give the later line its own lane (the help names the nearest one), fan out from a `junction`, or spread ports with `side@t` |
| `through` / `group-transit` | move the segment to a `free` interval or route around; images block lines like boxes |
| `near` | keep `clearance` (default 6px) from box edges and parallel lines |
| `overlap` / `spill` | move or resize; only a `group` may contain shapes; keep children below its title band |
| `diagonal-tail` | finish with `hv`/`vh` or `x`/`y` |
| `endpoint-off-edge` | use the anchor from the help instead of a literal |
| `port-crowding` | spread endpoints ≥8px apart (`side@t`) or meet at a `junction` |
| `label-collision` | move the label to a clear `label@t` from the help, or the line to another side |
| `image-distort` | use `fit=contain`/`cover`, or the size in the help line |
| `out-of-canvas` | move it in or enlarge `size`; curves and labels count at their true extent |
| `low-contrast` / `off-grid` | as the message says |

Exit codes: `0` clean, `1` warnings, `2` errors, `3` tool/usage failure. `render` writes nothing on errors unless `--force` is given (it then marks each error in red). `render -o file.png` writes PNG; `--debug` overlays rulers, ids and anchor dots to help you locate problems. The `block-reducible` and `text-outside-shape` warnings aren't reported yet.
