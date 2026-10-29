## Line paths

A line is `start -> segment -> segment ...`. Each segment ends at a point, which starts the next one. The optional id follows `line` directly, with no colon: `line api_db api.right -> db.left`. `arrow` is the same statement with `end=arrow` as its default: `arrow api_db api.right -> db.left`; an explicit `end=` wins. `arrow` can't be an id.

| segment | path |
|---|---|
| `<point>` | straight to the point |
| `h dx` / `v dy` | move by dx / dy; negative goes left / up |
| `x X` / `y Y` | move to x=X / y=Y; X or Y is a number or a point (`y db.top`, not `y db.top.y`) |
| `hv <point>` | horizontal, then vertical: one elbow |
| `vh <point>` | vertical, then horizontal |
| `vhv <point>` / `hvh <point>` | vertical, horizontal halfway, vertical (or the other way round): two elbows; `vhv@t` puts the middle run `t` of the way (`0 < t < 1`); straight when the point is level |
| `curve left N <point>` | curve that bulges N px to the left of the direction of travel |
| `curve right N <point>` | the same, to the right |
| `curve via (c1) (c2) <point>` | cubic Bézier with explicit control points |

| attribute | default | meaning |
|---|---|---|
| `start=` `end=` | `none` (`end` on `arrow`: `arrow`) | {markers}. `arrow`, `open`, `diamond` and `dot` put their tip on the endpoint; the hollow UML ends `triangle` (generalization), `hollow-diamond` (aggregation) and `circle` show the fill beneath. Ends grow with `stroke-width` above 1.5. Crow's-foot (ER) ends `one`, `many`, `one-many`, `zero-one`, `zero-many` sit on the line without trimming it; a `zero-*` circle is open |
| `label="..."` `label@t="..."` | `t=0.5` | label at fraction `t` of the path length, markup allowed; repeatable |
| `label-side=` | `on` | `on` centers labels on the line and cuts it out under them; `above`/`below` (horizontal segment) or `left`/`right` (vertical) put them beside it, 4px off the stroke, unmasked |
| `radius=N` | `0` | rounds every corner |
| `stroke` `stroke-width` `stroke-style` `style=` | style | a style applies only its stroke and font keys |
| `ortho` | off | any diagonal segment is an error |
| `allow=<code>[:<id>],...` `reason="..."` | | suppress findings |

**The last segment sets the arrow direction.** Enter `left`/`right` with a horizontal hop (`vh`, `hvh`, or `x`/`y` and then the anchor) and `top`/`bottom` with a vertical one (`hv`, `vhv`). Name your lines; unnamed ones are `line@L<n>`.

**Labels beside the line.** `label-side` applies to every label of the line. A side that doesn't match the segment under the label (`above` on a vertical segment, anything but `on` on a diagonal or curve) is an `attr` error, and the label stays on the line. At a corner either segment counts.

```kash
diagram "Messages" size (320,140) kashika="0.1.0"
arrow req (20,40) -> h 280 label="request" label-side=above
arrow res (300,100) -> h -280 stroke-style=dashed label="reply" label-side=below
```
