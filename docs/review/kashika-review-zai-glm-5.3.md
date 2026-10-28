# Kashika Design Review — Z.ai GLM-5.3

Review of `docs/review/kashika-review-packet.md`. Per the packet instructions, I did Tasks A1–A3 before reading Part B; friction notes below were recorded as I hit them, not retro-fitted. All answers are from my perspective as an agent that would have to author and maintain `.ksh` files without seeing rendered output.

**Summary judgment up front.** The core premise is sound: for an agent, absolute placement is not a burden, and a geometric checker is a genuinely good substitute for eyes. The two things that hurt most in practice were (1) *lane planning* — assigning non-crossing corridors to a dozen orthogonal lines is the real cognitive load, not coordinate arithmetic — and (2) spec silence on how lines and groups interact (borders, along-edge runs, lines declared inside groups). Both are fixable in the spec and in diagnostics.

---

## A1 — The diagram

### Process notes (honest ones)

- I re-planned the routing **five times** before settling on this layout. Two full topology restarts (swapping kafka/db columns; re-routing the load-balancer fan) and three lane-assignment fixes. Every failure was a crossing I discovered while checking my own arithmetic by hand — exactly the class of error `check` exists for, which is good news for the tool and bad news for my unaided throughput.
- Deliberate conventions I adopted to survive: even box widths (so midpoint anchors land on grid), lanes on multiples of 10, "exit lanes" for the load balancer fan at staggered y, a dedicated vertical corridor between the k8s boundary (x=700) and the right column (x=760) with assigned x-lanes (720/730/750), and horizontal lanes below the app row and below the workers.
- A repeatedly recurring trap: lines that **run along** a box edge (e.g. approaching `rep.right` from directly below, or a horizontal at exactly a group's border y). I had to route around these; the spec never says whether they're legal.
- I kept all lines at top level because the spec says group children can be `line`, but never defines what coordinates mean for a line inside a group.

### `upload.ksh`

```
diagram "Upload pipeline" size (1260,940) grid=10

style svc { fill=#e8f0fe stroke=#1a73e8 }
style data { fill=#fef7e0 stroke=#f9ab00 }
style ext  { fill=#f1f3f4 stroke=#5f6368 }

box browser "Browser" at (40,80) size (120,60) style=ext
box cdn "CDN" at (240,80) size (120,60) style=ext
box lb "Load Balancer" at (460,80) size (160,60) style=svc

box s3 "S3" at (1080,600) size (120,60) style=ext

group vpc "VPC" at (40,220) size (1000,620) style=boundary {
  group k8s "k8s cluster" at (60,40) size (600,520) style=boundary {
    box app1 "App 1" at (20,60) size (160,60) style=svc
    box app2 "App 2" at (220,60) size (160,60) style=svc
    box app3 "App 3" at (420,60) size (160,60) style=svc
    box w1 "Worker 1" at (20,300) size (160,60) style=svc
    box w2 "Worker 2" at (220,300) size (160,60) style=svc
  }
  box db "Postgres" at (720,40) size (160,80) shape=cylinder style=data
  box kafka "Kafka" at (720,220) size (160,80) style=queue
  box rep "Replica" at (720,380) size (160,80) shape=cylinder style=data
}

line browser.right -> cdn.left "HTTPS" end=arrow
line cdn.right -> lb.left end=arrow

line lb.bottom-(90,0) -> v 30 -> h -250 -> app1.top end=arrow
line lb.bottom -> v 50 -> h -140 -> app2.top end=arrow
line lb.bottom+(60,0) -> app3.top end=arrow

line app1.bottom+(60,0) -> v 30 -> h 460 -> v -130 -> h 40 -> db.left@0.25 "SQL" end=arrow
line app2.bottom+(60,0) -> v 50 -> h 270 -> v -130 -> h 30 -> db.left@0.5 "SQL" end=arrow
line app3.bottom+(60,0) -> v 70 -> h 90 -> v -130 -> h 10 -> db.left@0.75 "SQL" end=arrow

line kafka.left@0.5 -> h -20 -> v 50 -> h -540 -> v 30 -> w1.top label@0.6="consume" end=arrow
line kafka.left@0.75 -> h -10 -> v 50 -> h -350 -> v 10 -> w2.top label@0.6="consume" end=arrow

line w1.right -> h 800 -> v 40 -> s3.left label@0.3="write" end=arrow
line w2.right-(0,10) -> h 660 -> v 20 -> s3.top label@0.3="write" end=arrow

line w2.right-(0,20) -> h 330 -> curve bend=25 -> kafka.bottom-(30,0) label@0.4="retry" end=open dash

line db.right+(20,0) -> h 280 -> v 400 -> h -380 -> v -20 -> rep.bottom label@0.5="replicate" end=arrow
```

### Resolved geometry (my own hand `inspect`, so my work can be checked)

Absolute positions: vpc (40,220)–(1040,840); k8s (100,260)–(700,780); app1 (120,320), app2 (320,320), app3 (520,320), each 160×60; w1 (120,560), w2 (320,560); db (760,260); kafka (760,440); rep (760,600); s3 (1080,600).

| line | resolved polyline |
|---|---|
| lb→app1 | (450,140)→(450,170)→(200,170)→(200,320) |
| lb→app2 | (540,140)→(540,190)→(400,190)→(400,320) |
| lb→app3 | (600,140)→(600,320) |
| app1→db | (260,380)→(260,410)→(720,410)→(720,280)→(760,280) |
| app2→db | (460,380)→(460,430)→(730,430)→(730,300)→(760,300) |
| app3→db | (660,380)→(660,450)→(750,450)→(750,320)→(760,320) |
| kafka→w1 | (760,480)→(740,480)→(740,530)→(200,530)→(200,560) |
| kafka→w2 | (760,500)→(750,500)→(750,550)→(400,550)→(400,560) |
| w1→s3 | (280,590)→(1080,590)→(1080,630) |
| w2→s3 | (480,580)→(1140,580)→(1140,600) |
| retry | (480,570)→(810,570)→curve→(810,520) |
| db→rep | (940,300)→(1220,300)→(1220,700)→(840,700)→(840,680) |

I verified pairwise that no two lines intersect and no line enters a non-endpoint box (including several 5–10px near-misses: kafka→w2's vertical at x=750 vs the retry curve's westward bulge to x≈785; w2→s3's lane at y=580 passing 10px above w1→s3's descent at x=1080). I would not trust these near-misses without running the real `check`.

---

## A2 — Move the load balancer 200px right

`box lb` moves to `at (660,80)`. Lines that had to change:

1. `box lb` — the move itself.
2. `line lb.bottom-(90,0) -> v 30 -> h -250 -> app1.top` — start resolves to (650,140) now; the horizontal jog must become `h -450`.
3. `line lb.bottom -> v 50 -> h -140 -> app2.top` — start is now (740,140); jog becomes `h -340`.
4. `line lb.bottom+(60,0) -> app3.top` — **restructured, not just re-numbered**. The old single vertical (600,140)→(600,320) now starts at (800,140), which is misaligned with app3.top (x=600). A naive fix (`-> v 180 -> h -200 -> app3.top`) crosses the app2 line's new descent at x=740. The working route is `-> v 70 -> h -200 -> v 110 -> app3.top`, and the jog must avoid both the app2 descent and running along the VPC's top border (y=220), so it lands at y=210.

Unchanged: `browser.right -> cdn.left` and `cdn.right -> lb.left` (both fully anchor-based — this is the design working as intended), and everything else.

**Observations.** Four lines touched, and the change was not mechanical: moving one box 200px forced re-planning the fan's lane structure, not just updating deltas, because the fan's exit lanes collide with each other in new ways. If my intermediate waypoints had been absolute `(x,y)` instead of relative `h/v`, more lines would have changed. Relative placement (B3.7) plus a "horizontal/vertical until aligned with anchor" segment (see A3.2) would reduce this to essentially the one `box lb` line.

---

## A3 — Friction report

### A3.1 Where I hesitated, guessed, or wasn't sure

1. **Signed `h`/`v`.** "`h <dx>` — horizontal move by dx (relative to previous point)". Every real route I wrote needs negative deltas (`h -250`). The worked example only shows positive values. I guessed negative is allowed. If it isn't, the language is unusable; if it is, say so.
2. **Is a group a "box" for `through`?** "`through` — a line passes through a box that is not its start or end." My lb fan crosses the VPC and k8s top borders; app→db lines cross the k8s right border; db→rep crosses the VPC right border. If group borders count, virtually every nested diagram warns. I assumed they don't. Nothing in the spec resolves this.
3. **Lines declared inside groups.** "Children are `box`, `group`, `line`, `text`. Child coordinates are relative to the group." Do line points inside a `group { }` use group-relative coordinates? Do anchors? I dodged the question entirely by putting all lines at top level; another agent will plausibly do the opposite and produce a different file.
4. **Along-edge runs.** A segment collinear with a box edge (approaching `rep.right` from directly below; a horizontal at exactly a group's border y). Is that `through`, `endpoint-off-edge`, legal, or degenerate? I hit this four separate times and rerouted around it each time out of caution.
5. **`curve bend` sign.** "positive = to the left of the direction of travel." For upward travel I had to re-derive that "left" means −x, twice. I chose `bend=25` for a curve traveling upward and am still not fully confident of the bulge direction without drawing it.
6. **`curve` consumes the *next* point.** "`curve bend=<N>` — smooth curve from previous point to the **next** point". In a `start -> seg -> ... -> end` chain, a segment that modifies the transition to the *following* element is unusual; I first read `curve bend=30` as a destination. The `via (c1) (c2)` form having *two* control points in one segment reinforces the confusion.
7. **Label geometry.** "`<label>` — label at path midpoint" — centered *on* the line? Offset above it? Which side? Does it get a background? This determines `label-collision` outcomes and I can't compute it from the spec.
8. **Text baseline.** "`at` is the text anchor point … vertically the baseline." Baseline anchoring means visually centering free text requires knowing cap-height ≈ 0.7×font-size. Agents will guess differently.
9. **Anchor arithmetic composability.** `db.top+(10,0)` is defined, and `top@t` is defined, but is `db.top@0.3+(10,0)` legal? I avoided the combination.
10. **Non-rect anchor projection.** "edge anchors are projected onto the actual outline" — for a cylinder, is `left` on the barrel silhouette at the barrel's mid-height, or on the ellipse's widest point? `db.left@t` on my Postgres cylinders: does t run along the projected outline or the bounding-box edge? I silently assumed bounding box.
11. **Off-grid scope.** "`off-grid` — coordinate not a multiple of `grid`". Literal source coordinates only, or resolved anchors too? A 170-wide box has `center` at an off-grid x even when placed on-grid. (I made every width a multiple of 20 to dodge the whole question — that workaround should be documented as a recommendation, or the ambiguity removed.)
12. **Line `style=`.** Line attributes list `style=`, but style keys include `fill`, `font-size` — what do those mean on a line? Presumably only stroke-ish keys apply; unsaid.
13. **Bare-string label position.** The example shows `"SQL"` sitting between the endpoint and `end=arrow`; is attribute order free? Trivial, but I noticed myself hesitating.

### A3.2 Syntax I instinctively reached for that isn't supported

- **Perpendicular joins, TikZ-style `a -| b` / `a |- b`** ("go horizontal then vertical until aligned with b"). This is the single thing I wanted most; most of my routes are hand-rolled `-|` chains.
- **Mixed-coordinate waypoints**: `-> (db.left.x, app1.bottom.y)` — "the x of that, the y of this". Would have replaced nearly all of my `h`/`v` mental arithmetic.
- **Relative placement** `at db.tr+(40,0)` (B3.7 — I wanted it constantly for the second worker, the replica, the second app server).
- **Edge bundling / multi-target lines**: `line app1,app2,app3 -> kafka`, or Mermaid's `a --> b & c`. Three near-identical fan lines is where most of my crossing bugs lived.
- **Named constants and arithmetic**: `let lane1 = 410`, `at (x+40, y)`.
- **Replication/array sugar**: `3 box app size (160,60) spacing 40` — "these side by side" is in the task description itself and I had to lay it out by hand.
- **Line-to-line attachment**: the retry arrow naturally wants to branch off the consume line, or a label wants to anchor to another line's midpoint.
- **Line continuations**: my longest statement is ~100 characters with five segments; one-statement-per-line is workable but tight (B3.8).
- **Label styling** (background pill, font) — the diagrams I'm imitating all have them.

### A3.3 Ambiguities that would make two agents produce different files

1. Lines inside groups: allowed child, undefined coordinate semantics (see A3.1.3). High divergence.
2. Group-border policy for `through`/`spill`: warn or not.
3. Label side/offset/padding: determines label-collision behavior and therefore where labels get placed.
4. Whether fan-out relationships are drawn as N lines or one "bus" line — spec is silent on bundling, and the *task* itself ("3 app servers … publish to Kafka") is ambiguous about whether reviewers expect 3 edges.
5. Box sizing for labels: no auto-size, so agents pick sizes by guesswork; combined with unknown label metrics, two agents pick different boxes for the same labels.
6. `bend` sign and near-edge routing legality: different agents will curve opposite ways and route differently around edges they believe are forbidden.
7. Whether negative `h`/`v` and negative coordinates exist.

### A3.4 Where the coordinate arithmetic was hard

- **Lane planning is the dominant cost, not arithmetic.** With ~14 orthogonal lines, the hard problem is assigning non-overlapping corridors: which line takes which y-lane between the app row and the worker row, which x-lane in the 60px corridor between the k8s boundary and the right column. I re-planned five times; each "fix" for one crossing created another two until the lane assignment was globally consistent. This is exactly what `check` will catch — but the *fix* is still expensive without help.
- **Inverting `@t` fractions**: to hit a specific y on `db.left` I had to compute `t = (y − top)/h`. Offsets (`db.left+(0,20)`) were far easier; I switched to offsets mid-exercise. Recommend documenting offsets as the preferred form.
- **Relative delta bookkeeping**: `h -540` from a point I computed three steps earlier; one sign error sends a line through a box.
- **Grid discipline on derived anchors**: midpoint anchors of odd-sized boxes silently land off-grid.

**What would reduce the burden**, in order: (1) mixed-coordinate waypoints and/or `-|`/`|-` sugar; (2) relative placement (B3.7); (3) diagnostics that report *free corridors from actual geometry* (B2's proposal — it directly addresses my dominant failure mode); (4) `kashika inspect` (I hand-built its output above to check myself); (5) recipes in `ref` for the fan-out and lane patterns I kept rediscovering.

---

## Part B

### B4.1 Tooling triage

**Essential:** one-command render+check with summary line; exit-code contract (0/1/2); JSON diagnostics with the B2 shape; `explain`; deterministic, capped, TTY-only-color output; `-` stdin. For a blind agent, `inspect` is essential — it's the only way to close the loop on "what did I actually build" without a render.

**Nice-to-have but high value:** `fix` restricted to machine-applicable edits (typo'd anchors, near-miss endpoint snapping, label-fit box growth) — these are precisely the errors I'd otherwise fix by hand-deriving the same numbers; `ref` (essential for the first week, then a lookup); `--debug` overlay (essential for multimodal agents, nice-to-have otherwise); `fmt`.

**Unnecessary:** nothing in the list is dead weight. `fmt` is lowest value — I'd tolerate unformatted files — but it's cheap.

**Missing:**
- **A corridor/free-space query.** The natural agent workflow is "is the band y 530..550 free between x 200 and 750?" Today I'd dump `inspect` and compute it myself; a `kashika inspect --free-rect (x1,y1)-(x2,y2)` or free-corridor listing would remove my most expensive manual step. Even better as part of `cross`/`through` diagnostics (see B4.2).
- **`--version` and a diagnostics stability guarantee** — I will write scripted workflows against the JSON schema and need to pin behavior.
- **A diff/regression mode**: run `check` on two files (or two runs) and diff diagnostics, so a refactor like A2 can be validated as "no new warnings" mechanically.
- **Render-to-stdout or `render --check-only`** so `render` doesn't clobber an output file when I only want verification — actually `check` covers it; noting only that I'd want `render` to be idempotent/side-effect-free on unchanged input.

### B4.2 Are the diagnostics actionable in one edit?

Mostly yes. Intersection point + the two source lines + a corridor hint is exactly what I hand-computed. For `through`, the entry/exit facts plus "route above/below" is a one-insert fix (`-> v -50 -> h 340 -> v 50`). The label-overflow machine suggestion is perfect.

Gaps:

1. **Corridors should be computed from the real geometry, not just the two sides of the offending box.** "Above y<110 or below y>170" is generic; what I needed was "the free horizontal corridors in this region are y 200..220 and y 530..550, considering all other lines" — i.e., the actual free bands. That turns my five-iteration lane re-planning into one informed edit.
2. **`shared-segment` needs the overlapping span** (e.g. "x 480..1080 at y=590"), and `cross` should say which line is more cheaply rerouted (fewer segments? later in file?).
3. **`cross` between two multi-segment lines should name the segments** — "segment 2 of line#9 crosses segment 1 of line#10" — so I know which jog to change.
4. **Label-collision** should include the label's bbox and suggest a `t` shift or offset, not just report.
5. **Group semantics** (A3.1.2–3) must be pinned down in the checker's documented behavior; right now I can't predict whether my nested diagram is clean or a warning farm.
6. On the open question "how far should hints go": **corridors only is the right call.** Full reroute suggestions would fight authorial intent exactly as feared, and corridors + geometry facts are enough for an agent that can do arithmetic. The one extension I'd allow: for the machine-applicable `fix` set, *snapping a 5–10px near-miss* is already proposed and is the right ceiling.

### B4.3 B3 answers

1. **Comments: switch to `//`.** `fill=#e8f0fe` vs `# comment` in attribute position is a tokenizer special-case and a hazard for regex-based edits agents do; `//` is unambiguous everywhere. (Keep `#` accepted as an alias if you must, but canonicalize to `//` in `fmt`.)
2. **Yes, rename the stroke pattern** to `stroke-style=dashed|dotted` (or bare `dashed`/`dotted` keywords). `dot` meaning "dotted stroke" on the same statement where `end=dot` means "dot arrowhead" is a genuine reading trap; I double-checked the spec on exactly this.
3. **Unify on `label=`.** The bare positional string and `label@t=` are two spellings of one concept; one form (`label="x"`, `label@0.3="x"`) removes a decision and simplifies `fmt`. I'd keep the bare string only if you want Mermaid-familiar sugar, and even then canonicalize.
4. **`bend=left:30` / `bend=right:30` is clearer.** I had to re-derive the sign of "left of direction of travel" for upward travel more than once (see A3.1.5). Relative-to-travel keywords remove the sign entirely. If you keep signed `bend=N`, at least state the upward-travel case explicitly in the anchor/path docs with a worked example.
5. **Accept both, canonicalize to short.** `tl`/`tr` are fine and terse, but there's no reason to reject `top-left`; unknown-anchor did-you-mean already gets you 90% of the way.
6. **Yes, warn on the diagonal tail after orthogonal segments.** My A2 exercise produced exactly this bug (a straight vertical that became diagonal after the box moved). Make it a lint (`diagonal-tail`?) so intent can be declared with `allow=` or an explicit `diag` segment.
7. **Yes to relative placement — this is the highest-value language change on the list.** `at db.tr+(40,0)` plus group-relative children would have cut my authoring time substantially and made A2 a one-line change. Dependency chains are manageable: it's a DAG, cycles are errors, `inspect` already resolves absolute geometry. Design decision needed: do dependents *follow* when a base moves (yes — that's the point), and does `fmt` rewrite relative refs as absolute or preserve them (preserve).
8. **Long lines need continuation.** My db→rep line is five segments; a routing-heavy line can hit 150+ chars. A backslash continuation, or "a `line` statement may span lines while indented deeper than the `line` keyword" — either is fine; one-statement-per-line with no escape is the one spec rule I'd actually hit.

### B4.4 Wrong-looking diagrams that check clean

1. **Visual rhythm.** Nothing checks equal spacing, row alignment, or consistent lane pitch. My diagram passes with 10px near-misses and asymmetric gaps that would look sloppy to a human. A `tidy` lint (boxes in a row not aligned / not equidistant, lanes not on a consistent pitch) would catch the worst of it.
2. **Grazing geometry.** A line clearing a box by 1px, an arrowhead overlapping a shape edge, a label 2px from a box — all pass. Distance-threshold lints (`clearance<N`, warn) would help; they'd also have flagged my legit-but-tight 10px lanes.
3. **Semantics.** Wrong arrow direction, a line attached to `db.left` when the story says the replica, three lines where a human reads one relationship — geometrically clean, wrong diagram. The checker can't see intent; labels are the only carrier, and nothing checks *those* for sense.
4. **Color/contrast.** A `style` with white text on white fill is invisible and unflagged. A cheap contrast check on fill vs color would be genuinely useful for a blind author — this is exactly the class of error we can't catch ourselves.
5. **Curve quality.** A huge `bend` that loops near a box or a `via` control point that drags the curve across empty space awkwardly: non-degenerate, so clean. Maybe warn when a curve's convex hull intersects a non-endpoint box (that's just `through` generalized to curves — make sure `through` handles curves at all; the spec says checks run on "resolved absolute geometry", which must include Béziers, flattened or sampled).
6. **Group labels.** The group label is "drawn at the group's top-left inside edge" — a child box at the group's top-left collides with it, and it's unclear whether group labels participate in `label-collision`.

### B4.5 What `kashika ref` should contain

One read, then searchable lookups. Concretely:

- A 30-line annotated quickstart (one group, one curved line, one offset anchor, one lint suppression).
- Statement grammar in compact EBNF, one statement each with all attributes.
- The anchor table (verbatim from the spec) **plus**: offset composition rules, `@t` direction conventions per edge, projection rules for non-rect shapes with a worked cylinder example, and the "use offsets not `@t` for grid alignment" recommendation.
- Path segment semantics including the signed `h`/`v` convention, the `curve`-consumes-next-point rule with an ASCII picture, and bend sign with all four travel directions.
- The full lint table, each with a 3-line minimal repro and its fix — this doubles as the `explain` index.
- The exit-code contract and JSON schema.
- **Recipes**: fan-out from one anchor without `shared-segment`; routing through a gap with assigned lanes; nested groups; the A2-style "move a box" maintenance pattern. These are the patterns I had to rediscover; codifying them is the cheapest possible productivity win.

Size: **~400–600 lines (~15–20KB)**, topic-sliced (`ref anchors`, `ref paths`, `ref lints`, `ref recipes`). Small enough to read whole once; structured enough to grep. It must include the *decisions* this review surfaces (groups vs `through`, along-edge behavior, negative deltas) — the spec draft currently leaves them out.

### B4.6 Top 5 changes, ranked by impact

1. **Relative placement and mixed-coordinate waypoints** — `at db.tr+(40,0)` for boxes, and for lines a waypoint form like `(db.left.x, app1.bottom.y)` or `h to db.left.x` ("horizontal until aligned with X"). This attacks the single largest authoring cost (deriving and maintaining absolute coordinates) and the largest maintenance cost (A2). Groups already prove the mechanism.
2. **Pin down line/group semantics in both language and checker**: lines inside groups (coordinate meaning or forbid them), group borders vs `through`/`spill`, along-edge segments (error or defined snap), and guarantee `through`/`cross` cover curves. Right now the highest-divergence ambiguities all live here.
3. **Geometry-aware corridor hints in diagnostics** plus segment-level attribution for `cross`/`shared-segment` and span facts. Turns my dominant failure mode (lane re-planning) from five blind iterations into one informed edit. Keep the "corridors only, never full reroutes" line — it's correct.
4. **Perpendicular-join sugar** (`a -| b`, `a |- b`) and/or a `diagonal-tail` lint so misaligned endpoints are caught or auto-elbowed. Most Kashika routes in practice are hand-built `-|` chains; making the common case one token removes most `h`/`v` sign errors.
5. **Syntax hygiene bundle** (cheap, do it all at once): `//` comments, `stroke-style=dashed|dotted`, unified `label=`, `bend=left:30` (or keep signed but document the four directions), both short and long anchor names canonicalized by `fmt`, line continuations, and documented negative `h`/`v`. None of these is individually large, but together they remove every spot where I hesitated or guessed in A3.1.

**One thing to protect:** the refusal to auto-layout is the product. Every temptation to "helpfully" reroute or nudge boxes should be resisted beyond machine-applicable typo/snapping fixes — the moment the tool starts moving geometry, the author can no longer reason causally about the output, and the blind-agent story dies. The proposals in B1/B2 sit on the right side of that line.
