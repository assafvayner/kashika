# Kashika language and interface review

Date: October 28, 2026  
Requested review: GPT-6.1 Sol  
Scope: paper review of [the review packet](kashika-review-packet.md) and [the language draft](spec-draft.md), from the perspective of an agent authoring and revising diagrams.

## Assessment

The explicit coordinates, named anchors, nested coordinate systems, and proposed `inspect` command make a useful foundation. I would use this for diagrams where spatial arrangement matters and I expect to revise the source. Anchors already eliminate much of the arithmetic required when moving a box.

The design is not precise enough yet for an agent to rely on a clean check as evidence that its intended diagram was produced. The largest gaps are the meaning of intersections and group boundaries, the geometry of text and curves, and the relationship between render, diagnostics, and automatic fixes. Resolve those contracts before expanding the shape catalog.

There is also a problem with the review exercise itself: under the natural interpretation that **each** app connects to the load balancer, primary Postgres, and Kafka, the requested graph contains K₃,₃ and cannot be drawn in the plane without a crossing or shared routes. Curved lines do not remove that constraint. This is an opportunity to specify intentional crossings and shared buses explicitly.

**Method limitation:** I loaded the packet in full before completing A1–A3, so these are not blinded first impressions as requested by the packet. The Part A observations below are an exercise-based assessment; they should not be treated as an uncontaminated first-use study. No Kashika implementation is present, so the source examples have not been parsed or rendered by Kashika.

## Part A: authoring and editing

### A1. Complete source and the crossing constraint

Assumptions: all three apps receive load-balanced traffic, all three write to the primary, and all three publish to Kafka. Both workers consume Kafka and write to S3. The retry is a separate worker-to-Kafka edge. Postgres and Kafka can be inside the VPC without being inside the cluster.

The nine edges between `{app1, app2, app3}` and `{lb, primary, kafka}` form K₃,₃. These six nodes each have three neighbors in the other partition. A crossing-free drawing with independent edges would require this graph to be planar, which it is not. Sharing line segments can hide the crossing visually, but introduces overlap and an undefined junction convention. Adding a gateway or omitting an app connection changes the requested topology.

Consequently, this is a complete, faithful **one-crossing attempt**, rather than a claim to satisfy the impossible combination of constraints. All shapes are rectangles so the example does not depend on the unspecified outline projection for nonrectangular shapes. The explicit cubic retry avoids guessing the sign and construction of `curve bend=N`.

```ksh
diagram "Browser to storage" size (1700,1480)

style node { font-size=14 }

box browser "Browser" at (100,30) size (140,60) style=node
box cdn "CDN" at (320,30) size (140,60) style=node

group vpc "VPC" at (40,200) size (1580,1220) style=boundary {
  box lb "Load balancer" at (320,170) size (160,70) style=node
  box primary "Primary\nPostgres" at (1040,170) size (160,70) style=node
  box replica "Read replica" at (1280,170) size (160,70) style=node
  box s3 "S3" at (680,1100) size (140,70) style=node

  group cluster "k8s cluster" at (160,340) size (1160,720) style=boundary {
    box app1 "App 1" at (70,80) size (140,60) style=node
    box app2 "App 2" at (520,80) size (140,60) style=node
    box app3 "App 3" at (970,80) size (140,60) style=node
    box kafka "Kafka" at (520,380) size (140,70) style=node
    box worker1 "Worker 1" at (370,590) size (140,60) style=node
    box worker2 "Worker 2" at (670,590) size (140,60) style=node
  }

  line lb_app1: lb.bottom -> cluster.app1.top end=arrow
  line lb_app2: lb.right -> cluster.app2.top@0.25 end=arrow
  line lb_app3: lb.top -> (400,80) -> (1480,80) -> (1480,450) -> cluster.app3.right end=arrow

  line app1_db: cluster.app1.right -> (450,550) -> (1000,550) -> primary.bottom end=arrow
  line app2_db: cluster.app2.top@0.75 -> primary.left end=arrow
  line app3_db: cluster.app3.top -> primary.right end=arrow

  line app1_kafka: cluster.app1.bottom -> cluster.kafka.left end=arrow
  line app2_kafka: cluster.app2.bottom -> cluster.kafka.top end=arrow
  line app3_kafka: cluster.app3.bottom -> cluster.kafka.right end=arrow

  line replication: primary.top -> (1120,140) -> (1360,140) -> replica.top end=arrow
  line consume1: cluster.kafka.bottom@0.25 -> cluster.worker1.top end=arrow
  line consume2: cluster.kafka.bottom@0.75 -> cluster.worker2.top end=arrow
  line retry: cluster.worker1.left -> curve via (400,960) (430,800) -> cluster.kafka.left@0.8 end=arrow
  line store1: cluster.worker1.bottom -> s3.left end=arrow
  line store2: cluster.worker2.bottom -> s3.right end=arrow
}

line browser_cdn: browser.right -> cdn.left end=arrow
line cdn_lb: cdn.bottom -> (390,130) -> (260,130) -> (260,405) -> vpc.lb.left end=arrow
```

`app1_db` has a horizontal section at VPC-local `y=550`, from `x=450` to `x=1000`. `app2_kafka` runs vertically at `x=750`, from `y=480` to `y=720`. They cross at local **(750,550)**, or canvas **(790,750)**. I left that crossing unsuppressed so the example does not conceal the conflict with the task.

All route coordinates inside `vpc` are interpreted in the VPC coordinate frame, including the cubic control points. The source intentionally uses no line labels, grid, or rounded corners, to isolate routing from other unspecified behavior. It assumes a group is a boundary that its descendants' connections may cross, rather than an obstacle to those connections.

### A2. Move the load balancer 200 pixels right

For the literal move, one source line must change:

```diff
-  box lb "Load balancer" at (320,170) size (160,70) style=node
+  box lb "Load balancer" at (520,170) size (160,70) style=node
```

To preserve the vertical departure and right-angle turn of `lb_app3`, I would also change its first waypoint:

```diff
-  line lb_app3: lb.top -> (400,80) -> (1480,80) -> (1480,450) -> cluster.app3.right end=arrow
+  line lb_app3: lb.top -> (600,80) -> (1480,80) -> (1480,450) -> cluster.app3.right end=arrow
```

Those are all the changes I would make. The named endpoints of `lb_app1`, `lb_app2`, and `cdn_lb` follow the moved box automatically. Without the second edit, the program is still syntactically plausible, but `lb_app3` starts with a diagonal. That is the distinction between preserving references and preserving route intent.

### A3. Hesitations, missing expressions, and arithmetic

These are the uncertainties I encountered while constructing and evaluating the example. Quotations are from Part A of the packet.

| Wording | What I had to decide or could not determine |
|---|---|
| “There should be no line crossings.” | Does this mean distinct edges, shared buses, or merely no transverse intersections? The fully connected interpretation is impossible with independent edges. |
| “Inside a `group`, child coordinates are relative to the group's top-left corner.” | I interpreted waypoints and Bézier control points in the declaring group's frame, even when the endpoints reference other groups. Say so explicitly; the word “Absolute” in the point grammar otherwise suggests canvas coordinates. |
| “Inside a group's block, siblings may be referenced by bare id.” | `cluster.app1` seems intended, but ancestor lookup, shadowing, absolute paths, forward references, and the scope of line IDs are not specified. I declared nodes before their edges to avoid forward-reference assumptions. |
| “two lines intersect” | Does a shared terminal anchor count? What about tangency, endpoint contact, self-intersection, and an intentional junction? My example uses different ports to avoid relying on a shared-terminal exception. |
| “a line passes through a box that is not its start or end” | Are groups boxes for this purpose? A connection must cross the cluster boundary. Also, exempting the entire source and destination shapes could permit a line to leave and later pass back through its own endpoint shape undiagnosed. |
| “two non-nested boxes overlap” | Is nesting semantic parenthood or geometric containment? An unrelated small box entirely inside a large box still needs a diagnostic. |
| “Comments start with `#` and run to end of line.” | Taken literally this eats `fill=#e8f0fe`. I avoided color literals in the exercise. The lexer needs an explicit rule. |
| “edge anchors are projected onto the actual outline” | Projected in which direction? A ray from the center and a perpendicular from a bounding-box side give different points. Corners, concave outlines, and multiple intersections need rules too. I used rectangles. |
| “perpendicular bulge in px at the midpoint” | Is this a quadratic Bézier, a cubic, or an arc? Is “midpoint” parameter midpoint or half arc length? I used explicit controls. |
| “positive = to the left of the direction of travel” | With downward-positive y, a conventional algebraic normal can point to visual right. A horizontal worked example or formula is necessary. |
| “round every polyline corner with radius N” | What happens on short segments, near reversals, and beside curves? Are anchors and label positions evaluated before or after rounding? I omitted rounding. |
| “label at fraction t of path length” | I expect arc length of the final visible path, but the text orientation, normal offset, padding, background, and interaction with arrowhead trimming are undefined. I omitted edge labels. |
| “Labels are centered” / “label overflow (with required size)” | Font family, font resolution, padding, line height, fallback glyphs, and shape-specific usable text areas are unspecified. A reported required size cannot be trusted without them. |
| “Drawing order = file order; a group is drawn behind its children.” | Is an entire group an indivisible paint subtree? Do lines crossing a group border paint over it? Can a later filled shape hide an earlier line without a warning? |
| “grid … warns on off-grid coordinates” | Does this include sizes, relative offsets, inferred anchors, control points, or label positions? A centered anchor can be off-grid even when its box's position and dimensions follow the grid. I omitted `grid`. |
| “degenerate curves” | Is a straight cubic an error, or merely harmless? Are coincident endpoints allowed for a loop with nonzero extent? Which numeric ranges and non-finite values are invalid? |
| “line#N (1-based file order)” | Is N the count of line statements, and does traversal include nested groups? Adding one edge should not make all later diagnostic references unusable. I named every line. |

The expressions I reached for were named ports, a named shared junction/bus, absolute-axis waypoints such as “go to the destination's x at this y,” relative box placement, and multiline edge declarations. None is specified. A bus would be useful here only with explicit connectivity semantics; silently treating every intersection as a connection would be dangerous.

Two agents could produce very different files for the same prose because the topology's fan-out is implicit, routing corridors and ports are unconstrained, defaults are not enumerated, and intersection/group semantics are incomplete. Multiple valid layouts are acceptable. Multiple meanings for the same file are the problem to eliminate.

The hard arithmetic was composing the two group origins, reserving corridors for nine app edges, and keeping a waypoint aligned when moving the load balancer. For example, `app2` has canvas top-left `(40+160+520, 200+340+80) = (720,620)`; its bottom anchor is `(790,680)`. `inspect` should make those computations unnecessary. Restricted relative placement and axis-projected waypoints would make revisions cheaper without selecting a layout for me.

## Part B: interface and tooling

### B4.1. Essential, useful later, and missing

| Tooling | Priority | Reason |
|---|---|---|
| Render runs checks; explicit output path; stable exit status | Essential | It must be possible to render and evaluate one exact source revision in one invocation. |
| Compiler-style text plus structured JSON | Essential | Text supports direct editing; JSON supports reliable automation. Use one format flag consistently across commands. |
| Deterministic ordering and non-TTY output | Essential | Repeated runs should be comparable and easy to parse. |
| Stdin input | Essential | Include a logical filename option so spans identify the input meaningfully. Define output handling for SVG and PNG. |
| Version-matched `ref` | Essential | An agent should not guess which grammar the installed binary accepts. |
| `inspect` with ID filtering | Essential | It supplies resolved positions, ports, labels, and paths when image inspection is unavailable. Add JSON output. |
| `fmt` / `fmt --check` | Essential, with limits | Stable attribute order and local indentation help. Aligning whole columns can turn one longer ID into unrelated edits. Preserve comments and make formatting idempotent. |
| Output cap | Useful, conditional | Good for terminal text. JSON must expose the total and whether results were truncated; provide an uncapped mode. |
| `explain` | Useful later | A short explanation can initially live in `ref lints`; a separate command becomes useful as diagnostics grow. |
| Debug overlay | Useful | Especially valuable for matching a visible issue to IDs. It does not replace machine-readable geometry. |
| `fix` | Defer | Candidate patches are useful; calling the proposed edits machine-applicable is premature. |

None of the proposed capabilities is inherently unnecessary. A broad automatic-fix command is the least justified part of v0, and a large first release of independent convenience commands would distract from the render/check/inspect contract.

Missing interface contracts:

- **One versioned JSON envelope** containing tool/schema version, diagnostics, counts, truncation state, and artifact status. Keep stdout machine-readable and send progress to stderr. `--json` in Part A and `--format json` in Part B should converge.
- **Transactional output behavior.** Write a new artifact atomically only on successful rendering. On failure, report that no artifact was produced for this invocation, even if an older file still exists at the requested path. Include an input digest or equivalent revision identity in machine output.
- **A warning policy suitable for automation.** Default warning exit code 1 is workable if documented. Add a way for warnings to be non-failing and a strict policy for selected warnings. State that warnings still produce output, and distinguish diagnostic failures from I/O or renderer failures in JSON.
- **Inspectable rendering defaults.** Include the resolved font, styles, padding, paint order, and visible bounds, not just node bounding boxes. Provide `--version` and a compact complete example.
- **Explicit route intent.** Support an orthogonal route assertion and, if shared routes are supported, named junctions/ports. These remain authored constraints rather than automatic layout.

### B4.2. Are the diagnostics actionable in one edit?

The proposed facts, involved elements, and concrete spans are a strong starting point. They usually let me identify the fault. They do **not** guarantee that a local repair will produce a globally valid diagram.

For example, a corridor above `cache` can intersect another box or cross an unrelated edge. Label resizing changes the box's center and edge anchors, moving attached edges and potentially overlapping its neighbor. This is especially relevant because `fix` says it “never … moves boxes”: growing a box preserves its top-left but still moves its center and ports.

I would make these changes:

1. Report the specific subpath or segment, the coordinate frame of each fact, the relevant stroke/marker/label bounds, and the tolerance used. For curves, include the intersection parameter or a bounded interval.
2. Distinguish **local geometric advice** from a **globally checked candidate**. “Above y=110” describes one obstacle; it does not establish a free corridor through the scene. Include stroke clearance rather than only centerline clearance.
3. Give edits complete source ranges, a document identity, and a defined column encoding. Multi-edit suggestions must be atomic. Recheck a candidate before claiming that it resolves the reported condition without introducing new errors.
4. Treat typo correction, endpoint snapping, and resizing as tentative unless intent is unambiguous. A unique nearest anchor is not proof that it was the intended anchor. Prefer a dry-run patch and an explicit opt-in application command.
5. Keep overflow geometry inspectable. A failed final render should still permit `inspect`; a clearly marked diagnostic preview would help an agent repair text without a successful production artifact.
6. Scope suppressions narrowly. `allow=cross` on a busy edge can conceal future, unrelated intersections with every other edge. Prefer identifying the other edge, and retain suppressed results in machine output when requested.

There is also an arithmetic mistake in the proposed `inspect` example. Its path has lengths `40 + 80 + sqrt(130² + 60²)`, approximately **263.18**, rather than the displayed `len=310`. That final segment is diagonal. Reference examples should be generated from fixtures so agents can trust their numbers.

### B4.3. Answers to the eight language questions

| Question | Recommendation |
|---|---|
| 1. `#` comments versus hex colors | Use `//` comments outside strings and preserve `#` for colors; explicitly define escaping and lexical boundaries. |
| 2. Dotted/dashed strokes | Use `stroke-style=solid|dashed|dotted`; keep `end=dot` for the marker so attribute names explain their meaning. |
| 3. Label forms | Use `label="SQL"` as shorthand for `label@0.5="SQL"`, with repeatable positioned labels and a rule for duplicate positions. |
| 4. Signed curve bend | Prefer `bend=left:30` / `bend=right:30`, but define a concrete curve formula and screen-coordinate examples; better spelling alone does not resolve geometry. |
| 5. Corner names | Pick one canonical spelling, preferably `top-left`; aliases can wait until compatibility requires them. |
| 6. Final diagonal after `h`/`v` | Do not infer intent from a mixed path; offer an explicit orthogonal assertion that checks the final segment too. |
| 7. Relative box placement | Yes, in a restricted form with explicit coordinate conversion, acyclic dependencies, cycle diagnostics, and inspectable results. |
| 8. One statement per line | Permit continuation when a line ends in `->`, or use explicit delimiters; long paths and labels need readable multiline formatting. |

For relative placement, specify which point of the new box is being positioned: under the current `at` rule, `at db.tr+(40,0)` places the new box's **top-left** there. Cross-group references should first resolve to canvas geometry and then convert into the containing frame. Relative positions should never silently change parentage. Axis-projected line waypoints are at least as useful as relative boxes and avoid proliferating magic numbers during edits.

### B4.4. How a clean diagram could still look wrong

The most likely failure is incorrect **connectivity or meaning**: a missing arrow, an edge connected to the wrong but valid port, a reversed replication arrow, or three coincident paths that visually look like one relationship. All can have perfectly valid coordinates. `check` should describe its geometric guarantees without presenting them as semantic correctness.

Other likely failures are:

- A renderer or PNG environment substitutes a font, changing glyph widths after the checker accepted the labels.
- Later shapes occlude earlier edges, or label backgrounds hide arrowheads. Centerline checks alone do not establish visibility.
- Labels collide with lines, standalone text, group titles, or their own arrowheads; the current collision rule only mentions boxes and other labels.
- An edge leaves and re-enters its own endpoint box because source/destination shapes are wholly exempted from `through`.
- A path grazes a node or another path without intersecting its centerline. Stroke width, marker extent, and a minimum clearance policy matter.
- A technically nonintersecting drawing uses huge detours, uneven spacing, tiny text, or ambiguous arrow approaches. Geometry cannot establish visual hierarchy or readability by itself.
- A broad suppression hides a new collision after a later edit.

Rendering and checking should use the same resolved scene, including shaped text and final path geometry. Require no overlaps only where the user has declared that policy; recognize legitimate nesting, boundary crossings, and intentional crossings explicitly.

### B4.5. What `kashika ref` should contain

Aim for approximately **1,500–2,000 words**, including one copyable complete example and compact tables. Use topic-specific detail for the longer rules. A single read should answer:

1. Minimal program, commands, output destinations, JSON mode, exit statuses, and the repair loop: author → render/check → inspect → edit.
2. Exact token and string rules: comments, escapes, identifiers, numbers, colors, separators, continuations, and unknown/duplicate attributes.
3. Coordinates, nested scopes, point resolution, forward references, anchors and ports, and cross-group conversion, with one numeric example.
4. Straight, relative-axis, and cubic paths; the exact bend construction; corner rounding; marker trimming; label placement on final arc length.
5. Concrete default styles, fonts, padding, shape text areas, and override precedence.
6. One diagnostic example, stable lint codes, suppression scope, and the limits of a clean result.

Keep the first example rectangular and uncomplicated. Add a small nested-group example and a curve example. Include a grammar/version identifier, and make every sample a parser/render/check fixture when an implementation exists.

### B4.6. Top five changes, ranked

1. **Define topology and intersection semantics.** Specify shared endpoints, ports, crossings versus junctions, shared segments, ancestor boundary exemptions, and edge re-entry into its endpoint shapes. Correct A1's impossible requirement or explicitly allow intentional crossings/buses.
2. **Specify one deterministic geometry model.** Define anchor projection, cubic/bend construction, rounding and marker bounds, tolerances, shaped text and fonts, and label placement. Render, check, and inspect must agree on that scene.
3. **Reduce edit arithmetic while preserving authored intent.** Add constrained relative placement and axis-projected waypoints, explicit orthogonal assertions, and coordinate-frame-aware inspection. The two-line load-balancer edit is a useful regression fixture.
4. **Make the machine interface reliable.** Standardize versioned JSON, source ranges, complete diagnostic facts, warning policies, truncation, and atomic artifact status. Return locally valid hints as local hints; defer broad automatic fixes.
5. **Freeze a small, testable language surface.** Resolve comment/color lexing, continuation syntax, ID scope, string/number rules, style precedence, default metrics, and error recovery. Generate reference examples from conformance fixtures before adding more shapes or convenience syntax.

## Verification and limits

This is a design review, with no implementation changes. The example is intended to exercise the documented grammar, not to claim that an unavailable compiler accepted it. Rectangular node coordinates, straight route intersections, and a sampled cubic retry were checked separately; that is an arithmetic check, not a substitute for Kashika parsing, exact curve intersection, font measurement, or rendered-image review.

The expected proper crossing is `(790,750)` on the canvas. The example deliberately retains it and does not add a broad suppression. The most useful acceptance test for the future implementation is agreement among the rendered image, `check`, and `inspect` about that crossing and the load-balancer move.
