## Syntax

```text
diagram "<title>" size (w,h) [kashika="0.1.0"] [grid=N] [clearance=N] [background=C] [allow=<code>,...]
style <name> { key=value ... }
box   <id> ["<label>"] at <point> size (w,h) [pin=<anchor>] [shape=<shape>] [style=<name>] [sublabel="..."] [text-align=left|center|right] [text-valign=top|middle|bottom] [gap=N] [style keys] [{ text block }]
group <id> ["<label>"] at <point> size (w,h) [pin=<anchor>] [style=<name>] [sublabel="..."] [text-align=...] [gap=N] { children }
junction <id> at <point> [dot]
text  [<id>] "<content>" at <point> [size=N] [color=C] [align=left|center|right] [muted]
line  [<id>:] <point> -> <segment> [-> <segment> ...] [line attributes] [ortho]
```

- `diagram` comes first. Units are pixels. The canvas is transparent unless `background=C` (`kashika ref styles`). One statement per line; a line that starts with `->`, `key=` or `ortho` continues the statement above. After the id and label, `at`, `size` and attributes may come in any order.
- Comments are `//` and `/* */`. `#` starts a color (`#rgb`, `#rrggbb`, `#rrggbbaa`), never a comment. Strings use `\"`, `\\` and `\n`.
- Style keys: `fill`, `stroke`, `stroke-width`, `stroke-style=solid|dashed|dotted`, `font-size`, `font-weight=normal|bold`, `color`, `radius`, `padding`, `muted`. Precedence: inline > `style=` > built-in. Built-in styles: `service`, `datastore`, `external`, `queue`, `boundary`, `note`.
- `pin=<anchor>` puts that anchor of the new shape on `at` (default `tl`): `at api.right+(80,0) pin=left`. `side@t` works too: `pin=left@0.25`.
- Text in boxes: `sublabel` is one muted line under the label. Markup works in labels and texts: `**bold**` and `` `code` ``; escape with `\*` and `` \` ``. For three or more lines, use a block, `{ text "..." [size=N] [muted] ... }`. Items without `at` stack `gap` apart. Don't combine `label` with stacked items (`text-conflict`). An item with `at (x,y)` is absolute, in box-local coordinates.
- Groups hold `box`, `group`, `junction`, `text` and `line`, use the `boundary` style, and reserve a title band at the top. A group's `text-valign` is an error.
- More shapes, `import`, `image` and `icon=`: `kashika ref shapes`.
- `text` is centered vertically on `at`. Paint order: groups (file order), boxes, lines, then labels and texts in file order.

```kash
diagram "Text" size (360,200)
box pay "Payments API" at (20,20) size (160,60) sublabel="v2.4.1" style=service
box api at (20,100) size (200,90) style=service text-align=left {
  text "**Payments API**"
  text "`POST /charge`" size=12
  text "• validates cards" size=12
}
```
