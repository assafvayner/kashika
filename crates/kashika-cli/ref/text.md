# Text

- Markup works in any label or text, a line's `label=` included: `**bold**` and `` `code` `` (JetBrains Mono, spaces kept). No nesting; `\*` and `` \` `` are literal; an unclosed marker is a `parse` error. There are no italics: a single `*…*` is literal text.
- `\n` breaks a line. There is no automatic wrapping.
- `sublabel="..."` adds one muted line under a box's or group's label, at 0.8 × `font-size`, `gap` (default 2) below it.
- Placement in a box: `text-align=left|center|right` (default center), `text-valign=top|middle|bottom` (default middle).
- A group's label (and sublabel) sits top-left in its title band. `text-align` works on a group, `text-valign` is an error. Keep group titles short: a line leaving a child near the group's top edge often runs through the title.
- A free `text` is centered on `at`, or aligned to it with `text-align=left|right`, and takes `font-size`, `color` and `muted`. Use it only on empty canvas.
- `text-area=below` on an SVG shape puts the label under it (`kashika ref shapes`); lines leaving that side collide with it, so route them from another side.
- The bundled fonts cover Latin, Greek, Cyrillic, common symbols and arrows, box drawing (`┌─┐│└┘`, from the mono face) and color emoji, sequences such as `👍🏽`, `👩‍💻` and `🇺🇸` included. Emoji keep their own colors and are 1 em wide. CJK, Hangul and dingbats such as `✘` are `missing-glyph`: use an `icon=` or `image`. A tab is four spaces.

## Text blocks

For three or more lines, bullets or mixed sizes, give the box a block instead of a `label`:

```kash
diagram "Text" size (360, 200)
box pay "Payments API" at (20, 20) sublabel="v2.4.1" style=service
box api at (20, 100) size (200, 90) style=service text-align=left {
  text "**Payments API**"
  text "`POST /charge`" font-size=12
  text "• validates cards" font-size=12
}
```

- Items stack top to bottom, `gap` apart. Each takes `font-size`, `color`, `text-align` and `muted` (`size` and `align` are aliases).
- An item with `at (x,y)` is absolute, in box-local coordinates (`at` is its alignment point and vertical center), and leaves the stack. This is how to put text anywhere inside a shape: `box a … { text "x" at (100,60) }`.
- Don't combine `label`/`sublabel` with stacked items (`text-conflict`); absolute items are fine. A block of one or two plain items is `block-reducible`: use the `label`/`sublabel` from the help.
