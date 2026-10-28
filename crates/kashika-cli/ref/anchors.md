## Anchors and points

```text
point := (x,y) | ref [+(dx,dy)]
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

- Offsets use `+` with signed parts, `db.top+(-10,0)`, and combine with `@t`: `db.left@0.25+(0,4)`.
- Groups have anchors too: `vpc.top`.
- Edge anchors on rounded shapes move straight onto the outline, so a vertical line into `db.top` stays vertical.
