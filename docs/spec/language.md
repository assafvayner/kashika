# Kashika — Language & CLI Specification (v1)

Kashika is a text-to-diagram language for technical design diagrams (architecture, data flow, system boundaries). **There is no automatic layout.** The author states the exact position of every shape and the exact path of every line. Coding agents are the main authors, since they handle that precision well; humans read and review the results.

The CLI renders `.kash` files to SVG, PNG, JPEG or WebP and runs geometric checks (crossings, overlaps, near-misses, text overflow). It also exposes the resolved geometry, so an author who cannot see the image can still verify and repair it.

---

## 1. Coordinates and frames

- Units are pixels. The origin `(0,0)` is the canvas **top-left**; **x grows right, y grows down**.
- Every `group` defines a **frame** whose origin is the group's top-left corner.
- **Rule: every literal coordinate is in the frame of the group where the statement is written.** This covers `at`, literal points in lines, `curve via` control points and numeric `x`/`y` targets. Top-level statements use the canvas frame.
- Anchor references (`db.left`) always resolve to the referenced shape's true position, whatever frame you write them in.
- Relative moves (`h`, `v`) and offsets (`+(dx,dy)`) are deltas, so they are the same in every frame.
- Every resolved coordinate (positions, line points, curve control points, and curve amounts) must lie within ±1,000,000 of the canvas origin; anything further, or a value too large to be a finite number, is a `range` error ("value is out of range"). Every size must be at most 1,000,000 (`size` error).

`kashika inspect` prints both canvas and local coordinates for everything.

## 2. Lexical rules

- Comments: `//` to end of line, and `/* ... */` block comments, both outside strings. Block comments don't nest. A block comment that spans lines counts as one line break, so it never joins two statements. An unclosed `/*` is a `parse` error. `#` is never a comment character: a `#` that begins a line, or stands alone outside an attribute value, is one `parse` error suggesting `//`, and the rest of that line is skipped.
- A UTF-8 byte order mark at the start of the file is ignored.
- Strings: `"..."` with escapes `\"`, `\\`, `\n`, and the markup escapes `\*` and `` \` `` (§6). Any other escape, such as `\q`, is a `parse` error.
- Identifiers: `[a-zA-Z_][a-zA-Z0-9_-]*`, where a `-` that starts `->` or `-(` ends the identifier (`a->b`, `db.top-(10,0)`). **Ids are unique across the whole file** (boxes, groups, images, junctions, lines, texts) and are always referenced by bare id. There are no dotted paths. Only the names of imported shapes and styles are dotted (`shape=aws.s3`, §3a); shape and style names are separate from ids.
  Identifiers are ASCII only: a run of other characters, such as the `é` in `café`, is one `parse` error.
- The segment keywords `h`, `v`, `x`, `y`, `hv`, `vh`, `vhv`, `hvh` and `curve` are reserved: declaring an id with one of these names is a `parse` error, since it could not be used as a bare segment target. So is the statement keyword `arrow` (§5). Like any statement that fails to parse, the statement is dropped, so it is never resolved or linted.
- Numbers: integers or decimals, optionally negative. Exponent notation (`1e5`) is not supported and is a `parse` error. Colors: `#rgb`, `#rrggbb` or `#rrggbbaa`, one of the 148 CSS Color Module Level 4 names (`steelblue`, `rebeccapurple`; case-insensitive, no `transparent`), or `none` where a style allows it. A name resolves to lowercase `#rrggbb` hex, so the scene, the SVG and `low-contrast` only ever see hex. An unknown word where a color is expected is an `attr` error with a did-you-mean over the names.
- **Statements** end at a newline, except in three cases:
  - inside `{ }` blocks;
  - when the next non-blank line starts with `->`. A long path can then be written one segment per line:
    ```
    line app1_db app1.top
      -> y 40
      -> hv pg.left
      end=arrow label="SQL"
      ortho
    ```
  - when the next non-blank line starts with an attribute (`key=` or `key@t=`) or, on a `line` or `arrow`, with a line flag (`ortho`). Neither can begin a new statement, because no statement keyword is followed by `=` or `@` and no statement keyword is also a flag.
- Attributes are `key=value` pairs in any order. Unknown or duplicate attributes are errors.
- After a statement's id and label, `at <point>`, `size (w,h)` and attributes come in any order: `box a "A" style=service size (120,60) at (20,20)` is the same box as the form in §3. A missing `at`, a missing `size` on a statement that requires one (§3), or a repeated `at` or `size`, is a `parse` error that drops the statement.
- `auto` is a size axis, not a word: it is recognized only inside `size (…)` (`size (120, auto)`), so it remains a valid id.

## 3. Statements

```
diagram "<title>" [size (w|auto, h|auto)] [margin=N] [kashika="0.1.0"] [grid=N] [clearance=N] [background=C] [icons="<dir>"] [allow=<code>,...]
import "<file.kash>" [as <ns>]                      // §3a
style <name> { key=value ... }
shape <name> { ... }  |  shape <name> from "<file.svg>" [...]   // §3a
box   <id> ["<label>"] at <point> [size (w|auto, h|auto)] [pin=<anchor>] [shape=<shape>] [style=<name>] [layer=under|over] [allow=<code>[:<id>],...] [reason="..."] [attrs]
group <id> ["<label>"] at <point> [size (w|auto, h|auto)] [pin=<anchor>] [style=<name>] [allow=<code>[:<id>],...] [reason="..."] { ... }
junction <id> at <point> [dot]
text  [<id>] "<content>" at <point> [font-size=N] [color=C] [text-align=left|center|right] [muted]
line  [<id>] <point> (-> <segment>)+ [attrs]
arrow [<id>] <point> (-> <segment>)+ [attrs]   // a line with end=arrow (§5)
image <id> "<file>" at <point> size (w,h) [pin=<anchor>] [fit=contain|cover|stretch] [layer=under|over]   // §3a
```

`diagram` comes first, then any `import` statements, then everything else. "First" means first in the source: a statement before `diagram` makes it misplaced even when that statement failed to parse. Boxes accept `sublabel=`, `text-align=`, `text-valign=`, `gap=`, `icon=` and a `{ }` text block (§6). Groups accept `sublabel=`, `text-align=`, `gap=` and `icon=`; a group's `{ }` holds its children, and `text-valign` on a group is an `attr` error because the title always sits at the top.

- A `box` may leave out `size`: it is then **fitted to its text**, the smallest size whose text area holds the label, sublabel and text block, exactly the size `label-overflow` reports (§6): it accounts for the shape's text area, `padding`, an icon, the sublabel and the block (stacked items, and each absolute item's own extent). `size (w, auto)` fixes the width and fits the height, `size (auto, h)` the other way round, and `size (auto, auto)` is the same as no `size`. A fitted axis is rounded up to whole pixels, then up to a multiple of `grid` when the diagram sets one. A box with nothing to fit (no label, sublabel or block item), or whose shape has `text-area=below` (its text sits under it, §3a), is a `size` error with help ``give it a size: `size (w,h)` ``. Everything downstream sees the fitted size: `pin`, anchors, shapes placed against the box, checks, `inspect` and the JSON. An `image` takes a numeric `size (w,h)`, and `auto` there is a `parse` error. A box or group without `size` takes its `style`'s `size` (below).
- A `group` may leave out `size` too (its own and its style's), or give an axis as `auto`: it is then **fitted to its children**. Its top-left corner is placed as usual and children are never moved; a fitted axis runs from the group's origin to the farthest right or bottom edge of everything painted inside it, at any depth (as for the canvas below: shapes and nested groups with half their stroke, their text and icons, junction dots, texts, and lines with their markers and labels), plus `padding` (`x` on the right, `y` at the bottom). It is at least large enough for the group's own label, sublabel and icon, so its title band always fits. The axis is rounded up to whole pixels, then up to a multiple of `grid`. Nested fitted groups fit from the inside out. Children at negative local coordinates stick out of the top or left edge and `spill` reports them. A fitted group with no children and no label is a `size` error with help ``give it a size: `size (w,h)` ``. Siblings and shapes outside the group may be placed against its fitted anchors, but anything inside it (including a line's point) referencing the group itself is a `placement-cycle` error whose help says to give the group a size. `pin` on a fitted group must name an anchor that stays put as it grows (on the left edge for a fitted width, on the top edge for a fitted height, such as `tl`), and pins its bounding-box point; any other pin is an `attr` error and the group is pinned at `tl`.
- The `diagram` may leave out `size` too: the canvas then runs from `(0,0)` to the farthest right and bottom edge of everything painted, rounded up to whole pixels, plus a **margin** on the right and bottom. "Everything painted" is shapes and group borders (with half their stroke), their labels, sublabels, text blocks, icons and the labels under `text-area=below` shapes, junction dots, texts, and lines with half their stroke or their markers' half-width (whichever is wider) and their labels. `margin=N` sets the margin (default `20`, at least 0, else an `attr` error). `size (w, auto)` fixes the width and fits the height, and `size (auto, h)` the other way round; `margin=` applies to the fitted axes only, so on a diagram whose `size` has no `auto` axis it is an `attr` error. Content is never moved: anything at a negative coordinate still sticks out of the top or left edge, so `out-of-canvas` reports it there, and past a fixed axis; it never fires on the far side of a fitted axis. `render` (SVG and raster), the checks and `inspect` all use the fitted canvas.
- `grid=N` makes `off-grid` check authored literals only, never resolved anchors. `clearance=N` (default `6`) is the minimum gap used by the `near` lint, and the shortest step the `jog` lint accepts.
- `background=C` paints the whole canvas in color `C` (the same color values as styles) under everything else. Without it, or with `background=none`, the canvas is transparent in SVG, PNG and WebP output; JPEG has no alpha, so it composites onto white (§8). Any other value is an `attr` error.
- Numeric attributes must be in range, or they are `attr` errors: `font-size`, `stroke-width`, `icon-size` and `grid` must be greater than 0; `radius`, `padding`, `gap` and `clearance` must be at least 0. All of them must be at most 1,000,000. Each value of `padding=(x,y)` follows the `padding` rule; any other `padding` value is an `attr` error. A `font-size=` on a `text` or a block item (§6) outside that range is a `size` error.
- `style` keys:
  - `fill`, `stroke`, `stroke-width`, `stroke-style=solid|dashed|dotted`
  - `font-size`, `font-weight=normal|bold`, `color`
  - `radius`, `padding`, `muted` (the color for sublabels and `muted` text)
  - `size=(w,h)`, in a `style` definition only, for boxes and groups: the size a box or group gets when it has no `size` of its own; an inline `size` replaces it whole. Either axis may be `auto` (`size=(140, auto)`), which a box fits to its text and a group to its children, as in their own `size`. Each number must be greater than 0 and at most 1,000,000 (`size` error), and any other value, such as `size=12`, is an `attr` error whose help points to `font-size`. Lines ignore a style's `size`, as they ignore its `shape`, and images take no `style=`. `size` in a `style boundary` definition (or a library's `boundary` imported without `as`) is an `attr` error there and is dropped, since every group starts from `boundary`.
  - `shape`, in a `style` definition only, for boxes: any name `shape=` takes on a box. It is resolved where the style is defined, so an unknown name is an `unknown-shape` error there, with did-you-mean. The resolved shape drives everything an inline `shape=` does: outline, anchors, text area, checks, the `rounded` radius default and the scene's `shape`.
  - `padding=N` insets the text area by `N` on every side; `padding=(x,y)` insets it by `x` on the left and right and `y` on the top and bottom. Below, "`padding`" means `x` for horizontal distances and `y` for vertical ones.

  Built-in styles: `service`, `datastore`, `external`, `queue`, `boundary`, `note`. A user style with a built-in's name overrides it **key by key**, and so does a style a library defines under that name when it is imported without `as` (imported `as aws`, it is `aws.service`, a separate style). Precedence: inline attribute > `style=` > built-in default; for `shape`, inline `shape=` > the style's `shape` > `rect`; for `size`, inline `size` > the style's `size` > fitted to the text (boxes) or to the children (groups). Built-in styles set no shape or size; a user override of one may (except `boundary`'s size).
- `box` shapes: built-in `rect` (default), `rounded`, `cylinder`, `diamond`, `ellipse`, `hexagon`, `doc`, `cloud`, `person`, or any `shape` that is defined or imported (§3a). Any other `shape=` name is an `unknown-shape` error.
- `pin=<anchor>` picks which anchor of the new shape lands on `at` (default `tl`). Any anchor works, including `side@t`: `pin=left@0.25`. For example, `box cache at db.right+(60,0) size (120,60) pin=left` vertically centers `cache` beside `db`.
- `at` may reference other shapes, so boxes can be placed relative to each other. Placement references form a DAG; a cycle is an error that prints the cycle (the first few shapes and its length when it is long). A chain of more than 256 placement references is also a `placement-cycle` error, whatever the statement order; a shape inside a group counts a reference to the group, whose frame it uses.
- `group`: children are `box`, `group`, `junction`, `text`, `line` (and `arrow`). `image` is top-level only; inside a group it is a `parse` error. Groups nest at most 64 deep; a deeper `group` is a `parse` error. Groups start from the `boundary` style (including a user override of it), and `style=` and inline attributes apply on top. Groups take no shape: a group's `style=` naming a style that sets `shape` is an `attr` error at that attribute (the rest of the style still applies), and `shape` in a `style boundary` definition is an `attr` error there and is dropped. The label sits inside the top-left corner, inset by `padding`, and its title band (`font line-height + padding`, using `y`) is reserved space. Children that intrude on the title band trigger `spill`.
- `junction`: a zero-size named point that lines can start or end at. Lines meeting at a junction are connected, not crossing. `dot` draws a 6px dot there. Junctions model buses and fan-in/fan-out.
- `text`: `at` is the anchor point for the alignment (`text-align`). `color=none` is an `attr` error. Vertically, `at` is the **center** of the text block, not the baseline.
- Text attribute names: free `text` statements and block items (§6) take `font-size` and `text-align`, the names boxes and styles use. `size` and `align` are accepted there as aliases, and giving both names on one item (`size=12 font-size=14`) is a duplicate-attribute `attr` error at the second, whose help says to keep one. `fmt` keeps whichever name is written. Where a name belongs to another context, the unknown-attribute error suggests the right one: `align=` on a box suggests `text-align`, `size=12` on a box suggests `font-size`, `valign=` suggests `text-valign`, and `size=(w,h)` suggests the `size (w,h)` clause. Attributes other tools use get the Kashika form as help: `label=` on a box, group or text (the label is the quoted string after the id), `arrow=`/`arrowhead=` on a line (`end=arrow`, `start=arrow` for `start`, both for `both`, or the `arrow` statement), `width=`/`height=` on a box, group or image (the `size (w,h)` clause), and a bare `dashed`/`dotted`/`solid` on a line or `style=dashed` naming no style (`stroke-style=dashed`). A word where a string is expected (`icon=server`) gets help to quote it. Where one of a fixed set of words is expected (an anchor name, `pin`, a line end), the help also maps common synonyms: `middle`/`mid`/`centre` to `center`, `north`/`south`/`east`/`west` to `top`/`bottom`/`right`/`left`, and `arrowhead` to `arrow`.

**Paint order is fixed:** `layer=under` boxes and images → groups → boxes and images → lines → line labels and texts → `layer=over` boxes and images. Groups paint in file order, so a group paints right after its parent and before any later sibling. `layer=under` is relative to the shape's parent: a top-level one paints before every group, and one inside a group paints right after that group's own fill and border, before the group's other children and any later group, so the group's fill never hides it. Within every layer, file order applies, so line labels and texts interleave by where their statements appear. Lines are never hidden by boxes, except by `layer=over` ones.

- `layer=under|over` on a `box` or `image` moves it (with its own label, sublabel, text block and icon) to the first or last paint layer above. Without it a shape paints in the normal layer. Groups, texts, junctions and lines take no `layer`; on a group it is an unknown-attribute `attr` error.
- A layered shape is a backdrop or a highlight (a board square, a selection ring), not an obstacle. It is skipped, both as the shape reported and as the thing hit, by `overlap`, `through`, `near` (its edges; lines attached to it are not attached to a box), `label-collision` (as an obstacle, and the label under it as a subject), `endpoint-off-edge`, and the `free` intervals, `inspect --corridors` and `route` obstacles and lanes. Its own checks still apply: `spill`, `out-of-canvas`, `label-overflow`, `text-outside-shape`, `block-reducible`, `image-distort`, `port-crowding` and `off-grid`. `low-contrast` follows the paint order: an under-layer fill counts behind every text (inside a group, above that group's fill), and an over-layer fill only behind that shape's own text and later over-layer shapes' text.

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
- All built-in shapes except `rect` and `rounded` are defined this way in a standard library. `ref shapes` prints those definitions. `rect` and `rounded` keep native outlines, because a corner `radius` is in pixels.
- `cylinder`, `hexagon`, `doc`, `cloud` and `person` have features measured in pixels, so stretching a box does not distort them: the cylinder's cap height is `min(20, h/4)`, the hexagon's side inset is `min(h/4, w/4)`, the doc's wave depth is `min(12, 0.15 × h)`, the cloud's bumps are `min(24, 0.3 × h, 0.175 × w)` high and repeat along the top and sides (a wider or taller cloud gets more of them), and the person's head is a circle of radius `r = min(18, 0.2 × h, 0.22 × w)` at the top, `max(3, r/4)` above a full-width body whose shoulders are rounded with radius `min(0.6 × body height, 1.6 × r)`. Their unit paths are therefore computed for each box size; `ref shapes` prints them for one example size. The person's head and body are drawn as separate subpaths; its outline is a third, undrawn subpath that joins them with a neck, so `top` is the top of the head.
- Shape names can't reuse a built-in shape name, and defining a name twice is a `duplicate-style` error.
- Checks use the outline, flattened into straight segments.

### SVG files as shapes
```
shape s3 from "icons/aws/s3.svg" [outline=bbox|hull] [text-area=below|inside]
```
- The SVG's drawing is scaled to the box.
- Anchors and checks use the bounding box (default) or the convex hull of the drawing.
- `text-area=below` puts the label under the icon, the usual style in architecture diagrams. The label sits in a band `padding` (`y`) under the box, as tall as the text and at least as wide as the box.
- Anchors, `through`, `near` and `overlap` use the shape's outline only, never the label under it. `out-of-canvas`, `spill`, `label-collision` and `low-contrast` check that label where it sits, against the canvas or whatever is drawn behind it.
- The SVG's drawing replaces the style's fill and stroke, so the shape adds no fill behind text for `low-contrast`.

### Images
```
box s3 "Uploads" at (600,300) size (120,60) icon="icons/aws/s3.svg" [icon-pos=left|top] [icon-size=N]
image logo "assets/hf.png" at (20,20) size (80,80) [fit=contain|cover|stretch]
```
- Formats: SVG, PNG, JPEG.
- `fit` defaults to `contain`. Images take `pin=`, can be anchored to like boxes, and paint with the boxes in file order.
- `icon-pos` defaults to `left` and `icon-size` to 24. A `left` icon sits `padding` (`x`) from the left edge, vertically centered; a `top` icon sits `padding` (`y`) from the top edge, horizontally centered. The text area shrinks by `icon-size + padding` on that side. On a group, the icon sits in the title band before the title, its size defaults to the title's line height (`1.25 × font-size`), and `icon-pos=top` is an `attr` error. `icon-pos` or `icon-size` without `icon=` is an `attr` error.
- An SVG icon that paints with `currentColor` (all bundled icons do) is drawn in the shape's `color`, the color of its label, so it stays visible on a dark fill. When neither the shape nor a style it names sets `color` (a group's built-in `boundary` color doesn't count), `currentColor` is `#334155`, the default stroke. An icon that sets `color` on its root element, or paints with explicit colors only, keeps its own colors. The color is resolved before the usvg re-serialization, so the embedded SVG has no `currentColor` left. Images and `shape ... from` SVGs don't follow a `color`: a bundled icon used as one is drawn in `#334155`, and a user SVG's `currentColor` is black.
- The checker treats an image as an opaque rectangle: lines are checked against it like a box, and it takes part in `overlap`, `spill` and `out-of-canvas`. It adds no fill behind text for `low-contrast`.
- `inspect` reports each image's natural size and aspect ratio.

### Imports
```
import "lib/aws.kash"          // brings in shape and style definitions
import "lib/aws.kash" as aws   // namespaced: shape=aws.s3 style=aws.service
```
- A `shape=` inside a library's `style` names shapes as the library writes them: a built-in, or a shape the library defines or imports, resolved under the library's namespace. In `lib/aws.kash` imported `as aws`, `style service { shape=s3 }` draws `aws.s3` wherever `style=aws.service` is used, and `shape=net.lb` in it names `aws.net.lb` for an `import "net.kash" as net` inside the library.
Library files may contain only `shape`, `style` and `import`; any other statement is a `parse` error naming it. An import cycle is an error.
- Imported names share one namespace with the main file's own: a local `style` or `shape` named like one a plain import (or two plain imports) already defined is a `duplicate-style` error, and the first definition is kept for an imported shape or a style two libraries define, while a local style replaces the imported one. Importing one library both plainly and `as aws` defines `s3` and `aws.s3`, which do not collide.
- Errors inside an imported file are reported at the path string of the `import` in the file being checked, and the message starts with ``in `lib/aws.kash` at 3:5: `` (the line and column inside the library). Nested imports chain the prefix, one hop per file, each at the path string of the next `import`. A chain of more than 4 hops shows the first two, ``… N more imports …`` and the last.
- `kashika check` accepts a library on its own: a file with no `diagram` that holds only `shape`, `style` and `import`. It resolves the definitions (imports, shape files, style keys) and reports their errors, with no lints. `render` on a library is a tool error. A file with no `diagram` and any other statement is a `diagram` error.

### Rules for files
- **Local files only.** No URLs.
- **Path resolution:** paths are relative to the importing file. A path that escapes the project root is an error unless `--allow-path` is given or the file is included with `--include`.
- **Project root:** the input file's directory; for stdin, the current working directory (`--stdin-name` does not change it). Directories on the search path (`icons=`, `KASHIKA_ICON_PATH`) are additional trusted roots, as is anything under `--allow-path`.
- **Included files:** `--include <file>` (repeatable) allows exactly that one file outside the project root, e.g. `kashika check diagrams/board.kash --include lib/pieces.kash` for a diagram that writes `import "../lib/pieces.kash"`. The path is relative to the current directory and is canonicalized, so the sandbox compares the real file; a symlink is included by its target. It grants nothing else: an included library's own relative references (`import`, `shape ... from`) still resolve from its own directory, as for any import, and each must be inside the project root, on the search path, or included too. A `--include` that does not name an existing file (a missing path or a directory) is a usage error (exit 3).
- **Search path:** besides the importing file's directory, Kashika searches `icons=` on the `diagram` line and `KASHIKA_ICON_PATH`, then falls back to the bundled icons for a bare name such as `icon="queue"`.
- **SVG sanitizing:** scripts, external references and `foreignObject` are stripped. An `<image>` survives only as a `data:` URI; an SVG that loses an `<image>` or a `foreignObject` this way is `svg-dropped`.
- **SVG text:** `<text>` in an SVG file is set in the bundled fonts and embedded as glyph outlines. Any family other than `monospace` (or JetBrains Mono by name), including one that is not bundled, uses Inter; `monospace` uses JetBrains Mono.
- **Image size limits:** a PNG or JPEG, attached or embedded in an SVG, is judged by its header before any pixel is decoded and is a `file` error beyond 8,192 px per side or 32,000,000 px in total. Raster output has its own, larger canvas cap (§8).
- **File size limits:** an image or SVG file (`image`, `icon=`, `shape ... from`) may be at most 16 MiB, and an imported library at most 1 MiB; a larger file is a `file` error, refused before it is read in full.
- **SVG nesting:** an SVG, or an SVG embedded in one as a `data:` image, whose elements nest more than 256 deep is a `file` error, refused before it is parsed.
- **Import fan-out:** each library file is read and parsed once, however often it is imported. Each (file, namespace) pair registers its definitions once; more than 1,000 such library instances in one diagram is a `file` error, reported once. A chain where every library imports the next under two namespaces doubles at each level and reaches the limit after about ten levels.
- **Self-contained output:** images are embedded in SVG output and rasterized into PNG output.
- **Missing files:** a missing file is an error, with did-you-mean suggestions drawn only from files the sandbox allows. A bare name with no close file that is a search keyword (as in `kashika icons --search`) names the bundled icon it finds, or else the built-in shape (`icon="database"`: `shape=cylinder`). Naming a directory is an error that says so.
- **Licensing:** no third-party icon sets are bundled. Kashika ships a small neutral set (`server`, `browser`, `queue`, `lock`, `cloud`, `person`). Users point the search path at icon sets they download themselves.

## 4. Points and anchors

```
point  := base offset*                     // offsets add up: db.center+(0,40)-(10,0)
base   := "(" coord "," coord ")"
        | ref
        | "(" point ")"                      // grouped: (db.center+(0,40)) is db.center+(0,40)
offset := ("+" | "-") "(" dx "," dy ")"
ref    := <id> "." <anchor> | <junction-id>
coord  := number                             // literal, in the current frame
        | ref ".x" | ref ".y"                // mixed: (db.left.x, lane1.y)
anchor := top | bottom | left | right | center
        | tl | tr | bl | br                   // aliases: top-left, top-right, bottom-left, bottom-right
        | (top|bottom|left|right) "@" t      // 0 <= t <= 1
```

- **Mixed points** take each axis from a different reference, so lanes can be expressed as relationships: `(app1.bottom.x, lane1.y)`. A `junction` without `dot` that no line ends at works as a named guide point.
- `@t` and offsets compose: `db.left@0.25+(0,4)`, `db.left@0.25-(0,4)`.
- **Parentheses** around a point group it and change nothing; they must be closed: `(db.center+(0,40))` and `(fan)`. Any point may be grouped, including a grouped one, and parentheses nest at most 16 deep: `((a.left.x, b.y)+(0,4))` is valid, and a 17th set is a `parse` error ("a point nests at most 16 sets of parentheses"). After `(`, a number starts a literal point; otherwise, a `,` directly inside the parentheses makes a literal or mixed point, and its absence a grouped point.
- **Offsets** may follow any point, including a literal, mixed or grouped one, and several may follow one another; they add up: `(10,20)+(5,0)` is `(15,20)`, `(a.left.x, b.y)+(0,26)`, `(db.center)+(0,40)` and `db.center+(1,2)+(3,4)`, which is `db.center+(4,6)`. An offset is a delta, so a literal point's offset is the same in every frame.
- **Groups have anchors** just like boxes, e.g. `k8s.top`.

- `top@t`/`bottom@t` run left→right; `left@t`/`right@t` run top→bottom. `top` is the same as `top@0.5`.
- Corner anchors are always bounding-box corners.
- On non-rectangular shapes, edge anchors are **axis-projected**:
  - `top@t`/`bottom@t` keep the bounding-box point's x and move vertically onto the outline.
  - `left@t`/`right@t` keep the y and move horizontally.

  A vertical line into `db.top` therefore stays vertical. Outlines are defined in `ref shapes`. For example, the top of a cylinder is the upper arc of its cap ellipse, whose height is `min(20, h/4)`.
- `+(dx,dy)` adds the offset and `-(dx,dy)` subtracts it, negating both components. Components are signed after either sign: `db.top+(-10,0)` and `db.top-(10,0)` are the same point, and `db.top-(-10,0)` is 10px to the right of `db.top`.
- `@t` errors are reported as `unknown-anchor`: a `t` outside 0..1 ("`t` must be between 0 and 1"), and `@t` on a corner or `center` ("`@t` only applies to `top`/`bottom`/`left`/`right`").

## 5. Line paths

A line is `start -> seg -> seg ...`. Every segment ends at a point, and that point becomes the "previous point" for the next segment.

**`arrow`** is a `line` statement whose `end` defaults to `arrow`: `arrow api_db api.right -> db.left` draws the same line as `line api_db api.right -> db.left end=arrow`. Everything else is the same: the grammar, the id rules, the attributes and every check. An explicit `end=` wins (`arrow a.right -> b.left end=open`, or `end=none` for no marker), and `end=arrow` on a `line` stays valid. It is a line everywhere downstream (the scene, `inspect` and the JSON call it one), `fmt` keeps the keyword as written, and `route --line` writes its suggestions with the keyword the line uses.

The optional id comes right after `line` (or `arrow`), with no colon: `line api_db api.right -> db.left`. The first token is the id exactly when it is an identifier followed by something that starts a point (an identifier or `(`); a point that starts with an identifier continues only with `.`, `+`, `-(`, `->` or the end of the statement, never with another identifier or `(`, so `line a -> b` is unnamed and `line l a -> b` is named `l`. These common mistakes are `parse` errors whose help line gives the rewrite:

- `line api_db: api.right -> db.left` (help: remove the `:`);
- `line api_db from api.right to db.left`, with or without the `:` (help: ``write `line api_db api.right -> db.left` ``);
- `curve left:30 db.top` (help: ``write `curve left 30 db.top` ``);
- `a.right --> b.left` and `a.right => b.left` (help: write `->`);
- `-> to b.left` (help: ``write `-> b.left` ``);
- `x b.left.x` or `y b.top.y` (help: ``write `x b.left` ``: `x` and `y` take a point).

Outside lines, `size 100x50`, `size 100 x 50` and `size 100,50` (help: ``write `size (100,50)` ``), a literal point without parentheses, `at 20,30` (help: `(20,30)`), and a position without `at` (`box a "A" (20,30)`) are `parse` errors with the rewrite in their help too.

| segment | path drawn |
|---|---|
| `<point>` | straight to the point |
| `h <dx>` / `v <dy>` | horizontal or vertical **move by** dx / dy. Negative values go left or up |
| `x <X>` / `y <Y>` | horizontal or vertical **move to** x=X / y=Y. X/Y is a number (in the current frame) or a point (its x or y is used), e.g. `y db.top` |
| `hv <point>` | horizontal, then vertical, to the point (one elbow) |
| `vh <point>` | vertical, then horizontal, to the point |
| `vhv <point>` / `vhv@t <point>` | vertical to `t` of the way to the point's y (default `0.5`, halfway), horizontal to its x, then vertical to the point (two elbows). `t` must be strictly between 0 and 1, else a `parse` error. When the point has the current x, it is one straight vertical run |
| `hvh <point>` / `hvh@t <point>` | the same with the axes swapped: horizontal to `t` of the way, vertical, horizontal |
| `curve left <N> <point>` / `curve right <N> <point>` | quadratic curve to the point that bulges N px at its midpoint, to the left or right **as seen on screen by someone walking along the line**. Example: a line traveling right with `left 30` bulges **up** |
| `curve via (c1) (c2) <point>` | cubic Bézier with explicit control points |

Line attributes:
- `start=` / `end=`: `arrow`, `open`, `diamond`, `dot`, `none`, one of the hollow (UML) ends or one of the crow's-foot (ER) ends below (both default to `none`). The stroke is trimmed so the marker's tip sits on the endpoint. The trim follows the end segment, whether straight or curved. When both markers sit on the same segment and their lengths add up to more than the segment, each end is trimmed in proportion to its marker's length.
  - Marker sizes below are for `stroke-width` up to the default 1.5. On a thicker line every marker dimension (lengths, widths, radii, offsets and the trim) is multiplied by `stroke-width / 1.5`, so a 4px line draws its ends 2.67 times as large.
  - `arrow` is a filled triangle 10px long and 8px wide; `open` is its two sides as an open chevron; `diamond` is a filled diamond 12px long and 8px wide; `dot` is a filled circle of radius 3px centered on the endpoint.
  - Hollow (UML) ends are unfilled outlines in the line's `stroke` and `stroke-width`, always solid: `triangle` (generalization) is a closed triangle 12px long and 12px wide, `hollow-diamond` (aggregation; `diamond` is composition) is the `diamond` outline, and `circle` is a circle of radius 4px that touches the endpoint. The stroke is trimmed to the marker's base (the triangle's base, the diamond's far corner, the circle's far side) and masked out inside it, so the canvas, group or box fill beneath shows through and nothing paints the canvas.
  - Crow's-foot ends sit on the line and never trim it: the stroke runs all the way to the endpoint, and the prongs and bars touch it. Each part is placed along the end segment and crosses it at a right angle there, so the ends follow a curve. Their strokes use the line's `stroke` and `stroke-width`, always solid.
  - `one`: exactly one; a 12px bar across the line, 8px from the endpoint.
  - `many`: many; a crow's foot of three prongs that meet the endpoint 12px apart, spread 6px to either side of the line, and converge 12px back along it.
  - `one-many`: one or more; the crow's foot plus a bar 16px from the endpoint.
  - `zero-one`: zero or one; the bar plus an open circle (radius 4px) beyond it, 12–20px from the endpoint.
  - `zero-many`: zero or more; the crow's foot plus the open circle beyond it, 16–24px from the endpoint.
  - The open circle is not filled: the line is masked out inside it (as under a label), so the canvas, group or box fill beneath shows through and nothing paints the canvas.
- `stroke-style=solid|dashed|dotted`, `stroke`, `stroke-width`, `style=`. On lines, a style applies only its `stroke*` and font keys.
- `radius=N` rounds every polyline corner. The radius is clamped to half the shorter adjacent segment.
- `label="..."` is the same as `label@0.5="..."`. `label@t="..."` is repeatable. `t` is the fraction of the final rendered path's **arc length**. Labels are horizontal, centered on the point, and cut their own line (stroke and markers) out of the label's rect, so whatever is beneath (the canvas, a group or box fill) shows between and around the words. Other lines are not cut; a line crossing another line's label is `label-collision`.
- `label-side=on|above|below|left|right` (default `on`) sets where every label of the line sits; `label@t` still picks the point. `on` is the placement above. `above`/`below` need the point on a horizontal segment: the label stays centered on the point horizontally and moves up or down so its near edge is `stroke-width / 2 + 4` from the line. `left`/`right` need a vertical segment and move the label sideways the same distance, centered on the point vertically. At a vertex either adjoining segment may supply the orientation. A label beside its line does not cut the line, and its own line counts against it in `label-collision` like any other. A side that does not match the segment under the point (`above` on a vertical segment, any side but `on` on a diagonal or curve) is an `attr` error at the attribute that names the label and the segment's orientation; that label falls back to `on`.
- `ortho`: asserts that every segment is horizontal or vertical. Any other segment is an error.
- `allow=<code>[:<other-id>],...` and optional `reason="..."` (see §7).

Unnamed lines are referred to as `line@L<n>`, where n is the source line of the statement. Naming lines is recommended.

## 6. Text

### Text in shapes: sublabel by default, block when needed

- **Default:** `box api "Payments API" sublabel="v2.4.1" ...`
  - The label is the name.
  - The sublabel is one muted line below it, at `0.8 × font-size` in the style's `muted` color.
- `text-align=left|center|right` (default center) and `text-valign=top|middle|bottom` (default middle) position the whole text stack within the shape's **text area**.
- **Inline markup** works in `label`, `sublabel`, all `text` and a line's `label=`: `**bold**` and `` `code` `` (monospace). Markup does not nest. The markers are `**` and `` ` ``; a single `*` is literal. Escape with `\*` and `` \` ``, which always produce a literal `*` or `` ` ``. An unclosed marker is a `parse` error. A tab in text is replaced by four spaces (no tab stops); other control characters have no glyph and are reported as `missing-glyph`. Unicode default-ignorable code points (the soft hyphen U+00AD, zero-width space U+200B, ZWNJ U+200C, ZWJ U+200D, word joiner U+2060, U+FEFF, variation selectors U+FE00–FE0F and U+E0100–E01EF, bidi and other invisible format controls) have zero width and draw nothing, and are never `missing-glyph`.
- **Text block**, for 3 or more lines, bullet lists, or mixed sizes and alignment:
  ```
  box api at (100,80) size (200,120) style=service text-align=left {
    text "**Payments API**"
    text "v2.4.1" font-size=11 muted
    text "• validates cards" font-size=12
    text "• retries charges" font-size=12
  }
  ```
  - Block items **without `at` stack** top to bottom, separated by `gap` (box attribute, default 2), and aligned by the box's `text-align`/`text-valign`. No coordinates are needed.
  - Items **with `at`** are absolute, in the **box-local frame** (origin is the box's top-left). They leave the stack. This form is advanced.
  - Block text items take no id. They take `font-size`, `color`, `text-align` and `muted`, like a `text` statement; `font-size` defaults to the box's, `color` to its style's (`muted` picks the muted color) and `text-align` to the box's `text-align`.
  - A block that `label`/`sublabel` would draw the same is the `block-reducible` warning (§7).
  - Using `label`/`sublabel` together with stacked block items is an error (`text-conflict`). It is allowed when every block item has its own `at`.
- `ref text` shows `sublabel` in its example. The block is documented under "when sublabel is not enough", and absolute `at` comes last.

### Fonts and metrics

- Kashika bundles Inter (Regular and Bold) and JetBrains Mono (for `` `code` ``), so measurements are identical on every machine.
- Defaults: `font-size=14`, line height `1.25 × font-size`, `padding=8`.
- A box's **text area** is the shape's own text area (the whole box for `rect` and `rounded`, otherwise as below) inset by `padding` on every side (`x` on the left and right, `y` on the top and bottom with `padding=(x,y)`):

  | shape | text area |
  |---|---|
  | diamond | the inscribed rectangle 56% wide and 44% tall |
  | ellipse | the inscribed rectangle, `1/√2` of each axis |
  | hexagon | horizontally inset by `min(h/4, w/4)` |
  | cylinder | below the top cap |
  | doc | above the wave |
  | cloud | inside the bumps: inset by the side bump depth on the left and right, the bump height on top and half of it at the bottom |
  | person | the body, inset by 0.3 × the shoulder radius on the top, left and right |
  | rounded | same as `rect` |

  Every text area lies inside the shape's outline, except that a cylinder's runs down to the bottom of the box. The fitted size (§3) is found numerically from the shape's text area, so it is the smallest whole-pixel size whose text area holds the text.
- The stacked text (or each absolute item) must fit the text area. `label-overflow` reports the required box size, which is the size a box without `size` gets (§3); for an absolute item it fires only when the item is larger than the text area. Placing an item outside the text area is the `text-outside-shape` warning (§7), not an error. For small text areas, such as a diamond's, it suggests a roomier shape.
- `kashika measure "text" [--size N] [--weight bold]` prints a label's exact width and height, with markup applied. `kashika ref metrics` lists average character widths at common sizes.

## 7. Checks

All checks run on the same resolved scene that the renderer draws. Tolerance is 0.5 px. Curves take part in every check; they are flattened to within 0.25px.

### Errors

These block output unless `--force` is given:

| code | trigger |
|---|---|
| `parse` | Includes expected tokens and a suggestion. Also: unknown string escapes, reserved ids (§2), groups nested more than 64 deep, points in more than 16 sets of parentheses, the old `line <id>:` and `curve left:<N>` forms, `from … to` lines (§5) and a `kashika=` statement (§8), each with the rewrite in its help |
| `unknown-id` / `unknown-anchor` | Includes did-you-mean. `unknown-anchor` also covers a bad `@t`: out of 0..1, or on a corner or `center`. A reference to an id whose statement failed to parse after its id is not reported: the `parse` error explains it |
| `duplicate-id` | Two statements use the same id. A statement that failed to parse does not count |
| `duplicate-style` | Two `style` statements, or two `shape` definitions, define the same name, or a `shape` reuses a built-in shape name. Style and shape names are separate from element ids. When the first definition was imported, the message names its library file and the main file's `import` that brought it in |
| `placement-cycle` | Relative placements form a cycle, or a chain of more than 256 of them. Anything inside a fitted group that references the group is a cycle (§3) |
| `label-overflow` | Reports the required size |
| `text-conflict` | `label`/`sublabel` combined with stacked (no `at`) block items (§6) |
| `not-ortho` | A segment is diagonal on a line marked `ortho` |
| `degenerate-curve` | A zero-length curve or a negative curve amount |
| `zero-length` | A line whose segments all have zero length |
| `attr` | Unknown, duplicate, mistyped (including an unknown color name) or out-of-range attribute (§3), a duplicate label position, a `label-side` that does not match the segment under a label (§5), a style `shape` that reaches a group, or `size` in `style boundary` (§3) |
| `unknown-style` | `style=` names no built-in or user style |
| `unknown-shape` | `shape=`, on a box or in a `style`, names no built-in, defined or imported shape. Includes did-you-mean, where a built-in shape's search keyword (any case) is a match (`database` suggests `cylinder`, `decision` suggests `diamond`), help on using a bundled icon's name (`icon="server"`, or `shape server from "server"`), and the list of built-in shapes |
| `size` | A non-positive size, or one larger than 1,000,000 (including a style's `size=`); a box with no `size` and nothing to fit, or whose text sits below it; a fitted group with no children and no label (§3) |
| `range` | A resolved coordinate or curve amount more than 1,000,000 from the origin, or not a finite number (§1) |
| `diagram` | Missing, misplaced or repeated `diagram` statement. A `diagram` statement that failed to parse is not reported missing |
| `version` | `kashika=` is not a quoted semver string, or names a newer version or a different major version than the tool |
| `file` | A referenced file (`import`, `shape ... from`, `image`, `icon=`, `icons=`) is missing, is a directory, escapes the project root without `--allow-path` or `--include`, is a remote URL, can't be read or decoded, has the wrong type, is over the file size limits, is an image over the size limits or an SVG nested too deep (§3a); an import cycle; imports nested more than 32 deep; more than 1,000 library instances. Errors inside an imported file are reported at the `import` with an ``in `file` at L:C:`` prefix |

### Warnings

| code | trigger |
|---|---|
| `cross` | Two lines intersect, including tangency, at a point that is not a shared endpoint or junction |
| `shared-segment` | Two lines overlap collinearly for more than 1px. The help gives the nearest free lane, and when the two lines start or end at the same point, says to spread them with `side@t` or fan out from a `junction` |
| `through` | A line enters a box's or image's interior. For a line's own start and end boxes, only the run from an endpoint strictly inside the box (such as `center`) out to its outline is exempt. A line that reaches an endpoint on the outline from inside its own box (it turns inside the box and arrives at the edge anchor backwards, or leaves into the box) is reported as running through the inside of its own endpoint box, with help on approaching from outside; leaving and re-entering is reported like any other box. Touching or running along the outline is not entering it. Groups and `layer=under`/`over` shapes (§3) are never obstacles |
| `near` | A line runs within `clearance` of a box edge it doesn't attach to, or within `clearance` of a parallel line for more than 20px. **This includes a line running along its own box's edge**: when its first or last segment runs along a side of the box it starts or ends on (`a.right -> hv b.left`), the message names that side and the help names the elbow that leaves or enters it square (`vh` into `left`/`right`, `hv` into `top`/`bottom`), with no free intervals; and a line running parallel to a group's border within `clearance` of it, on the border included (crossing the border is fine) |
| `overlap` | Two shapes have intersecting interiors. A group whose rectangle contains the other shape is not an overlap, and a shape nested inside a group in the source is left to `spill`. A box containing another shape is an overlap. `layer=under`/`over` shapes never overlap anything |
| `spill` | A child extends outside its group or into the group's title band. Touching the edge is fine |
| `diagonal-tail` | A line's final segment is diagonal when it has at least one other straight segment and all of those are horizontal or vertical. A final segment that is `near-axis` is reported only as `near-axis` |
| `near-axis` | On a line without `ortho`, a straight segment whose smaller extent (horizontal or vertical) is more than 0.5px but at most `tan 5° ×` its larger extent, so it is within 5° of an axis without being on it and renders as a visible slant (`users.right -> cdn.left` with the anchors 5px apart in y). Curves are exempt. The help gives the fix: for the first or last segment, the anchor on the attached box that levels it (`cdn.left@0.42`, or `cdn.left+(0,-5)` when no two-decimal `t` lands within 0.25px), and always the offset to remove, or `hv`/`vh` for a deliberate elbow |
| `jog` | A straight horizontal or vertical segment that is not the first or last, shorter than `clearance` (and longer than 0.5px), joins two parallel runs going the same way, so the line shows a small Z or S step (`svc.bottom -> v 20 -> hv rds.top@0.35` with the two x 4.5px apart). Consecutive pieces on one axis going the same way count as one run, and corners are taken before `radius=` rounds them. A step between runs going opposite ways (a U-turn) is not a jog, and curves and diagonals break the runs. Applies to `ortho` lines too. The help gives the fix: when the run before the step starts the line, or the run after it ends the line, at a box, the anchor that lines the runs up (`rds.top@0.42`, or an offset as for `near-axis`), and always the coordinate to move either run to |
| `endpoint-off-edge` | A literal endpoint is within 8px of a shape outline but not on it. A start point is literal unless it is written `id.anchor[+offset]` or a junction (a mixed point is literal). An end is literal when the last segment is `h`, `v`, a numeric `x`/`y`, or goes to a literal point; `x`/`y` with a point target is not |
| `port-crowding` | Two endpoints on the same shape are within 8px of each other, or closer than their two end markers' half-widths add up to (crow's-foot ends and `triangle` reach 6px to each side of the line, arrowheads and diamonds 4px, `circle` 4px, dots 3px, all scaled with a thicker stroke as in §5), so two crow's feet need 12px. This includes identical endpoints where both lines draw a marker (stacked arrowheads). Junctions are exempt |
| `group-transit` | A line passes through a group's interior while neither of its endpoints is inside that group. A line also connects into the group when a junction lies on it (an endpoint or a point along it) and another line with an endpoint at that junction has an endpoint inside the group: a trunk that taps a child through a junction does not transit the child's group |
| `low-contrast` | The contrast ratio between text color and its background is below 3:1, or between an icon drawn in `currentColor` (§3a) and the fill under its center. Icons with their own colors are not judged. The background is the `background=` color with every group and box fill under the text painted on top in paint order; a transparent canvas counts as white. Texts and line labels use the fills under the center of their box (not the `at` anchor of a left- or right-aligned text); line labels hide only their own line |
| `block-reducible` | A text block holds only 1–2 stacked items, in one alignment, that a `label` would draw the same: the first in the box's font size and color, the second either in that style too (a second label line, `\n`) or in the `sublabel` style (`0.8 × font-size`, `muted`, not bold). The help line shows the `label`/`sublabel` rewrite |
| `text-outside-shape` | An absolute block item lies outside the shape's text area (§6), or outside the band under a `text-area=below` shape. The help line gives the text area in the box-local frame |
| `image-distort` | `fit=stretch` changes an image's aspect ratio by more than 5% |
| `svg-dropped` | An SVG file (`image`, `icon=` or `shape ... from`) lost elements when it was embedded (§3a): `<image>`s that are not `data:` URIs and `foreignObject`s. Reported once per file, at the first shape that uses it. The facts give the file and the count per element |
| `missing-glyph` | A text, label, sublabel, block item or line label contains a character none of the bundled fonts has a glyph for (§6), other than a default-ignorable one, which draws as nothing. It is drawn as the font's empty-box `.notdef` glyph, with that glyph's width. The facts list the code points |
| `label-collision` | A label or text box intersects a box, another label or text, a group title, a group's border (it straddles the group's rectangle: partly inside, partly outside), or a line other than its own (its own line too, for a label beside it with `label-side`). A line label also must not cover its own line's markers (arrowheads, diamonds, dots, and crow's-foot ends from the endpoint to their farthest bar, prong or circle); sitting on its own stroke is fine. A line that crosses the box of a group's title text is also reported, naming the line and the group, with the title's extent and the bottom of its title band in the help, and the group `text-align` that moves the title to the side away from where the line enters it. A line label's help lists its clear `label@t` positions; when there are none and its straight segment is shorter than the label's extent along it plus its longest end marker at each end plus `clearance`, the help gives the label's size, the segment's length and that run, and offers `label-side` when no box or image is in the way and a side fits the segment. A free `text` whose point lies in a box gets help moving it into the box's text block, at box-local coordinates |
| `out-of-canvas` | An element extends beyond the diagram size; children of a group that itself extends beyond it are not reported again. On a fitted (`auto`) axis only the top or left edge can be crossed (§3). Past the top or left edge the help gives the move right or down (and that a fitted canvas grows only right and down); past a fixed right or bottom edge it says to move the element inside or enlarge `size` |
| `off-grid` | An authored literal is not a multiple of `grid` |

### Suppression

- `allow=cross` on a line silences all crossings for that line.
- `allow=cross:other_line` silences only crossings with that one line. This is preferred, so a later edit doesn't introduce hidden crossings.
- `allow=` and `reason=` work the same on `box` and `group`, e.g. `box badge ... allow=overlap:card`. A diagnostic is suppressed when any element it names allows its code (and, for `code:id`, also names that id). `allow=` on `diagram` silences a code everywhere.
- `reason="..."` is optional and kept in the JSON output. A suppressed finding carries the reason of the first line, then shape, in file order, among those it names that allow its code and give a reason. `diagram` takes no `reason=`, so a finding only the diagram silences has none.
- **Suppressed findings are counted in every summary**, and `--show-suppressed` lists them.

### What a clean check does not prove

A clean check does not prove that arrows point the right way or that the right boxes are connected. It only covers the geometry listed above. `ref` states this explicitly.

## 8. CLI

```
kashika render  <file|-> -o <out.svg|out.png|out.jpg|out.jpeg|out.webp> [--force] [--debug]
kashika check   <file|->
kashika inspect <file|-> [ids...] [--ports] [--corridors <x1,y1,x2,y2>]
kashika fmt     <file|-> [--check] [--stdin-name <name>]  // `-` formats stdin to stdout
kashika measure "<text>"... [--size N] [--weight bold]
kashika route   <file|-> <from> <to> [corner|arc|bezier] [--line <id>]
kashika icons   [dir...] [--search <term>]     // list built-in shapes, bundled icons and importable files: name, path, natural size, library; --search matches names and built-in keywords ("user" finds person)
kashika ref     [topic] [--json]          // no topic: the core guide; topics: rules syntax anchors paths sizing text shapes styles recipes examples lints metrics cli limits, or a diagnostic code
kashika --version
```

Common flags: `--format text|json`, `--fail-on error|warn|never` (default `warn`), `--show-suppressed`, `--focus <id,...>` (only warnings involving these ids), `--max N` (default 50), `--stdin-name <name>`, `--summary` (print per-code counts; JSON always has `by_code`), `--allow-path` and `--include <file>` (§3a). `inspect` takes only `--format`, `--stdin-name`, `--allow-path`, `--include`, ids, `--ports` and `--corridors`; `--corridors` without ids prints only the corridors. `route` takes `--allow-path` and `--include` too. `fmt` reads no other files, so it takes neither; of the common flags it takes only `--stdin-name`.

- `--focus` filters warnings only; errors are always reported. An unknown id is a usage error.
- Suppressed findings are excluded from `warn` and from the exit code, and counted in `suppressed`. `crossings` in the metrics counts every crossing, suppressed or not, and is computed before `--focus` filtering.
- `check` on a library file (§3a) reports only the errors in its definitions and prints no metrics; `render` on one is a usage error (exit 3).
- `inspect` never fails on errors in the file; it exits 0 except on usage or I/O errors.
- `format` is an alias of `fmt`, and `references` and `docs` are aliases of `ref`.
- `fmt` exits 2 on a parse error. `icons --search` with no match exits 1 and a directory argument that isn't one exits 3. `measure` takes several texts; one gives a single result, several give one each. Numeric options (`--radius`, `route --max`, `--corridors`) reject non-finite or out-of-range values with exit 3.
- A JSON tool error (exit 3) carries the schema of the command that failed.

**Versioning.** The optional `kashika="MAJOR.MINOR.PATCH"` on the `diagram` line is the minimum tool version that can process the file, always written with all three parts. A tool accepts a file when the file's version is at most the tool's version and the major versions are equal; anything else, including a bare number or a malformed string, is a `version` error. Releases that add language features bump the tool's minor version, so authors set `kashika=` to the release that introduced the newest feature they use. `kashika="…"` written as a statement of its own before `diagram` is a `parse` error whose help moves it onto the `diagram` line; that line doesn't also make `diagram` misplaced.

### render

- Runs every check.
- Writes output when there are no errors. With `--force` it writes even with errors, drawing red markers at each error.
- Writes are **atomic**; a symlinked output is written through and an existing file keeps its permissions. The result always states whether output was written for this run.
- `--debug` overlays rulers, element ids, anchor dots and diagnostic markers.
- The format follows the extension: `.svg`, `.png`, `.jpg`/`.jpeg` (quality 90, composited onto white, or onto the `background=` color when set) or `.webp` (lossless, keeps alpha). SVG, PNG and WebP keep the canvas transparent unless `background=` sets a color. Any other extension exits 3. Raster output (PNG, JPEG, WebP) is 1 px per diagram unit and is capped at 16,384 px per side and 100,000,000 px in total; a larger canvas exits 3 with a message naming the limits (SVG has no such cap).

### Exit codes

`0` clean, `1` warnings (when `--fail-on warn`), `2` errors, `3` usage or I/O failure.

### Text output

The text output is compiler-style, deterministic and has no color unless on a TTY. Each diagnostic shows its source line, the source line of every other element it names, and its `= help:` lines; a suppressed finding (with `--show-suppressed`) is marked `(suppressed)` and adds `= reason:`. It ends with a summary that includes metrics:

```
arch.kash:11:1: warn[cross]: l_d2 crosses l_lb3 at (690,95)
   11 | line l_d2 app2.top@0.8 -> v -85 -> hv pg.top end=arrow
   10 | line l_lb3 lb.right -> hv app3.top end=arrow
      = help: segment 2 runs at y=95; free y-intervals across x 504..820: 0..14, 316..400
      = help: if intentional: allow=cross:l_lb3 on l_d2
0 errors, 1 warning · crossings=1 wire=1346px
```

Errors are listed before warnings, each in source order, and every diagnostic is printed in full. `--max` never hides an error: every error is reported, and warnings fill what is left of `--max` in source order. When warnings are cut, the output ends with `... and N more`. The summary counts every diagnostic. To keep memory bounded, `check` and `render` keep at most 1,000 warnings (or `--max`, if larger) per code and suppression state, the first in source order, and only count the rest: they count toward the summary, `counts`, `by_code`, `crossings` and `... and N more` (and set `truncated`). In JSON, `diagnostics` lists every error and, up to `--max` entries in all, the first warnings, and `truncated` says whether `--max` cut any warning.

Control characters (C0, DEL and C1) and bidirectional overrides that come from the input, in source lines, messages, help and file names, are printed as `\u{1b}`-style escapes, so a file can't send escape sequences to the terminal. Tabs are kept. JSON output carries the original text.

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
- `free` lists intervals that are clear of **every** box, image, line, junction, text, line label and label under a `text-area=below` shape, and of every group border, with clearance applied, across the offending segment's span. The offending line and its labels are ignored. It is not just the extent of the obstacle.
- `cross` and `shared-segment` give each line's segment index and the overlapping span. `shared-segment` names the nearest free parallel lane. `label-collision` suggests `t` values that are clear.
- `suggestions` contain `{span, replacement, ripple: [ids whose geometry would change]}`. They are never applied automatically.
- Spans are 1-based, columns count Unicode scalar values, and positions refer to the input as given.

### inspect

Prints the resolved geometry: every shape's bounding box and anchors in both canvas and local frames, every line's vertices, segment kinds and arc length, every label's bounding box, and the placement dependency graph.

- `--ports` lists, for each shape side, which anchors are already used and by which lines.
- `--corridors` lists the free horizontal and vertical intervals within a rectangle, clear of the same things as the `free` fact.
- The text output starts with the canvas size (`canvas 640x320`) when it lists everything.
- `--format json` writes one object with schema `kashika.inspect/1`: `canvas` (`[w, h]`, after any `auto` axis is fitted), `shapes` (with `bbox`, `local_bbox`, `anchors`, `local_anchors`, and `image` with `path`, `natural`, `aspect` and `fit` for images and SVG-file shapes), `junctions`, `texts`, `lines` (with `vertices`, `segments`, `length`, `labels`), `placement`, and `ports`/`corridors` when requested. Positional ids limit the output to those elements. An unknown id is a usage error.
- Errors don't stop it, so a file with errors can still be inspected.

### route

```
kashika route <file|-> <from> <to> [corner|arc|bezier] [--line <id>] [--radius N] [--max N]
```

Suggests up to `--max` (default 3) candidate paths from `<from>` to `<to>`. Both are points, e.g. `app1.bottom` or `pg.left@0.25`, parsed and resolved exactly as in a `line` statement (anchors, `@t`, offsets, junctions and mixed points); each must be a single point, or it is a usage error. References and mixed points are written into the suggestions as given. It **never edits the file**; the author chooses which suggestion, if any, to paste.

- **The third parameter sets the style** of the suggestions (default `corner`):

  | style | output |
  |---|---|
  | `corner` | orthogonal paths with hard corners, written with `h`/`v`/`x`/`y`/`hv`/`vh`/`vhv`/`hvh` |
  | `arc` | the same orthogonal paths plus `radius=N` for rounded corners (default 8, set with `--radius`) |
  | `bezier` | smooth paths written with `curve left N` / `right N` where possible, otherwise `curve via` |
- `--line <id>` reroutes an existing line. The line is ignored as an obstacle, its endpoints are used when `<from>`/`<to>` are omitted, and its attributes are kept in the output.
- Candidates must not cross into any box (`through`), and paths must keep `clearance`. They are ranked by:
  1. crossings plus group transits: a transit (a `group-transit` finding, one per group the path passes through) weighs the same as a crossing
  2. `near` findings
  3. number of segments
  4. length
  5. on a tie, lanes that follow the endpoints' own boxes: fewer lanes on other boxes, then fewer literal lanes

Each candidate lists these numbers, which line it crosses and where, which group it transits and where it enters it, and the corridor it uses. `crossings` and `transits` are listed separately, and every count is printed even when it is 0. It is printed as a ready-to-paste `line` statement:

```
#1  crossings=0 transits=0 near=0 segments=3 length=412px  via corridor y 200..220
    line app1_db app1.top -> y 210 -> hv pg.left@0.25 end=arrow
#2  crossings=1 (l_lb3 at (640,210)) transits=0 near=0 segments=2 length=388px
    line app1_db app1.right -> hv pg.left@0.25 end=arrow
#3  crossings=0 transits=1 (vpc at (520,180)) near=0 segments=2 length=396px
    line app1_db app1.bottom -> vh pg.left@0.25 end=arrow
```

Suggestions use anchors and axis targets rather than literals wherever possible, so they survive later moves. `--format json` returns the same data structured: each candidate has `crossings` and `crossed` (`line`, `at`), and `transits` and `transited` (`group`, `at`, the entry point). It is a suggestion tool, not a layout engine; the search is bounded:

- `bezier` tries single curves only. `curve left N` / `right N` is tried only when it leaves and enters side anchors square; `curve via` places its control points out along the side normals, written as offsets from the endpoints' references.
- `corner` and `arc` try the straight path, `hv`, `vh`, and one lane (`x <lane> -> vh`, `y <lane> -> hv`) at the midpoint, `2 × clearance` outside any box or group edge, or the middle of a free interval between the endpoints (as `inspect --corridors` reports it) that holds no lane other than a group's, at least `clearance` inside the canvas, written as a literal. Of lanes within 0.5px of each other only one is kept, preferring a lane written relative to a box the path starts or ends at, then the box or group nearest the endpoints, then the literal midpoint. A literal lane exactly halfway between the point before it and the end, followed by the elbow to the end, is written as one `hvh <to>` (or `vhv <to>`), which draws the same path. Of the one-lane paths that enter no box and leave and enter side anchors square, only the 32 best by estimated crossings and group transits, near-misses, segments and length are scored.
- They also try detours with 3 or 4 elbows through two or three lanes: `x -> y -> hv` and `y -> x -> vh`, which end on the other axis than they start, and `x -> y -> x -> vh` and `y -> x -> y -> hv`, which end on the same one. Detour lanes are the same lanes, at least `clearance` inside the canvas. Only the 16 lanes per axis nearest either endpoint are used, and of the detours that enter no box and leave and enter side anchors square, only the 32 best by the same estimate are scored.
- Paths that double back (a zero-length segment, or one that reverses the segment before it) are dropped, and of paths that draw the same geometry only the first is kept.

A detour lists its second lane as its corridor. When no candidate is clear of every box, `route` prints `no candidate path avoids every box with at most 4 elbows (or 1 curve)` and exits 1.

### ref

`ref` is embedded in the binary and matches its version. With no topic it prints the core guide, at most about 2,500 words, which an agent reads first:
- the loop: write, `check`, fix errors then warnings from their help, `inspect`/`route` for coordinates, `render` and look
- the rules people get wrong (y-down; the frame rule, including the literal axis of a mixed point; ids are global; prefer relationships; the last segment sets the arrow direction; spacing labeled lines; text inside shapes; what `allow` hides; what a clean check doesn't prove)
- the grammar, and the anchor, segment and line attribute tables, with defaults
- the fixes for the diagnostics met most
- the list of topics, each with one line on what it covers

`ref <topic>` prints one topic: `rules`, `syntax`, `anchors` and `paths` (also on the default page), `sizing`, `text`, `shapes`, `styles`, `recipes` (the routing recipes: staircase, wrap over the top, gap corridor, outside margin, the comb, unavoidable crossings; and diagram recipes: fan-out via a junction, two lines on one side, self-loops, sequence diagrams, ER table cards, guides, graphs and timelines), `examples` (a complete worked example), `lints` (the diagnostics table, with a one-line fix for each code, and suppression), `metrics`, `cli` and `limits` (raster and file limits, and an explicit "not supported" list). `ref <code>` prints one diagnostic's row. An unknown topic is a usage error (exit 3) with a did-you-mean and the list of topics.

All examples in `ref` are tested fixtures.

### Deferred (not in v1)

- `fix` (auto-applying suggestions)
- `diff --geometry`
- arithmetic expressions in literals
- named guides (use junctions without `dot`)
- remote URLs for images and imports
- recoloring icons beyond `currentColor` (§3a)
- font files supplied by the user
- automatic text wrapping (use `\n` or a text block)
- themes
- WASM
