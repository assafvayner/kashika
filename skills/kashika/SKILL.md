---
name: kashika
description: Use when creating or editing a technical diagram (architecture, data flow, flowchart, sequence, ER, system boundaries) as a .kash file, or when asked for a diagram where exact placement and line routing matter. Covers how Kashika works, the write → check → fix → render loop, and where `kashika ref` documents the language.
---

# Writing Kashika diagrams

Kashika is a text-to-diagram language with **no auto-layout**. You place every box and route every line in a `.kash` file; the `kashika` CLI checks the geometry and renders SVG or PNG. The full language reference is built into the tool: **run `kashika ref` first**, then the topics it lists as you need them.

## How it works

- Coordinates are pixels and **y grows down**. A box sits `at` a point; without `size` it fits its text.
- Every shape has **anchors**: `a.top`, `a.right`, `a.center`, corners `a.tl`, points along a side like `a.top@0.25`, and offsets like `a.right+(80,0)`. Place shapes relative to each other (`at a.right+(80,0) pin=left`) so they move together; in rows and grids of mixed shapes chain on corners (`at a.tr+(80,0) pin=tl`), since edge anchors sit on a cloud's or parallelogram's outline.
- A **line is a series of hops from anchor to anchor**: `a.right -> hv b.top` goes horizontal, then vertical into `b`'s top. The last hop sets the arrow's direction, so enter a `left`/`right` side horizontally and a `top`/`bottom` side vertically.
- A **group** draws a boundary around its children. Literal coordinates inside a group are relative to it; anchors are always absolute, and ids are global (`db.left`, never `vpc.db.left`).
- **`kashika check` is your eyes**: it reports crossings, lines through boxes, near-misses, overlaps, labels on things and text that doesn't fit, each with a help line naming the fix. Then render and look, since a clean check is not a good diagram.

```kash
diagram "Checkout"
box web "Web" at (20, 20) style=service
box api "Payments API" at web.right+(100, 0) pin=left style=service
box db "Orders DB" at api.bottom+(0, 60) pin=top shape=cylinder style=datastore
arrow web_api web.right -> api.left label="HTTPS" ortho
arrow api_db api.bottom -> db.top ortho
```

The diagram has no `size`, so the canvas fits the drawing. `arrow` is a `line` with an arrowhead at the end, and `ortho` makes any diagonal hop an error (`diagram … ortho` sets it for every line).

## The loop

1. **Write** the file. Sketch rows and columns first, with 60–80px gaps and free lanes for the lines.
2. **Check:** `kashika check file.kash`. Fix every error first, starting with the first, then every warning, using each `help` line: did-you-mean names, required sizes, free lanes, clear `label@t` positions, the elbow to use. `kashika route file.kash a.right b.left` suggests ready-to-paste paths, and `kashika inspect file.kash <ids>` prints absolute coordinates.
3. Repeat until the check is clean. Suppress a warning only when it is intentional, and narrowly (`kashika ref lints`).
4. **Render and look:** `kashika render file.kash -o file.png`, then open or read the PNG. Fix what looks wrong (arrows pointing the wrong way, missing connections, cramped spacing) and check again.

## Where to look

| you need | run |
|---|---|
| the core guide: rules, syntax, anchors, line paths, common fixes | `kashika ref` |
| how big a box must be; fitted sizes | `kashika ref sizing` |
| labels, markup, multi-line text in a box | `kashika ref text` |
| shapes (cylinder, diamond, …), icons, images, imports | `kashika ref shapes` |
| colors, styles, the built-in styles | `kashika ref styles` |
| fan-out, self-loops, sequence diagrams, ER tables, timelines, routing patterns | `kashika ref recipes` |
| a complete worked example | `kashika ref examples` |
| what a diagnostic means and how to fix it | `kashika ref <code>`, e.g. `kashika ref through` |
| every diagnostic, and how to suppress one | `kashika ref lints` |
| every command and flag (`check`, `inspect`, `route`, `measure`, `fmt`, `icons`) | `kashika ref cli` |
| exact text widths | `kashika measure "Payments API"` |
| available icons and shapes | `kashika icons --search database` |
