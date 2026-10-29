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
| `shape` | `rect` | boxes only: any `shape=` name (`kashika ref shapes`) |

The canvas is transparent by default (JPEG: white). `background=C` on the `diagram` line paints it, and `background=none` keeps it transparent. `low-contrast` checks canvas text against `background=`, or white. Line labels cut only their own line, so the fill beneath shows between the words.

| built-in | fill | stroke |
|---|---|---|
| `service` | `#e8f0fe` | `#1a73e8` |
| `datastore` | `#e6f4ea` | `#137333` |
| `external` | `#f1f3f4` | `#5f6368` |
| `queue` | `#fef7e0` | `#b06000` |
| `note` | `#fffbe6` | `#d4b106` |
| `boundary` | none | `#94a3b8`, dashed, text `#475569` |

- Precedence: inline attribute > `style=` > built-in default. This holds for `shape` too: an inline `shape=` beats the style's.
- A user style with a built-in's name overrides it key by key.
- Groups start from `boundary`. Groups take no shape: a group's `style=` naming a style with `shape` is an error, and so is `shape` in `style boundary`.
- In an imported library, a style's `shape` names the library's own shapes as the library writes them: `shape=s3` in `lib/aws.kash`, imported `as aws`, is `aws.s3`.
- On lines, a style applies only its `stroke*` and font keys.

```kash
diagram "commits" size (200,60)
style commit { shape=ellipse fill=#e8f0fe stroke=#1a73e8 }
box c1 at (20,20) size (20,20) style=commit
box c2 at (60,20) size (20,20) style=commit
box tag at (100,20) size (20,20) style=commit shape=diamond
```
