# Kashika Design Review Packet

You are reviewing the design of **Kashika**, a new text-to-diagram language and CLI intended to be written primarily by AI coding agents (such as yourself). There is no implementation yet — this is a paper review. We are collecting independent reviews from several models, so please answer from your own perspective as an agent that would have to use this tool.

**Motivation.** Mermaid/PlantUML auto-layout makes it nearly impossible to control where shapes go and where lines cross. For humans, specifying every coordinate is tedious; for agents it isn't. Kashika therefore gives the author full control: exact shape positions, exact line paths (straight, orthogonal, curved), and a geometric checker so an author who cannot see the image can still verify it.

**How to do this review.** Do the tasks in Part A *before* reading Part B (the proposed tooling and open questions), so your first impressions of the language are uninfluenced.

---

# Part A — The language (read this, then do Tasks A1–A3)

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

---

## Tasks A1–A3 (do these before reading Part B)

**A1.** Using only the spec above, write complete `.ksh` source for: a browser → CDN → load balancer → 3 app servers (side by side) → primary Postgres with a read replica. The app servers also publish to a Kafka queue consumed by 2 workers that write to S3. App servers and workers sit inside a "k8s cluster" group, which is inside a "VPC" group. There should be no line crossings. Include at least one curved line (e.g. a retry arrow from a worker back to Kafka).

**A2.** Move the load balancer 200px to the right. List every line of your file you had to change.

**A3.** Report, as specifically as you can:
1. Every point where you hesitated, guessed, or weren't sure what the spec meant. Quote the spec wording that caused it.
2. Syntax you instinctively reached for (from Mermaid, D2, Graphviz, TikZ, SVG and similar) that isn't supported.
3. Ambiguities that would lead two agents to produce different files for the same intent.
4. Where the coordinate arithmetic was hard to do mentally, and what would reduce that burden.

---

# Part B — Proposed agent-facing tooling and open questions

## B1. CLI ergonomics for agents

Proposed principles and commands:

- **One command does everything.** `kashika render` runs every check, prints diagnostics, and writes output (no output if there are errors). A final summary line reads e.g. `2 errors, 3 warnings`.
- **Compiler-style text output by default**, plus `--format json`. The text format looks like this:
  ```
  arch.ksh:14:1: warn[cross]: line#3 (api.right -> db.left) crosses line#5 (worker.top -> queue.bottom) at (312,140)
     14 | line api.right -> db.left "SQL" end=arrow
     19 | line worker.top -> queue.bottom end=arrow
     = help: line#5 could pass left of x=300 or right of x=330 (segment 1 spans x 300..330)
     = help: if intentional, add allow=cross to either line
  ```
  Output is deterministic and stably ordered. ANSI color is used only on a TTY. Output is capped, with an `... and N more` message.
- **`-` reads from stdin**, so agents can check snippets without writing files.
- **`kashika ref [topic]`** prints the language reference built into the binary. It always matches the installed version and is sized for an agent's context window. Topics include `anchors`, `paths`, `styles` and `lints`.
- **`kashika explain <code>`** gives a longer explanation and an example fix for each diagnostic code, in the style of `rustc --explain`.
- **`kashika inspect <file> [ids...]`** dumps the resolved geometry: the absolute bounding box of every shape, the coordinates of every anchor, the polyline/Bézier points of every line, and the bounding box of every label. This is the agent's "text vision" when it can't view images. Example:
  ```
  box vpc.worker  bbox=(60,240)-(200,300)  anchors: top=(130,240) right=(200,270) bottom=(130,300) left=(60,270) center=(130,270)
  line#2  api.bottom(180,140) -> (180,180) -> (260,180) -> vpc.worker.top(130,240)   len=310  label: none
  ```
- **`kashika render --debug`** overlays a coordinate grid with rulers, element IDs, anchor dots and markers at each diagnostic location. This is for multimodal agents that do view the PNG and need to map what they see back to coordinates.
- **`kashika fix`** applies only *machine-applicable* suggestions: typo'd anchors and IDs, snapping a near-miss endpoint to the anchor it was obviously meant to hit, and growing a box to fit its label. It never reroutes lines or moves boxes.
- **`kashika fmt`** canonicalizes the file (column alignment, attribute order) so diffs stay small.
- **Exit codes:** `0` clean, `1` warnings, `2` errors.

## B2. Actionable diagnostics

Every diagnostic carries:
- `code` and `severity`
- the primary span, with a source snippet
- **every element involved**, with its source location and resolved geometry
- **geometric facts**: the intersection point, the overlap rectangle, the required size
- `help` entries, and optionally `suggestions`: concrete edits `{span, replacement, applicability: "machine" | "maybe"}`

Example JSON for a line passing through a box:
```json
{
  "severity": "warn", "code": "through",
  "message": "line#4 passes through box 'cache'",
  "span": {"file": "arch.ksh", "line": 22, "col": 1},
  "elements": [
    {"ref": "line#4", "span": {"line": 22}, "segment": 1, "from": [180,140], "to": [520,140]},
    {"ref": "cache", "span": {"line": 9}, "bbox": [[300,110],[380,170]]}
  ],
  "facts": {"entry": [300,140], "exit": [380,140]},
  "help": [
    "segment 1 runs at y=140; box 'cache' spans y 110..170",
    "route above (y < 110) or below (y > 170), e.g. insert `-> v -50 -> h 340 -> v 50`"
  ],
  "suggestions": []
}
```

Example for label overflow (machine-applicable):
```json
{"severity":"error","code":"label-overflow",
 "message":"label \"Postgres Replica\" (172x18 at 14px) does not fit box 'db' (140x60, padding 8)",
 "suggestions":[{"span":{"line":7,"col":38,"len":14},"replacement":"size (188,60)","applicability":"machine"}]}
```

**Open question:** how far should hints go? Reporting *corridors* ("route above y<110 or below y>170") is cheap and generic. Suggesting full reroutes is effectively auto-layout and risks fighting the agent's intent. The current proposal is corridors only.

## B3. Language questions the designers already have

1. Comments use `#`, but so do hex colors (`fill=#e8f0fe`). Should comments switch to `//`?
2. `dot` means a dotted stroke, but `end=dot` means a dot arrowhead. Should the stroke pattern become `stroke-style=dashed|dotted`?
3. Line labels have two forms: a positional `"SQL"` and `label@0.3="..."`. Should these be unified into a single `label=` form?
4. Is `curve bend=N` ("positive = left of the direction of travel") intuitive for an agent, or would a `bend=left:30` form be clearer?
5. Are short corner names (`tl`, `tr`) better than `top-left`, or should both be accepted?
6. After `h`/`v` segments, the final segment to an anchor may be diagonal. Should `check` warn when the other segments are orthogonal?
7. Should boxes be placeable relative to other boxes (`at db.tr+(40,0)`)? This would cut down the arithmetic and keep relationships intact when things move, at the cost of dependency chains.
8. Is one-statement-per-line too restrictive? Are there cases where a long `line` would be clearer written across several lines?

## B4. Review questions

Please answer concisely:
1. Of the tooling in B1, which items are essential, which are nice-to-have, and which are unnecessary? Is anything missing that you'd want as an agent?
2. Are the diagnostics in B2 actionable enough that you could fix the problem in one edit? What is missing?
3. Give your answer to each question in B3, with a one-line justification.
4. What would most likely make you produce a *wrong-looking* diagram while `check` still reports it as clean?
5. What should go into the reference that ships with the tool (`kashika ref`) to make you productive in a single read? Roughly how long should it be?
6. List your top 5 changes to the whole design, ranked by impact.
