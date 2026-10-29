# Shapes

`shape=` on a `box` takes a built-in shape or one you define or import. Corner anchors (`tl`, `tr`, `bl`, `br`) and `center` are always on the bounding box. Edge anchors (`top`, `left@0.3`, ...) keep the bounding-box point's x (top/bottom) or y (left/right) and slide along that axis onto the outline, so a vertical line into `db.top` stays vertical. Checks use the same outline.

| shape | outline | text area |
|---|---|---|
| `rect` (default) | the box | the box inset by `padding` |
| `rounded` | the box with `radius` corners (default 8) | the box inset by `padding` |

The other built-ins are unit paths (0..1, stretched onto the box). The text area is the unit rect mapped onto the box, then inset by `padding`. A `cylinder` cap is `min(20, h/4)` px tall and a `hexagon` side inset is `min(h/4, w/2)` px, so those two depend on the box size; the table shows them for a 120x80 box.

{builtins}

## Custom shapes

```kash
diagram "Custom" size (400,140)
shape tri {
  path "M 0.5 0 L 1 1 L 0 1 Z"
  text-area (0.25,0.5,0.5,0.5)
}
box a "A" at (20,30) size (120,80) shape=cylinder style=datastore
box t "Go" at (240,20) size (120,100) shape=tri
line l a.right -> t.left end=arrow
```

- `path` takes absolute `M L H V C Q A Z` with coordinates in 0..1. Several subpaths are drawn; `outline N` (default 1) picks the one used for anchors and checks.
- `text-area (x,y,w,h)` is a unit rect, inset by `padding`. Default: the whole box.
- Names are unique among shapes, and can't be a built-in name (`duplicate-style`).

## SVG files, images and icons

```kash
diagram "Assets" size (400,200)
shape srv from "server" text-area=below
box s "API" at (20,20) size (60,60) shape=srv
box q "Jobs" at (140,20) size (140,60) icon="queue" style=queue
image logo "browser" at (20,120) size (60,60)
```

- `shape <name> from "<file.svg>" [outline=bbox|hull] [text-area=below|inside]` draws the SVG stretched onto the box instead of a fill and stroke. Anchors and checks use the box, or the drawing's convex hull with `outline=hull`. `text-area=below` puts the label `padding` under the box; checks look for it there.
- `image <id> "<file>" at <point> size (w,h) [pin=<anchor>] [fit=contain|cover|stretch] [layer=under|over]` places an SVG, PNG or JPEG. It is top-level only, defaults to `fit=contain`, and checks treat it as an opaque box. `fit=stretch` that distorts the aspect ratio by over 5% is `image-distort`. `kashika inspect` shows the natural size.
- `icon="<file>"` on a box or group, with `icon-pos=left|top` (default `left`) and `icon-size=N` (default 24; on a group, the title's line height). The text area shrinks by `icon-size + padding` on the icon's side. Groups only take `left`.
- Bundled icons, by bare name: `server`, `browser`, `queue`, `lock`, `cloud`, `person`.
- An icon that paints with `currentColor` (every bundled icon) is drawn in the shape's `color`, so `fill=#1e293b color=white icon="lock"` shows a white lock. Without a `color` on the shape or its style it is `#334155`. Icons with explicit colors keep them.

## Imports and files

- `import "<file.kash>" [as <ns>]` comes right after `diagram`. A library holds only `shape`, `style` and `import`. With `as aws`, use `shape=aws.s3` and `style=aws.service`.
- Errors inside a library are reported at the `import` path, as ``in `lib/aws.kash` at 3:5: ...``. `kashika check lib/aws.kash` checks a library on its own.
- Files are local. They resolve from the `.kash` file's directory, then `icons="<dir>"` on the `diagram` line, then `KASHIKA_ICON_PATH`, then the bundled icons. Paths outside the file's directory need `--include <file>` for each file, or `--allow-path`. Problems are `file` errors, with did-you-mean.
- `kashika icons [dir...] [--search <term>]` lists what's available, with natural sizes.
