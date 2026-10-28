# Styles

```text
style <name> { key=value ... }
box api "API" at (40,40) size (140,60) style=service fill=#ffffff
```

| key | default | values |
|---|---|---|
| `fill` | `#ffffff` | color |
| `stroke` | `#334155` | color |
| `stroke-width` | `1.5` | number |
| `stroke-style` | `solid` | `solid`, `dashed`, `dotted` |
| `font-size` | `14` | number |
| `font-weight` | `normal` | `normal`, `bold` |
| `color` | `#0f172a` | text color |
| `radius` | `0` | corner radius; `shape=rounded` defaults to 8 |
| `padding` | `8` | inset of the text area |
| `muted` | `#64748b` | color of sublabels and muted text |

The canvas is transparent by default (JPEG: white). `background=C` on the `diagram` line paints it, and `background=none` keeps it transparent. `low-contrast` checks canvas text against `background=`, or white. Line labels cut only their own line, so the fill beneath shows between the words.

| built-in | fill | stroke |
|---|---|---|
| `service` | `#e8f0fe` | `#1a73e8` |
| `datastore` | `#e6f4ea` | `#137333` |
| `external` | `#f1f3f4` | `#5f6368` |
| `queue` | `#fef7e0` | `#b06000` |
| `note` | `#fffbe6` | `#d4b106` |
| `boundary` | none | `#94a3b8`, dashed, text `#475569` |

- Precedence: inline attribute > `style=` > built-in default.
- A user style with a built-in's name overrides it key by key.
- Groups start from `boundary`.
- On lines, a style applies only its `stroke*` and font keys.
