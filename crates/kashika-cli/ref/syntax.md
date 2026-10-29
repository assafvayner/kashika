## Syntax

```text
diagram "<title>" [size (w,h)] [margin=N] [kashika="0.1.0"] [grid=N] [clearance=N] [background=C] [icons="<dir>"] [allow=<code>,...]
import "<file.kash>" [as <ns>]
style <name> { key=value ... }
shape <name> { path "..." } | shape <name> from "<file.svg>"
box   <id> ["<label>"] at <point> [size (w,h)] [pin=<anchor>] [shape=<shape>] [style=<name>] [layer=under|over] [sublabel="..."] [icon="<file>"] [style keys] [{ text block }]
group <id> ["<label>"] at <point> [size (w|auto, h|auto)] [pin=<anchor>] [style=<name>] [sublabel="..."] { children }
junction <id> at <point> [dot]
text  [<id>] "<content>" at <point> [font-size=N] [color=C] [text-align=left|center|right] [muted]
image <id> "<file>" at <point> size (w,h) [pin=<anchor>] [fit=contain|cover|stretch]
line  [<id>] <point> -> <segment> [-> <segment> ...] [line attributes] [ortho]
arrow [<id>] <point> -> <segment> [-> <segment> ...] [line attributes] [ortho]   // a line with end=arrow
```

- `diagram` comes first, then any `import`s. Units are pixels. Without `size` the canvas fits everything drawn, plus `margin=N` (default 20) on the right and bottom; `size (w, auto)` or `size (auto, h)` fits one axis. A fitted canvas never grows left or up, so keep content at x, y ≥ 0. `margin=` with a fixed `size` is an error. The canvas is transparent unless `background=C`.
- One statement per line; a line that starts with `->`, `key=` or `ortho` continues the statement above. The label is the quoted string after the id (there is no `label=` on a box). After the id and label, `at`, `size` and attributes may come in any order.
- Comments are `//` and `/* */`. `#` starts a color (`#rgb`, `#rrggbb`, `#rrggbbaa`, or a CSS name like `steelblue`), never a comment. Strings use `\"`, `\\` and `\n`.
- Style keys: `fill`, `stroke`, `stroke-width`, `stroke-style=solid|dashed|dotted`, `font-size`, `font-weight=normal|bold`, `color`, `radius`, `padding`, `muted`. Inline keys override `style=`, which overrides the built-in (`kashika ref styles`).
- A box without `size` fits its text (`kashika ref sizing`); a box with no text needs a `size`, and so does an image. A group without `size` (or with an `auto` axis) fits its children plus `padding`, and at least its title. Nothing inside a fitted group may reference the group itself (`placement-cycle`).
- `pin=<anchor>` puts that anchor of the new shape on `at` (default `tl`): `at api.right+(80,0) pin=left`. `side@t` works too: `pin=left@0.25`.
- Groups hold `box`, `group`, `junction`, `text` and `line`, use the `boundary` style, and reserve a title band at the top. A junction is a named point lines can start or end at; it draws nothing without `dot`, so one without `dot` also works as a guide that other shapes are placed against (`kashika ref recipes`).
- `layer=under|over` on a box or image (not a group) paints it first or last. Layered shapes are backdrops and highlights, never obstacles: `overlap`, `through`, `near`, `label-collision`, `endpoint-off-edge`, `route` and `--corridors` ignore them; `spill`, `out-of-canvas` and their own text checks still apply. A `layer=under` shape inside a group paints right after that group's fill. Otherwise shapes paint in file order, then lines, then labels and texts.

```kash
diagram "Board" size (200,120)
box a1 at (20,20) size (80,80) fill=#b58863 stroke=none layer=under
box b1 at (100,20) size (80,80) fill=#f0d9b5 stroke=none layer=under
box k "K" at (40,40) size (40,40)
box ring at (36,36) size (48,48) fill=none stroke=#dd3333 radius=24 layer=over
```
