# Kashika Design Review

**Reviewer:** qwen-3.8-2.4T-A95B (independent agent review)
**Subject:** Kashika language & CLI draft v0 (paper review, no implementation)
**Date:** 2025

I did Part A before reading Part B, as instructed. Everything below is from my own attempt to author the A1 diagram under the spec.

---

## Part A

### A1 — The diagram

```ksh
diagram "Upload pipeline" size (1960,820) grid=10

style service { fill=#e8f0fe stroke=#1a73e8 }
style worker { fill=#e6f4ea stroke=#188038 }

box browser "Browser" at (100,60) size (160,60) style=external
box cdn "CDN" at (360,60) size (160,60) style=external
box lb "Load Balancer" at (620,60) size (160,60) style=service

group vpc "VPC" at (100,200) size (1540,560) style=boundary {
  box pg "Postgres primary" at (1140,120) size (160,80) shape=cylinder style=datastore
  box replica "Read replica" at (1140,280) size (160,80) shape=cylinder style=datastore

  group k8s "k8s cluster" at (60,60) size (1060,440) style=boundary {
    box app1 "App server 1" at (60,60) size (160,60) style=service
    box app2 "App server 2" at (280,60) size (160,60) style=service
    box app3 "App server 3" at (500,60) size (160,60) style=service
    box kafka "Kafka" at (820,60) size (160,80) style=queue
    box worker1 "Worker 1" at (740,220) size (140,60) style=worker
    box worker2 "Worker 2" at (900,220) size (140,60) style=worker
  }
}

box s3 "S3" at (1700,480) size (160,80) style=external

line browser.right -> cdn.left "HTTPS" end=arrow
line cdn.right -> lb.left end=arrow
line lb.bottom@0.25 -> v 100 -> h -360 -> vpc.k8s.app1.top end=arrow
line lb.bottom@0.5 -> v 110 -> h -180 -> vpc.k8s.app2.top end=arrow
line lb.bottom@0.75 -> v 200 -> vpc.k8s.app3.top end=arrow

line vpc.k8s.app1.bottom-(40,0) -> v 70 -> h 640 -> v -160 -> h 420 -> v 30 -> vpc.pg.top "SQL" end=arrow
line vpc.k8s.app2.bottom-(40,0) -> v 60 -> h 440 -> v -140 -> h 340 -> v 20 -> vpc.pg.top-(60,0) "SQL" end=arrow
line vpc.k8s.app3.bottom-(40,0) -> v 40 -> h 180 -> v -140 -> h 520 -> v 40 -> vpc.pg.tr "SQL" end=arrow

line vpc.k8s.app3.right -> vpc.k8s.kafka.left-(0,10) "publish" end=arrow
line vpc.k8s.worker1.top -> v -80 -> vpc.k8s.kafka.bottom-(90,0) "consume" end=arrow
line vpc.k8s.worker2.top -> v -80 -> vpc.k8s.kafka.bottom+(70,0) "consume" end=arrow
line vpc.k8s.worker2.right -> curve bend=30 -> vpc.k8s.kafka.right+(0,20) "retry" end=open dash

line vpc.pg.bottom -> vpc.replica.top "async replication" end=arrow

line vpc.k8s.worker1.bottom -> v 60 -> h 830 -> v -40 -> s3.bottom+(60,0) "writes" end=arrow
line vpc.k8s.worker2.bottom -> v 40 -> h 610 -> v -20 -> s3.bottom+(40,0) "writes" end=arrow
```

Layout rationale: browser/CDN/LB across the top outside the VPC; VPC containing the k8s group (apps, Kafka, workers) plus Postgres primary and read replica stacked on the right; S3 outside the VPC bottom-right. The LB fans out to the three app servers using `bottom@0.25/0.5/0.75` start points so the three drops don't share their initial vertical segments.

#### Honest report on producing this file

This took me much longer than writing the equivalent Mermaid would, and nearly all of the effort went into one thing: **hand-routing a many-to-many topology with zero crossings**. Specifics worth having on record:

- The three app→Postgres lines were the hardest part of the whole exercise. Three sources fanning into one destination edge ("comb routing") required me to discover, by trial and error, a nesting invariant: lane depths must increase from right source to left source, riser x-positions must sit outside the other lines' lane spans, and landings on the shared edge must be staggered with `+`/`-` anchor offsets (three arrowheads cannot all land on `pg.left`). I failed many times before getting it, with a different failure each attempt: `cross` at a riser/lane intersection, `through` (a lane passed through Kafka; an approach passed through Postgres), `shared-segment` (parallel lanes at the same y, or two drops overlapping), and plain box-overlap (I initially placed the replica where it overlapped a worker).
- **I could not route everything the task asked for.** "The app servers also publish to Kafka" — I drew only the app3→Kafka publish line. I could not find crossing-free routes for app1→Kafka and app2→Kafka on top of the LB fan, the app→pg comb, and the worker consume lines; every corridor I tried was already occupied. Rather than ship a knowingly-broken file, I collapsed the tier-level relationship into one line. That's a design failure, not a topology failure: the intent ("the app tier publishes to Kafka") has no expression, so it must be spelled as N independent physical lines that each need hand-routing and that interact with each other's corridors.
- Even-sized boxes are load-bearing: I made every box width/height a multiple of 20 so that edge-midpoint anchors (`top`, `left`, …) stay on the grid=10. A 150-wide box would have put `right` at x=295 and tripped `off-grid` (probably — see A3.1 on scope). I worked around a spec interaction instead of the language supporting it.
- I verified the polyline pairs by hand and believe there are no crossings/shared-segments/throughs. I could **not** verify: the retry curve's swept area against nearby geometry (I don't trust my mental Bézier math), and label collisions (no font metrics; the "SQL" label on the app2 line lands exactly on the (920,440) corner of its own path, which I suspect a checker would flag or render awkwardly). This is precisely what `check` is for, and I wanted it badly while writing.

### A2 — Move the load balancer 200px right

Four of the fifteen lines change:

```diff
-box lb "Load Balancer" at (620,60) size (160,60) style=service
+box lb "Load Balancer" at (820,60) size (160,60) style=service

-line lb.bottom@0.25 -> v 100 -> h -360 -> vpc.k8s.app1.top end=arrow
+line lb.bottom@0.25 -> v 100 -> h -560 -> vpc.k8s.app1.top end=arrow

-line lb.bottom@0.5 -> v 110 -> h -180 -> vpc.k8s.app2.top end=arrow
+line lb.bottom@0.5 -> v 110 -> h -380 -> vpc.k8s.app2.top end=arrow

-line lb.bottom@0.75 -> v 200 -> vpc.k8s.app3.top end=arrow
+line lb.bottom@0.75 -> v 120 -> h -200 -> v 80 -> vpc.k8s.app3.top end=arrow
```

What this revealed:

- **Anchors moved themselves; hand-written deltas did not.** `line cdn.right -> lb.left` needed no edit at all — good, the anchor system works. But the `h -360`/`h -180` deltas encode the old LB's x, so they had to be recomputed by mental arithmetic (860→300 is 560, 900→520 is 380).
- **Line 5 didn't just need new numbers — it needed a new shape.** The old route was a single vertical that happened to line up with `app3.top`. After the move, the drop point (940,120) no longer aligns with app3.top (740,320), so a straight `v 200` now ends in a diagonal hop. I replaced it with a bus at y=240 — and notably, my first instinct (bus at y=220) would have collided with line 3's bus (`shared-segment`, overlap at x 740..860), which I only caught by simulating the checker in my head. A "just bump the coordinate" edit produces a file that fails check in two non-obvious ways.
- Nothing else in the file references the LB's position, so 11 of 15 lines were untouched. That's actually a decent result — but it's the fan-out lines, i.e. the hardest lines, that break.

### A3 — Friction report

#### A3.1 Where I hesitated, guessed, or wasn't sure (with spec quotes)

1. **"positive = to the left of the direction of travel"** (`curve bend=N`). In a y-down coordinate system I could not confidently derive which way the bulge goes. I guessed on the retry line and cannot tell you whether my curve arcs over or under the intended side. Two sign conventions are plausible and the spec gives no worked example with numbers.
2. **"horizontal move by dx (relative to the previous point)"** — the spec never shows a *negative* `h`/`v`. I assumed `h -360` is legal (it must be, or orthogonal routing is impossible), but the sign conventions deserve an example.
3. **"`through` — a line passes through a box that is not its start or end."** Groups have `at` and `size`, so is a group a box? Lines *must* cross group boundaries to reach children, so I assumed groups are exempt — but the spec never says. Related and also unspecified: my app→pg lines run through the k8s group's **interior** (between the app row and the workers). Legal? Linted? No idea.
4. **"Children are `box`, `group`, `line`, `text`. Child coordinates are relative to the group."** Does that mean a line declared inside a group has its absolute `(x,y)` waypoints interpreted relatively? And its anchors? I couldn't tell, so I declared every line at top level with dotted paths — verbose, but the only interpretation I was confident in.
5. **"Comments start with `#`"** vs **`fill=#e8f0fe`**. Under a strict lexer reading, does `#e8f0fe` start a comment? Obviously not intended, but the rule as written doesn't carve out the exception. I guessed hex values are fine.
6. **"`"<label>"` — label at path midpoint."** Midpoint by arc length or by parameter? Identical for polylines, different for curves. I guessed arc length.
7. **Anchor composition**: is `lb.bottom@0.25+(10,0)` legal (fraction *and* offset)? The spec shows `@t` and `+offset` separately, never combined. I avoided it and paid for it in routing flexibility.
8. **"`radius=N` — round every polyline corner"** — including the start and end where arrowheads attach? I guessed yes and didn't use it.
9. **"`off-grid` — coordinate not a multiple of `grid`"** — literals only, or resolved geometry too? A 150-wide box's `right` anchor sits at a half-grid x. I avoided the question by using even sizes; a second agent might not.
10. **Do groups have anchors?** "Anchors on every shape" — is a group a shape? I wanted `vpc.k8s.top` for bus alignment and didn't dare use it.
11. **`text`**: "vertically the baseline" is clear but unusual (I expected top-left); I skipped free text to avoid finding out.
12. **Two lines ending on the same anchor** (three arrowheads on `pg.left`): a shared *point* isn't a shared *segment*, so presumably legal — but it looks broken. I staggered with `+`/`-` offsets on faith that the checker wouldn't stop me either way.

#### A3.2 Syntax I instinctively reached for that isn't supported

- **TikZ `-|` and `|-` elbows** (vertical-then-horizontal / horizontal-then-vertical). The single thing I missed most: `app1.bottom -| pg.top` would have eliminated the corner-coordinate arithmetic I did by hand a dozen times. Kashika forces me to compute the corner myself; `-|` makes the corner *implicit*.
- **Coordinate mixing / relative waypoints**: `(db.left.x, 300)` ("at db's x, at height 300") and `(+100,-20)`. My bus lanes are defined by *other elements'* anchor coordinates, and there is no way to say so.
- **Named waypoints**: `point bus1 = (900,450)`, then `-> bus1`. My comb's lanes, risers, and landings are geometrically interdependent across separate `line` statements, and nothing can express the dependency — it's encoded in repeated literals.
- **Tier-level / many-to-one edges**: something like `apps{1,2,3} -> pg`, or a trunk/bus line that others branch from. The comb routing that ate most of my time exists because this is missing.
- Mermaid's `a -->|label| b` compact edge form (minor).
- **Auto-fit boxes** (`size auto` / omit size and fit the label) — I chose sizes to fit labels by estimation.
- **Relative box placement** (`at db.tr+(40,0)`) — I reached for this placing the replica beside pg (this is B3.7; my answer there is emphatically yes).
- `above`/`below`/`left-of`/`right-of` label nudges.
- Auto-size groups (`size auto` to wrap children).
- Iteration/repetition for the three near-identical app servers (typed out three times).
- Line "jumps" (hop markers) for intentional crossings.
- Explicit z-order/layer control beyond file order.

#### A3.3 Ambiguities that would make two agents diverge

1. **Group semantics in the checker** — whether `through`/`cross` apply to group boxes and boundary strokes. Two agents will produce structurally different files (one routes around groups, one routes through them) from the same intent.
2. **The `bend` sign convention** — mirrored retry curves across agents.
3. **Lines inside groups** — relative vs absolute waypoints (see A3.1.4). This changes where lines can even be declared.
4. **`off-grid` scope** — literals vs resolved anchors (A3.1.9).
5. **Label midpoint definition** for curved paths, and where the label sits relative to the path (on it? above it? is its bbox measured for `label-collision` against its own line?).
6. **Whether bare-id references are visible outside groups** — "Inside a group's block, siblings may be referenced by bare id" implies dotted paths outside, which I used; but can a top-level line reference `app1` bare when only one `app1` exists? Ambiguous leniency means divergent files.
7. **Group anchors** (A3.1.10).

#### A3.4 Where the coordinate arithmetic was hard, and what would reduce it

Ranked by pain:

1. **The comb.** Routing 3 sources into one edge with zero crossings required discovering the nesting invariant (deepest lane ↔ leftmost riser ↔ staggered landings) empirically, with each failed attempt surfacing a *different* lint (`cross`, `through`, `shared-segment`, `overlap`). Nothing in the language helps: no elbows, no relative waypoints, no shared trunks. I estimate this was >70% of my total A1 effort. Reducers: `-|`/`|-` elbows; `(anchor.x, y)` mixing; named points; and — most of all — diagnostics that report **free corridors as clearance intervals** (see B4.2) so the invariant doesn't have to be rediscovered by trial and error.
2. **Anchor midpoints.** Every `top`/`left` anchor requires halving width/height mentally (box at (220,320) size 160x60 → top=(300,320)). I standardized all sizes to multiples of 20 purely to keep anchors on-grid. Reducers: `kashika inspect` (proposed in B1 — this is exactly what I needed mid-authoring, not just after); even-size convention documented; or grid-snap for anchors.
3. **Sign slips on upward moves** (`v -160` = *up* in y-down coords). Reducer: `inspect` round-trips.
4. **10px clearances.** My lanes sit 10px below box edges and 10px apart, chosen "because grid" — there's no way to express "stay N clear of X," so clearance is hope, not specification. Reducer: a proximity lint (see B4.4).
5. **The A2 cascade.** Recomputing `h` deltas by hand is easy arithmetic but error-prone in aggregate, and the shared-segment collision on line 5's bus was genuinely non-obvious. Reducers: relative waypoints; `check` output that catches it (it would have); `fmt`-stable diffs.

---

## Part B

### B4.1 Tooling triage

**Essential:** `render`-runs-everything; compiler-style text output + `--format json`; `kashika ref`; `kashika inspect`; stdin (`-`); deterministic, capped, stably-ordered output. `inspect` is the single most important item for a non-vision agent — I effectively hand-simulated it throughout Part A and would have called it constantly.

**Nice-to-have:** `explain` (cheap, rustc-style, occasionally decisive); `fix` (the machine-applicable-only scoping is exactly right — endpoint snapping and label-fit were two of my own failure modes); `fmt` (real value in agent edit loops: small diffs, canonical attribute order); `--debug` overlay (useful for multimodal agents; not for me).

**Unnecessary:** nothing in the list is unnecessary. `--debug` is the one I'd use least.

**Missing:**

1. **A proximity/clearance lint** (`tight`): minimum separation between parallel lines, minimum line-to-box clearance, arrowheads piled on one point. My A1 file is check-clean by design but visually cramped (10px lanes); nothing in the proposed lint set would notice. This is my top gap (see B4.4).
2. **`check --focus <ids>`** — filter diagnostics to specific elements; in an edit loop on one line I don't want to re-read 14 unrelated warnings.
3. **`kashika ref examples`** — 3–5 complete, *known-clean* example files (fan-out with `@t`; comb-into-one-edge with staggered landings; nested groups; curve with both bend signs). Worth more than any amount of prose: the comb example alone would have saved me the bulk of my A1 time.
4. **A version marker** in the file (`diagram ... version=1` or similar) so the agent's mental model and the installed binary provably agree — `ref` matches the binary, but nothing says the file was written against this one.
5. **`inspect` should print group memberships and the group boundary boxes too** — routing through a group interior requires knowing the boundary rectangle, and I had to track it by hand.

### B4.2 Are the B2 diagnostics actionable enough?

Mostly yes — the `through` example (entry/exit facts + a corridor hint + a concrete insert suggestion) would let me fix in one edit. Gaps:

- **Corridors should be clearance intervals, not half-planes.** "Route above y<110 or below y>170" is good; strictly better and still cheap: for the offending segment, sweep the crossing axis and report the *free bands* — e.g. "free lanes at y 210..220, 250..260, 460..470 across x 300..660". That turns the comb invariant from rediscovery into lookup, and it's a sweep-line computation, not auto-layout. I'd keep the "no full reroutes" principle and just make corridors maximal-interval precise.
- **`shared-segment` should suggest the nearest free parallel lane** ("occupied: y 420, 440; nearest free: 450"). I hit this exact case in A2.
- **`label-collision` should suggest clear `t` values** along the path — cheap, and it's almost always the right first fix.
- **`cross` should include corridors for *both* involved segments**, and say which segments are involved by index (the example does this — good).
- **State that curves participate in `cross`/`through`** — the spec says "two lines intersect" and I had to assume Bézier-vs-polyline math is included; my retry curve is exactly the case I couldn't verify by hand.
- **A near-miss family**: `endpoint-off-edge` exists; add near-parallel close lines and near-`through` (passing within ε of a box corner). My 10px-everything style is exactly what these would catch before a human does.

### B4.3 B3 answers

1. **Comments → `//`: yes.** Hex `#` and comment `#` in the same grammar is a guaranteed tokenizer ambiguity; `//` costs nothing and removes it entirely. (I hit this hesitation myself — A3.1.5.)
2. **`stroke-style=dashed|dotted`: yes.** The `dot`-flag vs `end=dot` overload is a real footgun; I'd guess wrong under time pressure.
3. **Unify labels: yes.** `label="SQL"` for the midpoint, keep `label@t=` for positions; the bare positional quoted string is ambiguous sugar (it reads like an ID or a text statement) and clutters the grammar.
4. **Keep `bend=N`, fix the spec.** `bend=left:30` is redundant state — the sign already encodes it — and adds a second way to say one thing. The actual problem (A3.1.1) is that "left of the direction of travel" is under-specified in y-down coordinates: define the convention, state the y-down caveat, and give two worked examples (one `+`, one `−`) in `ref`.
5. **Both: `tl`/`tr` canonical, `top-left` etc. accepted as aliases.** Zero cost; aliases only help.
6. **Yes, warn** (lint, `allow=`-able) on a diagonal final hop after orthogonal segments. I hit this repeatedly and it was almost always an arithmetic slip — this lint would have caught my A2 line-5 breakage immediately.
7. **Yes, add `at <anchor>+(dx,dy)`.** My most-wanted language change; I re-derived positions from neighboring boxes constantly (replica-beside-pg, workers-under-kafka). Dependency chains are manageable: cycles are errors, `inspect` prints resolved coordinates, and `fix`/`fmt` already exist as homes for resolution reporting.
8. **Keep one-statement-per-line, add explicit continuation** (trailing `\`). My 6-segment comb lines were genuinely unwieldy; continuation preserves the parse model while letting long paths be read and diffed.

### B4.4 What would look wrong while `check` reports clean

Ranked by likelihood for an agent like me:

1. **Crowding.** My own A1 file is the proof: 10px-apart parallel lanes, 10px box clearances, three verticals 140px apart in a shared corridor. Every proposed lint passes; a human sees a circuit board. The check set has no *minimum-separation* concept, and this is the most likely way the tool earns a reputation for "technically clean, visually broken."
2. **Semantic misuse of groups.** My app→pg lines cut straight through the k8s cluster's interior (through the gap between app row and workers). No box is touched, no lines cross — but a human reads it as "database connections threading through the pod cluster," which is misleading. `through` presumably doesn't apply to groups (A3.1.3), and nothing else flags interior passage.
3. **The `bend` sign.** A retry curve bulging the wrong way through empty space checks clean and reads backwards (retry arcing the wrong side of Kafka). I could not resolve this sign from the spec, so this is a live risk in my own file.
4. **Arrowhead pileups.** Three lines ending on the same anchor (`pg.left`) share a point, not a segment — no lint fires, and the result looks broken. I dodged it with `+`/`-` offsets; a time-pressed agent won't.
5. **Directional errors.** `start=`/`end=` defaults and the arrowhead/end confusion produce diagrams with arrows on the wrong end — geometrically perfect, semantically wrong. No check can catch intent, so this is where the human review step matters most.
6. **Labels on corners.** My app2 "SQL" label lands on the (920,440) corner of its own path — no box collision, but visually half-detached from the flow. `label-collision` doesn't cover label-vs-own-path placement.

### B4.5 What should ship in `kashika ref`

One read, roughly 150–250 lines. Contents, in order:

1. **Coordinate system** with sign conventions and a tiny ASCII sketch — y-down, `at` = top-left, negative `h`/`v` means left/up. Three lines that prevent the most basic errors.
2. **One-line grammar summary per statement** (`diagram`, `style`, `box`, `group`, `text`, `line`).
3. **The full anchor table** including `@t` semantics and one worked example with real numbers (`box at (220,320) size (160,60)` → every anchor's coordinates).
4. **8–10 canonical complete example lines**: anchor-to-anchor; anchor+offset; `@t` fan-out; `h`/`v` elbow; **the comb — three lines into one edge with staggered `+`/`-` landings** (the example I needed most and had to invent); `curve bend` with both signs; `curve via`; `label@t`; `allow=`.
5. **Groups**: relative child coordinates, dotted paths, one two-level nesting example.
6. **Styles**: built-in list plus one custom override.
7. **Lint table**, one line each, plus `allow=` syntax.
8. **CLI one-liners** with flags.
9. **Gotchas**: hex `#`, `dot` overload, text baseline, final-diagonal hops, `off-grid` scope, group exemption from `through` (once decided — A3.1.3).

Plus a `ref examples` subcommand holding 3–5 complete known-clean files. If I could only have one thing in the whole tool, it's this: the comb example would have converted my hardest hour into a two-minute adaptation.

### B4.6 Top 5 changes, ranked by impact

1. **Relative positioning and waypoint leverage.** Add `at <anchor>+(dx,dy)` for boxes (B3.7), and for lines add elbow sugar (`-|`, `|-`), relative waypoints (`(+100,-20)`), coordinate mixing (`(db.left.x, 240)`), and named points (`point bus1 = (900,450)`). This is one change in spirit: let coordinates *reference each other* instead of being restated. It removes most of the mental arithmetic, makes A2-style moves local instead of cascading, and lets the geometric dependencies my comb depends on be expressed rather than hoped for.
2. **Proximity and legibility lints + clearance-interval diagnostics.** Minimum line-line separation, minimum line-box clearance, arrowhead pileups, label placement suggestions; and upgrade corridor hints to free-band intervals. This directly closes the "check says clean, human says wrong-looking" gap — the biggest credibility risk to the whole concept, and my own A1 file is exhibit A.
3. **Decide and document group semantics in the checker.** Whether `through`/`cross` see groups, whether boundary strokes count, whether interior passage is linted, whether groups have anchors, and the coordinate rules for lines declared inside groups. This is the largest divergence risk between two agents producing "the same" diagram (A3.3.1/3/7), and right now every agent must guess.
4. **Support the many-to-many pattern, or at least canonize it.** Either a trunk/branch primitive for tier-level relationships, or — the cheaper version — `ref` documentation of the comb pattern plus a `fan` lint that suggests staggered landings when N lines target one anchor. My A1 failure to route three publish lines alongside three SQL lines is the stress case the current design has no answer for; the fix can be documentation first and syntax later.
5. **Lexical/syntax cleanups.** `//` comments, `stroke-style=dashed|dotted`, unified `label=`, an explicitly defined `bend` convention with two worked examples, and line continuation for long paths. Each is small; together they remove a predictable class of agent errors I personally hit or nearly hit (A3.1.1/2/5, A3.2).

---

## Overall assessment

The core bet — agents can make many small precise decisions, so give them full control plus a geometric oracle instead of a layout engine — is right, and the checker is what makes it viable: I wanted `check`, `inspect`, and corridor diagnostics *while writing*, not after. The anchor system also genuinely works; A2's anchor-only lines needed zero edits.

The weakness is leverage, not control. My A1 took far longer than the equivalent Mermaid *because* control without abstraction means every relationship (bus heights aligned to anchor x's, lane depths ordered by source, staggered landings) is encoded in anonymous literals and re-derived by hand on every edit — and the many-to-many pattern that dominates real architecture diagrams reduces to comb-routing puzzles that I solved only by trial and error, and for the 3×publish + 3×SQL case could not solve at all within a reasonable budget. The five changes above — especially relative positioning and clearance-aware checking — would convert most of that pain into routine work. As drafted: promising, immediately useful for moderate topologies, and I'd adopt it today for the checker alone; I'd want changes 1–3 before trusting it with dense fan-in/fan-out systems.
