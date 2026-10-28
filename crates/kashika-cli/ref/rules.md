# Kashika reference

Kashika draws technical diagrams from `.kash` files. There is no auto-layout: you place every box and route every line, and `kashika check` reports what is wrong with the geometry. More detail: `kashika ref <topic>` with `syntax`, `anchors`, `paths`, `shapes`, `styles`, `metrics`, `lints`, any diagnostic code, `examples` or `cli`.

## Five rules people get wrong

1. **y grows down.** `(0,0)` is the top-left corner of the canvas, and `v -40` moves up.
2. **Literal coordinates are in the frame of the enclosing group.** `at (10,20)` inside `group vpc at (100,100) ...` lands at canvas `(110,120)`. Anchors, `h`/`v` moves and `+(dx,dy)` offsets are the same in every frame.
3. **Ids are global and bare.** Boxes, groups, junctions, lines and texts share one namespace. Write `db.left`, never `vpc.db.left`.
4. **Anchors resolve to the shape's true position**, wherever you write them. Prefer anchors, `x`/`y` targets and `hv`/`vh` elbows to literal points: they keep working when a box moves.
5. **`allow=` hides findings; it doesn't fix them.** `allow=cross:l2` silences crossings with `l2` only. Suppressed findings still count in the summary.
