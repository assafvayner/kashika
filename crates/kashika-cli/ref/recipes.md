## Routing recipes

Each recipe checks clean. `kashika route` suggests paths like these.

**Staircase.** Leave sideways, change rows in a free column, arrive sideways.

```kash
diagram "Staircase" size (420,200)
box a "A" at (20,20) size (100,40)
box b "B" at (280,120) size (100,40)
line a_b a.right -> x 200 -> vh b.left end=arrow
```

**Wrap over the top.** Leave upward, run above the row, come down.

```kash
diagram "Wrap" size (520,140)
box a "A" at (20,60) size (100,40)
box b "B" at (210,60) size (100,40)
box c "C" at (400,60) size (100,40)
line a_c a.top -> y 30 -> hv c.top end=arrow
```

**Gap corridor.** Run the lane through a gap at least 2×`clearance` wide.

```kash
diagram "Gap" size (460,280)
box a "A" at (20,100) size (100,40)
box hi "Upper" at (180,20) size (100,80)
box lo "Lower" at (180,160) size (100,80)
box b "B" at (340,200) size (100,40)
line a_b a.right -> x 310 -> vh b.left end=arrow
```

**Outside margin.** When the middle is full, go around the outside.

```kash
diagram "Margin" size (440,260)
box a "A" at (40,40) size (100,40)
box mid "Middle" at (40,110) size (360,40)
box b "B" at (300,190) size (100,40)
line a_b a.left -> x 20 -> vh b.left end=arrow
```

**Fan-out via a junction.** One trunk, then one branch per target.

```kash
diagram "Fan-out" size (460,240)
box src "Source" at (180,20) size (100,40)
box a "A" at (20,160) size (100,40)
box b "B" at (180,160) size (100,40)
box c "C" at (340,160) size (100,40)
junction fan at (src.bottom.x, 110)
line src_fan src.bottom -> fan
line fan_a fan -> hv a.top end=arrow
line fan_b fan -> b.top end=arrow
line fan_c fan -> hv c.top end=arrow
```

**The comb.** N sources into one edge. Land them in source order with `@t`, and give the top source the lane nearest the target, so no lane crosses another.

```kash
diagram "Comb" size (520,280)
box s1 "S1" at (20,20) size (100,40)
box s2 "S2" at (20,90) size (100,40)
box s3 "S3" at (20,160) size (100,40)
box sink "Sink" at (360,140) size (140,120)
line s1_sink s1.right -> x 320 -> vh sink.left@0.25 end=arrow
line s2_sink s2.right -> x 290 -> vh sink.left@0.5 end=arrow
line s3_sink s3.right -> x 260 -> vh sink.left@0.75 end=arrow
```

**Unavoidable crossings.** Some graphs are non-planar. Cross once, at a right angle, and say so.

```kash
diagram "Crossing" size (420,200)
box a "A" at (20,20) size (100,40)
box b "B" at (20,140) size (100,40)
box c "C" at (300,20) size (100,40)
box d "D" at (300,140) size (100,40)
line a_d a.right -> x 200 -> vh d.left@0.25 end=arrow
line b_c b.right -> x 230 -> vh c.left@0.75 end=arrow allow=cross:a_d reason="non-planar"
```
