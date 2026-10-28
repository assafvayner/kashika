# Kashika — Language & CLI Draft (v0)

Kashika is a text-to-diagram language for technical design diagrams (architecture, data flow, system boundaries). Unlike Mermaid or PlantUML, **there is no automatic layout**: the author specifies the exact position of every shape and the exact path of every line. It is designed to be written primarily by AI coding agents, who are good at making many small precise decisions, and to be reviewed by humans.

A CLI renders `.ksh` files to SVG/PNG and runs geometric checks (crossing lines, overlaps, text overflow) so an agent that cannot see the output can still verify it.

---

## 1. Coordinate system

- Units are pixels. Origin `(0,0)` is the **top-left** of the canvas. **x grows right, y grows down.**
- A shape's `at (x,y)` is its **top-left corner**; `size (w,h)` is width and height.
- Inside a `group`, child coordinates are **relative to the group's top-left corner**.

## 2. File structure

One statement per line, except `style { }` and `group { }` blocks. Comments start with `#` and run to end of line. Drawing order = file order; a group is drawn behind its children.

```
diagram "Upload path" size (900,500) grid=10

style service { fill=#e8f0fe stroke=#1a73e8 }

box api "API Gateway" at (100,80) size (160,60) style=service
box db  "Postgres"    at (500,80) size (140,70) shape=cylinder

group vpc "VPC" at (40,200) size (600,260) style=boundary {
  box worker "Worker" at (20,40) size (140,60)
}

line api.right -> db.left  "SQL"  end=arrow
line api.bottom -> v 40 -> h 80 -> vpc.worker.top  radius=8 end=arrow dash
line vpc.worker.right -> curve bend=30 -> db.bottom  end=arrow  label@0.3="async"
text "v2 draft" at (760,470) size=12 color=#888
```

## 3. Statements

### `diagram`
`diagram "<title>" size (w,h) [grid=N] [allow=<lint>,...]`
Must be first. `grid` is an optional snap grid used by `check` (warns on off-grid coordinates). `allow` disables a warning for the whole file.

### `style`
`style <name> { key=value ... }` — defines a reusable style. Keys: `fill`, `stroke`, `stroke-width`, `font-size`, `font-weight`, `color` (text color), `radius` (corner radius for rect).
Built-in styles: `service`, `datastore`, `external`, `queue`, `boundary`, `note`. A user `style` with the same name overrides the built-in.

### `box`
`box <id> ["<label>"] at (x,y) size (w,h) [shape=<shape>] [style=<name>] [key=value ...]`
Shapes: `rect` (default), `rounded`, `cylinder`, `diamond`, `ellipse`, `hexagon`, `doc`, `person`, `cloud`.
Labels are centered; use `\n` for explicit line breaks (no automatic wrapping).

### `group`
`group <id> ["<label>"] at (x,y) size (w,h) [style=<name>] { ...children... }`
Children are `box`, `group`, `line`, `text`. Child coordinates are relative to the group. Label is drawn at the group's top-left inside edge.

### `text`
`text "<content>" at (x,y) [size=N] [color=C] [align=left|center|right]`
Free-standing text; `at` is the text anchor point (left/center/right per `align`, vertically the baseline).

### `line`
`line [<id>:] <point> -> <segment> -> ... -> <point> ["<label>"] [attrs...]`

## 4. Points and anchors

A **point** is one of:
- Absolute: `(x,y)`
- Anchor: `<shape-id>.<anchor>` — e.g. `db.left`
- Anchor with offset: `db.top+(10,0)`, `db.top-(0,5)`

Anchors on every shape:
| anchor | position |
|---|---|
| `top`, `bottom`, `left`, `right` | midpoint of that edge |
| `center` | center of the bounding box |
| `tl`, `tr`, `bl`, `br` | corners of the bounding box |
| `top@t`, `bottom@t`, `left@t`, `right@t` | point at fraction `t` (0..1) along that edge; top/bottom run left→right, left/right run top→bottom |

For non-rectangular shapes (ellipse, diamond, cylinder…), edge anchors are projected onto the actual outline.

Shapes inside groups are referenced by dotted path: `vpc.worker.right`. Inside a group's block, siblings may be referenced by bare id.

## 5. Line paths

`start -> seg -> seg -> ... -> end`. Each segment after the start is one of:

| segment | meaning |
|---|---|
| `(x,y)` or anchor | straight line to that point |
| `h <dx>` | horizontal move by dx (relative to previous point) |
| `v <dy>` | vertical move by dy |
| `curve bend=<N>` | smooth curve from previous point to the **next** point; N is the perpendicular bulge in px at the midpoint, positive = to the left of the direction of travel |
| `curve via (c1) (c2)` | cubic Bézier to the next point with explicit control points |

The final element must be a point (absolute or anchor).

Line attributes:
- `radius=N` — round every polyline corner with radius N
- `start=` / `end=` — arrowheads: `arrow`, `open`, `diamond`, `dot`, `none` (default `none` at start, `none` at end)
- `dash` or `dot` — stroke pattern
- `"<label>"` — label at path midpoint; `label@t="<text>"` — label at fraction t of path length (repeatable)
- `stroke=`, `stroke-width=`, `style=`
- `allow=<lint>,...` — silence specific warnings for this line

Lines without an id are referred to in diagnostics as `line#N` (1-based file order).

## 6. CLI

```
kashika render <file> -o <out.svg|out.png>   # also runs check
kashika check  <file> [--json]
kashika fmt    <file> [--check]              # canonical formatting
```

Exit codes: `0` clean, `1` warnings only, `2` errors (render does not produce output on errors).

## 7. Checks

All checks run on resolved absolute geometry.

**Errors:** parse errors (with expected tokens + suggestion), unknown/duplicate ids, unknown anchors (with did-you-mean), label overflow (with required size), degenerate curves.

**Warnings (lint codes):**
| code | trigger |
|---|---|
| `cross` | two lines intersect |
| `through` | a line passes through a box that is not its start or end |
| `overlap` | two non-nested boxes overlap |
| `spill` | child extends outside its group |
| `endpoint-off-edge` | absolute endpoint near but not on a shape edge |
| `label-collision` | line label overlaps a box or another label |
| `out-of-canvas` | element outside diagram size |
| `shared-segment` | two lines overlap along a segment |
| `off-grid` | coordinate not a multiple of `grid` |

JSON diagnostic shape:
```json
{"severity":"warn","code":"cross","span":{"line":12,"col":1},
 "elements":["line#3","line#5"],"at":[312,140],
 "message":"line#3 crosses line#5 at (312,140)"}
```
