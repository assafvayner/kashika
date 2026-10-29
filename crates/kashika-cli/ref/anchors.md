## Anchors and points

```text
point := ((x,y) | ref | (point)) [+(dx,dy) | -(dx,dy)]...
ref   := <id>.<anchor> | <junction-id>
x, y  := number | ref.x | ref.y        // mixed point: (db.left.x, lane.y)
```

| anchor | position |
|---|---|
| `top` `bottom` `left` `right` | edge midpoints |
| `top@t` `bottom@t` | along the edge from left to right, `0 <= t <= 1` |
| `left@t` `right@t` | along the edge from top to bottom |
| `center` | center |
| `tl` `tr` `bl` `br` | bounding-box corners; also `top-left` and so on |

- `+(dx,dy)` adds an offset and `-(dx,dy)` subtracts it; parts are signed either way, so `db.top+(-10,0)` is `db.top-(10,0)`. Offsets combine with `@t`: `db.left@0.25-(0,4)`.
- Offsets may follow any point and add up: `(10,20)+(5,0)`, `(db.left.x, lane.y)+(0,26)`, `db.top+(0,4)+(10,0)`.
- Parentheses around a point change nothing and must be closed: `(db.center+(0,40))` is `db.center+(0,40)`, and `(db.center)+(0,40)` is the same point. They nest at most 16 deep: `((db.left.x, lane.y)+(0,4))`.
- Groups have anchors too: `vpc.top`.
- Edge anchors on non-rectangular shapes move straight onto the outline, so a vertical line into `db.top` stays vertical.
