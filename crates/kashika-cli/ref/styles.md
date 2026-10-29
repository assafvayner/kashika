# Styles

```text
style <name> { key=value ... }
box api "API" at (40,40) size (140,60) style=service fill=#ffffff
```

| key | default | values |
|---|---|---|
| `fill` | `#ffffff` | color: hex or a CSS name like `steelblue` |
| `stroke` | `#334155` | color |
| `stroke-width` | `1.5` | number |
| `stroke-style` | `solid` | `solid`, `dashed`, `dotted` |
| `font-size` | `14` | number |
| `font-weight` | `normal` | `normal`, `bold` |
| `color` | `#0f172a` | text color |
| `radius` | `0` | corner radius; `shape=rounded` defaults to 8 |
| `padding` | `8` | inset of the text area; `(x,y)` sets left/right and top/bottom |
| `muted` | `#64748b` | color of sublabels and muted text |
| `text-align` | `center` (groups `left`) | `left`, `center`, `right`; boxes and groups |
| `text-valign` | `middle` | `top`, `middle`, `bottom`; boxes only |
| `shape` | `rect` | boxes only: any `shape=` name (`kashika ref shapes`) |
| `size` | fitted | boxes and groups without their own `size`: `(w,h)`, either axis `auto` |

The canvas is transparent by default (JPEG: white). `background=C` on the `diagram` line paints it, and `background=none` keeps it transparent. `low-contrast` checks canvas text against `background=`, or white. Line labels cut only their own line, so the fill beneath shows between the words.

{styles}

- Precedence: inline attribute > `style=` > built-in default. This holds for `shape` and `size` too: an inline `shape=` beats the style's, and an inline `size (w,h)` replaces the style's `size=(w,h)` whole.
- A user style with a built-in's name overrides it key by key, and so does a library style of that name imported without `as`.
- Groups start from `boundary`. Groups take no shape: a group's `style=` naming a style with `shape` is an error, and so is `shape` in `style boundary`. A group may take its `size` from its style, but `size` in `style boundary` is an error. Groups take `text-align` from a style but not `text-valign`: naming a style that sets it is an error, and so is `text-valign` in `style boundary`. Without one, or on an `auto` axis, a group fits its children.
- `size=12` in a style is an error: the text size is `font-size`. Lines ignore a style's `size`.
- In an imported library, a style's `shape` names the library's own shapes as the library writes them: `shape=s3` in `lib/aws.kash`, imported `as aws`, is `aws.s3`.
- On lines, a style applies only its `stroke*` and font keys, never `text-align` or `text-valign`. Free `text` takes no `style=`.

```kash
diagram "commits" size (200,60)
style commit { shape=ellipse fill=#e8f0fe stroke=#1a73e8 size=(20,20) }
box c1 at (20,20) style=commit
box c2 at (60,20) style=commit
box tag at (100,20) style=commit shape=diamond
```
