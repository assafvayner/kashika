# Kashika — independent design review

**Reviewer:** deepseek-v4.1-flash-1017 (an agent that would be a *user* of this tool, not its author)
**Scope:** `docs/review/kashika-review-packet.md`, Parts A and B
**Verdict in one line:** the coordinate model and the anchor system are right, the *diagnostics* are close to right, and the *authoring loop* is missing the one primitive that makes the whole thing usable — a way to say "route orthogonally to that anchor" without first computing that anchor's absolute coordinates. Everything else in this review follows from that.

**One finding deserves to travel alone:** Part A's `A1` is not achievable. The requested diagram has a crossing-number floor of 2 (proven in §A1.3, not asserted), so a reviewer who reports "no crossings" is either simpler than my attempt or reporting an unverified claim. Any zero-crossing goal in this language has to be conditional on the graph, and the tool must be able to say *"this diagram cannot be drawn without crossings, here are the two cheapest"*.

## Method, and a disclosure

The packet is a single file, so Part B was on screen before I finished Part A. I therefore did Part A **first and mechanically**, without looking at B1/B2/B3/B4, and only then read B. Where my A3 items coincide with B3's leading questions I say so explicitly, and where my answers were formed before reading B3 I've marked them with ⚑.

`kashika-review-gpt-6.1-sol.md` was already present in this directory; I deliberately did not open it, so that this review stays independent.

Because there is no implementation, I could not verify any claim by testing. So I wrote a throwaway checker (`~230 lines` of Python: group-offset resolution, the full anchor table, `h`/`v`/`curve bend`, and the `cross`/`through`/`overlap`/`spill`/`out-of-canvas` lints) and used it to *drive* A1 and A2 rather than trusting my own arithmetic. Appendix A has the notable numbers. This is itself the most important datum in the review:

> I abandoned mental verification after roughly three minutes and wrote a checker instead. My file has 17 lines, 76 segments, 34 anchor references and 32 `h`/`v` segments; pairwise crossing analysis is ~2,900 segment pairs. **No agent will ever do that in its head.** Kashika's entire value proposition depends on `check` and `inspect` being excellent, because they are not a convenience — they are the execution model.

---

# Part A — the language

## A1 — attempted answer

This is my best artifact after ~10 layout iterations. It satisfies the requested topology. It has **2 line crossings**, which I could not remove; §A1.2 explains why the tension is structural and §A1.3 brute-forces the floor and shows **2 is the minimum**, so no reviewer can do better on this prompt.

```kashika
diagram "Upload path" size (1560,1000) grid=5

style svc  { fill=#e8f0fe stroke=#1a73e8 }
style data { fill=#e6f4ea stroke=#137333 }
style q    { fill=#fef7e0 stroke=#b06000 }

box browser "browser" at (80,20) size (150,60) style=svc
box cdn     "CDN edge" at (330,20) size (150,60) style=svc

group vpc "VPC" at (60,120) size (1120,840) style=boundary {

  box lb "load balancer" at (60,20) size (170,60) style=svc

  group k8s "k8s cluster" at (140,190) size (620,440) style=boundary {
    box app1    "app-1"    at (0,90)   size (150,60) style=svc
    box app2    "app-2"    at (225,90) size (150,60) style=svc
    box app3    "app-3"    at (450,90) size (150,60) style=svc
    box kafka   "kafka"    at (0,210)  size (600,60) shape=rounded style=q
    box worker1 "worker-1" at (75,370) size (150,60) style=svc
    box worker2 "worker-2" at (330,370) size (150,60) style=svc
  }

  box pg      "postgres" at (960,120) size (150,120) shape=cylinder style=data
  box replica "replica"  at (960,420) size (150,80)  shape=cylinder style=data
}

box s3 "S3" at (1240,760) size (160,80) shape=cloud style=external

line browser.right -> cdn.left "HTTPS" end=arrow
line cdn.right -> h 20 -> v 90 -> lb.tr "edge" end=arrow

# --- load balancer -> app tier -------------------------------------------
line lb.left       -> h -30 -> v 245 -> h 110 -> app1.left@0.25 end=arrow
line lb.bottom@0.8 -> v 20  -> h 134 -> v 210 -> h 35  -> app2.left end=arrow allow=cross
line lb.right      -> h 900 -> v 210 -> h -390 -> v 20 -> app3.tr end=arrow allow=cross

# --- app tier -> kafka ---------------------------------------------------
line app1.bottom -> v 60 -> kafka.top@0.125 end=arrow "publish"
line app2.bottom -> v 60 -> kafka.top@0.5   end=arrow "publish"
line app3.bottom -> v 60 -> kafka.top@0.875 end=arrow "publish"

# --- app tier -> postgres (three lanes above the app row) ----------------
line app1.top@0.1 -> v -150 -> h 805 -> pg.left@0.08 end=arrow "SQL"
line app2.top@0.6 -> v -130 -> h 505 -> pg.left@0.25 end=arrow "SQL"
line app3.top@0.6 -> v -110 -> h 280 -> pg.left@0.42 end=arrow "SQL"

line pg.bottom -> replica.top end=arrow "replicate"

# --- kafka -> workers ----------------------------------------------------
line kafka.bottom@0.25  -> v 100 -> worker1.top end=arrow "consume"
line kafka.bottom@0.675 -> v 100 -> worker2.top end=arrow "consume"

# --- retry (the curved line) ---------------------------------------------
line retry: worker1.left -> h -40 -> v -60 -> curve bend=20 -> kafka.bottom@0.06 end=arrow dash

# --- workers -> S3 -------------------------------------------------------
line worker1.bottom -> v 190 -> h 1070 -> v -130 -> s3.right       end=arrow "put"
line worker2.bottom -> v 120 -> h 675  -> v -20  -> s3.bottom@0.25 end=arrow "put"

text "v0 draft" at (80,960) size=12 color=#888
```

My checker's verdict on this file:

```
canvas 1560x1000  boxes 14  lines 17
  ('cross', 'lb.bottom@0.8 -> app2.left', 'app1.top@0.1 -> pg.left', (390, 250))
  ('cross', 'lb.right -> app3.tr',         'pg.bottom -> replica.top', (1095, 380))
```

Both crossings are on lines marked `allow=cross`. Nothing else — no overlaps, no spills, no off-canvas, no lines through boxes.

### A1.1 — what the file looks like it says

Ten of the 17 lines are plumbing (`v 20 -> h 134 -> v 210 -> h 35`). The *intent* occupies 7 tokens (`lb.bottom@0.8`, `app2.left`); the *geometry* occupies 5 numbers, every one of which is a hand-computed function of some other box's absolute position. That ratio is the review.

### A1.2 — why the crossings are unavoidable (structural, not my failure)

The requested shape is: a 1-D row of three `app` boxes; family **A** = load balancer → each app (from above); family **B** = each app → kafka (below); family **C** = each app → postgres (to the right, in practice also via the top band).

Only the top edge and the bottom edge of a middle box are reachable. So A and C must share the top band. Now take `app1`'s lane to postgres — it must run rightward, so its horizontal span covers every box to its right. Every feeder that must *reach* `app2` or `app3` crosses that span at some y inside its own vertical drop. Two arrangements are possible and both fail:

* feeders descend **above** the lanes (y < lane y): their drops cross the lanes;
* feeders descend **below** the lanes: they cross the app-tops' vertical risers at `x = app1.center, app2.center`.

That is exactly the crossing-number argument for K₃,₂, and it is not an artefact of my particular coordinates — I found the same wall from six different arrangements before working out why. Two escapes exist and v0 offers neither:

1. **A bus / merge node.** "All three app pods publish to kafka" is one edge in reality. Neither `line` merging nor a `junction` statement exists, and `shared-segment` actively punishes the manual version of it.
2. **`TikZ`-style orthogonal-to-point** (`->|-`) plus a way to reserve a corridor. With `-> v -150 ->| pg.left` the author never computes the anchor's x, so lanes can be allocated in the *order the author names them* and the drop points become free variables rather than derived numbers.

I got from 6 crossings → 4 → 3 → 2 by hand over ten iterations. I'd guess a human diagrammer stops at "close enough" and an agent stops at "check is green, ship it" — which is why the second crossing (a feeder passing a `pg → replica` link purely for want of a free channel) survived into my deliverable.

### A1.3 — the floor is 2, and that is what the file achieves

"Unavoidable" is a claim, so I made it falsifiable: I brute-forced every orthogonal elbow path from the LB's boundary to each app (6 exits × 202 descent x's × 90 return altitudes ≈ 110k candidates), scoring each against the frozen postgres-lane geometry and rejecting anything that passes through a box:

| feeder | minimum crossings | cheapest shape found |
|---|---|---|
| LB → `app-1` | **0** | `lb.left -> h -30 -> v 245 -> h 110 -> app1.left@0.25` |
| LB → `app-2` | **1** | `lb.right -> h 60 -> v 260 -> h 75 -> app2.left`, crossing `app-1`'s lane at y=250 |
| LB → `app-3` | **1** | over the top, down the right margin at x≈1170–1190, back left — crossing the `pg → replica` link |
| **total** | **2** | ← what the submitted file has (same counts, different elbows) |

Why `app-2` cannot do better: `app-1`'s postgres lane is a horizontal wall at y=250 spanning x 215→1020, and `app-2` sits inside that span, so any approach from above must pierce it once. Why `app-3` cannot do better: coming down the right margin is free (x>1020 is outside every lane), but returning leftward to `app-3` is blocked at *every* useful altitude — y∈(240,360) by the `postgres` box itself, y∈(360,420) by the replicate link, y∈(420,500) by the `replica` box. The only slits are y<240 (which then drops through two lanes) or y>500 (which arrives at `app-3`'s lower right and collides with the producer band).

So: **the topology in the prompt has a crossing-number floor of 2, and it is reached by an ordinary elbow router.** That is a much sharper result than "hand-routing is hard" — it means Kashika's zero-crossing promise is not merely inconvenient for this diagram, it is *false* for this diagram, and the tool's first duty is to say so honestly (which is what `allow=cross` does, badly — see B4.4a) rather than to imply the author is at fault.

It also means the prompt's A1 is, as written, unachievable. That is worth fixing in the packet before you collect more reviews, because every reviewer will burn their budget re-deriving this (I spent 25 of my 40 minutes on it) and the differences between their answers will be about *where* they put the unavoidable crossings, not about the design.

### A1.4 — the one thing that would have changed the outcome

`kashika inspect --corridors`: the free horizontal y-intervals and vertical x-intervals in a given band, with their extents. Even better, `kashika plan lb.right app3.tr` would have returned the row above in milliseconds — the search is ~110k elbow candidates over geometry the tool already has. I spent ~25 of my ~40 minutes discovering, by iterated failure, that the band above my app row had exactly three altitude slots. Every iteration was: guess numbers → run checker → read two crossings → adjust. That is a compiler in a debug loop over a problem the compiler already has all the inputs for.

---

## A2 — move the load balancer 200px right

`box lb ... at (60,20)` → `at (260,20)`.

**Lines I had to change: 4** (the `box` line plus three `line` statements, one number each):

| # | line | change |
|---|---|---|
| 1 | `box lb "load balancer" at (60,20) ...` | `at (260,20)` |
| 2 | `line lb.left -> h -30 -> v 245 -> h 110 -> app1.left@0.25` | `h -30` → `h -230` |
| 3 | `line lb.bottom@0.8 -> v 20 -> h 134 -> v 210 -> h 35 -> app2.left` | `h 134` → `h -66` |
| 4 | `line lb.right -> h 900 -> v 210 -> h -390 -> v 20 -> app3.tr` | `h 900` → `h 700` |

**And one line changed shape without me touching it** — which is worse than the four edits:

`line cdn.right -> h 20 -> v 90 -> lb.tr`: `lb.tr` moved from `(290,140)` to `(490,140)`. The statement text is identical; the final segment went from a 210px leftward run through empty space to a 10px stub. `check` reports nothing. `fmt` reports nothing. Only a human looking at the PNG would call it a regression.

**Honest reading of this result.** 4 edits for a 200px move sounds tolerable only because I got lucky that all three LB lines are `h`/`v` + number. Look at what the numbers *are*: `-230`, `-66`, `700`. Every one exists solely to align a corner with an anchor whose absolute x I had to recompute. The anchors did *not* protect me: `app1.left@0.25`, `app2.left`, `app3.tr` all still moved in absolute space as far as my authoring effort was concerned, because the final hop of a `line` is a straight segment, not an orthogonal one (see A3.1 #2). If `->|` existed, the answer to A2 would be **one line**.

**Secondary A2 observation:** moving the LB made two previously-fine lines `through` a box and three `cross`. `check` caught 5 of the 6 consequences. The one it missed (the `cdn → lb.tr` stub) is the one a human reviewer would actually complain about. That's the shape of the whole reliability problem: the lints cover what is computable (intersections) and miss what is perceptual (does this look like it was drawn on purpose).

---

## A3 — hesitations, missing syntax, ambiguities, arithmetic

### A3.1 — where I hesitated, guessed, or wasn't sure what the spec meant

Ordered by how much damage the ambiguity did to me.

**1. Anchors are not usable as orthogonal destinations.** Spec: *"The final element must be a point (absolute or anchor)"* and *"`h <dx>` — horizontal move by dx (relative to previous point)"*. So `-> v 245 -> app1.left@0.25` produces a vertical move and then a **straight diagonal** to the anchor. Every one of my 34 anchor references was, in effect, an instruction to compute the anchor's absolute coordinates and hand-write the delta that lands on it. That is the opposite of what `db.left` is for, and it is why §A2 is 4 edits rather than 1. For an agent — which is the stated primary author — this single omission converts the anchor system from a time-saver into a documentation feature.

**2. Are absolute coordinates inside a group relative to the group?** Spec: *"Inside a `group`, child coordinates are relative to the group's top-left corner."* It says *child coordinates*; it does not say whether `(x,y)` inside a `line` that lives in a group is also offset. I assumed yes (and so does my checker). An agent that assumes no will produce a file that is silently 140px,190px wrong in my diagram. **This needs one sentence and it changes every file.**

**3. Group reference paths.** *"Shapes inside groups are referenced by dotted path: `vpc.worker.right`. Inside a group's block, siblings may be referenced by bare id."* May I write `vpc.k8s.app1.top` from a line inside `k8s`? Does a bare id resolve lexically outward, and out to what depth? I used bare ids only, and one dotted path (`pg`, referenced from a line at diagram scope but living inside `vpc` — I wrote `pg.left` and asserted that it resolves from the root because `pg` is unique case-insensitively; the spec does not grant me that).

**4. Attribute vs path ordering in `line`.** The grammar ends `... -> <point> ["<label>"] [attrs...]`, and one example is `line vpc.worker.right -> curve bend=30 -> db.bottom  end=arrow  label@0.3="async"` — label after attrs. My file has `end=arrow "publish"` (label last). I guessed any order works. If it doesn't, half of agents' first attempts are parse errors.

**5. `curve bend=N` sign.** *"N is the perpendicular bulge in px at the midpoint, positive = to the left of the direction of travel."* In a y-down coordinate system "left of travel" is exactly the phrase that flips between implementers (rotate the direction by +90° or −90°). My `retry` line's correctness (does the arc clear `worker-1` and the `consume` return lane?) depends on which way it bulges. I picked one and could not verify it. See B3.4.

**6. `curve bend` is not a segment, it's a modifier on the next point.** The table lists it as a segment but the prose says "to the next point", so `-> curve bend=20 -> (585,720)` consumes the following element. That means `curve bend` cannot be the final element, which is consistent with "the final element must be a point" only by inference.

**7. What counts as a crossing?** Two lines that share an anchor (both leaving `kafka.bottom`) start out collinear for at least one segment. Is that `shared-segment`, `cross`, both, or neither? I avoided shared anchors entirely by hand — which cost me design freedom for a rule I couldn't predict. Same question for two lines that meet exactly at a shared endpoint (my `lb.left`-style feeders).

**8. Edge-coincident segments.** My first draft had `cdn.right -> lb.top` where `lb.top` sat exactly below `cdn.right`, and the segment ran 30px *along* `cdn`'s own right border. Spec's `through` is *"a line passes through a box that is not its start or end"* — so a compliant checker stays silent about a line sliding down its own box's edge. Visually it reads as a doubled stroke. Nobody owns this case.

**9. Label placement: midpoint of what?** *"`"<label>"` — label at path midpoint"*. By arc length, or by vertex count, or by bounding box? On my `lb.right -> ... -> app3.tr` line (5 segments, one 900px long) these three answers put the label in three different rooms, one of which is inside the `postgres` box.

**10. Label metrics.** `text ... at (x,y)` says y is *"the baseline"*, and `box` labels are *"centered"*. Without a specified font, font size → px-per-char ratio, and padding, nobody can predict a label bbox. Meanwhile `label-overflow` is an **error** and `label-collision` a warning. So the checks depend on a constant the author cannot see. (B2's overflow example even cites *"padding 8"* — a value that appears nowhere in the language spec.)

**11. Non-rectangular anchors.** *"For non-rectangular shapes (ellipse, diamond, cylinder…), edge anchors are projected onto the actual outline."* Projected how? `diamond.top@0.25`: ray-from-centre through the bbox point, or vertical drop from the bbox point to the outline? These differ by tens of px on a diamond. And `cylinder`, `doc`, `person`, `cloud` have **no defined outline at all** — the spec never says what the geometry of a `cloud` is. Two agents will produce different resolved geometry and therefore different crossing counts for the same source file. Since `check` is supposed to be the source of truth, an unspecified shape geometry undermines the checker.

**12. `grid` vs `@t` are mutually hostile.** Spec: `grid` warns on *"coordinate not a multiple of `grid`"*, and `@t` means *"point at fraction t (0..1) along that edge"*. `lb.bottom@0.8` on a 170px-wide box resolves to x = 256 — off-grid by construction. My file produces three such coordinates (`256`, `249.6`, `290.4`) purely as a consequence of using the anchor features the language advertises. Either `grid` must ignore resolved anchor positions, or the author must abandon `@t`, or everyone lives at exit code 1 forever.

**13. `style` override semantics.** *"A user `style` with the same name overrides the built-in."* Wholesale (drop the built-in's other keys) or per key? If wholesale, redefining `service` to change only `fill` silently loses `stroke`. This is the difference between a two-word edit and a broken diagram, and I couldn't tell which I was buying.

**14. Spill/touching.** Is a child whose edge lands exactly on the group's edge (0px overhang) a `spill`? I moved `kafka`'s bottom off the group boundary rather than find out. "Extends outside" is clear; "equals" is not.

**15. `endpoint-off-edge` has no threshold.** *"absolute endpoint near but not on a shape edge"* — near is what? 1px, 4px, the arrowhead length? This code fires on the single most common agent error (hand-typed absolute endpoint 3px off the anchor) and its precision is undefined.

**16. Does `allow=` suppress or hide?** If I `allow=cross` a line, does `check` still tell me the crossing exists in any form? For a review workflow this matters enormously (see B4.4).

**17. `render` refuses to write output on errors.** A single `label-overflow` error means no image at all. In an iterate-and-look loop, I'd rather have a deliberately ugly image than no image. There's no `--force`.

**18. Ordering: is the LB-in-VPC allowed to be drawn behind app boxes?** *"Drawing order = file order; a group is drawn behind its children."* Fine — but there is no `z=` or `behind=` for the case where two non-nested boxes must layer (my `s3` overlapping nothing, but a legend would).

### A3.2 — syntax I instinctively reached for that isn't there

Ranked by how often I tried to type it.

1. **`->|-` / `-|->` (TikZ)** — "go horizontally then vertically to that point". I wanted this ~15 times. This is the single missing primitive.
2. **`h to db.left` / `v to db.top`** — a directional move that stops at an anchor's coordinate on one axis only. Cheaper to implement than `->|-` and covers 80% of the same need.
3. **Junctions / buses / merges.** "all three app pods → kafka" as one edge with three origins (or a `junction j1` I can draw lines into). Without it, `shared-segment` penalises the honest drawing.
4. **Relative placement** — `at db.tr+(40,0)` as a *placement* (the spec has anchor *offsets* for line endpoints but not for box positions). I tried this twice before remembering I had to type numbers.
5. **Named coordinates / guides** — TikZ `\coordinate`, D2 `near`. I wanted to reserve "the corridor at x=400" once and refer to it five times.
6. **Expressions** — `h -230`, `at (480+170, 50)`. Every arithmetic step I did ended up in the file as a literal.
7. **`anchor=west` / compass names** — I typed `app1.west` twice before remembering `left`.
8. **Horizontal/vertical alignment helpers** — `align-x app1, app2, app3` on a group (D2/Graphviz `rank`-ish). I wanted the three app tops at one y and had to keep it consistent by hand.
9. **Port rotation / per-side attachment ordering** — SVG/block-diagram style `db.left@auto[2/3]`.
10. **Sub-path arrows** — `end=arrow` at t=0.5 on a segment, for retry indicators. My `retry` line is a whole extra line because arrows only terminate paths.
11. **`clip` on groups** and rounded boundary corners — cosmetic but everyone expects it.
12. **`include`/`import`** for a shared `style` file across diagrams.
13. **A legend primitive.** I faked it with `text`; a `legend` block would be used in most real design diagrams.
14. **Raw SVG path escape hatch** for shapes outside the nine (a lock, a lambda, a browser chrome). Without it, agents will draw the missing glyph as a `box` with an emoji label.
15. **`fmt`-stable comments** — I wanted to keep `# --- section ---` separators and have `fmt` leave the blank lines alone.
16. **Group-relative arithmetic** — `v -30` inside `k8s` at 190px depth: is the delta relative? (yes) but I wanted `v -30` *to the group's top edge*, i.e. `v to group.top`.
17. **Multi-line statements.** `line app1.top@0.1 -> v -150 -> h 805 -> pg.left@0.08` is 48 chars; my longest is 76 and unreadable. See B3.8.

### A3.3 — ambiguities that would make two agents produce different files for the same intent

I'll flag the ones where the *files differ substantially*, not just cosmetically. ⚑ = I formed this before reading B3.

1. ⚑ **Relative-vs-absolute coordinates inside groups** (A3.1 #2). Whole-file offset error.
2. ⚑ **The final-hop rule** (A3.1 #1). One agent writes diagonals everywhere and doesn't notice; the other computes 34 absolute coordinates. Two completely different files, both "clean" (well, one gets an `endpoint-off-edge` storm if it's careless, which is at least a signal).
3. ⚑ **Directional convention for `h`/`v`.** Does `h 900` mean "move 900px right" or "go to x=900"? The spec says relative, but the token reads like a coordinate and I caught myself writing `h 900` for the second meaning at least twice. The `-` sign disambiguates nothing.
4. ⚑ **`curve bend` sign** (A3.1 #5), especially with y-down.
5. ⚑ **Shape outlines** for `cylinder`/`doc`/`person`/`cloud` (A3.1 #11). Different resolved geometry ⇒ different lint output for the same source.
6. ⚑ **Label midpoint semantics** (A3.1 #9) ⇒ different collisions.
7. ⚑ **`style` override: wholesale or per-key** (A3.1 #13).
8. **`#` disambiguation.** `fill=#e8f0fe` and `# comment` coexist. Is `fill=#e8f0fe # mid-blue` legal? Is `color=#888#888` two colors? Two agents will pick different hedging strategies (one quotes colors, one avoids comments, one writes `fill=rgb(...)` which doesn't exist).
9. **`dot`**: *"`dash` or `dot` — stroke pattern"* vs *"`start=`/`end=` — arrowheads: `arrow`, `open`, `diamond`, `dot`"*. A bare `dot` token can only be a stroke flag by position, so it parses — but two agents will each be sure the other is wrong, and one of them will reach for `end=dot dot` and get an unreadable line.
10. **Whether two lines may share an anchor** (A3.1 #7). One agent writes a neat fan from `kafka.bottom`, the other hand-offsets every endpoint by 5px to dodge `shared-segment`. Files are visibly different.
11. **`allow=` scope and visibility.** One agent allows 12 crossings globally, another allows 0 and accepts 3 crossings in the file. "Same intent, different files" is literally true, and `check` output can't distinguish them.
12. **Group label placement** — *"drawn at the group's top-left inside edge"*: baseline? inset padding? If the label is 14px tall and the inset is 4px, is a child at rel `(0,0)` under the label a collision? No code for it, so every agent guesses, and the PNGs differ.

### A3.4 — where the coordinate arithmetic hurt, and what would fix it

**Where it hurt, in order:**

1. **Choosing corridor x's in the gaps between boxes** (`350..425`, `575..650`). Requires knowing each box's absolute right/left, and the gaps are 75px — so the corridor and the *two* neighbour lines that also want that gap have to be mentally reserved. This was the single most expensive activity.
2. **Making a vertical land exactly on an anchor's y** (all 32 `h`/`v` segments). Pure accumulation of group offsets: `vpc(60,120) + k8s(140,190) + child(y)`. I got it wrong on the first attempt for every one of `app2`, `app3`, `worker*`, and had to recompute.
3. **Detecting collinear overlaps between corridors** — two verticals at the same x with overlapping y spans. That's invisible in arithmetic and only the checker found it.
4. **Picking `@t` fractions that are simultaneously (a) aligned with a source box's centre, (b) grid-friendly, (c) not already used.** `kafka.top@0.125`, `@0.875`, `@0.675` are what falls out; `@0.675` is 200+0.675×600 = 605 because that's where `worker-2`'s centre is. A human would have moved `worker-2` instead.
5. **Realising the last hop is diagonal.** I wrote `-> v 245 -> app1.left@0.25` and believed it was orthogonal for three iterations.

**What would reduce it, most-impactful first:**

* **`->|-` / `-|->` / `h to <anchor>`** — removes (2) and half of (1).
* **Relative box placement and named guides** — removes most of (1) and (4).
* **`inspect --corridors`** — turns (1) from search into selection.
* **`check --metrics`** (segment count, total wire length, crossings) — lets an agent A/B two layouts numerically instead of by opinion.
* **`fmt --align=columns`** — I hand-aligned the `at`/`size` columns; at 17 lines that's fine, at 200 it's the difference between a reviewable file and a wall.
* **Arithmetic in literals** (`at (480+170,50)`) — cheap, and kills a whole class of transcription errors.
* **`kashika inspect --gaps`** — literally "the free intervals are `[350,425]`, `[575,650]`", so the corridor decision becomes a lookup.

---

# Part B — tooling and open questions

## B1 — CLI ergonomics

### Essential
* **`render` runs everything** and one command is right. But `render` must **not** refuse to write on errors by default, or you remove the agent's ability to look. Add `--force` / `--emit-on-error`; an ugly PNG with the overflow visible is worth more than no PNG.
* **`inspect`** is the most valuable item in the entire document. It's what I had to write for myself. Make it the flagship.
* **`ref` built into the binary** — correct decision. Docs rot; a 0.3.0 binary with 0.2.0 web docs is the normal state of the world. See B4.5 for size/contents.
* **`-` on stdin** — yes, and it should work for `inspect` too (I wanted to check a snippet without a tempfile).
* **Deterministic, stably-ordered output; ANSI only on TTY; capped** — all correct and cheap.
* **`fmt`** — essential once files pass ~80 lines, but only if it is *stable* (idempotent) and only if it does not rewrite attribute order in a way that invalidates the `suggestions` spans from the same run. Those two features fight; see B2.

### Nice-to-have
* **`render --debug`** — genuinely useful for multimodal agents, useless for me (I can't see PNGs in this harness unless the file is handed back to me). Keep it, don't prioritise it.
* **`explain <code>`** — near-free to build, and it's how a new agent learns the difference between `cross` and `shared-segment` without burning context on the whole ref.
* **`fix`** — see below; the idea is right, the risk is high.

### Unnecessary / risky as specified
* **`fix`** is the only item I'd push back on. "Applies only machine-applicable suggestions" is a good principle, but growing a box silently changes the geometry of *every line touching that box* (my A2 experiment: moving one box produced 5 new lints). `fix` must therefore either (a) be `--dry-run` by default, printing a diff, or (b) run check both before and after and refuse to write if the warning count increases. I would not let `fix` touch a file I authored until it can show me a diff. Also: `fix` must run *after* `fmt` (or `fmt` after `fix`) deterministically, otherwise `fix && fmt --check` fails.
* **`fmt --check`** — fine, but nothing in B1 says `fmt` preserves comments, blank lines and the author's section separators. Given that `fmt` is described as "column alignment, attribute order", I'd expect it to normalise blank lines too, and that would destroy the readability of a 200-line file. Specify it.

### Missing — what I'd actually want as an agent

1. **`kashika inspect --corridors [--band y1..y2]`** — free horizontal/vertical intervals with extents. *The* missing feature for me.
2. **`kashika plan <src-anchor> <dst-anchor> [--crossings 0]`** — 2–4 ranked orthogonal candidate paths with segment count, estimated crossings and the corridor each uses. Explicitly *not* a layout engine: it proposes, the author disposes. This is the honest reading of B2's open question — don't put reroutes in `check` hints, put them behind a command the author chooses to invoke.
3. **`kashika check --metrics`** — crossings, total wire length, per-group extents. So I can compare two candidate layouts numerically.
4. **`kashika diff <a> <b> --geometry`** — "3 lines changed shape, 2 got longer, 1 new near-miss". This is A2 automated. It's also the only way to notice the `cdn -> lb.tr` stub that no lint sees.
5. **`ref --json`** (grammar + tables machine-readable) and **`ref <statement>`** (one statement's full grammar). Prose costs tokens; I'd rather parse the grammar.
6. **`check --fail-on=error|warn|none` / `--exit-warnings-as=0`.** Exit code 1 for warnings means any file with one `off-grid` is permanently "failing" in a CI loop, and agents work around that by adding `allow=`, which is how suppression rots.
7. **`check --only=<code>` and `--count`** — for iterating on one lint without drowning.
8. **A bundled, documented font-metric table** (`kashika ref metrics`: px-per-char at each `font-size`, plus `padding`). Without it I cannot predict `label-overflow` myself and I'm fully dependent on the error message. With it, I can place labels correctly first try.
9. **`check --assert clean`** — a hard mode where warnings are errors, so a repo can gate "no crossings ever" in CI. Related: always print `N suppressed` counts (see B4.4).
10. **A deterministic `--seed`-free renderer.** Two runs of the same file on two machines must produce byte-identical SVG, or the "agent writes, human reviews the diff" workflow collapses. State this as a guarantee.

## B2 — are the diagnostics actionable enough for a one-edit fix?

**Mostly yes for `through`; not yet for `cross`; and the `suggestions` mechanism is under-designed.**

The `through` example is close to ideal: it names the segment, gives the box's y-span, and offers a corridor. The two `help` lines are exactly the right shape of information. What's missing is one line: *which of my segments is cheapest to move, given everything else already routed there.* The hint "route above y<110 or below y>170" is the obstacle's extent; the useful object is the **complement of the union of all obstacles and existing routes** in that band. Give me the free intervals, not the blocked one.

For `cross`, the example hint is weaker: *"line#5 could pass left of x=300 or right of x=330 (segment 1 spans x 300..330)"*. That tells me where line#5's segment is. What I need to know is whether moving it there collides with the four other lines in that band — which `check` already knows and doesn't say. Concretely, add:

```json
"facts": {
  "at": [312,140],
  "freeCorridors": {"vertical": [[null,290],[344,600],[640,null]],
                    "horizontal": [[null,120],[186,300]]},
  "crossingCount": {"before": 3, "ifMovedToLeft": 5, "ifMovedToRight": 1}
}
```

That last field is the one that converts "here are two options" into "here is the better option", and it's a pure by-product of a check pass.

Other gaps:

* **`suggestions` need a `result` or a `--dry-run`.** For `label-overflow`, `replacement: "size (188,60)"` is machine-applicable but will grow `db` into two neighbours' right edges. The suggestion should carry `"ripple": ["line#7", "line#9"]` or the tool should refuse to mark it `machine` when the ripple is non-empty.
* **No junction concept, so no way to say "this intersection is intended".** Today the only escape is `allow=cross`, which is line-scoped and permanent. A `junction (312,140)` statement or `end=arrow join` would let the author express intent, and would let the checker distinguish a defect from a decision. This is my #4 overall change.
* **`cross` vs touch.** Two lines sharing an endpoint are not crossing, but it's not stated, and my checker (and I suspect most implementations) will flag them at t=0/t=1.
* **No `count` in the diagnostic set.** Give me a header: `3 crossings, 2 shared segments, 1 through` so I can see whether my edit helped or hurt.
* **`spans` need a stable contract**: 1-based line, 1-based column, byte or char, and — crucially — computed against the **formatted** file, since `fmt` is in the same tool. If `fix`'s spans are computed pre-`fmt` and applied post-`fmt`, the edits land in the wrong place. Say which.
* **Curves.** `cross`/`through` for a Bézier: sample, or solve? If sample, at what tolerance, and is it deterministic? Two implementations will disagree at tangency, and tangency is exactly what my `retry` arc produces.

**On B2's open question (how far should hints go).** Corridors only is the right call — but not the version in the packet. "The obstacle's extent" is a corridor; "the free space with a crossing delta for each option" is a *decision*. Implement the latter, keep it out of `check`'s default output (it's expensive), and expose it as `kashika plan`. Full reroute proposals inside `check` would fight authorial intent exactly as feared, and worse: they'd make the tool's suggestion the default, which is how you get 200 diagrams that all look like the router drew them.

## B3 — answers to the designers' questions

1. **`#` comments.** Keep `#`; don't switch to `//`. Hex colours are lexable in context (`=` followed by `#` + 3/6/8 hex digits) and the fix costs a churn of every existing example. But *specify* it: "`#` begins a comment when it is not immediately preceded by `=`" and decide `fill=#e8f0fe # comment`. One sentence, zero migration.
2. **`dot` overloading.** Yes, unify: `stroke-style=dashed|dotted` (or `line-style=`), keeping `end=dot` for arrowheads. One token meaning two things in the same statement is a trap for exactly the author you're targeting — a code generator that never gets to look at the picture.
3. **Line labels.** Yes, unify on `label=` with an optional `@t` (`label@0.3="..."` / `label="..."`). Two spellings for one concept is two chances to be wrong, and `fmt` will have to canonicalise it anyway.
4. **`curve bend=N` sign.** Not intuitive — `bend=left:30` is better, and `bend=up:30` / `bend=down:30` is better *still* for orthogonal drawings because it's screen-absolute. "Left of travel" is direction-dependent (redraw the line bottom-up and the bulge flips) and coordinate-system-dependent (y-down). Ship `bend=left|right|up|down:<px>` and keep a signed shorthand as an alias if you must, but document the sign as *screen* left, i.e. −x, not mathematical left.
5. **`tl` vs `top-left`.** Accept both, canonicalise to the short form in `fmt`. Cost: a few lines of lexer. Benefit: agents' prior distributions differ and you stop burning an error message on it.
6. **Warn on a diagonal tail after `h`/`v`?** ⚑ Yes — but *don't stop there*: the diagonal tail exists because the language can't express an orthogonal tail. Add the operator (`->|-`), make the diagonal a lint you can allow (`diagonal-tail`), and then the lint is teaching rather than nagging.
7. **Relative box placement.** ⚑ Yes, and it's my #3 overall change. Cut the arithmetic and keep the relationships. Two conditions: cycles must be a hard error with the cycle path printed, and `inspect` should print the dependency graph so a reader can see why a box moved. Also extend it to `guide` lines so relationships can be to *corridors*, not just boxes.
8. **One statement per line.** Fine, *with* an explicit continuation token (trailing `\` or a leading `...`) and a `fmt` rule for when to wrap. My longest line is 76 chars and unreadable; a 3-way fan that needs 5 segments each gets to ~110. Don't allow implicit continuation — it makes span reporting and `diff` semantics ambiguous, which matters much more than line length.

## B4 — review questions

### 1. B1 essentials, luxuries, and what's missing

See B1 above. Summary: **essential** = `render`-does-all, `inspect`, `ref`-in-binary, stdin, deterministic output; **essential-but-fix** = `render` must not withhold output on errors, `fix` must default to a diff, exit-1-on-warnings needs an opt-out; **nice** = `--debug`, `explain`; **unnecessary as specified** = nothing I'd delete outright, but `fmt --check` needs a stated comment/blank-line policy. **Missing, in priority order**: `inspect --corridors`, `plan` (routing oracle), `check --metrics`, `diff --geometry`, `ref --json`, a bundled font-metric table, and `count suppressed` in every summary line.

### 2. Are B2 diagnostics actionable in one edit?

For `through`: yes, one edit, if you replace "the obstacle's extent" with "the free intervals" in the hint. For `cross`: no — it needs the crossing delta per option, and it needs the "is this intentional?" escape (junction) so that the one-edit fix isn't always `allow=cross`. For `label-overflow`: the suggestion is machine-applicable but unsafe; add a ripple list. And `suggestions[].span` must be defined against the post-`fmt` file or `fix` will corrupt.

### 3. My answers to B3

Each with a one-line justification:

1. **`#`** — keep it; make the lexer rule explicit. *Local, solvable, not worth a breaking churn.*
2. **`dot`** — rename to `stroke-style=dashed|dotted`. *One token must not mean two things in one statement.*
3. **Labels** — unify to `label=` + optional `@t`. *Two spellings = two bugs, and `fmt` must pick one anyway.*
4. **`bend`** — `bend=left|right|up|down:<px>`. *"Left of travel" is both direction- and axis-convention-dependent.*
5. **Corners** — accept both, canonicalise to `tl`. *Trivial lexer cost, removes a class of first-try errors.*
6. **Diagonal tail lint** — yes, *and* add `->|-` so the diagonal is a choice rather than the only option. *A lint that punishes a missing feature is user-hostile.*
7. **Relative placement** — yes, with cycle errors and a printed dependency graph. *It's the only way relationships survive moves; it's why A2 is 4 edits.*
8. **One statement per line** — yes, plus an explicit continuation token; let `fmt` wrap. *Implicit continuation breaks span/diff semantics, which matters more than line length.*

### 4. What would most likely make me produce a wrong-looking diagram while `check` says clean?

Five concrete mechanisms, in order of how likely they are to bite:

**(a) `allow=cross` rot, and the fact that suppressed diagnostics cannot be seen.** I have two `allow=cross` in my A1. When someone later moves the LB, new crossings will appear on those lines and *nothing will report them*. A file can drift from 2 intentional crossings to 9 accidental ones while every run says "0 warnings". **Fix: every summary line must read `0 errors, 0 warnings (2 suppressed)`, and `check --strict` should fail on any suppressed diagnostic that doesn't appear in a `known` list with a justification string.**

**(b) A line sliding along its own box's edge.** My first draft's `cdn.right -> lb.top` ran 30px down `cdn`'s right border because `lb.top` happened to be directly below `cdn.right`. `through` explicitly excludes the line's start/end box, so **a compliant checker stays silent**. It renders as a doubled stroke — immediately obvious to a human, invisible to the tool. (Related: an arrowhead landing on a corner, as my `app3.tr` connection does, reads as attached to two edges at once.)

**(c) Correct geometry, wrong semantics.** My diagram is `check`-clean while `lb → app3.tr` visually attaches the load balancer to `app3`'s top-right corner, where `app3`'s *producer* also attaches. A reader may reasonably conclude the LB feeds kafka. Nothing in `check` knows what an arrow means. Same class: swapped line labels ("publish" on the consume line), an arrow pointing backwards (`db.left -> api.right end=arrow` renders a rightward-pointing arrowhead at `api.right` — no lint cares), a shape whose `stroke` equals the background colour (invisible box, no contrast lint), `dash` used for a fast path and solid for a slow one (nothing enforces conventions), two boxes touching with 0px gap (`overlap` needs a positive area, so a merged-looking pair is clean).

**(d) Off-grid/fractional drift.** `@t` fractions produce coordinates like 249.6. If the author's grid is 1 (no grid flag), nothing complains, and two days later a different agent's `fmt` pass changes nothing but the diagram looks subtly unsnapped. Also `text` baselines and label midpoints are unspecified, so the labels drift while the boxes stay put.

**(e) The diagonal tail.** After a chain of `h`/`v` deltas, the final segment to an anchor is a straight line. It can be a 3° slant that nobody would call wrong — until it isn't, and the line cuts a corner of a box that isn't its start/end... at which point `through` *does* fire. So the failure mode is: mostly invisible, occasionally catastrophic, and in both cases un-diagnosable from `check` alone. My `lb.right -> ... -> app3.tr` line ends with a 20px vertical so it's fine; two of my earlier drafts ended with ~200px diagonals that `check` passed and that looked obviously wrong.

The meta-point: `check` currently proves *"no obvious geometric defects"*, and every one of the five above is a defect that is geometric only in a sense the lints don't encode. The tool should say that out loud in `ref`, so an agent doesn't over-trust a green run.

### 5. What should `kashika ref` contain, and how long?

**Target: ~800 tokens for the default page (≈110–130 lines), and ≤1.5k tokens per topic.** If it's longer, agents will skip it and hallucinate syntax, which is worse than not shipping it. Tables only, no prose. Suggested contents of the default page, in this order:

1. **The five non-obvious rules** (10 lines): coordinates are y-down; group children are relative (including points inside `line`); anchors resolve to absolute geometry; the final element of a `line` is a straight segment, **not** an orthogonal one; `allow=` hides a lint.
2. **Compressed EBNF** for `diagram`, `style`, `box`, `group`, `text`, `line` (15 lines).
3. **Anchor table** with the `@t` direction conventions, plus the corner names (10 lines).
4. **Segment table**, including that `curve bend` consumes the next point (6 lines).
5. **Attribute table** with defaults for `line`, `box`, `style` (15 lines).
6. **The nine lint codes**, one line each: trigger + how to silence (10 lines).
7. **A complete worked example**, 12–15 lines, that includes a group, an anchor, a `curve`, and an `allow` — so I have a copy-paste skeleton.
8. **The five routing idioms** for orthogonal paths in a y-down space (the "staircase", the "over-the-top wrap", the "gap corridor", the "outside margin", the "nested ladder"). This is the section I'd have paid for.
9. **Font metrics**: px-per-char per `font-size`, and the label padding constant. Without this, `label-overflow` is unpredictable and every agent guesses.
10. **What is NOT supported** — no auto-layout, no text wrap, no bus/junction, no relative box placement, no rotation, no raw SVG. Explicit absence is the cheapest defence against hallucinated syntax.
11. **Exit codes and the `allow=`/suppressed-count behaviour.**

Then topics: `anchors`, `paths`, `styles`, `lints`, `inspect`, `errors` — each ≤1.5k tokens, each independently useful. And `ref --json` for agents that would rather parse than read; I would use that by default and only read the prose when a diagnostic's `help` pointed me at a topic.

### 6. Top 5 changes to the whole design, by impact

1. **Orthogonal-to-point segments** (`->|-`, `-|->`, and `h to <anchor>` / `v to <anchor>`). *Without it an anchor is not a destination, only a lookup table, so every relationship still costs absolute arithmetic — which is why A2 is 4 edits and why the "keep relationships intact when things move" premise doesn't hold today. It also removes most diagonal tails and therefore most `endpoint-off-edge` noise.*
2. **A routing oracle behind an explicit command** (`kashika plan` + `inspect --corridors`), not in `check`'s hints. *I burned ~25 of 40 minutes discovering the free corridors by iterated failure. This keeps the author in control while removing the constraint-solving tax — and it's the difference between "agents can use this" and "agents can use this on diagrams with fewer than 3 shared destinations".*
3. **Relative placement + named guides/corridors.** *Turns A2 into one edit, lets an author reserve a channel once and reference it five times, and is the substrate that makes #1 and #2 composable.*
4. **Make intersections first-class and auditable**: a `junction`/`merge` statement, `allow=` with a mandatory justification, and a `(N suppressed)` count in every summary. *Otherwise the escape hatch rots and "clean" stops meaning anything — see B4.4(a). This is the highest-severity correctness hole in the current design, because it silently degrades over time.*
5. **Deterministic, documented text metrics** (a bundled font-metric table, `padding` as a real style key, `inspect` reporting label bboxes and the space available for a label). *`label-overflow` is an error and `label-collision` a warning, and today neither is predictable by the author. That is the one check an agent most needs to trust before it commits a file.*

Honourable mentions that would be #6–8: exit-1-on-warnings needs a `--fail-on` escape (otherwise `allow=` becomes the universal workaround); `render` should write output on errors unless asked not to; and specify shape outlines for `cylinder`/`doc`/`person`/`cloud`, or the checker's answers depend on the renderer's mood (A3.1 #11).

---

## Appendix A — numbers from my attempt

| metric | value |
|---|---|
| boxes (incl. 2 groups) | 14 |
| line statements | 17 |
| resolved segments | 76 |
| anchor references | 34 |
| `h`/`v` segments | 32 |
| layout iterations to reach 2 crossings | ~10 (from 6 → 4 → 3 → 2) |
| **verified minimum crossings for this topology** | **2** (brute-forced, §A1.3) |
| wall-clock, A1 only | ~40 min |
| of which spent discovering free corridors | ~25 min |
| throwaway Python checker written to compensate | ~230 lines |
| coordinates made off-grid *by* anchor fractions/offsets (`grid=5`) | 3 (`256`, `249.6`, `290.4`) |
| lints my checker found that `check` would have had to explain | 34 (across all iterations) |

The last row is the ROI argument for the whole document: I generated 34 defects and fixed 34, and every one of them would have been a `check` diagnostic in a working implementation — but only 2 of the 34 were things I could see without a machine.

## Appendix B — the throwaway checker (why it should ship)

`/tmp/ksh/check.py`, ~230 lines: group offset stack, `@t` anchor resolution with offsets, `h`/`v`/`curve bend` (quadratic, sampled 28×), segment–segment intersection, segment–box entry/exit, box nesting and overlap, out-of-canvas, spill. I wrote it in one pass and it found six defects I had asserted were absent.

Two observations for the design:

1. I wrote a *worse* version of `check` in one pass because the language is simple enough that a parser is a conceit. Any agent will do this. The defence is not to make the language harder to parse — it's to make `inspect` and `check` so much better than a home-grown version that writing one is not worth it. `--corridors`, `--metrics` and crossing deltas are what make that true.
2. My checker disagrees with the spec in at least one place (`through` on the line's own start/end box), and I only noticed because I wrote the check before re-reading §7. Two implementations of this language will disagree about endpoints, tangency, "near", and midpoint labels. **Every one of those words needs a number in the spec**, or `check`'s verdict will depend on the version of `check`.
