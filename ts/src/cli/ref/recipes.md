# Recipes

Each example checks clean.

## Routing recipes

`kashika route` suggests paths like these.

**Staircase.** Leave sideways, change rows in a free column, arrive sideways.

```kash
diagram "Staircase" size (420, 200)
box a "A" at (20, 20) size (100, 40)
box b "B" at (280, 120) size (100, 40)
line a_b a.right -> x 200 -> vh b.left end=arrow
```

**Wrap over the top.** Leave upward, run above the row, come down.

```kash
diagram "Wrap" size (520, 140)
box a "A" at (20, 60) size (100, 40)
box b "B" at (210, 60) size (100, 40)
box c "C" at (400, 60) size (100, 40)
line a_c a.top -> y 30 -> hv c.top end=arrow
```

**Gap corridor.** Run the lane through a gap at least 2×`clearance` wide.

```kash
diagram "Gap" size (460, 280)
box a "A" at (20, 100) size (100, 40)
box hi "Upper" at (180, 20) size (100, 80)
box lo "Lower" at (180, 160) size (100, 80)
box b "B" at (340, 200) size (100, 40)
line a_b a.right -> x 310 -> vh b.left end=arrow
```

**Outside margin.** When the middle is full, go around the outside.

```kash
diagram "Margin" size (440, 260)
box a "A" at (40, 40) size (100, 40)
box mid "Middle" at (40, 110) size (360, 40)
box b "B" at (300, 190) size (100, 40)
line a_b a.left -> x 20 -> vh b.left end=arrow
```

**The comb.** N sources into one edge. Land them in source order with `@t`, and give the top source the lane nearest the target, so no lane crosses another.

```kash
diagram "Comb" size (520, 280)
box s1 "S1" at (20, 20) size (100, 40)
box s2 "S2" at (20, 90) size (100, 40)
box s3 "S3" at (20, 160) size (100, 40)
box sink "Sink" at (360, 140) size (140, 120)
line s1_sink s1.right -> x 320 -> vh sink.left@0.25 end=arrow
line s2_sink s2.right -> x 290 -> vh sink.left@0.5 end=arrow
line s3_sink s3.right -> x 260 -> vh sink.left@0.75 end=arrow
```

**Unavoidable crossings.** Some graphs are non-planar. Cross once, at a right angle, and say so.

```kash
diagram "Crossing" size (420, 200)
box a "A" at (20, 20) size (100, 40)
box b "B" at (20, 140) size (100, 40)
box c "C" at (300, 20) size (100, 40)
box d "D" at (300, 140) size (100, 40)
line a_d a.right -> x 200 -> vh d.left@0.25 end=arrow
line b_c b.right -> x 230 -> vh c.left@0.75 end=arrow allow=cross:a_d reason="non-planar"
```

## Diagram recipes

**Fan-out via a junction.** One trunk into a junction, then one line per child into its `top` (`hv` when it's off to the side), so no two lines share a segment. When two branches leave the junction the same way, run the trunk to the farthest child and put a second junction on it for the nearer one (`mid` below). A trunk that taps a child through a junction counts as connecting into that child's group, so it is no `group-transit`.

**Two lines on one side** of a box: spread them with `side@t` (`core.right@0.3`, `core.right@0.7`) to avoid `port-crowding` and `shared-segment`.

**Self-loop.** `curve via` from `top@0.8` back to `top@0.2`, with control points above the box. `out-of-canvas` measures a curve's true extent, labels included, so leave room.

```kash
diagram "Fan-out" size (500, 260) kashika="0.1.0"
box src "Source" at (20, 60) size (100, 40)
box a "A" at (20, 200) size (100, 40)
box b "B" at (200, 200) size (100, 40)
box c "C" at (380, 200) size (100, 40)
junction fan at src.bottom+(0, 50) dot
junction mid at (b.top.x, fan.y) dot
line src_fan src.bottom -> fan
line fan_a fan -> a.top end=arrow
line fan_c fan -> hv c.top end=arrow
line mid_b mid -> b.top end=arrow
line again src.top@0.8 -> curve via src.top@0.8+(0, -50) src.top@0.2+(0, -50) src.top@0.2 end=arrow label="retry"
```

**Sequence diagrams.** Lifelines are 2px-wide dashed boxes under each participant, with `layer=under` so messages that skip past them raise no `through`, `near` or `label-collision`. An activation bar is a narrow white box on the lifeline, also `layer=under`, written after the lifeline so it paints on top. Place each message at an absolute height with an offset from the lifeline's top (`lc.top+(0,30) -> x ls.left`) and point it at the bar's side while active. Give messages `label-side=above`; on a message that skips lifelines, pick a `label@t` between two of them. Space lifelines by label width + ~30px.

```kash
diagram "Lifelines" size (520, 240) kashika="0.1.0"
box c "Client" at (40, 20) size (100, 40)
box s "Server" at (210, 20) size (100, 40)
box d "DB" at (380, 20) size (100, 40)
box lc at c.bottom size (2, 160) pin=top stroke-style=dashed layer=under
box ls at s.bottom size (2, 160) pin=top stroke-style=dashed layer=under
box ld at d.bottom size (2, 160) pin=top stroke-style=dashed layer=under
box act at ls.top+(0, 40) size (10, 80) pin=top fill=white layer=under
line req lc.top+(0, 30) -> x act.left end=arrow label="request" label-side=above
line q act.tr+(0, 20) -> x ld.left end=arrow label="query" label-side=above
line skip lc.top+(0, 130) -> x ld.left end=arrow label@0.75="audit" label-side=above
line res act.bl+(0, -10) -> x lc.right end=arrow stroke-style=dashed label="reply" label-side=above
```

**Table cards (ER diagrams).** A `layer=under` outline box, a header box, then one row box per column stacked with `at prev.bl`, each with a `row` style: `stroke=none padding=3 text-align=left` and the row's size. `` `code` `` text is monospace and keeps its spaces, so pad the names to line the columns up. Draw relationships between row boxes (`orders_cust.right -> customers_id.left`) and offset a card's y so the two rows are level. Anchor at a row's center (`r2.right`, not `@0.2`): `near` measures a row's whole box even when it draws no border. Mark cardinality with crow's-foot ends: `start=many end=one` on a foreign key, `zero-one`/`zero-many` when optional, `one-many` for at least one. Two crow's feet on one side need 12px between them.

```kash
diagram "Tables" size (520, 160) kashika="0.1.0"
style row { stroke=none padding=3 text-align=left font-size=12 size=(200, 22) }
box orders at (20, 20) size (200, 96) stroke=#475569 layer=under
box orders_h "**orders**" at orders.tl size (200, 26) fill=#dbeafe stroke=none padding=4
box orders_id "`id           uuid  PK`" at orders_h.bl style=row
box orders_cust "`customer_id  uuid  FK`" at orders_id.bl style=row
box orders_at "`created_at   tstz`" at orders_cust.bl style=row
box customers at (300, 42) size (200, 74) stroke=#475569 layer=under
box customers_h "**customers**" at customers.tl size (200, 26) fill=#dbeafe stroke=none padding=4
box customers_id "`id     uuid  PK`" at customers_h.bl style=row
box customers_em "`email  text`" at customers_id.bl style=row
line fk orders_cust.right -> customers_id.left start=many end=one
```

**Guides for rows and columns.** A junction without `dot` draws nothing, so use junctions as named lanes and columns, chained so they move together, and place shapes on them with mixed points. Moving a lane or the first column is then a one-line edit. Put repeated looks, `shape=` included, in a `style`.

**Rows and grids of mixed shapes.** Chain on corner anchors, not edge anchors. `right` and `bottom` move onto the outline (a cloud's bumps, a document's wave, a parallelogram's slant, a person's shoulders), so `at a.right+(40,0) pin=left` drifts along a row of mixed shapes. Corners are always the bounding box's: `at a.tr+(40,0) pin=tl` for the next column, `at a.bl+(0,40) pin=tl` for the next row, or place every shape on guide junctions as above.

**Graphs and timelines** (commit graphs, state dots): place small markers with `pin=center` on guide columns and lanes, leave `grid=` off, and give hollow markers `fill=white`. Lines to a diamond's `left`/`right` meet its tips. Diagonal and curved edges are fine here; leave `ortho` off.

```kash
diagram "Guides" size (420, 200) kashika="0.1.0"
style main { fill=#d33a3a stroke=#d33a3a shape=ellipse }
style dev { fill=#1e9e5a stroke=#1e9e5a shape=ellipse }
junction main_lane at (0, 60)
junction dev_lane at (0, 140)
junction c0 at (120, 0)
junction c1 at c0+(70, 0)
junction c2 at c1+(70, 0)
text "**main**" at (20, main_lane.y) text-align=left
text "**develop**" at (20, dev_lane.y) text-align=left
box m0 at (c0.x, main_lane.y) size (16, 16) pin=center style=main
box d1 at (c1.x, dev_lane.y) size (16, 16) pin=center style=dev
box m1 at (c2.x, main_lane.y) size (16, 16) pin=center style=main
line m0.right -> d1.left
line d1.right -> m1.left
line m0.right -> m1.left
```
