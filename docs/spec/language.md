# Kashika — Language & CLI Specification (v1)

Kashika is a text-to-diagram language for technical design diagrams (architecture, data flow, system boundaries). **There is no automatic layout.** The author states the exact position of every shape and the exact path of every line. Coding agents are the main authors, since they handle that precision well; humans read and review the results.

The CLI renders `.kash` files to SVG or PNG and runs geometric checks (crossings, overlaps, near-misses, text overflow). It also exposes the resolved geometry, so an author who cannot see the image can still verify and repair it.

How v0 became v1 is recorded in `docs/review/review-synthesis.md`.

---

## 1. Coordinates and frames

- Units are pixels. The origin `(0,0)` is the canvas **top-left**; **x grows right, y grows down**.
- Every `group` defines a **frame** whose origin is the group's top-left corner.
- **Rule: every literal coordinate is in the frame of the group where the statement is written.** This covers `at`, literal points in lines, `curve via` control points and numeric `x`/`y` targets. Top-level statements use the canvas frame.
- Anchor references (`db.left`) always resolve to the referenced shape's true position, whatever frame you write them in.
- Relative moves (`h`, `v`) and offsets (`+(dx,dy)`) are deltas, so they are the same in every frame.
- Every resolved coordinate (positions, line points, curve control points, and curve amounts) must lie within ±1,000,000 of the canvas origin; anything further is a `degenerate-curve` error ("value is out of range"). Every size must be at most 1,000,000 (`size` error).

`kashika inspect` prints both canvas and local coordinates for everything.

## 2. Lexical rules

- Comments: `//` to end of line, and `/* ... */` block comments, both outside strings. Block comments don't nest. A block comment that spans lines counts as one line break, so it never joins two statements. An unclosed `/*` is a `parse` error. `#` is never a comment character.
- A UTF-8 byte order mark at the start of the file is ignored.
- Strings: `"..."` with escapes `\"`, `\\`, `\n`, and the markup escapes `\*` and `` \` `` (§6). Any other escape, such as `\q`, is a `parse` error.
- Identifiers: `[a-zA-Z_][a-zA-Z0-9_-]*`. **Ids are unique across the whole file** (shapes, groups, junctions, lines, texts) and are always referenced by bare id. There are no dotted paths.
- The segment keywords `h`, `v`, `x`, `y`, `hv`, `vh` and `curve` are reserved: declaring an id with one of these names is a `parse` error, since it could not be used as a bare segment target.
- Numbers: integers or decimals, optionally negative. Colors: `#rgb`, `#rrggbb` or `#rrggbbaa`.
- **Statements** end at a newline, except in three cases:
  - inside `{ }` blocks;
  - when the next non-blank line starts with `->`. A long path can then be written one segment per line:
    ```
    line app1_db: app1.top
      -> y 40
      -> hv pg.left
      end=arrow label="SQL"
      ortho
    ```
  - when the next non-blank line starts with an attribute (`key=` or `key@t=`) or, on a `line`, with a line flag (`ortho`). Neither can begin a new statement, because no statement keyword is followed by `=` or `@` and no statement keyword is also a flag.
- Attributes are `key=value` pairs in any order. Unknown or duplicate attributes are errors.

## 3. Statements

```
diagram "<title>" size (w,h) [kashika=1] [grid=N] [clearance=N] [icons="<dir>"] [allow=<code>,...]
import "<file.kash>" [as <ns>]                      // §3a
style <name> { key=value ... }
shape <name> { ... }  |  shape <name> from "<file.svg>" [...]   // §3a
box   <id> ["<label>"] at <point> size (w,h) [pin=<anchor>] [shape=<shape>] [style=<name>] [attrs]
group <id> ["<label>"] at <point> size (w,h) [pin=<anchor>] [style=<name>] { ... }
junction <id> at <point> [dot]
text  [<id>] "<content>" at <point> [size=N] [color=C] [align=left|center|right]
line  [<id>:] <point> (-> <segment>)+ [attrs]
image <id> "<file>" at <point> size (w,h) [pin=<anchor>] [fit=contain|cover|stretch]   // §3a
```

`diagram` comes first, then any `import` statements, then everything else. "First" means first in the source: a statement before `diagram` makes it misplaced even when that statement failed to parse. Boxes and groups accept `sublabel=`, `text-align=`, `text-valign=`, `gap=`, `icon=` and a `{ }` text block (§6).

- `grid=N` makes `off-grid` check authored literals only, never resolved anchors. `clearance=N` (default `6`) is the minimum gap used by the `near` lint.
- Numeric attributes must be in range, or they are `attr` errors: `font-size`, `stroke-width` and `grid` must be greater than 0; `radius`, `padding` and `clearance` must be at least 0. All of them must be at most 1,000,000. A `text` `size=` outside that range is a `size` error.
- `style` keys:
  - `fill`, `stroke`, `stroke-width`, `stroke-style=solid|dashed|dotted`
  - `font-size`, `font-weight=normal|bold`, `color`
  - `radius`, `padding`, `muted` (the color for sublabels and `muted` text)

  Built-in styles: `service`, `datastore`, `external`, `queue`, `boundary`, `note`. A user style with a built-in's name overrides it **key by key**. Precedence: inline attribute > `style=` > built-in default.
- `box` shapes: built-in `rect` (default), `rounded`, `cylinder`, `diamond`, `ellipse`, `hexagon`, `doc`, `cloud`, `person`, or any `shape` that is defined or imported (§3a).
- `pin=<anchor>` picks which anchor of the new shape lands on `at` (default `tl`). It takes a plain anchor name, so `@t` is not allowed. For example, `box cache at db.right+(60,0) size (120,60) pin=left` vertically centers `cache` beside `db`.
- `at` may reference other shapes, so boxes can be placed relative to each other. Placement references form a DAG; a cycle is an error that prints the cycle. A chain of placement references deeper than 256 is also a `placement-cycle` error.
- `group`: children are `box`, `group`, `junction`, `text`, `line`. Groups nest at most 64 deep; a deeper `group` is a `parse` error. Groups start from the `boundary` style (including a user override of it), and `style=` and inline attributes apply on top. The label sits inside the top-left corner, and its title band (`font line-height + padding`) is reserved space. Children that intrude on the title band trigger `spill`.
- `junction`: a zero-size named point that lines can start or end at. Lines meeting at a junction are connected, not crossing. `dot` draws a 6px dot there. Junctions model buses and fan-in/fan-out.
- `text`: `at` is the anchor point for the alignment. `color=none` is an `attr` error. Vertically, `at` is the **center** of the text block, not the baseline.

**Paint order is fixed:** groups (outer before inner) → boxes and images → lines → line labels and texts. Within a layer, file order applies. Lines are never hidden by boxes.

## 3a. Shapes, images and imports

### Custom shapes

Paths are written in **unit coordinates**, from 0 to 1, and stretched to each box's size:
```
shape lb_icon {
  path "M 0 0.5 L 0.5 0 L 1 0.5 L 0.5 1 Z  M 0.2 0.5 L 0.8 0.5"   // SVG subset: M L H V C Q A Z
  outline 1                    // subpath used for anchors and checks (default 1)
  text-area (0.2,0.3,0.6,0.4)  // unit rect (default: bbox inset by padding)
}
box lb "LB" at (100,80) size (120,80) shape=lb_icon
```
- All built-in shapes are defined this way in a standard library. `ref shapes` prints those definitions.
- `cloud` and `person` are built-ins.
- Checks use the outline, flattened into straight segments.

### SVG files as shapes
```
shape s3 from "icons/aws/s3.svg" [outline=bbox|hull] [text-area=below|inside]
```
- The SVG's drawing is scaled to the box.
- Anchors and checks use the bounding box (default) or the convex hull of the drawing.
- `text-area=below` puts the label under the icon, the usual style in architecture diagrams.

### Images
```
box s3 "Uploads" at (600,300) size (120,60) icon="icons/aws/s3.svg" [icon-pos=left|top] [icon-size=N]
image logo "assets/hf.png" at (20,20) size (80,80) [fit=contain|cover|stretch]
```
- Formats: SVG, PNG, JPEG.
- The checker treats an image as an opaque rectangle.
- `inspect` reports each image's natural size and aspect ratio.

### Imports
```
import "lib/aws.kash"          // brings in shape and style definitions
import "lib/aws.kash" as aws   // namespaced: shape=aws.s3 style=aws.service
```
Library files may contain only `shape`, `style` and `import`. An import cycle is an error.

### Rules for files
- **Local files only.** No URLs.
- **Path resolution:** paths are relative to the importing file. A path that escapes the project root is an error unless `--allow-path` is given.
- **Search path:** besides the importing file's directory, Kashika searches `icons=` on the `diagram` line and `KASHIKA_ICON_PATH`.
- **SVG sanitizing:** scripts, external references and `foreignObject` are stripped.
- **Self-contained output:** images are embedded in SVG output and rasterized into PNG output.
- **Missing files:** a missing file is an error, with did-you-mean suggestions.
- **Licensing:** no third-party icon sets are bundled. Kashika ships a small neutral set (`server`, `browser`, `queue`, `lock`, `cloud`, `person`). Users point the search path at icon sets they download themselves.

## 4. Points and anchors

```
point  := "(" coord "," coord ")"
        | ref ["+(" dx "," dy ")"]
ref    := <id> "." <anchor> | <junction-id>
coord  := number                             // literal, in the current frame
        | ref ".x" | ref ".y"                // mixed: (db.left.x, lane1.y)
anchor := top | bottom | left | right | center
        | tl | tr | bl | br                   // aliases: top-left, top-right, bottom-left, bottom-right
        | (top|bottom|left|right) "@" t      // 0 <= t <= 1
```

- **Mixed points** take each axis from a different reference, so lanes can be expressed as relationships: `(app1.bottom.x, lane1.y)`. A `junction` without `dot` that no line ends at works as a named guide point.
- `@t` and offsets compose: `db.left@0.25+(0,4)`.
- **Groups have anchors** just like boxes, e.g. `k8s.top`.

- `top@t`/`bottom@t` run left→right; `left@t`/`right@t` run top→bottom. `top` is the same as `top@0.5`.
- Corner anchors are always bounding-box corners.
- On non-rectangular shapes, edge anchors are **axis-projected**:
  - `top@t`/`bottom@t` keep the bounding-box point's x and move vertically onto the outline.
  - `left@t`/`right@t` keep the y and move horizontally.

  A vertical line into `db.top` therefore stays vertical. Outlines are defined in `ref shapes`. For example, the top of a cylinder is the upper arc of its cap ellipse, whose height is `min(20, h/4)`.
- Offsets use `+` only, with signed components: `db.top+(-10,0)`.
- `@t` errors are reported as `unknown-anchor`: a `t` outside 0..1 ("`t` must be between 0 and 1"), and `@t` on a corner or `center` ("`@t` only applies to `top`/`bottom`/`left`/`right`").

## 5. Line paths

A line is `start -> seg -> seg ...`. Every segment ends at a point, and that point becomes the "previous point" for the next segment.

| segment | path drawn |
|---|---|
| `<point>` | straight to the point |
| `h <dx>` / `v <dy>` | horizontal or vertical **move by** dx / dy. Negative values go left or up |
| `x <X>` / `y <Y>` | horizontal or vertical **move to** x=X / y=Y. X/Y is a number (in the current frame) or a point (its x or y is used), e.g. `y db.top` |
| `hv <point>` | horizontal, then vertical, to the point (one elbow) |
| `vh <point>` | vertical, then horizontal, to the point |
| `curve left:<N> <point>` / `curve right:<N> <point>` | quadratic curve to the point that bulges N px at its midpoint, to the left or right **as seen on screen by someone walking along the line**. Example: a line traveling right with `left:30` bulges **up** |
| `curve via (c1) (c2) <point>` | cubic Bézier with explicit control points |

Line attributes:
- `start=` / `end=`: `arrow`, `open`, `diamond`, `dot`, `none` (both default to `none`). The stroke is trimmed so the marker's tip sits on the endpoint. The trim follows the end segment, whether straight or curved. When both markers sit on the same segment and their lengths add up to more than the segment, each end is trimmed in proportion to its marker's length.
- `stroke-style=solid|dashed|dotted`, `stroke`, `stroke-width`, `style=`. On lines, a style applies only its `stroke*` and font keys.
- `radius=N` rounds every polyline corner. The radius is clamped to half the shorter adjacent segment.
- `label="..."` is the same as `label@0.5="..."`. `label@t="..."` is repeatable. `t` is the fraction of the final rendered path's **arc length**. Labels are horizontal, centered on the point, and drawn over a background halo.
- `ortho`: asserts that every segment is horizontal or vertical. Any other segment is an error.
- `allow=<code>[:<other-id>],...` and optional `reason="..."` (see §7).

Unnamed lines are referred to as `line@L<n>`, where n is the source line of the statement. Naming lines is recommended.

## 6. Text

### Text in shapes: sublabel by default, block when needed

- **Default:** `box api "Payments API" sublabel="v2.4.1" ...`
  - The label is the name.
  - The sublabel is one muted line below it, at `0.8 × font-size` in the style's `muted` color.
- `text-align=left|center|right` (default center) and `text-valign=top|middle|bottom` (default middle) position the whole text stack within the shape's **text area**.
- **Inline markup** works in `label`, `sublabel` and all `text`: `**bold**` and `` `code` `` (monospace). Markup does not nest. The markers are `**` and `` ` ``; a single `*` is literal. Escape with `\*` and `` \` ``, which always produce a literal `*` or `` ` ``. An unclosed marker is a `parse` error.
- **Text block**, for 3 or more lines, bullet lists, or mixed sizes and alignment:
  ```
  box api at (100,80) size (200,120) style=service text-align=left {
    text "**Payments API**"
    text "v2.4.1" size=11 muted
    text "• validates cards" size=12
    text "• retries charges" size=12
  }
  ```
  - Block items **without `at` stack** top to bottom, separated by `gap` (box attribute, default 2), and aligned by the box's `text-align`/`text-valign`. No coordinates are needed.
  - Items **with `at`** are absolute, in the **box-local frame** (origin is the box's top-left). They leave the stack. This form is advanced.
  - Block text items take no id.
  - Using `label`/`sublabel` together with stacked block items is an error (`text-conflict`). It is allowed when every block item has its own `at`.
- `ref` shows `sublabel` in the main example. The block is documented under "when sublabel is not enough", and absolute `at` comes last.

### Fonts and metrics

- Kashika bundles Inter (Regular and Bold) and JetBrains Mono (for `` `code` ``), so measurements are identical on every machine.
- Defaults: `font-size=14`, line height `1.25 × font-size`, `padding=8`.
- A box's **text area** is its bounding box inset by `padding`, with a further reduction for some shapes:

  | shape | text area |
  |---|---|
  | diamond | central 50% |
  | ellipse | the inscribed rectangle |
  | hexagon | horizontally inset by `h/4` |
  | cylinder | below the cap |
- The stacked text (or each absolute item) must fit the text area. `label-overflow` reports the required box size. For small text areas, such as a diamond's, it suggests a roomier shape.
- `kashika measure "text" [--size N] [--weight bold]` prints a label's exact width and height, with markup applied. `kashika ref metrics` lists average character widths at common sizes.

## 7. Checks

All checks run on the same resolved scene that the renderer draws. Tolerance is 0.5 px. Curves take part in every check; they are flattened to within 0.25px.

### Errors

These block output unless `--force` is given:

| code | trigger |
|---|---|
| `parse` | Includes expected tokens and a suggestion. Also: unknown string escapes, reserved ids (§2), groups nested more than 64 deep |
| `unknown-id` / `unknown-anchor` | Includes did-you-mean. `unknown-anchor` also covers a bad `@t`: out of 0..1, or on a corner or `center` |
| `duplicate-id` | Two statements use the same id |
| `placement-cycle` | Relative placements form a cycle, or a chain of them is deeper than 256 |
| `label-overflow` | Reports the required size |
| `not-ortho` | A segment is diagonal on a line marked `ortho` |
| `degenerate-curve` | A zero-length curve, a negative curve amount, or a non-finite or out-of-range value (a coordinate more than 1,000,000 from the origin, §1) |
| `zero-length` | A line whose segments all have zero length |
| `attr` | Unknown, duplicate, mistyped or out-of-range attribute (§3), or a duplicate label position |
| `unknown-style` | `style=` names no built-in or user style |
| `size` | A non-positive size, or one larger than 1,000,000 |
| `diagram` | Missing, misplaced or repeated `diagram` statement |
| `version` | `kashika=N` doesn't match the binary |
| `unsupported` | A spec feature this build doesn't implement yet |

### Warnings

| code | trigger |
|---|---|
| `cross` | Two lines intersect, including tangency, at a point that is not a shared endpoint or junction |
| `shared-segment` | Two lines overlap collinearly for more than 1px |
| `through` | A line enters a box's interior. For a line's own start and end boxes, only the first and last segments are exempt, so leaving and re-entering is flagged. Groups are never obstacles |
| `near` | A line runs within `clearance` of a box edge it doesn't attach to, or within `clearance` of a parallel line for more than 20px. **This includes a line running along its own box's edge** |
| `overlap` | Two shapes, neither containing the other, have intersecting interiors. Containment without a group is still an overlap |
| `spill` | A child extends outside its group or into the group's title band. Touching the edge is fine |
| `diagonal-tail` | A line's final segment is diagonal when all its other straight segments are horizontal or vertical |
| `endpoint-off-edge` | A literal endpoint is within 8px of a shape outline but not on it |
| `port-crowding` | Two endpoints on the same shape are within 8px of each other. This includes identical endpoints where both lines draw a marker (stacked arrowheads). Junctions are exempt |
| `group-transit` | A line passes through a group's interior while neither of its endpoints is inside that group |
| `low-contrast` | The contrast ratio between text color and its background fill is below 3:1 |
| `block-reducible` | A text block holds 1–2 stacked items in a uniform style. The help line shows the `label`/`sublabel` rewrite |
| `text-outside-shape` | An absolute block item lies outside the shape's text area |
| `image-distort` | `fit=stretch` changes an image's aspect ratio by more than 5% |
| `label-collision` | A label or text box intersects a box, another label or text, a group title, or a line other than its own |
| `out-of-canvas` | An element extends beyond the diagram size |
| `off-grid` | An authored literal is not a multiple of `grid` |

### Suppression

- `allow=cross` on a line silences all crossings for that line.
- `allow=cross:other_line` silences only crossings with that one line. This is preferred, so a later edit doesn't introduce hidden crossings.
- `reason="..."` is optional and kept in the JSON output.
- **Suppressed findings are counted in every summary**, and `--show-suppressed` lists them.

### What a clean check does not prove

A clean check does not prove that arrows point the right way or that the right boxes are connected. It only covers the geometry listed above. `ref` states this explicitly.

## 8. CLI

```
kashika render  <file|-> -o <out.svg|out.png> [--force] [--debug]
kashika check   <file|->
kashika inspect <file|-> [ids...] [--ports] [--corridors <x1,y1,x2,y2>]
kashika fmt     <file> [--check]
kashika measure "<text>" [--size N] [--weight bold]
kashika route   <file|-> <from> <to> [corner|arc|bezier] [--line <id>]
kashika icons   [dir...] [--search <term>]     // list importable shapes and icons: name, path, natural size, library
kashika ref     [topic] [--json]          // topics: syntax anchors paths shapes styles metrics lints <lint-code> examples cli
kashika --version
```

Common flags: `--format text|json`, `--fail-on error|warn|never` (default `warn`), `--show-suppressed`, `--focus <id,...>` (only diagnostics involving these ids), `--max N`, `--stdin-name <name>`.

An optional `kashika=1` on the `diagram` line declares which language version the file was written for. A mismatch with the binary is an error.

### render

- Runs every check.
- Writes output when there are no errors. With `--force` it writes even with errors, drawing red markers at each error.
- Writes are **atomic**. The result always states whether output was written for this run.
- `--debug` overlays rulers, element ids, anchor dots and diagnostic markers.

### Exit codes

`0` clean, `1` warnings (when `--fail-on warn`), `2` errors, `3` usage or I/O failure.

### Text output

The text output is compiler-style, deterministic and has no color unless on a TTY. It ends with a summary that includes metrics:

```
arch.kash:14:1: warn[cross]: l_d2 crosses l_lb3 at (640,165)
   14 | line l_d2: app2.top@0.8 -> v -85 -> hv pg.top
   11 | line l_lb3: lb.bottom@0.8 -> vh app3.top
   = free x-intervals for l_lb3's vertical in band y 130..230: [520,600] [700,780]
   = if intentional: allow=cross:l_lb3 on l_d2
2 errors, 3 warnings (1 suppressed) · crossings=2 wire=4210px · output: not written
```

### JSON output

A single versioned envelope is written to stdout. Progress goes to stderr.

```json
{"schema":"kashika.check/1","tool":"0.1.0",
 "input":{"name":"arch.kash","sha256":"…"},
 "output":{"path":"arch.svg","written":false},
 "counts":{"error":2,"warn":3,"suppressed":1},"truncated":false,
 "metrics":{"crossings":2,"wire_px":4210},
 "diagnostics":[{
   "severity":"warn","code":"through","message":"l_4 passes through box 'cache'",
   "span":{"line":22,"col":1,"end_line":22,"end_col":58},
   "elements":[
     {"id":"l_4","kind":"line","span":{"line":22},"segment":1,"from":[180,140],"to":[520,140]},
     {"id":"cache","kind":"box","span":{"line":9},"bbox":[[300,110],[380,170]],"frame":"vpc","local_bbox":[[260,70],[340,130]]}],
   "facts":{"entry":[300,140],"exit":[380,140],
            "free":{"axis":"y","band":[180,520],"intervals":[[60,104],[176,230]]}},
   "help":["segment 1 runs at y=140; free y-intervals across x 180..520: 60..104, 176..230"],
   "suggestions":[]}]}
```

The fields:
- `free` lists intervals that are clear of **every** box and line, with clearance applied, across the offending segment's span. It is not just the extent of the obstacle.
- `cross` and `shared-segment` give each line's segment index and the overlapping span. `shared-segment` names the nearest free parallel lane. `label-collision` suggests `t` values that are clear.
- `suggestions` contain `{span, replacement, ripple: [ids whose geometry would change]}`. They are never applied automatically.
- Spans are 1-based, columns count Unicode scalar values, and positions refer to the input as given.

### inspect

Prints the resolved geometry: every shape's bounding box and anchors in both canvas and local frames, every line's vertices, segment kinds and arc length, every label's bounding box, and the placement dependency graph.

- `--ports` lists, for each shape side, which anchors are already used and by which lines.
- `--corridors` lists the free horizontal and vertical intervals within a rectangle.
- `--format json` is supported.
- Errors don't stop it, so a file with errors can still be inspected.

### route

```
kashika route <file|-> <from> <to> [corner|arc|bezier] [--line <id>] [--radius N] [--max N]
```

Suggests up to `--max` (default 3) candidate paths from `<from>` to `<to>`. Both are points, e.g. `app1.bottom` or `pg.left@0.25`. It **never edits the file**; the author chooses which suggestion, if any, to paste.

- **The third parameter sets the style** of the suggestions (default `corner`):

  | style | output |
  |---|---|
  | `corner` | orthogonal paths with hard corners, written with `h`/`v`/`x`/`y`/`hv`/`vh` |
  | `arc` | the same orthogonal paths plus `radius=N` for rounded corners (default 8, set with `--radius`) |
  | `bezier` | smooth paths written with `curve left:N` / `right:N` where possible, otherwise `curve via` |
- `--line <id>` reroutes an existing line. The line is ignored as an obstacle, its endpoints are used when `<from>`/`<to>` are omitted, and its attributes are kept in the output.
- Candidates must not cross into any box (`through`), and paths must keep `clearance`. They are ranked by:
  1. crossings
  2. `near` findings
  3. number of segments
  4. length

Each candidate lists these numbers, which line it crosses and where, and the corridor it uses. It is printed as a ready-to-paste `line` statement:

```
#1  crossings=0 near=0 segments=3 length=412px  via corridor y 200..220
    line app1_db: app1.top -> y 210 -> hv pg.left@0.25 end=arrow
#2  crossings=1 (l_lb3 at (640,210)) near=0 segments=2 length=388px
    line app1_db: app1.right -> hv pg.left@0.25 end=arrow
```

Suggestions use anchors and axis targets rather than literals wherever possible, so they survive later moves. `--format json` returns the same data structured. The search covers a bounded set of paths with at most 2 elbows (orthogonal styles) or a single curve (`bezier`). It is a suggestion tool, not a layout engine.

### ref

`ref` is embedded in the binary and matches its version. The default page is about 1,500–2,000 words. It contains:
- the five non-obvious rules (y-down; the frame rule; ids are global; anchors resolve absolutely; what `allow` hides)
- a compact grammar
- the anchor, segment and attribute tables, with defaults
- the lint table, with a one-line fix for each code
- routing recipes:
  - staircase
  - wrap over the top
  - gap corridor
  - outside margin
  - fan-out via a junction
  - the comb: N sources into one edge, with nested lanes and staggered landings
  - unavoidable crossings, i.e. non-planar graphs
- a complete worked example
- an explicit "not supported" list
- the limits of a clean check

All examples in `ref` are tested fixtures.

### Deferred (not in v1)

- `fix` (auto-applying suggestions)
- `diff --geometry`
- arithmetic expressions in literals
- named guides
- remote URLs for images and imports
- recoloring icons
- font files supplied by the user
- automatic text wrapping (use `\n` or a text block)
- themes
- WASM
