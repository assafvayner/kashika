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
4. `--max N` (default 50) caps the diagnostics listed, but every error is always reported; only warnings are cut (`truncated` in JSON). `--summary` adds per-code counts (JSON always has `by_code`), useful when many findings are suppressed. `--focus api,db` limits warnings to those ids; `kashika check - --stdin-name draft.kash < draft` checks stdin.
5. When a fix needs coordinates, `kashika inspect file.kash <ids> --ports` shows absolute boxes, anchors, line vertices and taken sides; `--corridors x1,y1,x2,y2` lists free lanes in a region (alone, it prints only the lanes).
6. `kashika route file.kash api.right db.left@0.3` suggests ready-to-paste `line` statements ranked by the `cross`, `group-transit` and `near` findings `check` would report (a group transit counts as much as a crossing; both are listed) (`arc`, `bezier`, `--line <id>` to reroute; without it an existing line between the same shapes is avoided, and a `hint:` names it). It never edits the file; it tries up to 4 elbows along lanes beside boxes and groups and through free gaps, and if nothing is clear, hand-route along a lane from `--corridors`.
7. If a warning is intentional, suppress it narrowly: `allow=cross:other_line reason="..."` on the line or shape. Prefer `code:id`; for a crossing, name the other line. Several codes share one reason: `allow=overlap:b,overlap:c reason="..."`. On the `diagram` line, `allow=<code>,...` silences a code everywhere. Suppressed findings still count in the summary. Don't add a broad `allow=` while you're still laying out: it hides new findings too, such as a later line running through a box.
8. `kashika fmt file.kash` normalizes layout (whitespace only; comments are kept; `fmt -` formats stdin to stdout). When the check is clean, `kashika render file.kash -o file.png` (or `.svg`) and look at it.

## Rules people get wrong

1. **y grows down.** `(0,0)` is the top-left. `v -40` moves up.
2. **Literal coordinates are relative to the enclosing group.** Anchors (`db.left`) are always absolute. `kashika inspect file.kash <id>` prints a child's absolute box.
3. **Ids are global** and referenced bare (`db.left`, never `vpc.db.left`). Shapes, groups and lines share one namespace, so a line can't be named after a box. The segment keywords `h v x y hv vh curve` and `arrow` can't be ids. Groups have anchors too, and a line can start or end on one (`web.right`).
4. **Prefer relationships to numbers.** `at api.right+(80,0) pin=left`, `-> hv db.top`, `-> y queue.left`, `junction j at stock.bottom+(0,50)`. They survive later edits; literals don't.
5. **The last hop must line up**, so the path stays orthogonal. `check` doesn't flag a diagonal, since some diagrams want them, so look for unintended ones in the render. `ortho` on a line makes any diagonal (or curve) in it an error: add it to lines meant to run at right angles, such as boxes-and-arrows architecture and flow diagrams, or when a reviewer points out a stray diagonal, so `check` catches it from then on. Leave it off where diagonals and curves are intended. The last segment sets the arrow direction: `hv` is horizontal then vertical, `vh` vertical then horizontal.

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
- Padding defaults to 8 per side, so for `rect`/`rounded` add 16 to each text dimension (`Payments API` is 91.64×17.5, so the box is at least 108×34). Other shapes put the text in a fraction of the box (`diamond` 0.5 × 0.5, `cloud` 0.6 × 0.4; `kashika ref shapes`): add the 16, then divide by the fraction. A `cylinder` loses up to 20px to each cap, so a one-line cylinder needs about 70px of height and a one-line cloud about 85. `icon=` adds `icon-size + padding` (32px) on the icon's side.
- Small boxes (badges, table rows under ~30px tall) need a smaller inset: `padding=N` sets one value for all four sides (`padding=3` fits size-12 text in a 22px row; `padding=(x,y)` sets left/right and top/bottom).
- Easiest for any shape: leave `size` out and the box fits its text (the size `label-overflow` would ask for, rounded up to `grid=`); `size (140, auto)` fixes the width and fits the height. `kashika inspect` shows the size it got. A box with no text still needs a `size`.

## Cheat sheet

```kash fragment
diagram "Title" size (900,500) kashika="0.1.0"  // leave out size to fit the canvas to the drawing (+ margin=20); optional: grid=10 (skip it for small markers or tight spacing) clearance=6 icons="dir" background=#rrggbb (default transparent)
import "lib/aws.kash" as aws  // library shapes and styles: shape=aws.s3 style=aws.service
shape tri { path "M 0.5 0 L 1 1 L 0 1 Z" }  // unit coordinates 0..1, stretched to the box; M L H V C Q A Z
shape s3 from "icons/s3.svg" text-area=below  // an SVG file as a shape; outline=bbox|hull
style svc { fill=#e8f0fe stroke=#1a73e8 }  // any style key, plus shape= (not on groups); built-ins: service datastore external queue boundary note; colors are hex or CSS names (steelblue), case-insensitive
box api "API" at (40,40) size (140,60) style=service fill=#1a73e833 stroke=steelblue stroke-width=2  // the text is positional (boxes have no label=); inline keys override style=; #rrggbbaa alpha works; shape=rect|rounded|cylinder|diamond|ellipse|hexagon|doc|cloud|person
box db "DB" at api.right+(120,0) size (140,60) pin=left  // pin: any anchor, `side@t` included (`pin=left@0.25`); default `tl`
box badge at db.tr size (16,16) pin=center allow=overlap:db reason="corner badge"
arrow api_db api.right -> db.left label="SQL"  // `arrow` is `line` with end=arrow; the optional id follows the keyword directly, with no colon
group vpc "VPC" at (20,180) size (600,280) {  // children use vpc-relative coordinates
  box pay "Payments API" at (20,60) size (140,60) sublabel="v2.4.1"  // muted second line at 0.8 × size
  box q "Jobs" at (300,60) size (160,60) icon="queue"  // icon-pos=left|top icon-size=24; bundled: server browser queue lock cloud person (use person for users); currentColor icons take the box's color=
}
junction fan at api.bottom+(0,40) dot  // a point lines meet at; draws nothing without `dot`
text note1 "note" at (880,480) size=12 align=right muted  // the id is optional (a text is not a line endpoint); centered on `at` unless align=left|right
image logo "assets/logo.png" at (780,20) size (80,80) fit=contain  // SVG/PNG/JPEG; fit=contain|cover|stretch
box sq at (40,400) size (60,60) fill=#f0d9b5 stroke=none layer=under  // boxes and images: under paints first (inside a group: right after the group's fill), over paints last (rings, highlights); neither is an obstacle for overlap/through/near/label-collision
```

Anchors: `top bottom left right center tl tr bl br`, plus `side@t` (e.g. `db.left@0.25`) and offsets `+(dx,dy)` or `-(dx,dy)` (`db.top-(10,0)` is `db.top+(-10,0)`). Mixed points take each axis from a reference: `(db.left.x, lane.y)`, `(b.top.x, fan.y)` with a junction. `.x`/`.y` only work inside such a parenthesized point. Parentheses around a point are allowed and change nothing: `at (db.center+(0,40))` is `at db.center+(0,40)`. Use one set at most; `((db.center))` is an error.

On non-rectangular shapes, `top`/`bottom`/`left`/`right` (and `@t`) slide along their axis onto the outline, so a vertical line into `db.top` stays vertical. Corners (`tl` …) stay on the bounding box. `kashika ref shapes` lists each shape's outline and text area.

Files (imports, SVG shapes, images, icons) are local only and resolve from the `.kash` file's directory, then `icons="<dir>"` on the `diagram` line, then `KASHIKA_ICON_PATH`, then the bundled icons. Paths outside the file's directory need `--include <file>` per file (relative to the current directory, repeatable; an included library's own relative imports need their own `--include`), or `--allow-path` for everything. `kashika icons [dir] --search <term>` lists what's available, with natural sizes; search also matches keywords like "user" or "database". `import` goes right after `diagram`. A library file has no `diagram` line and holds only `shape`, `style` and `import`; `kashika check lib.kash` checks it on its own, resolving its definitions. Images are top-level only. Image and SVG files are capped at 16 MiB and libraries at 1 MiB; an SVG nested more than 256 elements deep is refused, and importing one library under two namespaces at every level soon hits the 1,000 library-instance cap.

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

Line attributes: `start=`/`end=` (`arrow open diamond dot none`, plus the crow's-foot ER ends `one many one-many zero-one zero-many`, which sit on the line untrimmed), `stroke-style=dashed|dotted`, `stroke-width=2`, `radius=8` (rounded corners), `label="x"`, `label@0.25="x"` (`t` is the fraction of the whole path length, across all segments; a label masks the line under it), `label-side=above|below` (horizontal segment) or `left|right` (vertical) to put every label of the line beside it instead, unmasked, `ortho`. A line may start or end at a junction, with or without an arrow.

## Line recipes

- **Fan-out without `shared-segment`:** one line into a junction, then one line per child into its `top` (`hv` when it's off to the side). When two branches leave the junction the same way, run the trunk to the farthest child and put a second junction on it for the nearer one (`mid` below). A trunk that taps a child through a junction counts as connecting into that child's group, so it is no `group-transit`.
- **Two lines on one side** of a box: spread them with `side@t` (`core.right@0.3`, `core.right@0.7`) to avoid `port-crowding`/`shared-segment`.
- **Self-loop:** `curve via` from `top@0.8` back to `top@0.2`, with control points above the box. `out-of-canvas` measures a curve's true extent, labels included, so leave room.
- **Sequence diagram:** lifelines are 2px-wide dashed boxes under each participant, with `layer=under` so messages that skip past them raise no `through`/`near`/`label-collision`. An activation bar is a narrow white box on the lifeline, also `layer=under`, written after the lifeline so it paints on top. Place each message at an absolute height with an offset from the lifeline's top (`lc.top+(0,30) -> x ls.left`) and point it at the bar's side while active. Give messages `label-side=above` so the label sits over the arrow; on a message that skips lifelines, pick a `label@t` between two of them. Space lifelines by label width + ~30px.
- **Table cards (ER diagrams):** a `layer=under` outline box, a header box, then one row box per column stacked with `at prev.bl`, each with `stroke=none padding=3 text-align=left`. `` `code` `` text is monospace and keeps its spaces, so pad the names to line the columns up. Draw relationships between row boxes (`orders_cust.right -> customers_id.left`); offset a card's y so the two rows are level. Anchor at a row's center (`r2.right`, not `@0.2`): `near` measures a row's whole box even when it draws no border, so an off-center anchor runs close to the next row. Mark cardinality with crow's-foot ends: `start=many end=one` on a foreign key, `zero-one`/`zero-many` when optional, `one-many` for at least one. Two crow's feet on one side need 12px between them.
- **Guides for rows and columns:** a junction without `dot` draws nothing, so use junctions as named lanes and columns, chained so they move together, and place shapes on them with mixed points. Moving a lane or the first column is then a one-line edit. Put repeated looks, `shape=` included, in a `style` (see below).
- **Graphs and timelines** (commit graphs, state dots): place small markers with `pin=center` on guide columns and lanes, leave `grid=` off, and give hollow markers `fill=white`. Lines to a diamond's `left`/`right` meet its tips. Diagonal and curved edges are fine here; leave `ortho` off.

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
diagram "Lifelines" size (520,240) kashika="0.1.0"
box c "Client" at (40,20) size (100,40)
box s "Server" at (210,20) size (100,40)
box d "DB" at (380,20) size (100,40)
box lc at c.bottom size (2,160) pin=top stroke-style=dashed layer=under
box ls at s.bottom size (2,160) pin=top stroke-style=dashed layer=under
box ld at d.bottom size (2,160) pin=top stroke-style=dashed layer=under
box act at ls.top+(0,40) size (10,80) pin=top fill=white layer=under
line req lc.top+(0,30) -> x act.left end=arrow label="request" label-side=above
line q act.tr+(0,20) -> x ld.left end=arrow label="query" label-side=above
line skip lc.top+(0,130) -> x ld.left end=arrow label@0.75="audit" label-side=above
line res act.bl+(0,-10) -> x lc.right end=arrow stroke-style=dashed label="reply" label-side=above
```

```kash
diagram "Guides" size (420,200) kashika="0.1.0"
style main { fill=#d33a3a stroke=#d33a3a shape=ellipse }
style dev { fill=#1e9e5a stroke=#1e9e5a shape=ellipse }
junction main_lane at (0,60)
junction dev_lane at (0,140)
junction c0 at (120,0)
junction c1 at c0+(70,0)
junction c2 at c1+(70,0)
text "**main**" at (20, main_lane.y) align=left
text "**develop**" at (20, dev_lane.y) align=left
box m0 at (c0.x, main_lane.y) size (16,16) pin=center style=main
box d1 at (c1.x, dev_lane.y) size (16,16) pin=center style=dev
box m1 at (c2.x, main_lane.y) size (16,16) pin=center style=main
line m0.right -> d1.left
line d1.right -> m1.left
line m0.right -> m1.left
```

```kash
diagram "Tables" size (520,160) kashika="0.1.0"
box orders at (20,20) size (200,96) stroke=#475569 layer=under
box orders_h "**orders**" at orders.tl size (200,26) fill=#dbeafe stroke=none padding=4
box orders_id "`id           uuid  PK`" at orders_h.bl size (200,22) stroke=none padding=3 text-align=left font-size=12
box orders_cust "`customer_id  uuid  FK`" at orders_id.bl size (200,22) stroke=none padding=3 text-align=left font-size=12
box orders_at "`created_at   tstz`" at orders_cust.bl size (200,22) stroke=none padding=3 text-align=left font-size=12
box customers at (300,42) size (200,74) stroke=#475569 layer=under
box customers_h "**customers**" at customers.tl size (200,26) fill=#dbeafe stroke=none padding=4
box customers_id "`id     uuid  PK`" at customers_h.bl size (200,22) stroke=none padding=3 text-align=left font-size=12
box customers_em "`email  text`" at customers_id.bl size (200,22) stroke=none padding=3 text-align=left font-size=12
line fk orders_cust.right -> customers_id.left start=many end=one
```

## Text

- Markup in any label or text (including a line's `label=`): `**bold**`, `` `code` `` (monospace, spaces kept; `kashika measure` takes the backticks too). No nesting. `\*` and `` \` `` are literal. An unclosed marker is a `parse` error.
- `\n` breaks a line. There is no automatic wrapping.
- Placement in a box: `text-align=left|center|right` (default center), `text-valign=top|middle|bottom` (default middle), `gap=N` between lines (default 2). `text-area=below` on an SVG shape puts the label under it; lines leaving that side collide with it, so route them from another side.
- Groups: the label (and `sublabel`) sits top-left. `text-align` works, but `text-valign` doesn't. Keep group titles short: a line leaving a child near the group's top edge often runs through the title.
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

- `parse`: fix these first. A statement that fails to parse is dropped, but references to its id are not reported again as `unknown-id`.
- `label-overflow`: grow the box to the size in the help, shorten the text, break it with `\n`, or for a small box lower `padding=`.
- `cross` / `through` / `near`: move the segment to a lane from the `free` help; `allow=cross:<other>` only for a non-planar graph. `through` on a line's own endpoint box means a corner sits inside that box and the line reaches the anchor from inside: put the corner outside so a `left`/`right` end arrives horizontally (`vh`, `x …`) and a `top`/`bottom` end vertically (`hv`, `y …`).
- `shared-segment` / `port-crowding`: spread ports with `side@t`, or fan out from a `junction`.
- `not-ortho` / `diagonal-tail`: finish with `hv`/`vh` or `x`/`y`.
- `label-collision` on a group border or title: move the label to a `label@t` from the help, and route lines below a group's title band, not through its title, or move the title out of the way with `text-align=right` (or `center`) on the group.
- `near-axis`: a straight hop within 5° of horizontal/vertical (two anchors a few px apart) renders slanted; use the level anchor from the help (`cdn.left@0.42`) or `hv`/`vh`.
- `missing-glyph`: the bundled fonts cover Latin, Greek and Cyrillic only; replace CJK, emoji and similar characters with an `icon=` or `image`. A tab in text is four spaces.
- `text-conflict`: move `label`/`sublabel` into the block as its first items, or give every block item an `at`.

Exit codes: `0` clean, `1` warnings, `2` errors, `3` tool/usage failure. `render` writes nothing on errors unless `--force` is given (it then marks each error in red). `render -o file.png` writes PNG; raster output (png/jpg/webp) is capped at 16,384 px per side and 100,000,000 px in total (use `.svg` beyond that); attached PNG/JPEG images are limited to 8,192 px per side and 32,000,000 px in total; `--debug` overlays rulers, ids and anchor dots to help you locate problems.
