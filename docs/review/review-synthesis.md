# Review synthesis: v0 → v1

Sources (kept locally, not in git): reviews by Sonnet 5.5, GPT-6.1 Sol, Gemini 3.8 Flash and DeepSeek v4.1 Flash. In the Agree column, "4/4" means all four reviewers raised the point. A second round, from Qwen 3.8 and GLM 5.3, independently confirmed the top items: elbows and axis targets, relative placement, group semantics, corridor hints, near-miss lints and syntax hygiene.

## Additions from round 2 (Qwen, GLM)
- Mixed-coordinate points `(db.left.x, lane1.y)`. A junction can serve as a named guide point.
- Groups have anchors. `@t` composes with offsets. Negative `h`/`v` are documented. Lines apply only the `stroke*` and font keys of a style.
- Curves take part in every check (flattened to within 0.25px).
- `port-crowding` covers stacked arrowheads on a single anchor. New lints `group-transit` (Qwen) and `low-contrast` (GLM).
- Diagnostics give segment indices and spans for `cross`/`shared-segment`, the nearest free lane, and clear `t` values for labels.
- `--focus <ids>`, a `kashika=1` version marker, and a "comb" recipe in `ref` (Qwen: the most-wanted example).
- Rejected: `tidy` alignment and rhythm lint (subjective), repetition sugar and auto-sized boxes (YAGNI or auto-layout drift). Qwen preferred keeping a signed `bend`, but the other 5 preferred `left/right`, so that stays.

## Fix to the review packet
- **A1 was impossible.** LB, Postgres and Kafka are each connected to all three apps, which forms K₃,₃, a graph that can't be drawn without crossings. DeepSeek brute-forced the minimum as 2 crossings for its layout. Next packet: "minimize crossings and justify any that remain".
- The spec's own example had a diagonal tail (`h 80 -> vpc.worker.top`), and the `inspect` example's `len=310` was wrong (it's ≈263). This shows that hand-checked examples can't be trusted, so every example in `ref` will be a tested fixture.

## Decisions

| Topic | Agree | v1 decision |
|---|---|---|
| Axis-target moves and elbow segments | 4/4 (#1 for 3 of them) | `x <X>`, `y <Y>` (number or point), `hv <point>`, `vh <point>` |
| Relative box placement | 4/4 | `at <anchor>+(dx,dy)` plus `pin=<anchor>`; cycles are errors |
| Diagonal final segment | 4/4 | `diagonal-tail` warning plus an `ortho` line assertion |
| Frame of literal coordinates in lines | 4/4 flagged | One rule: every literal is in the frame of the group where it's written |
| Id scoping | 3/4 flagged | Ids are unique across the file and referenced by bare id; no dotted paths |
| `#` comments | 3 for `//`, 1 for keeping `#` | `//` |
| `dot` overload | 4/4 | `stroke-style=solid\|dashed\|dotted` |
| Line labels | 4/4 | `label=` / `label@t=`, with t measured along the final path's arc length |
| Curve bend sign | 4/4 | `curve left:N <pt>` / `right:N`, defined on screen with an example; the curve now carries its target point |
| Corner names | split | `tl` is canonical; `top-left` etc. are accepted as aliases |
| Multi-line statements | 4/4 | A continuation line starts with `->`; an attribute line after the last segment also continues |
| Non-rect anchors | 4/4 | Axis projection, with outlines defined in `ref shapes`; `cloud`/`person` deferred |
| Text metrics | 4/4 | Bundled Inter, documented padding and text areas, `kashika measure` |
| Crossing semantics | 3/4 | Shared endpoints and junctions aren't crossings; tangency is a crossing; a 0.5px tolerance |
| Buses / non-planar graphs | 4/4 | `junction` statement |
| `through` and groups | 3/4 | Groups are never obstacles; leaving and re-entering the line's own endpoint box is flagged |
| Near-misses (lines running along edges, near-parallel lines, crowded ports) | 4/4 | New `near` and `port-crowding` lints with configurable `clearance` |
| Paint order and occlusion | 2/4 | Fixed layers: groups → boxes → lines → labels |
| Suppression hides drift | 2/4 (DeepSeek's top concern) | `allow=cross:<other-id>`, `reason=`, suppressed counts in every summary |
| Hints are local and miss other obstacles | 3/4 | `free` intervals consider every obstacle; `inspect --corridors`; `suggestions` list a `ripple` of affected ids |
| `fix` | 3/4 say defer or risky | Deferred to after v1 |
| Render on error | 2/4 | `--force` writes output with error markers; writes are atomic and `output.written` is reported |
| Exit code on warnings | 2/4 | `--fail-on error\|warn\|never` |
| JSON contract | 3/4 | One versioned envelope with input hash, counts, truncation and metrics; facts in both canvas and local frames |
| `inspect` | 4/4 essential | Flagship command: JSON, `--ports`, `--corridors`, label boxes, dependency graph, works when the file has errors |
| `ref` size | 1.2k–2.5k | Default page ~1.5–2k words, topic pages, `--json`, a "not supported" list, limits of a clean check |
| `explain` | split | Folded into `ref <lint-code>` |
| `grid` vs `@t` | 1/4 (DeepSeek) | `grid` checks only authored literals |
| Style override | 1/4 | Key by key; precedence is inline > `style=` > built-in |

## Adopted after round 2: `kashika route`
DeepSeek ranked a routing oracle #2, and Qwen and GLM both reported lane planning as their dominant cost. v1 includes `kashika route <file> <from> <to> [corner|arc|bezier]`. It returns up to 3 ranked, ready-to-paste candidates in the chosen style, never edits the file, and searches only a bounded set of paths (≤2 elbows or a single curve).

## Not adopted for v1
- **Arithmetic in literals and named guides.** Relative placement plus `x`/`y` targets cover most of the need.
- **`diff --geometry`, a legend primitive, raw SVG shapes, includes, mid-path arrows:** deferred.
- **Semantic checks** (arrow direction, wrong port): out of scope. `ref` says so.
