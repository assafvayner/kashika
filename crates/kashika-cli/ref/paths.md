## Line paths

A line is `start -> segment -> segment ...`. Each segment ends at a point, which starts the next one.

| segment | path |
|---|---|
| `<point>` | straight to the point |
| `h dx` / `v dy` | move by dx / dy; negative goes left / up |
| `x X` / `y Y` | move to x=X / y=Y; X or Y is a number or a point (`y db.top`) |
| `hv <point>` | horizontal, then vertical: one elbow |
| `vh <point>` | vertical, then horizontal |
| `curve left:N <point>` | curve that bulges N px to the left of the direction of travel |
| `curve right:N <point>` | the same, to the right |
| `curve via (c1) (c2) <point>` | cubic Bézier with explicit control points |

| attribute | default | meaning |
|---|---|---|
| `start=` `end=` | `none` | `arrow`, `open`, `diamond`, `dot` or `none`; the tip sits on the endpoint |
| `label="..."` `label@t="..."` | `t=0.5` | label at fraction `t` of the path length, markup allowed; repeatable |
| `radius=N` | `0` | rounds every corner |
| `stroke` `stroke-width` `stroke-style` `style=` | style | a style applies only its stroke and font keys |
| `ortho` | off | any diagonal segment is an error |
| `allow=<code>[:<id>],...` `reason="..."` | | suppress findings |

**The last segment sets the arrow direction.** Enter `left`/`right` with a horizontal hop (`vh`, or `x`/`y` and then the anchor) and `top`/`bottom` with a vertical one (`hv`). Name your lines; unnamed ones are `line@L<n>`.
