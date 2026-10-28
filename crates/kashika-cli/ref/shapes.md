# Shapes

This build has two shapes:

| shape | outline | edge anchors |
|---|---|---|
| `rect` (default) | the box | on the box edges |
| `rounded` | the box with `radius` corners (default 8) | moved straight onto the rounded outline |

Corner anchors (`tl`, `tr`, `bl`, `br`) are always bounding-box corners. Both shapes have the same text area: the box inset by `padding`. Other built-in shapes, custom `shape` definitions and `shape from` SVG files aren't supported yet.
