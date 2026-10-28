---
name: kashika
description: Use when creating or editing a technical diagram (architecture, data flow, system boundaries) as a .kash file, or when asked for a diagram where exact placement and line routing matter. Covers the Kashika language and the write → check → fix → render loop.
---

# Writing Kashika diagrams

Kashika has **no auto-layout**. You decide every box position and every line path, and the `kashika` CLI checks the result. Treat `kashika check` as your eyes, and look at the render too: `kashika render file.kash -o file.png`, then open or read the PNG if you can view images. A clean check is not a good diagram.

## Loop

1. Sketch the layout as a grid first: rows/columns, ~60–80px gaps, and lanes for the lines.
2. Write the `.kash` file, sizing every box from its text (see Sizing). `kashika ref` prints the full reference; `kashika ref <topic>` takes `syntax`, `anchors`, `paths`, `shapes`, `styles`, `metrics`, `lints`, `examples` or `cli`, and `kashika ref <code>` explains a diagnostic (`kashika ref through`).
3. `kashika check file.kash --format json`. Errors are listed before warnings: fix every error first, starting with the first, then every warning. The `help` entries name the fix: did-you-mean names, required sizes, clear lanes (`free`), `label@t` positions, exact anchors.
4. `--max N` (default 50) caps the diagnostics listed, but every error is always reported (in text, repeats of the same error are collapsed after three with a count); only warnings are cut (`truncated` in JSON). `--summary` adds per-code counts (JSON always has `by_code`), useful when many findings are suppressed. `--focus api,db` limits warnings to those ids; `kashika check - --stdin-name draft.kash < draft` checks stdin.
5. When a fix needs coordinates, `kashika inspect file.kash <ids> --ports` shows absolute boxes, anchors, line vertices and taken sides; `--corridors x1,y1,x2,y2` lists free lanes in a region (alone, it prints only the lanes).
6. `kashika route file.kash api.right db.left@0.3` suggests ready-to-paste `line` statements ranked by the `cross`/`near` findings `check` would report (`arc`, `bezier`, `--line <id>` to reroute; without it an existing line between the same shapes is avoided, and a `hint:` names it). It never edits the file; it tries up to 4 elbows (detours along free lanes), and if nothing is clear, hand-route along a lane from `--corridors`.
7. If a warning is intentional, suppress it narrowly: `allow=cross:other_line reason="..."` on the line or shape. Prefer `code:id`; for a crossing, name the other line. Several codes share one reason: `allow=overlap:b,overlap:c reason="..."`. On the `diagram` line, `allow=<code>,...` silences a code everywhere. Suppressed findings still count in the summary.
8. `kashika fmt file.kash` normalizes layout (whitespace only; comments are kept; `fmt -` formats stdin to stdout). When the check is clean, `kashika render file.kash -o file.png` (or `.svg`) and look at it.

## Rules people get wrong

1. **y grows down.** `(0,0)` is the top-left. `v -40` moves up.
2. **Literal coordinates are relative to the enclosing group.** Anchors (`db.left`) are always absolute. `kashika inspect file.kash <id>` prints a child's absolute box.
3. **Ids are global** and referenced bare (`db.left`, never `vpc.db.left`). The segment keywords `h v x y hv vh curve` can't be ids.
4. **Prefer relationships to numbers.** `at api.right+(80,0) pin=left`, `-> hv db.top`, `-> y queue.left`, `junction j at stock.bottom+(0,50)`. They survive later edits; literals don't.
5. **The last hop must line up**, so the path stays orthogonal (`ortho` on a line makes any diagonal an error). The last segment sets the arrow direction: `hv` is horizontal then vertical, `vh` vertical then horizontal.

   | the line enters | finish with |
   |---|---|
   | `top` / `bottom` | `hv`: `a.right -> hv b.top` |
   | `left` / `right` | `vh`: `a.top -> vh b.left`; for a longer detour `a.right -> x 360 -> y b.left -> b.left` |

6. **Space labeled lines.** A labeled line needs a gap ≥ the label's `kashika measure` width + ~30px between its boxes, or move it with `label@t`. `check` won't flag a cramped label.
7. **Text inside a shape goes in its text block.** A free `text` over a box or ellipse raises `label-collision`. Put it in the shape's `{ }` block with `at (x,y)` (box-local coordinates, see Text); free `text` only on empty canvas.
8. **`check` covers geometry only:** crossings, overlaps, near-misses, boxes in the way, crowded ports, contrast, labels on things, the canvas and the grid. It doesn't prove that arrows point the right way or that the right boxes are connected.

## Sizing

- `kashika measure "Payments API"` prints the exact width and height of a text argument (quote it; `\n` for more lines; several arguments give one result each). Add `--size N`, and `--weight bold` for text in a bold style; `**` markup is applied as in a label.
- A line is `1.25 × size` tall (17.5px at the default 14). A sublabel adds a `0.8 × size` line (`--size 11.2`) after `gap`. A text block is as wide as its widest item.
- Padding defaults to 8 per side, so for `rect`/`rounded` add 16 to each text dimension (`Payments API` is 91.64×17.5, so the box is at least 108×34). Other shapes put the text in a fraction of the box (`diamond` 0.5 × 0.5, `cloud` 0.6 × 0.4; `kashika ref shapes`): add the 16, then divide by the fraction. `icon=` adds `icon-size + padding` (32px) on the icon's side.
- Easiest for any shape: write a guess, then use the size from the `label-overflow` help, which accounts for shape, icon and sublabel.

## Cheat sheet

```kash fragment
diagram "Title" size (900,500) kashika="0.1.0"  // optional: grid=10 clearance=6 icons="dir" background=#rrggbb (default transparent)
import "lib/aws.kash" as aws  // library shapes and styles: shape=aws.s3 style=aws.service
shape tri { path "M 0.5 0 L 1 1 L 0 1 Z" }  // unit coordinates 0..1, stretched to the box; M L H V C Q A Z
shape s3 from "icons/s3.svg" text-area=below  // an SVG file as a shape; outline=bbox|hull
style svc { fill=#e8f0fe stroke=#1a73e8 }  // built-ins: service datastore external queue boundary note; colors are hex or CSS names (steelblue), case-insensitive
box api "API" at (40,40) size (140,60) style=service fill=#1a73e833 stroke=steelblue  // inline keys override style=; #rrggbbaa alpha works; // shape=rect|rounded|cylinder|diamond|ellipse|hexagon|doc|cloud|person
box db "DB" at api.right+(120,0) size (140,60) pin=left  // pin: any anchor, `side@t` included (`pin=left@0.25`); default `tl`
box badge at db.tr size (16,16) pin=center allow=overlap:db reason="corner badge"
line api_db api.right -> db.left end=arrow label="SQL"  // the optional id follows `line` directly, with no colon
group vpc "VPC" at (20,180) size (600,280) {  // children use vpc-relative coordinates
  box pay "Payments API" at (20,60) size (140,60) sublabel="v2.4.1"  // muted second line at 0.8 × size
  box q "Jobs" at (300,60) size (160,60) icon="queue"  // icon-pos=left|top icon-size=24; bundled: server browser queue lock cloud person
}
junction fan at api.bottom+(0,40) dot  // a point lines meet at; draws nothing without `dot`
text note1 "note" at (880,480) size=12 align=right muted  // the id is optional (a text is not a line endpoint); centered on `at` unless align=left|right
image logo "assets/logo.png" at (780,20) size (80,80) fit=contain  // SVG/PNG/JPEG; fit=contain|cover|stretch
```

Anchors: `top bottom left right center tl tr bl br`, plus `side@t` (e.g. `db.left@0.25`) and offsets `+(dx,dy)` or `-(dx,dy)` (`db.top-(10,0)` is `db.top+(-10,0)`). Mixed points take each axis from a reference: `(db.left.x, lane.y)`, `(b.top.x, fan.y)` with a junction. `.x`/`.y` only work inside such a parenthesized point.

On non-rectangular shapes, `top`/`bottom`/`left`/`right` (and `@t`) slide along their axis onto the outline, so a vertical line into `db.top` stays vertical. Corners (`tl` …) stay on the bounding box. `kashika ref shapes` lists each shape's outline and text area.

Files (imports, SVG shapes, images, icons) are local only and resolve from the `.kash` file's directory, then `icons="<dir>"` on the `diagram` line, then `KASHIKA_ICON_PATH`, then the bundled icons. Paths outside the file's directory need `--allow-path`. `kashika icons [dir] --search <term>` lists what's available, with natural sizes. `import` goes right after `diagram`. A library file has no `diagram` line and holds only `shape`, `style` and `import`; `kashika check lib.kash` checks it on its own, resolving its definitions. Images are top-level only.

`grid=N` makes `off-grid` check every number you write for a position or distance: point coordinates (also inside mixed points), `+(dx,dy)` offsets, sizes, `h`/`v`, numeric `x`/`y` and `curve via` points. Anchors, `@t`, `curve left N` and attribute values aren't checked. Round measured sizes up to the grid.

Segments after `->`:

| segment | meaning |
|---|---|
| `(x,y)` / `db.left` | straight to the point |
| `h 40` / `v -20` | move by |
| `x 300` / `y db.top` | move to that x / y: a number or a point (`x bi.bottom`, not `x bi.bottom.x`) |
| `hv db.top` / `vh db.left` | one elbow: horizontal then vertical / vertical then horizontal |
| `curve left 30 db.top` | the curve's midpoint bulges 30px off the straight chord, left/right as seen walking along the line |
| `curve via (c1) (c2) db.top` | explicit cubic |

Line attributes: `start=`/`end=` (`arrow open diamond dot none`), `stroke-style=dashed|dotted`, `radius=8` (rounded corners), `label="x"`, `label@0.25="x"` (`t` is the fraction of the whole path length, across all segments; a label masks the line under it), `ortho`. A line may start or end at a junction, with or without an arrow.

## Line recipes

- **Fan-out without `shared-segment`:** one line into a junction, then one line per child into its `top` (`hv` when it's off to the side). When two branches leave the junction the same way, run the trunk to the farthest child and put a second junction on it for the nearer one (`mid` below).
- **Two lines on one side** of a box: spread them with `side@t` (`core.right@0.3`, `core.right@0.7`) to avoid `port-crowding`/`shared-segment`.
- **Self-loop:** `curve via` from `top@0.8` back to `top@0.2`, with control points above the box. `out-of-canvas` measures a curve's true extent, labels included, so leave room.
- **Sequence diagram:** lifelines are tall narrow boxes under each participant; messages are horizontal lines between `side@t` anchors at the same `t` (see below). Space lifelines by label width + ~30px.

```kash
diagram "Recipes" size (500,260) kashika="0.1.0"
box src "Source" at (20,60) size (100,40)
box a "A" at (20,200) size (100,40)
box b "B" at (200,200) size (100,40)
box c "C" at (380,200) size (100,40)
junction fan at src.bottom+(0,50) dot
junction mid at (b.top.x, fan.y) dot
line src_fan src.bottom -> fan
line fan_a fan -> a.top end=arrow
line fan_c fan -> hv c.top end=arrow
line mid_b mid -> b.top end=arrow
line again src.top@0.8 -> curve via src.top@0.8+(0,-50) src.top@0.2+(0,-50) src.top@0.2 end=arrow label="retry"
```

```kash
diagram "Lifelines" size (360,200) kashika="0.1.0"
box c "Client" at (40,20) size (100,40)
box s "Server" at (220,20) size (100,40)
box lc at c.bottom size (8,120) pin=top
box ls at s.bottom size (8,120) pin=top
line req lc.right@0.2 -> ls.left@0.2 end=arrow label="request"
line res ls.left@0.6 -> lc.right@0.6 end=arrow stroke-style=dashed label="reply"
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
  Items stack top to bottom. Each item takes `size`, `color`, `align` and `muted`. An item with `at (x,y)` is absolute, in box-local coordinates (`at` is its alignment point and vertical center), and leaves the stack. This is how to put text inside a shape: `text "x" at (100,60)` in the shape's block (a free `text` over a shape is `label-collision`). Don't combine `label`/`sublabel` with stacked items (`text-conflict`). Absolute items are fine.

Comments: `// to end of line` and `/* block */` (no nesting; an unclosed `/*` is a `parse` error). `#` is not a comment. After the id and label, `at`, `size` and attributes may come in any order. Long lines can continue on the next line by starting it with `->`; attributes and `ortho` may follow on their own line.

## Diagnostics

`kashika ref lints` lists every code; `kashika ref <code>` explains one. The fixes needed most:

- `label-overflow`: grow the box to the size in the help, shorten the text, or break it with `\n`.
- `cross` / `through` / `near`: move the segment to a lane from the `free` help; `allow=cross:<other>` only for a non-planar graph.
- `shared-segment` / `port-crowding`: spread ports with `side@t`, or fan out from a `junction`.
- `not-ortho` / `diagonal-tail`: finish with `hv`/`vh` or `x`/`y`.
- `text-conflict`: move `label`/`sublabel` into the block as its first items, or give every block item an `at`.

Exit codes: `0` clean, `1` warnings, `2` errors, `3` tool/usage failure. `render` writes nothing on errors unless `--force` is given (it then marks each error in red). `render -o file.png` writes PNG; raster output (png/jpg/webp) is capped at 16,384 px per side and 100,000,000 px in total (use `.svg` beyond that); attached PNG/JPEG images are limited to 8,192 px per side and 32,000,000 px in total; `--debug` overlays rulers, ids and anchor dots to help you locate problems.
