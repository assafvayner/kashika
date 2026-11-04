import { expect, it } from "vitest";

import { check } from "../../src/check/index.js";
import { startDir, endDir, segTo, Pt } from "../../src/geom/index.js";
import { Options, type Scene, compile } from "../../src/model/index.js";
import {
  type Candidate,
  Corridor,
  type Request,
  type RouteError,
  type Style,
  type Suggestion,
  suggest,
} from "../../src/route/index.js";
import { type Result } from "../../src/diag/rust.js";

const WALL =
  'diagram "w" size (460,240)\n' +
  'box a "A" at (20,100) size (100,40)\n' +
  'box wall "W" at (200,40) size (40,160)\n' +
  'box b "B" at (320,100) size (100,40)\n';

function scene(src: string): Scene {
  const [s, d] = compile(src);
  expect(d).toEqual([]);
  return s;
}

const DIAG =
  'diagram "c" size (420,220)\n' +
  "box a at (20,20) size (100,40)\n" +
  "box b at (300,160) size (100,40)\n";

const OPTS = new Options();

function req(src: string, from: string, to: string, style: Style): Request {
  return { src, from, to, style, line: null, radius: 8.0, max: 3, opts: OPTS };
}

function unwrap(r: Result<Suggestion, RouteError>): Suggestion {
  if (!r.ok) throw new Error(JSON.stringify(r.error));
  return r.value;
}

function corridor(axis: "x" | "y", band: [number, number]): Corridor {
  return new Corridor(axis, band);
}

it("routes_over_the_wall", () => {
  const s = unwrap(suggest(req(WALL, "a.top", "b.top", "corner")));
  expect(
    s.candidates.filter((c) => c.segments <= 3).length,
    "every other plan with at most 2 elbows enters a box or leaves the wrong way",
  ).toBe(1);
  const c = s.candidates[0]!;
  expect(c.statement).toBe("line a_b a.top -> y wall.tl+(0,-12) -> hv b.top end=arrow");
  expect([c.crossings, c.near, c.segments, c.length]).toEqual([0, 0, 3, 444.0]);
  expect(c.corridor).toEqual(corridor("y", [0.0, 34.0]));
});

it("detours_around_a_wall_between_facing_sides", () => {
  const r = req(WALL, "a.right", "b.left", "corner");
  r.max = 20;
  const s = unwrap(suggest(r));
  const c = s.candidates[0]!;
  expect([c.crossings, c.near, c.segments]).toEqual([0, 0, 5]);
  expect(
    c.statement.startsWith("line a_b a.right -> x ") && c.statement.includes(" -> vh b.left"),
    c.statement,
  ).toBe(true);
  expect(c.corridor).not.toBeNull();
  agreesWithCheck(WALL, s);
});

const QUEUE =
  'diagram "q" size (520,340)\n' +
  "box orders at (30,100) size (130,60)\n" +
  "box queue at (230,100) size (170,60)\n" +
  "box db at (370,200) size (100,100)\n";

it("detours_around_a_box_into_a_top_anchor", () => {
  const r = req(QUEUE, "orders.right", "db.top", "corner");
  r.max = 20;
  const s = unwrap(suggest(r));
  const c = s.candidates[0]!;
  expect([c.crossings, c.near, c.segments]).toEqual([0, 0, 4]);
  expect(c.statement).toBe(
    "line orders_db orders.right -> x orders.tr+(12,0) -> y db.tl+(0,-12) -> hv db.top end=arrow",
  );
  agreesWithCheck(QUEUE, s);
});

it("detours_in_arc_style_check_clean", () => {
  for (const [src, from, to] of [
    [WALL, "a.right", "b.left"],
    [QUEUE, "orders.right", "db.top"],
    [QUEUE, "db.left", "orders.bottom"],
  ] as const) {
    for (const style of ["corner", "arc"] as const) {
      const r = req(src, from, to, style);
      r.max = 50;
      const s = unwrap(suggest(r));
      expect(s.candidates[0]!.crossings, `${from} ${to}`).toBe(0);
      agreesWithCheck(src, s);
    }
  }
});

it("no_candidate_when_the_start_faces_a_box", () => {
  const src =
    'diagram "b" size (300,200)\n' +
    "box a at (20,80) size (100,40)\n" +
    "box c at (122,20) size (40,160)\n" +
    "box b at (220,80) size (60,40)\n";
  const s = unwrap(suggest(req(src, "a.right", "b.left", "corner")));
  expect(s.candidates).toEqual([]);
});

it("simple_paths_outrank_clean_detours", () => {
  let r = req(WALL, "a.top", "b.top", "corner");
  r.max = 50;
  let s = unwrap(suggest(r));
  expect(s.candidates[0]!.segments).toBe(3);
  for (const c of s.candidates.filter((c) => c.segments > 3)) {
    expect(
      s.candidates.every(
        (d) => d.segments > 3 || d.crossings > c.crossings || d.near > c.near || d.length > c.length,
      ),
      `a simpler path is as good as ${c.statement}`,
    ).toBe(true);
  }
  r = req(QUEUE, "orders.bottom", "db.left", "corner");
  r.max = 50;
  s = unwrap(suggest(r));
  expect(s.candidates[0]!.statement).toBe("line orders_db orders.bottom -> vh db.left end=arrow");
});

/// An `n` × `n` grid of boxes with a line between the first two of each row.
function grid(n: number): string {
  const side = 40 + 120 * n;
  let src = `diagram "big" size (${side},${side})\n`;
  for (let i = 0; i < n; i++) {
    for (let j = 0; j < n; j++) {
      src += `box b${i}_${j} at (${40 + 120 * j},${40 + 120 * i}) size (80,60)\n`;
    }
    src += `line r${i} b${i}_0.right -> b${i}_1.left\n`;
  }
  return src;
}

it("route_stays_fast_on_a_large_diagram", () => {
  let src = grid(20);
  const start = performance.now();
  let s = unwrap(suggest(req(src, "b0_0.bottom", "b1_0.top", "corner")));
  const took = performance.now() - start;
  expect(s.candidates[0]!.statement).toBe("line b0_0_b1_0 b0_0.bottom -> b1_0.top end=arrow");
  expect(took, `route took ${took}ms`).toBeLessThan(10000);

  src = grid(5);
  const r = req(src, "b0_0.right", "b4_4.left", "corner");
  r.max = 5;
  s = unwrap(suggest(r));
  expect([s.candidates[0]!.crossings, s.candidates[0]!.segments]).toEqual([0, 5]);
  agreesWithCheck(src, s);
});

it("counts_crossings_with_other_lines", () => {
  const src = `${WALL}line blocker (160,10) -> (160,230)\n`;
  const s = unwrap(suggest(req(src, "a.top", "b.top", "corner")));
  const c = s.candidates[0]!;
  expect(c.crossings).toBe(1);
  expect(c.crossed[0]!.line).toBe("blocker");
  expect(c.crossed[0]!.at).toEqual(new Pt(160.0, 28.0));
});

it("counts_near_misses_along_a_box_edge", () => {
  const src =
    'diagram "n" size (300,200)\n' +
    "box a at (20,20) size (40,40)\n" +
    "box b at (220,20) size (40,40)\n" +
    "box c at (100,76) size (40,40)\n";
  const r = req(src, "a.bottom", "b.bottom", "corner");
  r.max = 50;
  const s = unwrap(suggest(r));
  const close = s.candidates.find((c) => c.statement.includes("a.bottom -> y a.bl+(0,12) -> hv"));
  expect(close, "a lane just below a and b").toBeDefined();
  expect(close!.near, "lane at y=72 runs 4px above c").toBeGreaterThanOrEqual(1);
});

it("avoids_a_lane_on_a_group_border", () => {
  const src =
    'diagram "g" size (440,240)\n' +
    "box a at (20,20) size (100,40)\n" +
    'group vpc "VPC" at (200,110) size (220,120) style=dashed {\n}\n' +
    "box b at (300,160) size (100,40)\n";
  const r = req(src, "a.bottom", "b.top", "corner");
  r.max = 10;
  const s = unwrap(suggest(r));
  const border = s.candidates.find((c) => c.statement.includes("-> vhv b.top"));
  expect(border, "the midpoint lane on the VPC's top border").toBeDefined();
  expect(border!.near).toBe(1);
  expect(s.candidates[0]!.statement).toBe("line a_b a.bottom -> y a.bl+(0,12) -> hv b.top end=arrow");
  expect(s.candidates[0]!.corridor).toEqual(corridor("y", [66.0, 104.0]));
});

const TRANSIT =
  'diagram "groups" size (560,260)\n' +
  'box a "A" at (20,100) size (80,50)\n' +
  'box b "B" at (460,100) size (80,50)\n' +
  'group ns "namespace" at (170,40) size (220,170) {\n' +
  '  box p "pod" at (70,110) size (80,40)\n' +
  "}\n" +
  "line ab a.right -> b.left end=arrow\n";

it("detours_around_a_group_instead_of_transiting_it", () => {
  const r = rerouteReq(TRANSIT, "ab");
  r.max = 20;
  const s = unwrap(suggest(r));
  const c = s.candidates[0]!;
  expect([c.crossings, c.transits, c.near]).toEqual([0, 0, 0]);
  expect(c.transited).toEqual([]);
  const straight = s.candidates.find((c) => c.statement === "line ab a.right -> b.left end=arrow");
  expect(straight, "the straight path, ranked lower").toBeDefined();
  expect([straight!.crossings, straight!.transits]).toEqual([0, 1]);
  expect(straight!.transited[0]!.group).toBe("ns");
  expect(straight!.transited[0]!.at).toEqual(new Pt(170.0, 125.0));
  const src = TRANSIT.replace("line ab a.right -> b.left end=arrow\n", "");
  agreesWithCheck(src, s);
});

const CROSS_OR_TRANSIT =
  'diagram "t" size (490,260)\n' +
  "box a at (10,20) size (90,40)\n" +
  "box b at (400,200) size (80,40)\n" +
  'group g "G" at (150,100) size (200,60) {\n}\n' +
  "line w1 (0,130) -> (140,130)\n" +
  "line w2 (360,130) -> (490,130)\n";

it("a_transit_ranks_the_same_as_a_crossing", () => {
  const r = req(CROSS_OR_TRANSIT, "a.right", "b.left", "corner");
  r.max = 20;
  const s = unwrap(suggest(r));
  const rows: [number, number, number, number, string][] = s.candidates.map((c) => [
    c.crossings,
    c.transits,
    c.near,
    c.segments,
    c.statement,
  ]);
  expect(
    rows.every(([c, t, n, seg]) => c + t === 1 && n === 0 && seg === 3),
    "every lane crosses `w1` or `w2`, or transits `g`, with the same length",
  ).toBe(true);
  expect(rows.slice(0, 3), "a tie, broken by lanes on the endpoints' own boxes").toEqual([
    [1, 0, 0, 3, "line a_b a.right -> x a.tr+(12,0) -> vh b.left end=arrow"],
    [1, 0, 0, 3, "line a_b a.right -> x b.tl+(-12,0) -> vh b.left end=arrow"],
    [0, 1, 0, 3, "line a_b a.right -> hvh b.left end=arrow"],
  ]);
  expect(s.candidates[2]!.transited[0]!.group).toBe("g");
  agreesWithCheck(CROSS_OR_TRANSIT, s);
});

const STACKED =
  'diagram "s" size (600,400)\n' +
  'group upper "U" at (40,20) size (300,160) {\n}\n' +
  'group lower "L" at (40,220) size (520,160) {\n' +
  "  box t at (60,10) size (80,40)\n" +
  "}\n" +
  "box p at (420,40) size (100,50)\n";

it("one_lane_through_the_gap_between_two_groups", () => {
  const r = req(STACKED, "p.bottom", "t.top", "corner");
  r.max = 20;
  const s = unwrap(suggest(r));
  const c = s.candidates[0]!;
  expect([c.crossings, c.transits, c.near, c.segments]).toEqual([0, 0, 0, 3]);
  expect(c.statement).toBe("line p_t p.bottom -> y 200 -> hv t.top end=arrow");
  expect(c.corridor).toEqual(corridor("y", [186.0, 214.0]));
  agreesWithCheck(STACKED, s);
});

it("default_id_avoids_existing_ids", () => {
  const src = `${WALL}junction a_b at (10,10)\n`;
  const s = unwrap(suggest(req(src, "a.top", "b.top", "corner")));
  expect(s.candidates[0]!.statement.startsWith("line a_b_2 ")).toBe(true);
});

it("arc_adds_radius_and_counts_corners", () => {
  const s = unwrap(suggest(req(WALL, "a.top", "b.top", "arc")));
  const c = s.candidates[0]!;
  expect(c.statement.endsWith("end=arrow radius=8"), c.statement).toBe(true);
  expect(c.segments).toBe(3);
});

it("bezier_suggestions_are_curves_that_leave_and_enter_square", () => {
  const s = unwrap(suggest(req(DIAG, "a.right", "b.left", "bezier")));
  expect(s.candidates.length).toBeGreaterThan(0);
  expect(s.candidates.every((c) => c.statement.includes(" -> curve "))).toBe(true);
  expect(
    s.candidates[0]!.statement.includes("curve via a.right+(76,0) b.left+(-76,0)"),
  ).toBe(true);
});

it("routes_under_a_diagram_ortho_check_clean", () => {
  const src = DIAG.replace("size (420,220)", "size (420,220) ortho");
  let s = unwrap(suggest(req(src, "a.right", "b.left", "bezier")));
  expect(s.candidates[0]!.statement.endsWith(" end=arrow ortho=false")).toBe(true);
  agreesWithCheck(src, s);
  s = unwrap(suggest(req(src, "a.right", "b.left", "corner")));
  expect(s.candidates[0]!.statement.includes("ortho")).toBe(false);
  agreesWithCheck(src, s);
  let line = `${src}arrow ab a.right -> b.left ortho=false\n`;
  s = unwrap(reroute(line, "ab", "bezier"));
  expect(s.candidates[0]!.statement.endsWith(" ortho=false")).toBe(true);
  expect(s.candidates[0]!.statement.split("ortho").length - 1).toBe(1);
  line = `${DIAG}arrow ab a.right -> vh b.left ortho\n`;
  s = unwrap(reroute(line, "ab", "bezier"));
  expect(s.candidates[0]!.statement.includes("ortho")).toBe(false);
});

it("collinear_lanes_merge_into_the_straight_path", () => {
  const src =
    'diagram "s" size (300,120)\n' +
    "box a at (20,20) size (40,40)\n" +
    "box b at (220,20) size (40,40)\n";
  const r = req(src, "a.right", "b.left", "corner");
  r.max = 20;
  const s = unwrap(suggest(r));
  const statements = s.candidates.filter((c) => c.segments <= 3).map((c) => c.statement);
  expect(statements).toEqual(["line a_b a.right -> b.left end=arrow"]);
});

it("curves_leave_and_enter_side_anchors_along_the_normal", () => {
  const r = req(DIAG, "a.right", "b.top", "bezier");
  r.max = 20;
  const s = unwrap(suggest(r));
  expect(s.candidates.length).toBeGreaterThan(0);
  for (const c of s.candidates) {
    const lineScene = scene(`${DIAG}${c.statement}\n`);
    const path = lineScene.lines[0]!.path;
    const start = startDir(path[0]!);
    const end = endDir(path[path.length - 1]!);
    expect(start.x > 0.999, `leaves a.right horizontally: ${c.statement}`).toBe(true);
    expect(end.y > 0.999, `enters b.top vertically: ${c.statement}`).toBe(true);
  }
});

const GROUPED =
  'diagram "g" size (400,300)\n' +
  'group g "G" at (100,100) size (280,180) {\n' +
  '  box a "A" at (20,40) size (60,40)\n' +
  '  box b "B" at (180,40) size (60,40)\n' +
  '  line ab a.right -> (180,60) end=open label="x" radius=4\n' +
  "}\n";

function rerouteReq(src: string, id: string): Request {
  return { src, from: null, to: null, style: "corner", line: id, radius: 8.0, max: 3, opts: OPTS };
}

function reroute(src: string, id: string, style: Style): Result<Suggestion, RouteError> {
  return suggest({ ...rerouteReq(src, id), style });
}

it("reroutes_an_existing_line_in_its_frame_and_keeps_attributes", () => {
  const s = unwrap(reroute(GROUPED, "ab", "corner"));
  expect(s.candidates[0]!.statement).toBe('line ab a.right -> (180,60) end=open label="x"');
  expect(s.from.at).toEqual(new Pt(180.0, 160.0));
  expect(s.to.at).toEqual(new Pt(280.0, 160.0));
});

it("rerouted_line_is_not_its_own_obstacle", () => {
  const src = `${WALL}line ab a.top -> y 28 -> hv b.top\n`;
  const s = unwrap(reroute(src, "ab", "arc"));
  expect(s.candidates[0]!.crossings).toBe(0);
  expect(s.candidates[0]!.near).toBe(0);
  expect(s.candidates[0]!.statement.startsWith("line ab a.top -> ")).toBe(true);
});

it("unknown_line_is_a_usage_error", () => {
  const r = reroute(WALL, "nope", "corner");
  expect(!r.ok && r.error.kind === "Usage").toBe(true);
});

it("mixed_points_in_a_group_keep_their_text_and_frame", () => {
  const src =
    'diagram "m" size (400,300)\n' +
    'group g "G" at (100,100) size (280,180) {\n' +
    '  box a "A" at (20,40) size (60,40)\n' +
    '  box b "B" at (180,100) size (60,40)\n' +
    "  line ab a.bottom -> (b.left.x, 120) end=arrow\n" +
    "}\n";
  const s = unwrap(reroute(src, "ab", "corner"));
  expect(s.to.expr).toBe("(b.left.x, 120)");
  expect(s.to.at).toEqual(new Pt(280.0, 220.0));
  expect(s.candidates.length).toBeGreaterThan(0);
  for (const c of s.candidates) {
    expect(c.statement.endsWith("(b.left.x, 120) end=arrow"), c.statement).toBe(true);
    const replaced = src.replaceAll("  line ab a.bottom -> (b.left.x, 120) end=arrow", c.statement);
    const [sc, diags] = compile(replaced);
    expect(diags, c.statement).toEqual([]);
    const path = sc.lines[0]!.path;
    expect(segTo(path[path.length - 1]!)).toEqual(new Pt(280.0, 220.0));
  }
});

/// Every candidate's numbers are what `kashika check` reports once the statement is pasted.
function agreesWithCheck(src: string, s: Suggestion): void {
  expect(s.candidates.length, `no candidates for\n${src}`).toBeGreaterThan(0);
  for (const c of s.candidates) {
    const id = c.statement.slice("line ".length).split(" ")[0]!;
    const [sc, diags] = compile(`${src}${c.statement}\n`);
    expect(diags, c.statement).toEqual([]);
    const mine = check(sc).filter((d) => d.elements.some((e) => e.id === id));
    const count = (codes: string[]) => mine.filter((d) => codes.includes(d.code)).length;
    expect(count(["through"]), c.statement).toBe(0);
    expect(count(["cross"]), c.statement).toBe(c.crossings);
    expect(count(["group-transit"]), c.statement).toBe(c.transits);
    expect(count(["near", "shared-segment"]), c.statement).toBe(c.near);
  }
}

const DIAMOND =
  'diagram "d" size (320,240)\n' +
  "box a at (20,70) size (60,40)\n" +
  "box b at (150,0) size (80,40)\n" +
  "box d at (160,80) size (120,120) shape=diamond\n";

it("routes_through_the_bounding_box_corner_of_a_diamond", () => {
  const s = unwrap(suggest(req(DIAMOND, "a.right", "b.bottom", "corner")));
  const statements = s.candidates.filter((c) => c.segments <= 3).map((c) => c.statement);
  expect(statements).toEqual(["line a_b a.right -> hv b.bottom end=arrow"]);
  expect([s.candidates[0]!.crossings, s.candidates[0]!.near]).toEqual([0, 0]);
  agreesWithCheck(DIAMOND, s);
});

it("routes_over_a_cylinder_and_an_image", () => {
  for (const wall of [
    'box wall "W" at (200,40) size (40,160) shape=cylinder',
    'image wall "server" at (200,40) size (40,160)',
  ]) {
    const src = WALL.replaceAll('box wall "W" at (200,40) size (40,160)', wall);
    const s = unwrap(suggest(req(src, "a.top", "b.top", "corner")));
    expect(s.candidates[0]!.statement, wall).toBe(
      "line a_b a.top -> y wall.tl+(0,-12) -> hv b.top end=arrow",
    );
    agreesWithCheck(src, s);
  }
});

it("candidate_numbers_match_kashika_check", () => {
  const blocked = `${WALL}line blocker (160,10) -> (160,230)\n`;
  const near =
    'diagram "n" size (300,200)\n' +
    "box a at (20,20) size (40,40)\n" +
    "box b at (220,20) size (40,40)\n" +
    "box c at (100,76) size (40,40)\n";
  const cases: [string, string, string, Style][] = [
    [WALL, "a.top", "b.top", "corner"],
    [WALL, "a.top", "b.top", "arc"],
    [blocked, "a.top", "b.top", "corner"],
    [near, "a.bottom", "b.bottom", "corner"],
    [DIAG, "a.right", "b.left", "bezier"],
    [DIAG, "a.bottom", "b.left", "corner"],
  ];
  for (const [src, from, to, style] of cases) {
    const r = req(src, from, to, style);
    r.max = 20;
    agreesWithCheck(src, unwrap(suggest(r)));
  }
});

it("reroutes_a_line_through_a_wall_with_a_detour_in_its_group", () => {
  const line = "  line ab a.right -> b.left end=arrow";
  const src =
    'diagram "g" size (560,400)\n' +
    'group g "G" at (60,60) size (480,300) {\n' +
    "  box a at (20,120) size (100,40)\n" +
    "  box wall at (200,40) size (40,220)\n" +
    "  box b at (320,120) size (100,40)\n" +
    `${line}\n` +
    "}\n";
  const r: Request = {
    src,
    from: null,
    to: null,
    style: "corner",
    line: "ab",
    radius: 8.0,
    max: 10,
    opts: OPTS,
  };
  const s = unwrap(suggest(r));
  expect([s.candidates[0]!.crossings, s.candidates[0]!.segments]).toEqual([0, 5]);
  for (const c of s.candidates) {
    expect(c.statement.startsWith("line ab a.right -> "), c.statement).toBe(true);
    const replaced = src.replaceAll(line, `  ${c.statement}`);
    const [sc, diags] = compile(replaced);
    expect(diags, c.statement).toEqual([]);
    const through = check(sc).filter((d) => d.code === "through").length;
    expect(through, c.statement).toBe(0);
  }
});

it("crossings_name_unnamed_lines_as_check_does", () => {
  const src = `${WALL}line ab a.top -> b.top\nline (160,10) -> (160,230)\n`;
  const s = unwrap(reroute(src, "ab", "corner"));
  const c = s.candidates[0]!;
  expect(c.crossings).toBe(1);
  expect(c.crossed[0]!.line).toBe("line@L6");
  const replaced = src.replaceAll("line ab a.top -> b.top", c.statement);
  const [sc] = compile(replaced);
  expect(sc.lines.some((l) => l.id === "line@L6")).toBe(true);
});

it("existing_lines_between_the_same_shapes_are_listed", () => {
  const src =
    `${WALL}line ab a.top -> y 20 -> hv b.top\n` +
    "line ba b.bottom -> y 220 -> hv a.bottom\n" +
    "line aw a.right -> wall.left\n";
  let s = unwrap(suggest(req(src, "a.right", "b.left", "corner")));
  expect(s.existingLines).toEqual(["ab", "ba"]);
  s = unwrap(reroute(src, "ab", "corner"));
  expect(s.existingLines).toEqual([]);
});

it("mixed_points_from_the_command_line_keep_their_text", () => {
  const s = unwrap(suggest(req(DIAG, "(a.right.x, 100)", "b.left", "corner")));
  expect(s.candidates.length).toBeGreaterThan(0);
  for (const c of s.candidates) {
    expect(c.statement.startsWith("line p_b (a.right.x, 100) -> "), c.statement).toBe(true);
  }
  agreesWithCheck(DIAG, s);
});

it("control_points_follow_an_endpoint_written_with_spaces", () => {
  const s = unwrap(suggest(req(DIAG, "a.right + (0, 4)", "b.left", "bezier")));
  expect(s.candidates[0]!.statement).toBe(
    "line a_b a.right + (0, 4) -> curve via a.right+(75,4) b.left+(-75,0) b.left end=arrow",
  );
  agreesWithCheck(DIAG, s);
});

it("probes_carry_the_rerouted_line_attributes", () => {
  const line = 'line ab a.top -> b.top start=dot end=arrow label="x" radius=4';
  const src = `${WALL}${line}\nline blocker (160,10) -> (160,230)\n`;
  for (const style of ["corner", "arc"] as const) {
    const s = unwrap(
      suggest({ src, from: null, to: null, style, line: "ab", radius: 6.0, max: 10, opts: OPTS }),
    );
    expect(s.candidates.length).toBeGreaterThan(0);
    for (const c of s.candidates) {
      expect(c.statement.includes('start=dot end=arrow label="x"'), c.statement).toBe(true);
      const [sc, diags] = compile(src.replaceAll(line, c.statement));
      expect(diags, c.statement).toEqual([]);
      const mine = check(sc).filter((d) => d.elements.some((e) => e.id === "ab"));
      const count = (codes: string[]) => mine.filter((d) => codes.includes(d.code)).length;
      expect(count(["cross"]), c.statement).toBe(c.crossings);
      expect(count(["near", "shared-segment"]), c.statement).toBe(c.near);
    }
  }
});

it("keeps_only_the_allow_entries_each_suggestion_needs", () => {
  const line = 'line ab a.right -> b.left end=arrow allow=cross:l_feat,overlap reason="bus"';
  const src =
    'diagram "r" size (460,240)\n' +
    'box a "A" at (20,100) size (100,40)\n' +
    'box b "B" at (320,100) size (100,40)\n' +
    "line l_feat (220,60) -> (220,180)\n" +
    `${line}\n`;
  const s = unwrap(suggest({ ...rerouteReq(src, "ab"), max: 30 }));
  const clear = s.candidates[0]!;
  expect(clear.crossings).toBe(0);
  expect(clear.droppedAllow).toEqual(["cross:l_feat", "overlap"]);
  expect(clear.statement.endsWith(" end=arrow"), clear.statement).toBe(true);
  const crossing = s.candidates.find((c) => c.crossings === 1);
  expect(crossing, "the straight path is a candidate").toBeDefined();
  expect(crossing!.statement).toBe(
    'line ab a.right -> b.left end=arrow allow=cross:l_feat reason="bus"',
  );
  expect(crossing!.droppedAllow).toEqual(["overlap"]);
  for (const c of s.candidates) {
    const [sc, diags] = compile(src.replaceAll(line, c.statement));
    expect(diags, c.statement).toEqual([]);
    expect(
      check(sc)
        .filter((d) => d.code === "cross")
        .every((d) => d.suppressed),
      c.statement,
    ).toBe(true);
  }
});

it("keeps_reason_without_allow", () => {
  const src = `${WALL}line ab a.top -> y 28 -> hv b.top reason="x"\n`;
  const s = unwrap(reroute(src, "ab", "corner"));
  expect(s.candidates[0]!.statement.endsWith(' reason="x"')).toBe(true);
  expect(s.candidates[0]!.droppedAllow).toEqual([]);
});

it("ties_prefer_lanes_on_the_endpoint_boxes", () => {
  const src =
    'diagram "t" size (500,200)\n' +
    "box a at (20,60) size (100,60)\n" +
    "box b at (200,60) size (100,60)\n" +
    "box c at (380,60) size (100,60)\n";
  const r = req(src, "a.right", "c.left", "corner");
  r.max = 10;
  const s = unwrap(suggest(r));
  const onB = (c: Candidate) => c.statement.includes(" b.");
  const score = (c: Candidate) => JSON.stringify([c.crossings, c.near, c.segments, c.length]);
  expect(s.candidates.some(onB), "a lane on b ties with lanes on a and c").toBe(true);
  s.candidates.forEach((early, i) => {
    for (const late of s.candidates.slice(i + 1)) {
      expect(
        !(score(early) === score(late) && onB(early) && !onB(late)),
        `${early.statement} ranks before the equally scored ${late.statement}`,
      ).toBe(true);
    }
  });
});
