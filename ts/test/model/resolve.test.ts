import { describe, expect, test } from "vitest";

import { type Diagnostic } from "../../src/diag/diagnostic.js";
import { Pt, type Rect, segEval } from "../../src/geom/index.js";
import {
  EdgeKind,
  type Line,
  Marker,
  Options,
  Padding,
  PlacementEdge,
  type Scene,
  type Shape,
  SegmentKind,
  compile,
  compileLibraryWith,
} from "../../src/model/index.js";
import { TOOL_VERSION } from "../../src/model/version.js";

const HEAD = 'diagram "t" size (800,600)\n';

function fmt(d: unknown): string {
  return JSON.stringify(d, null, 2);
}

function ok(body: string): Scene {
  const [scene, diags] = compile(`${HEAD}${body}`);
  expect(diags.length, fmt(diags.map((d) => [d.code, d.message]))).toBe(0);
  return scene;
}

function errors(body: string): Diagnostic[] {
  return compile(`${HEAD}${body}`)[1];
}

function dbg(d: Diagnostic[]): string {
  return fmt(d.map((x) => [x.code, x.message, x.help]));
}

function shapeOf(s: Scene, id: string): Shape {
  const x = s.shapes.find((x) => x.id === id);
  if (x === undefined) throw new Error(`no shape ${id}`);
  return x;
}

function lineOf(s: Scene, id: string): Line {
  const x = s.lines.find((x) => x.id === id);
  if (x === undefined) throw new Error(`no line ${id}`);
  return x;
}

const xy = (p: Pt): [number, number] => [p.x + 0, p.y + 0];
const xys = (ps: readonly Pt[]): [number, number][] => ps.map(xy);
const rc = (r: Rect): [number, number, number, number] => [r.x + 0, r.y + 0, r.w + 0, r.h + 0];

function rect(s: Scene, id: string): Rect {
  return shapeOf(s, id).rect;
}

function pts(s: Scene, id: string): Pt[] {
  const l = lineOf(s, id);
  return [l.path[0]!.from, ...l.path.map((p) => p.to)];
}

function junction(s: Scene, id: string): Pt {
  const j = s.junctions.find((j) => j.id === id);
  if (j === undefined) throw new Error(`no junction ${id}`);
  return j.at;
}

function codesOf(d: Diagnostic[]): string[] {
  return d.map((x) => x.code);
}

function codes(body: string): string[] {
  return errors(body).map((d) => d.code);
}

function versionDiags(attr: string): Diagnostic[] {
  return compile(`diagram "t" size (1,1) kashika=${attr}\n`)[1];
}

const near = (a: number, b: number) => Math.abs(a - b) < 1e-9;

describe("resolve", () => {
  test("group_children_use_the_group_frame", () => {
    const s = ok("group g at (100,50) size (300,200) {\n  box a at (10,20) size (40,30)\n}\n");
    expect(rc(rect(s, "a"))).toEqual([110, 70, 40, 30]);
    expect(shapeOf(s, "a").parent).toBe("g");
  });

  test("relative_placement_with_pin", () => {
    const s = ok(
      "box api at (40,40) size (120,60)\nbox db at api.right+(160,0) size (120,60) pin=left\n",
    );
    expect(rc(rect(s, "db"))).toEqual([320, 40, 120, 60]);
  });

  test("forward_references_resolve", () => {
    const s = ok(
      "box db at api.right+(160,0) size (120,60) pin=left\nbox api at (40,40) size (120,60)\n",
    );
    expect(rc(rect(s, "db"))).toEqual([320, 40, 120, 60]);
  });

  test("mixed_point_inside_a_group_frame", () => {
    const s = ok(
      "box a at (200,0) size (100,40)\ngroup g at (0,100) size (400,300) {\n  junction j at (a.left.x, 50)\n}\n",
    );
    expect(xy(s.junctions[0]!.at)).toEqual([200, 150]);
  });

  test("parenthesized_points_resolve_like_bare_ones", () => {
    const src = (p: (x: string) => string) =>
      "box a at (40,20) size (80,40)\n" +
      `group g at ${p("a.center+(0,80)")} size (200,100) {\n  box b at ${p("a.left.x, 10")} size (40,40)\n}\n` +
      `junction fan at ${p("a.right+(60,0)")}\n` +
      `line l ${p("a.bottom+(20,0)")} -> ${p("fan")} -> x ${p("b.right")} -> y ${p("fan+(0,200)")} -> hv ${p("b.left@0.25-(4,0)")} -> curve via ${p("b.top+(10,-20)")} ${p("a.tr")} ${p("a.right")}\n`;
    const bare = ok(src((p) => (p.includes(",") && !p.includes("(") ? `(${p})` : p)));
    const grouped = ok(src((p) => `(${p})`));
    for (const id of ["a", "g", "b"]) {
      expect(rc(rect(grouped, id)), id).toEqual(rc(rect(bare, id)));
    }
    expect(xy(grouped.junctions[0]!.at)).toEqual(xy(bare.junctions[0]!.at));
    expect(xys(pts(grouped, "l"))).toEqual(xys(pts(bare, "l")));
  });

  test("offsets_add_up_after_any_point", () => {
    const s = ok(
      "box a at (40,20) size (80,40)\n" +
        "group g at (100,100) size (200,200) {\n  junction j at (10,20)+(5,0)-(1,1)\n}\n" +
        "junction k at (a.left.x, a.bottom.y)+(0,26)\n" +
        "junction m at ((a.center+(0,4))+(2,0))-(0,1)\n" +
        "junction n at a.center+(1,2)+(3,4)\n" +
        "junction o at ((((((((((((((((a.tl))))))))))))))))\n",
    );
    const at = (id: string) => xy(junction(s, id));
    expect(at("j")).toEqual([114, 119]);
    expect(at("k")).toEqual([40, 86]);
    expect(at("m")).toEqual([82, 43]);
    expect(at("n")).toEqual([84, 46]);
    expect(at("o")).toEqual([40, 20]);
  });

  test("groups_default_to_the_boundary_style", () => {
    const s = ok("group g at (0,0) size (10,10) {\n}\n");
    expect(s.shapes[0]!.style.strokeStyle).toBe("dashed");
  });

  test("user_style_overrides_builtin_key_by_key", () => {
    const s = ok("style service { fill=#ffffff }\nbox a at (0,0) size (1,1) style=service\n");
    expect(s.shapes[0]!.style.fill).toBe("#ffffff");
    expect(s.shapes[0]!.style.stroke).toBe("#1a73e8");
  });

  test("placement_cycle_is_reported_once", () => {
    const d = errors("box a at b.right size (10,10)\nbox b at a.left size (10,10)\n");
    const cycles = d.filter((d) => d.code === "placement-cycle");
    expect(cycles.length, dbg(d)).toBe(1);
    expect(cycles[0]!.message.includes("a -> b -> a")).toBe(true);
  });

  test("unknown_id_suggests", () => {
    const d = errors("box api at (0,0) size (10,10)\nline apo.left -> (50,50)\n");
    expect(d[0]!.code).toBe("unknown-id");
    expect(d[0]!.help).toEqual(["did you mean `api`?"]);
  });

  test("references_to_a_failed_statement_report_only_the_parse_error", () => {
    const d = errors(
      "box fan_a at (0,0) size (10,10)\njunction fan at fan_a.right+(40)\n" +
        "line l1 fan -> fan_a.left\nline l2 fan -> (50,50)\nbox b at fan+(0,4) size (1,1)\n" +
        "line l3 b.right -> (90,90) allow=cross:fan\n",
    );
    expect(d.length, dbg(d)).toBe(1);
    expect(d[0]!.code).toBe("parse");
  });

  test("a_failed_statement_id_can_be_reused", () => {
    const d = errors(
      "box c at (0,0) size (10,10\nbox c at (0,20) size (10,10)\nline l c.right -> ghost.left\n",
    );
    expect(codesOf(d), dbg(d)).toEqual(["parse", "unknown-id"]);
    expect(d[1]!.message).toBe("unknown id `ghost`");
  });

  test("a_failed_group_keeps_its_children_apart", () => {
    const d = errors(
      "group g at (0,0) size (90,90) {\n  box inner at (0,0) size (10,10\n  box ok at (0,20) size (10,10)\n}\n" +
        "line l inner.right -> ok.left\nline m g.left -> (200,200)\n",
    );
    expect(d.length, dbg(d)).toBe(1);
    expect(d[0]!.code).toBe("parse");
  });

  test("breaking_any_one_statement_never_panics_or_cascades", () => {
    const src =
      'diagram "t" size (400,300)\nbox a at (10,10) size (40,30)\n' +
      "group g at (100,10) size (200,200) {\n  box b at (10,40) size (40,30)\n  " +
      "junction j at b.bottom+(0,20)\n}\njunction fan at (a.right+(40,0))\n" +
      'text n "x" at a.bottom+(0,20)\nline l a.right -> fan -> b.left allow=cross:m\n' +
      "line m j -> y 250 -> (20,250)\n";
    const lines = src.split("\n");
    if (lines[lines.length - 1] === "") lines.pop();
    for (let i = 0; i < lines.length; i++) {
      const broken = [...lines];
      broken[i] = `${lines[i]!.replace(/[{ ]+$/, "")} (`;
      const [, d] = compile(broken.join("\n") + "\n");
      expect(
        d.every((d) => d.code !== "unknown-id" && d.code !== "diagram"),
        `line ${i}: ${dbg(d)}`,
      ).toBe(true);
    }
  });

  test("a_broken_diagram_statement_is_not_missing", () => {
    const [, d] = compile('diagram "t" size (800,600\nbox a at (0,0) size (10,10)\n');
    expect(d.length, dbg(d)).toBe(1);
    expect(d[0]!.code).toBe("parse");
    const lib = compileLibraryWith(
      'diagram "t" size (800\nstyle s { fill=#ffffff }\n',
      new Options(),
    );
    expect(lib).toBe(null);
  });

  test("unknown_anchor_suggests", () => {
    const d = errors("box api at (0,0) size (10,10)\nline api.rigth -> (50,50)\n");
    expect(d[0]!.code).toBe("unknown-anchor");
    expect(d[0]!.message).toBe("unknown anchor `rigth` on `api`");
    expect(d[0]!.help).toEqual(["did you mean `right`?"]);
  });

  test("junctions_have_no_anchors", () => {
    const d = errors("junction j at (0,0)\nline j.left -> (1,1)\n");
    expect(d[0]!.code).toBe("unknown-anchor");
  });

  test("boxes_need_an_anchor", () => {
    const d = errors("box a at (0,0) size (1,1)\nline a -> (1,1)\n");
    expect(d[0]!.code).toBe("unknown-anchor");
    expect(d[0]!.message.includes("needs an anchor")).toBe(true);
  });

  test("duplicate_ids", () => {
    const d = errors("box a at (0,0) size (1,1)\nbox a at (5,5) size (1,1)\n");
    expect(d[0]!.code).toBe("duplicate-id");
    expect(d[0]!.message.includes("line 2")).toBe(true);
  });

  test("unknown_attribute_suggests", () => {
    const d = errors("box a at (0,0) size (10,10) fil=#fff\n");
    expect(d[0]!.code).toBe("attr");
    expect(d[0]!.help).toEqual(["did you mean `fill`?"]);
  });

  test("unknown_style_suggests", () => {
    const d = errors("box a at (0,0) size (10,10) style=servce\n");
    expect(d[0]!.code).toBe("unknown-style");
    expect(d[0]!.help).toEqual(["did you mean `service`?"]);
  });

  test("missing_diagram", () => {
    const [, d] = compile("box a at (0,0) size (1,1)\n");
    expect(d[0]!.code).toBe("diagram");
  });

  test("version_accepts_equal_and_older", () => {
    for (const v of ['"0.1.0"', '"0.0.5"']) {
      expect(versionDiags(v).length, v).toBe(0);
    }
  });

  test("version_is_optional", () => {
    expect(compile('diagram "t" size (1,1)\n')[1].length).toBe(0);
  });

  test("version_rejects_newer_and_other_major", () => {
    for (const v of ['"0.2.0"', '"0.1.1"', '"1.0.0"']) {
      const d = versionDiags(v);
      expect(d.length, v).toBe(1);
      expect(d[0]!.code).toBe("version");
      expect(
        d[0]!.message.includes(`or newer (same major version); this is kashika ${TOOL_VERSION}`),
      ).toBe(true);
      expect(d[0]!.help.length > 0).toBe(true);
    }
  });

  test("version_rejects_bare_and_malformed", () => {
    for (const v of [
      "1",
      "2",
      '"1"',
      '"0.1"',
      '"abc"',
      '"0.1.0-beta"',
      '"0.1.0+b1"',
      '"0.1.0.0"',
    ]) {
      const d = versionDiags(v);
      expect(d.length, v).toBe(1);
      expect(d[0]!.code).toBe("version");
      expect(d[0]!.help, v).toEqual(['write kashika="0.1.0"']);
    }
  });

  test("allow_validates_codes_and_ids", () => {
    const d = errors("line l1 (0,0) -> h 5 allow=crosss\nline l2 (0,0) -> v 5 allow=cross:nope\n");
    expect(
      d.some(
        (d) => d.code === "attr" && d.help.length === 1 && d.help[0] === "did you mean `cross`?",
      ),
    ).toBe(true);
    expect(d.some((d) => d.code === "unknown-id")).toBe(true);
  });

  test("shapes_report_their_rects_after_errors_elsewhere", () => {
    const [scene, d] = compile(`${HEAD}box a at (0,0) size (10,10)\nline nope.left -> (1,1)\n`);
    expect(d.length).toBe(1);
    expect(scene.shapes.length).toBe(1);
    expect(scene.lines.length).toBe(0);
  });

  test("rounded_shape_projects_anchors", () => {
    const s = ok("box r at (0,0) size (100,60) shape=rounded\nline l r.top@0 -> (0,-50)\n");
    expect(xy(pts(s, "l")[0]!)).toEqual([0, 8]);
  });

  test("every_segment_kind_resolves", () => {
    const s = ok(
      "box a at (0,0) size (100,60)\nbox b at (300,200) size (100,60)\n" +
        "line l a.right -> h 50 -> y b.top -> x 400 -> v 10 -> hv (500,300) -> vh (520,320)\n",
    );
    expect(xys(pts(s, "l"))).toEqual([
      [100, 30],
      [150, 30],
      [150, 200],
      [400, 200],
      [400, 210],
      [500, 210],
      [500, 300],
      [500, 320],
      [520, 320],
    ]);
  });

  test("already_aligned_elbows_drop_zero_length_pieces", () => {
    const s = ok("line l (0,0) -> hv (100,0)\n");
    expect(s.lines[0]!.path.length).toBe(1);
  });

  test("numeric_axis_targets_use_the_group_frame", () => {
    const s = ok("group g at (100,100) size (300,300) {\n  line l (0,0) -> x 50 -> y 20\n}\n");
    expect(xys(pts(s, "l"))).toEqual([
      [100, 100],
      [150, 100],
      [150, 120],
    ]);
  });

  test("curve_left_bulges_up_when_travelling_right", () => {
    const s = ok("line l (0,100) -> curve left 20 (100,100)\n");
    const seg = s.lines[0]!.path[0]!;
    if (seg.kind !== "quad") throw new Error("expected quad");
    expect(xy(seg.ctrl)).toEqual([50, 60]);
    expect(xy(segEval(seg, 0.5))).toEqual([50, 80]);
  });

  test("curve_right_bulges_right_when_travelling_up", () => {
    const s = ok("line l (0,100) -> curve right 10 (0,0)\n");
    const seg = s.lines[0]!.path[0]!;
    if (seg.kind !== "quad") throw new Error("expected quad");
    expect(xy(seg.ctrl)).toEqual([20, 50]);
  });

  test("curve_via_makes_a_cubic", () => {
    const s = ok("line l (0,0) -> curve via (0,50) (100,50) (100,0)\n");
    expect(s.lines[0]!.path[0]!.kind).toBe("cubic");
  });

  test("radius_rounds_polyline_corners", () => {
    const s = ok("line l (0,0) -> h 100 -> v 100 radius=10\n");
    expect(s.lines[0]!.path.length).toBe(3);
    expect(s.lines[0]!.path[1]!.kind).toBe("quad");
  });

  test("labels_markers_and_unnamed_ids", () => {
    const s = ok('line (0,0) -> h 100 label="a" label@0.25="b" end=arrow start=dot\n');
    const l = s.lines[0]!;
    expect(l.id).toBe("line@L2");
    expect(l.named).toBe(false);
    expect(l.labels.map((x) => x.t)).toEqual([0.5, 0.25]);
    expect([l.start, l.end]).toEqual(["dot", "arrow"]);
  });

  test("ortho_rejects_diagonals", () => {
    const d = errors("line l (0,0) -> (10,10) ortho\n");
    expect(d[0]!.code).toBe("not-ortho");
  });

  test("degenerate_curve", () => {
    const d = errors("line l (0,0) -> curve left 10 (0,0)\n");
    expect(d[0]!.code).toBe("degenerate-curve");
  });

  test("zero_length_line", () => {
    const d = errors("line l (0,0) -> h 0\n");
    expect(d[0]!.code).toBe("zero-length");
  });

  test("failed_segment_stops_the_line", () => {
    expect(codes("line l (0,0) -> nope.left -> curve left 10 (0,0)\n")).toEqual(["unknown-id"]);
  });

  test("non_finite_values_are_rejected", () => {
    const big = "9".repeat(400);
    for (const body of [
      `box a at (${big},0) size (1,1)\n`,
      `box a at (0,0) size (${big},1)\n`,
      `line l (0,0) -> h ${big}\n`,
      `line l (0,0) -> v ${big}\n`,
      `line l (0,0) -> curve left ${big} (10,0)\n`,
      `line l (0,0) -> curve via (${big},0) (1,1) (2,2)\n`,
    ]) {
      const d = errors(body);
      expect(
        d.some((d) => d.code === "range" && d.message === "value is not a finite number"),
        `${body}: ${dbg(d)}`,
      ).toBe(true);
    }
    const [, d] = compile(`diagram "t" size (${big},1)\n`);
    expect(
      d.some((d) => d.message === "value is not a finite number"),
      dbg(d),
    ).toBe(true);
  });

  test("diagram_size_must_be_positive", () => {
    const [, d] = compile('diagram "t" size (0,10)\n');
    expect(
      d.some((d) => d.code === "size"),
      dbg(d),
    ).toBe(true);
  });

  test("group_honors_user_boundary_style", () => {
    const s = ok("style boundary { stroke=#ff0000 }\ngroup g at (0,0) size (10,10) {\n}\n");
    expect(s.shapes[0]!.style.stroke).toBe("#ff0000");
    expect(s.shapes[0]!.style.strokeStyle).toBe("dashed");
  });

  test("duplicate_ids_are_not_emitted_twice", () => {
    const [s, d] = compile(
      `${HEAD}box a at (0,0) size (1,1)\ngroup a at (5,5) size (10,10) {\n  box c at (0,0) size (1,1)\n}\n`,
    );
    expect(d.filter((d) => d.code === "duplicate-id").length).toBe(1);
    expect(s.shapes.filter((x) => x.id === "a").length).toBe(1);
    expect(s.shapes.every((x) => x.id !== "c")).toBe(true);
  });

  test("duplicate_label_positions", () => {
    const d = errors('line l (0,0) -> h 10 label="a" label@0.5="b"\n');
    expect(
      d.filter((d) => d.code === "attr" && d.message.startsWith("duplicate label at t=")).length,
    ).toBe(1);
    const [s] = compile(`${HEAD}line l (0,0) -> h 10 label@0.3="a" label@0.3="b"\n`);
    expect(s.lines[0]!.labels.length).toBe(1);
    expect(s.lines[0]!.labels[0]!.text).toBe("a");
  });

  test("label_side_shifts_the_rect_off_the_line", () => {
    const gap = 1.5 / 2.0 + 4.0;
    let s = ok('line l (0,100) -> h 200 label="a" label-side=above\n');
    const lab = s.lines[0]!.labels[0]!;
    expect(lab.side).toBe("above");
    expect(xy(lab.at)).toEqual([100, 100]);
    expect(near(lab.rect.bottom(), 100.0 - gap)).toBe(true);
    expect(near(lab.rect.center().x, 100.0)).toBe(true);

    s = ok('line l (0,100) -> h 200 stroke-width=4 label="a" label-side=below\n');
    expect(near(s.lines[0]!.labels[0]!.rect.y, 106.0)).toBe(true);

    s = ok('line l (100,0) -> v 200 label="a" label-side=left\n');
    const r = s.lines[0]!.labels[0]!.rect;
    expect(near(r.right(), 100.0 - gap)).toBe(true);
    expect(near(r.center().y, 100.0)).toBe(true);

    s = ok('line l (100,0) -> v 200 label="a" label-side=right\n');
    expect(near(s.lines[0]!.labels[0]!.rect.x, 100.0 + gap)).toBe(true);
  });

  test("label_side_at_a_vertex_uses_the_matching_segment", () => {
    let s = ok('line l (0,0) -> h 100 -> v 100 label="a" label-side=right\n');
    expect(s.lines[0]!.labels[0]!.side).toBe("right");
    s = ok('line l (0,0) -> h 100 -> v 100 label="a" label-side=below\n');
    expect(s.lines[0]!.labels[0]!.side).toBe("below");
  });

  test("label_side_mismatch_falls_back_to_on", () => {
    const body = 'line l (100,0) -> v 200 label="a" label-side=above\n';
    const d = errors(body);
    expect(d.length).toBe(1);
    expect(d[0]!.code).toBe("attr");
    expect(d[0]!.message.includes("vertical segment"), d[0]!.message).toBe(true);
    expect(d[0]!.help).toEqual(["use `label-side=left` or `label-side=right`"]);
    const [s] = compile(`${HEAD}${body}`);
    const lab = s.lines[0]!.labels[0]!;
    expect(lab.side).toBe("on");
    expect(near(lab.rect.center().y, 100.0)).toBe(true);
    expect(s.lines[0]!.labelSide).toBe("above");

    for (const b of [
      'line l (0,0) -> (100,100) label="a" label-side=left\n',
      'line l (0,0) -> curve left 20 (100,0) label="a" label-side=above\n',
    ]) {
      const d = errors(b);
      expect(d.length, dbg(d)).toBe(1);
      expect(d[0]!.help[0]!.startsWith("use `label-side=on`"), dbg(d)).toBe(true);
    }
    expect(errors('line l (0,0) -> h 100 label="a" label-side=top\n')[0]!.code).toBe("attr");
  });

  test("radius_makes_rect_outline_rounded", () => {
    let s = ok("box a at (0,0) size (100,60) radius=8\n");
    expect(s.shapes[0]!.outline).toBe("rounded");
    s = ok("box a at (0,0) size (100,60)\n");
    expect(s.shapes[0]!.outline).toBe("rect");
  });

  test("negative_curve_amount", () => {
    const d = errors("line l (0,0) -> curve left -10 (10,0)\n");
    expect(d[0]!.code).toBe("degenerate-curve");
    expect(d[0]!.message).toBe(
      "curve amount must not be negative; use `left`/`right` to choose the side",
    );
  });

  test("unknown_attributes_fail_clearly", () => {
    const d = errors("box a at (0,0) size (10,10) fil=#fff\n");
    expect(d[0]!.code).toBe("attr");
  });

  test("duplicate_label_attribute_reports_once", () => {
    const d = errors('line l (0,0) -> h 5 label@0.3="a" label@0.3="b"\n');
    expect(d.length, dbg(d)).toBe(1);
    expect(d[0]!.code).toBe("attr");
  });

  test("duplicate_icon_attribute_reports_once", () => {
    const d = errors("box a at (0,0) size (1,1) icon-size=4 icon-size=5\n");
    expect(d.map((x) => x.code).filter((c) => c === "attr").length, dbg(d)).toBe(2);
  });

  test("zero_curve_amount_is_allowed", () => {
    ok("line l (0,0) -> curve left 0 (10,0)\n");
  });

  test("negative_curve_message", () => {
    const d = errors("line l (0,0) -> curve left -1 (10,0)\n");
    expect(
      d.some((x) => x.message.includes("must not be negative")),
      dbg(d),
    ).toBe(true);
  });

  test("children_of_duplicate_group_are_not_reference_targets", () => {
    const d = errors(
      "group g at (0,0) size (100,100) {\n}\ngroup g at (200,0) size (100,100) {\n  box b at (0,0) size (10,10)\n}\nline l (0,0) -> b.left\n",
    );
    expect(codesOf(d), dbg(d)).toEqual(["duplicate-id"]);
  });

  test("grandchildren_of_duplicate_group_are_not_reference_targets", () => {
    let d = errors(
      "group g at (0,0) size (100,100) {\n}\ngroup g at (200,0) size (100,100) {\n  group inner at (0,0) size (50,50) {\n    box b at (0,0) size (10,10)\n  }\n}\nline l (0,0) -> b.left\n",
    );
    expect(codesOf(d), dbg(d)).toEqual(["duplicate-id"]);
    d = errors(
      "group g at (0,0) size (100,100) {\n}\ngroup g at (200,0) size (100,100) {\n  box b at (0,0) size (10,10)\n}\nline l (0,0) -> h 5 allow=cross:b\nline m (0,0) -> bb.left\n",
    );
    expect(
      d.every((d) => !d.message.includes("`b`")),
      dbg(d),
    ).toBe(true);
    expect(
      d.some((d) => d.code === "unknown-id" && d.message.includes("`bb`") && d.help.length === 0),
      `skipped ids are not suggested: ${dbg(d)}`,
    ).toBe(true);
  });

  test("nested_groups_compose_frames", () => {
    const s = ok(
      "group outer at (100,100) size (400,400) {\n  group inner at (50,60) size (200,200) {\n    box a at (5,5) size (10,10)\n    line l (0,0) -> h 10\n  }\n}\n",
    );
    expect(rc(rect(s, "inner"))).toEqual([150, 160, 200, 200]);
    expect(rc(rect(s, "a"))).toEqual([155, 165, 10, 10]);
    expect(xy(pts(s, "l")[0]!)).toEqual([150, 160]);
  });

  test("anchor_references_inside_a_group_are_absolute", () => {
    const s = ok(
      "box a at (300,300) size (100,60)\ngroup g at (100,100) size (100,100) {\n  box b at a.right+(10,0) size (20,20) pin=left\n  junction j at a.tl\n  line l (0,0) -> a.left\n}\n",
    );
    expect(rc(rect(s, "b"))).toEqual([410, 320, 20, 20]);
    expect(xy(junction(s, "j"))).toEqual([300, 300]);
    const p = pts(s, "l");
    expect(xy(p[p.length - 1]!)).toEqual([300, 330]);
  });

  test("references_into_groups_resolve_to_true_positions", () => {
    const s = ok(
      "group g at (100,100) size (300,300) {\n  box a at (10,10) size (40,20)\n}\nline l a.right -> h 50\n",
    );
    expect(xy(pts(s, "l")[0]!)).toEqual([150, 120]);
  });

  test("hv_and_curve_via_points_use_the_group_frame", () => {
    const s = ok(
      "group g at (100,100) size (300,300) {\n  line l (0,0) -> hv (50,50) -> curve via (60,50) (70,60) (70,70)\n}\n",
    );
    expect(xys(pts(s, "l"))).toEqual([
      [100, 100],
      [150, 100],
      [150, 150],
      [170, 170],
    ]);
    const seg = s.lines[0]!.path[2]!;
    if (seg.kind !== "cubic") throw new Error("expected cubic");
    expect([xy(seg.c1), xy(seg.c2)]).toEqual([
      [160, 150],
      [170, 160],
    ]);
  });

  test("text_and_junction_literals_use_the_group_frame", () => {
    const s = ok(
      'group g at (100,50) size (300,300) {\n  junction j at (10,20)\n  text t "hi" at (30,40)\n}\n',
    );
    expect(xy(junction(s, "j"))).toEqual([110, 70]);
    expect(xy(s.texts[0]!.at)).toEqual([130, 90]);
    expect(s.texts[0]!.parent).toBe("g");
  });

  test("negative_moves_go_left_and_up", () => {
    const s = ok("line l (100,100) -> h -30 -> v -40\n");
    expect(xys(pts(s, "l"))).toEqual([
      [100, 100],
      [70, 100],
      [70, 60],
    ]);
  });

  test("axis_targets_take_a_points_coordinate", () => {
    const s = ok(
      "box a at (200,300) size (100,60)\nline l (0,0) -> x a.right -> y a.bottom+(0,10)\n",
    );
    expect(xys(pts(s, "l"))).toEqual([
      [0, 0],
      [300, 0],
      [300, 370],
    ]);
  });

  test("every_anchor_and_alias_resolves", () => {
    const s0 = ok("box a at (100,100) size (100,60)\n");
    const r = rect(s0, "a");
    const cases: [string, [number, number]][] = [
      ["top", [150, 100]],
      ["bottom", [150, 160]],
      ["left", [100, 130]],
      ["right", [200, 130]],
      ["center", [150, 130]],
      ["tl", [100, 100]],
      ["tr", [200, 100]],
      ["bl", [100, 160]],
      ["br", [200, 160]],
      ["top-left", [100, 100]],
      ["top-right", [200, 100]],
      ["bottom-left", [100, 160]],
      ["bottom-right", [200, 160]],
      ["top@0.5", [150, 100]],
      ["top@0", [100, 100]],
      ["bottom@0.25", [125, 160]],
      ["left@0.25", [100, 115]],
      ["right@1", [200, 160]],
    ];
    for (const [name, want] of cases) {
      const s = ok(`box a at (${r.x},${r.y}) size (${r.w},${r.h})\njunction j at a.${name}\n`);
      expect(xy(junction(s, "j")), name).toEqual(want);
    }
  });

  test("anchor_t_and_offset_compose", () => {
    const s = ok(
      "box db at (0,0) size (100,40)\njunction j at db.left@0.25+(0,4)\njunction k at db.top+(-10,0)\n",
    );
    expect(xy(junction(s, "j"))).toEqual([0, 14]);
    expect(xy(junction(s, "k"))).toEqual([40, 0]);
  });

  test("group_anchors_work_like_box_anchors", () => {
    const s = ok(
      "group k8s at (100,100) size (200,100) {\n}\njunction j at k8s.top\nbox b at k8s.right+(20,0) size (10,10) pin=left\n",
    );
    expect(xy(junction(s, "j"))).toEqual([200, 100]);
    expect(rc(rect(s, "b"))).toEqual([320, 145, 10, 10]);
  });

  test("junction_references_take_offsets_and_axes", () => {
    const s = ok(
      "junction j at (40,50)\njunction k at j+(5,-5)\njunction m at (j.x, k.y)\nline l j -> k\n",
    );
    expect(xy(junction(s, "k"))).toEqual([45, 45]);
    expect(xy(junction(s, "m"))).toEqual([40, 45]);
    expect(xys(pts(s, "l"))).toEqual([
      [40, 50],
      [45, 45],
    ]);
  });

  test("mixed_points_take_each_axis_from_a_reference", () => {
    const s = ok(
      "box app1 at (100,0) size (60,40)\njunction lane1 at (0,200)\nline l app1.bottom -> (app1.bottom.x, lane1.y)\n",
    );
    expect(xys(pts(s, "l"))).toEqual([
      [130, 40],
      [130, 200],
    ]);
  });

  test("anchor_position_out_of_range", () => {
    let d = errors("box a at (0,0) size (10,10)\njunction j at a.top@1.5\n");
    expect(d[0]!.code).toBe("unknown-anchor");
    expect(d[0]!.message).toBe("`t` must be between 0 and 1");
    expect(d[0]!.help.length, dbg(d)).toBe(1);
    for (const corner of ["tl", "center", "bottom-right"]) {
      d = errors(`box a at (0,0) size (10,10)\njunction j at a.${corner}@0.5\n`);
      expect(d[0]!.code).toBe("unknown-anchor");
      expect(d[0]!.message).toBe("`@t` only applies to `top`/`bottom`/`left`/`right`");
      expect(d[0]!.help.length, dbg(d)).toBe(1);
    }
  });

  test("every_pin_anchor_lands_on_at", () => {
    const cases: [string, [number, number]][] = [
      ["tl", [100, 100]],
      ["top", [80, 100]],
      ["tr", [60, 100]],
      ["right", [60, 90]],
      ["br", [60, 80]],
      ["bottom", [80, 80]],
      ["bl", [100, 80]],
      ["left", [100, 90]],
      ["center", [80, 90]],
      ["top-left", [100, 100]],
      ["bottom-right", [60, 80]],
    ];
    for (const [pin, want] of cases) {
      const s = ok(`box a at (100,100) size (40,20) pin=${pin}\n`);
      expect(rc(rect(s, "a")), pin).toEqual([want[0], want[1], 40, 20]);
    }
  });

  test("pin_rejects_unknown_anchors_and_positions", () => {
    const d = errors("box a at (0,0) size (10,10) pin=lft\n");
    expect(d[0]!.code).toBe("attr");
    expect(d[0]!.help).toEqual(["did you mean `left`?"]);
  });

  test("pin_accepts_a_position_on_an_edge", () => {
    const s = ok(
      "box api at (40,40) size (120,60)\nbox db at api.right+(100,0) size (100,80) pin=left@0.25\n",
    );
    expect(rc(rect(s, "db"))).toEqual([260, 50, 100, 80]);
  });

  test("pin_position_errors", () => {
    expect(errors("box a at (0,0) size (10,10) pin=tl@0.5\n")[0]!.code).toBe("unknown-anchor");
    expect(errors("box a at (0,0) size (10,10) pin=left@2\n")[0]!.code).toBe("unknown-anchor");
    expect(errors("box a at (0,0) size (10,10) pin=lft@0.5\n")[0]!.code).toBe("attr");
  });

  test("placement_cycle_points_at_the_at_clause", () => {
    const d = errors("box a at b.right size (10,10)\nbox b at a.left size (10,10)\n");
    const c = d.find((d) => d.code === "placement-cycle")!;
    expect([c.span.line, c.span.col]).toEqual([2, 10]);
  });

  test("duplicate_style_has_its_own_code", () => {
    const d = errors("style s { fill=#fff }\nstyle s { fill=#000 }\n");
    expect(d.length, dbg(d)).toBe(1);
    expect(d[0]!.code).toBe("duplicate-style");
    expect(d[0]!.span.line).toBe(3);
    ok("style a { fill=#fff }\nbox a at (0,0) size (1,1) style=a\n");
  });

  test("statements_after_unbuilt_ones_still_resolve", () => {
    const [s, d] = compile(
      `${HEAD}import "lib.kash"\nshape s {\n  path "M 0 0"\n}\nimage i "x.png" at (0,0) size (1,1)\n` +
        'box a at (10,10) size (20,20) padding=0 {\n  text "x"\n}\nbox b at a.right+(10,0) size (20,20) pin=left\nline l a.right -> b.left\n',
    );
    expect(codesOf(d), dbg(d)).toEqual(["file", "parse", "file"]);
    expect(rc(rect(s, "b"))).toEqual([40, 10, 20, 20]);
    expect(xys(pts(s, "l"))).toEqual([
      [30, 20],
      [40, 20],
    ]);
  });

  test("group_pin_moves_its_frame", () => {
    const s = ok("group g at (100,100) size (40,20) pin=center {\n  box a at (0,0) size (1,1)\n}\n");
    expect(rc(rect(s, "g"))).toEqual([80, 90, 40, 20]);
    expect(rc(rect(s, "a"))).toEqual([80, 90, 1, 1]);
  });

  test("self_placement_is_a_cycle", () => {
    const d = errors("box a at a.left size (10,10)\n");
    expect(d[0]!.code).toBe("placement-cycle");
    expect(d[0]!.message.includes("a -> a"), dbg(d)).toBe(true);
  });

  test("group_placed_relative_to_its_own_child_is_a_cycle", () => {
    const d = errors("group g at c.left size (100,100) {\n  box c at (0,0) size (10,10)\n}\n");
    expect(
      d.some((x) => x.code === "placement-cycle"),
      dbg(d),
    ).toBe(true);
  });

  test("mixed_point_cycles_are_detected", () => {
    const d = errors("junction j at (k.x, 0)\njunction k at (j.x, 0)\n");
    expect(d.filter((x) => x.code === "placement-cycle").length, dbg(d)).toBe(1);
  });

  test("ids_are_unique_across_kinds_and_groups", () => {
    for (const body of [
      "box a at (0,0) size (1,1)\nline a (0,0) -> h 1\n",
      'junction a at (0,0)\ntext a "x" at (0,0)\n',
      "box a at (0,0) size (1,1)\ngroup g at (0,0) size (10,10) {\n  junction a at (0,0)\n}\n",
    ]) {
      const d = errors(body);
      expect(d.length, `${body}: ${dbg(d)}`).toBe(1);
      expect(d[0]!.code, body).toBe("duplicate-id");
    }
  });

  test("texts_and_lines_cannot_be_referenced", () => {
    const d = errors('text t "x" at (0,0)\njunction j at t.left\n');
    expect(d[0]!.code).toBe("unknown-id");
  });

  test("inline_attributes_override_named_style", () => {
    const s = ok(
      "style svc { fill=#111111 stroke=#222222 }\nbox a at (0,0) size (1,1) style=svc fill=#333333\n",
    );
    expect(s.shapes[0]!.style.fill).toBe("#333333");
    expect(s.shapes[0]!.style.stroke).toBe("#222222");
  });

  test("group_style_and_inline_apply_over_boundary", () => {
    const s = ok(
      "style blue { stroke=#0000ff }\ngroup g at (0,0) size (10,10) style=blue stroke-style=solid {\n}\n",
    );
    expect(s.shapes[0]!.style.stroke).toBe("#0000ff");
    expect(s.shapes[0]!.style.strokeStyle).toBe("solid");
  });

  test("styles_may_be_defined_after_use", () => {
    const s = ok("box a at (0,0) size (1,1) style=mine\nstyle mine { fill=#abcdef }\n");
    expect(s.shapes[0]!.style.fill).toBe("#abcdef");
  });

  test("every_style_key_is_accepted", () => {
    ok(
      "style s { fill=#fff stroke=#000 stroke-width=2 stroke-style=dotted font-size=12 font-weight=bold color=#111 radius=4 padding=6 muted=#888 }\n",
    );
    const d = errors("style s { fil=#fff }\n");
    expect(d[0]!.code).toBe("attr");
  });

  test("padding_pair_follows_style_precedence", () => {
    const s = ok(
      "style s { padding=(2,4) }\n" +
        "box a at (0,0) size (10,10) style=s\n" +
        "box b at (0,20) size (10,10) style=s padding=6\n" +
        "box c at (0,40) size (10,10) style=s padding=(1,0)\n" +
        "box d at (0,60) size (10,10) style=service\n",
    );
    const pad = (i: number): [number, number] => [
      s.shapes[i]!.style.padding.x,
      s.shapes[i]!.style.padding.y,
    ];
    expect(pad(0)).toEqual([2, 4]);
    const u = Padding.uniform(6);
    expect(pad(1)).toEqual([u.x, u.y]);
    expect(pad(2)).toEqual([1, 0]);
    const u8 = Padding.uniform(8);
    expect(pad(3)).toEqual([u8.x, u8.y]);
  });

  test("padding_pair_errors", () => {
    const range = "`padding` values must be at least 0 and at most 1000000";
    const shape = "`padding` expects a number or a pair `(x,y)`";
    const cases: [string, string][] = [
      ["box a at (0,0) size (10,10) padding=(-1,2)\n", range],
      ["box a at (0,0) size (10,10) padding=(1,2000000)\n", range],
      ["style s { padding=(0,-3) }\n", range],
      ["box a at (0,0) size (10,10) padding=wide\n", shape],
      ['box a at (0,0) size (10,10) padding="8"\n', shape],
    ];
    for (const [body, msg] of cases) {
      const d = errors(body);
      expect(codesOf(d), `${body}: ${dbg(d)}`).toEqual(["attr"]);
      expect(d[0]!.message).toBe(msg);
      expect(d[0]!.help.length).toBe(1);
    }
  });

  test("every_builtin_style_exists", () => {
    for (const name of ["service", "datastore", "external", "queue", "boundary", "note"]) {
      ok(`box a at (0,0) size (1,1) style=${name}\n`);
    }
  });

  test("line_style_applies_only_stroke_and_font_keys", () => {
    const s = ok(
      "style s { fill=#ff0000 stroke=#00ff00 radius=10 font-size=20 }\nline l (0,0) -> h 100 -> v 100 style=s\n",
    );
    const l = s.lines[0]!;
    expect(l.style.stroke).toBe("#00ff00");
    expect(l.style.fontSize).toBe(20.0);
    expect(l.style.fill).not.toBe("#ff0000");
    expect(l.path.length, "style radius must not round lines").toBe(2);
  });

  test("line_inline_stroke_attributes", () => {
    const s = ok("line l (0,0) -> h 10 stroke=#123456 stroke-width=3 stroke-style=dashed\n");
    const st = s.lines[0]!.style;
    expect([st.stroke, st.strokeWidth, st.strokeStyle]).toEqual(["#123456", 3.0, "dashed"]);
    expect(errors("line l (0,0) -> h 10 fill=#fff\n")[0]!.code).toBe("attr");
  });

  test("markers_default_to_none_and_accept_every_kind", () => {
    const s = ok("line l (0,0) -> h 10\n");
    expect([s.lines[0]!.start, s.lines[0]!.end]).toEqual(["none", "none"]);
    for (const m of [
      "arrow",
      "open",
      "diamond",
      "dot",
      "none",
      "one",
      "many",
      "one-many",
      "zero-one",
      "zero-many",
    ]) {
      ok(`line l (0,0) -> h 10 start=${m} end=${m}\n`);
    }
    expect(errors("line l (0,0) -> h 10 end=arow\n")[0]!.code).toBe("attr");
  });

  test("crows_foot_markers_parse_to_their_kinds", () => {
    const s = ok(
      "line l (0,0) -> h 100 start=zero-many end=one-many\n" +
        "line m (0,20) -> h 100 start=zero-one end=many\n" +
        "line n (0,40) -> h 100 end=one\n",
    );
    const got = s.lines.map((l) => [l.start, l.end]);
    expect(got).toEqual([
      ["zero-many", "one-many"],
      ["zero-one", "many"],
      ["none", "one"],
    ]);
    const d = errors("line l (0,0) -> h 10 end=one_many\n");
    expect(d[0]!.help).toEqual(["did you mean `one-many`?"]);
  });

  test("every_marker_name_parses_to_its_marker", () => {
    expect(Marker.NAMES.length).toBe(Marker.ALL.length);
    Marker.ALL.forEach((m, i) => {
      const name = Marker.NAMES[i]!;
      expect(Marker.name(m)).toBe(name);
      expect(Marker.fromName(name)).toBe(m);
      const s = ok(`line l (0,0) -> h 10 end=${name}\n`);
      expect(s.lines[0]!.end, name).toBe(m);
    });
    expect(Marker.fromName("arow")).toBe(null);
  });

  test("crows_foot_parts_sit_on_the_line_and_touch_the_tip", () => {
    const s = ok("line l (0,0) -> h 100 start=zero-one end=many\n");
    const l = s.lines[0]!;
    const foot = l.erParts(false);
    expect(foot.strokes.map((x) => xy(x[1]))).toEqual([
      [100, 6],
      [100, 0],
      [100, -6],
    ]);
    expect(foot.strokes.map((x) => xy(x[0]))).toEqual([
      [88, 0],
      [88, 0],
      [88, 0],
    ]);
    expect(foot.circle).toBe(null);
    const zeroOne = l.erParts(true);
    expect(zeroOne.strokes.map(([a, b]) => [xy(a), xy(b)])).toEqual([
      [
        [8, -6],
        [8, 6],
      ],
    ]);
    expect(zeroOne.circle === null ? null : [xy(zeroOne.circle[0]), zeroOne.circle[1]]).toEqual([
      [16, 0],
      4,
    ]);
    expect(Marker.length("many", 1.5)).toBe(0.0);
  });

  test("crows_foot_bars_follow_a_curve", () => {
    const s = ok("line l (0,0) -> curve left 30 (100,0) end=one\n");
    const l = s.lines[0]!;
    const [a, b] = l.erParts(false).strokes[0]!;
    const mid = a.add(b).mul(0.5);
    let onPath = Infinity;
    for (let i = 0; i <= 1000; i++) {
      onPath = Math.min(onPath, segEval(l.path[0]!, i / 1000).dist(mid));
    }
    expect(onPath < 0.5, `bar center ${xy(mid)} is ${onPath}px off the curve`).toBe(true);
  });

  test("label_position_must_be_in_range", () => {
    const d = errors('line l (0,0) -> h 10 label@1.5="x"\n');
    expect(d[0]!.code).toBe("attr");
  });

  test("diagram_ortho_applies_to_every_line_unless_it_opts_out", () => {
    const src = (line: string) => `diagram "t" size (800,600) ortho\n${line}\n`;
    let d = compile(src("line l (0,0) -> (10,10)"))[1];
    expect(d.length, dbg(d)).toBe(1);
    expect(d[0]!.code).toBe("not-ortho");
    expect(d[0]!.message).toBe(
      "segment is not horizontal or vertical, and the diagram sets `ortho`",
    );
    expect(compile(src("arrow l (0,0) -> curve left 10 (100,0)"))[1][0]!.code).toBe("not-ortho");
    let s: Scene;
    [s, d] = compile(src("line l (0,0) -> (10,10) ortho=false"));
    expect(d.length, dbg(d)).toBe(0);
    expect(s.ortho && !s.lines[0]!.ortho).toBe(true);
    [s, d] = compile(src("line l (0,0) -> h 10 ortho"));
    expect(d.length === 0 && s.lines[0]!.ortho, dbg(d)).toBe(true);
    [s, d] = compile('diagram "t" size (800,600)\nline l (0,0) -> (10,10) ortho=false\n');
    expect(d.length === 0 && !s.ortho && !s.lines[0]!.ortho, dbg(d)).toBe(true);
  });

  test("ortho_values_are_checked", () => {
    let d = errors("line l (0,0) -> h 10 ortho=true\n");
    expect(d[0]!.message).toBe("`ortho` is a flag; the only value it takes is `false`");
    d = compile('diagram "t" size (800,600) ortho=false\n')[1];
    expect(d[0]!.message).toBe("`ortho` takes no value");
  });

  test("ortho_rejects_curves", () => {
    const d = errors("line l (0,0) -> curve left 10 (100,0) ortho\n");
    expect(d[0]!.code).toBe("not-ortho");
    ok("line l (0,0) -> h 10 -> vh (20,20) ortho\n");
  });

  test("reason_is_kept", () => {
    const s = ok('line l (0,0) -> h 10 allow=cross reason="bus"\n');
    expect(s.lines[0]!.reason).toBe("bus");
  });

  test("unnamed_continued_line_is_named_after_its_first_line", () => {
    const s = ok("\nline (0,0)\n  -> h 10\n  -> v 10\n");
    expect(s.lines[0]!.id).toBe("line@L3");
  });

  test("diagram_attributes", () => {
    let s = ok("");
    expect([s.clearance, s.grid]).toEqual([6.0, null]);
    let d: Diagnostic[];
    [s, d] = compile('diagram "t" size (10,10) kashika="0.1.0" grid=10 clearance=4 allow=near\n');
    expect(d.length, dbg(d)).toBe(0);
    expect([s.clearance, s.grid, s.allow.length]).toEqual([4.0, 10.0, 1]);
  });

  test("diagram_background", () => {
    expect(ok("").background).toBe(null);
    const bg = (v: string) => compile(`diagram "t" size (10,10) background=${v}\n`);
    let [s, d] = bg("#0f172a");
    expect(d.length, dbg(d)).toBe(0);
    expect(s.background).toBe("#0f172a");
    [s, d] = bg("none");
    expect(d.length, dbg(d)).toBe(0);
    expect(s.background).toBe(null);
    for (const v of ["reddish", '"#ffffff"', "3"]) {
      [s, d] = bg(v);
      expect(codesOf(d), `${v}: ${dbg(d)}`).toEqual(["attr"]);
      expect(s.background).toBe(null);
    }
  });

  test("diagram_must_be_first_and_unique", () => {
    let [, d] = compile('box a at (0,0) size (1,1)\ndiagram "t" size (10,10)\n');
    expect(d[0]!.code).toBe("diagram");
    [, d] = compile('diagram "t" size (10,10)\ndiagram "u" size (10,10)\n');
    expect(d[0]!.code).toBe("diagram");
    [, d] = compile('style s { fill=#fff }\ndiagram "t" size (10,10)\n');
    expect(d[0]!.code).toBe("diagram");
  });

  test("diagram_inside_a_group_is_misplaced", () => {
    const d = errors('group g at (0,0) size (10,10) {\n  diagram "x" size (1,1)\n}\n');
    expect(d.length, dbg(d)).toBe(1);
    expect(d[0]!.code).toBe("diagram");
  });

  test("pin_at_t_on_a_diamond_follows_the_outline", () => {
    const s = ok("box d at (200,100) size (100,80) shape=diamond pin=top@0.25\n");
    const d = s.shapes[0]!;
    expect(rc(d.rect)).toEqual([175, 80, 100, 80]);
    const a = d.anchor("top", 0.25);
    expect(a === null ? null : xy(a)).toEqual([200, 100]);
  });

  test("shapes_need_positive_sizes", () => {
    for (const body of [
      "box a at (0,0) size (0,10)\n",
      "box a at (0,0) size (10,-1)\n",
      "group g at (0,0) size (0,0) {\n}\n",
      'text "x" at (0,0) size=0\n',
    ]) {
      const d = errors(body);
      expect(
        d.some((x) => x.code === "size"),
        `${body}: ${dbg(d)}`,
      ).toBe(true);
    }
  });

  test("box_shapes", () => {
    let s = ok('box a "A" at (0,0) size (40,40) shape=rect\n');
    expect(s.shapes[0]!.outline).toBe("rect");
    expect(s.shapes[0]!.label).toBe("A");
    s = ok("box a at (0,0) size (10,10) shape=rounded\n");
    expect(s.shapes[0]!.outline).toBe("rounded");
    const d = errors("box a at (0,0) size (10,10) shape=rectt\n");
    expect(d[0]!.code).toBe("unknown-shape");
  });

  test("groups_take_no_shape", () => {
    const d = errors("group g at (0,0) size (10,10) shape=rect {\n}\n");
    expect(d[0]!.code).toBe("attr");
  });

  test("junction_dot", () => {
    const s = ok("junction a at (0,0) dot\njunction b at (1,1)\n");
    expect(s.junctions.map((j) => j.dot)).toEqual([true, false]);
    expect(errors("junction a at (0,0) dot=1\n")[0]!.code).toBe("attr");
  });

  test("text_attributes", () => {
    const s = ok(
      'text t "hi" at (10,20) size=18 color=#ff0000 align=right\ntext "anon" at (0,0)\n',
    );
    const t = s.texts[0]!;
    expect(t.id).toBe("t");
    expect([t.size, t.color, t.align]).toEqual([18.0, "#ff0000", "right"]);
    expect(s.texts[1]!.id).toBe(null);
    expect(s.texts[1]!.align).toBe("center");
    expect(errors('text "x" at (0,0) align=middle\n')[0]!.code).toBe("attr");
  });

  test("duplicate_and_mistyped_attributes", () => {
    let d = errors("box a at (0,0) size (10,10) fill=#fff fill=#000\n");
    expect(d[0]!.code).toBe("attr");
    expect(d[0]!.message.includes("duplicate")).toBe(true);
    d = errors('box a at (0,0) size (10,10) stroke-width="2"\n');
    expect(d[0]!.code).toBe("attr");
  });

  test("style_inside_a_group_is_rejected", () => {
    const d = errors("group g at (0,0) size (10,10) {\n  style s { fill=#fff }\n}\n");
    expect(d[0]!.code).toBe("parse");
  });

  test("out_of_range_values_are_rejected", () => {
    for (const body0 of [
      "box a at (1000001,0) size (1,1)\n",
      "box a at (0,-1000001) size (1,1)\n",
      "box a at (999999,0) size (1,1)\nbox b at a.right+(10,0) size (1,1)\n",
      "line l (0,0) -> h 2000000\n",
      "line l (0,0) -> x 2000000\n",
      "line l (0,0) -> curve left 2000000 (10,0)\n",
      "line l (0,0) -> curve via (1e7,0) (1,1) (2,2)\n",
    ]) {
      const body = body0.replace("1e7", "10000000");
      const d = errors(body);
      expect(
        d.some(
          (d) =>
            d.code === "range" &&
            d.message === "value is out of range (more than 1000000 from the origin)",
        ),
        `${body}: ${dbg(d)}`,
      ).toBe(true);
    }
    for (const body of [
      "box a at (0,0) size (1000001,1)\n",
      "group g at (0,0) size (1,2000000) {\n}\n",
    ]) {
      const d = errors(body);
      expect(codesOf(d), `${body}: ${dbg(d)}`).toEqual(["size"]);
    }
    const [s, d] = compile('diagram "t" size (2000000,10)\n');
    expect(codesOf(d), dbg(d)).toEqual(["size"]);
    expect(s.size).toEqual([0.0, 0.0]);
    ok("box a at (-1000000,1000000) size (1000000,1)\n");
  });

  test("long_placement_chains_are_reported_not_crashed", () => {
    const n = 5000;
    let body = "";
    for (let i = 0; i < n; i++) {
      body += `box b${i} at b${i + 1}.right size (1,1) pin=left\n`;
    }
    body += `box b${n} at (0,0) size (1,1)\n`;
    const d = errors(body);
    expect(codesOf(d), dbg(d.slice(0, 3))).toEqual(["placement-cycle"]);
    expect(d[0]!.message).toBe(
      "placement chain from `b0` to `b5000` is 5000 references deep; the limit is 256",
    );

    body = "";
    for (let i = 0; i < 200; i++) {
      body += `box b${i} at b${i + 1}.right size (1,1) pin=left\n`;
    }
    body += "box b200 at (0,0) size (1,1)\n";
    const s = ok(body);
    expect(rect(s, "b0").x).toBe(200.0);
  });

  test("markup_and_escapes_are_accepted_in_texts_and_labels", () => {
    let s = ok(
      'box a "a \\*\\* b" at (0,0) size (60,40)\ntext "**t** \\*" at (0,0)\nline l (0,0) -> h 5 label="`q`"\n',
    );
    expect(s.shapes[0]!.label).toBe("a \\*\\* b");
    expect(s.texts[0]!.content).toBe("**t** \\*");
    expect(s.lines[0]!.labels[0]!.text).toBe("`q`");
    [s] = compile('diagram "a \\* b" size (10,10)\n');
    expect(s.title).toBe("a * b");
  });

  test("missing_diagram_in_an_empty_file_has_a_real_span", () => {
    const [, d] = compile("");
    expect(codesOf(d)).toEqual(["diagram"]);
    expect([d[0]!.span.line, d[0]!.span.col]).toEqual([1, 1]);
  });

  test("numeric_attributes_are_range_checked", () => {
    for (const body of [
      "box a at (0,0) size (10,10) font-size=0\n",
      "box a at (0,0) size (10,10) stroke-width=-1\n",
      "box a at (0,0) size (10,10) padding=-1\n",
      "box a at (0,0) size (10,10) radius=-2\n",
      "style s { font-size=-3 }\n",
      "line l (0,0) -> h 5 stroke-width=0\n",
      "box a at (0,0) size (10,10) font-size=2000000\n",
    ]) {
      const d = errors(body);
      expect(codesOf(d), `${body}: ${dbg(d)}`).toEqual(["attr"]);
    }
    for (const head of [
      'diagram "t" size (10,10) grid=0\n',
      'diagram "t" size (10,10) grid=-5\n',
      'diagram "t" size (10,10) clearance=-1\n',
    ]) {
      const [, d] = compile(head);
      expect(codesOf(d), `${head}: ${dbg(d)}`).toEqual(["attr"]);
    }
    const [, d] = compile(
      'diagram "t" size (10,10) grid=10 clearance=0\nbox a at (0,0) size (10,10) padding=0 radius=0 stroke-width=0.5\n',
    );
    expect(d.length, dbg(d)).toBe(0);
  });

  test("boxes_with_a_bad_size_are_not_placed", () => {
    const [s, d] = compile(
      `${HEAD}box a at (0,0) size (0,10)\nbox b at a.right size (10,10)\ngroup g at (0,0) size (-1,10) {\n  box c at (0,0) size (1,1)\n}\n`,
    );
    expect(codesOf(d), dbg(d)).toEqual(["size", "size"]);
    expect(s.shapes.length, fmt(s.shapes.map((x) => x.id))).toBe(0);
  });

  test("diagram_must_come_first_even_after_a_broken_statement", () => {
    for (const src of [
      'bogus\ndiagram "t" size (10,10)\n',
      'box a at\ndiagram "t" size (10,10)\n',
      'import "x.kash"\ndiagram "t" size (10,10)\n',
    ]) {
      const [, d] = compile(src);
      expect(
        d.some((d) => d.code === "diagram" && d.message.includes("first")),
        `${src}: ${dbg(d)}`,
      ).toBe(true);
    }
    const [, d] = compile('// c\n\n/* b */\ndiagram "t" size (10,10)\n');
    expect(d.length, dbg(d)).toBe(0);
  });

  test("text_color_cannot_be_none", () => {
    const d = errors('text "t" at (0,0) color=none\n');
    expect(codesOf(d), dbg(d)).toEqual(["attr"]);
  });

  test("elements_keep_their_frame_origin", () => {
    const s = ok(
      'group g at (100,50) size (300,200) {\n  box a at (10,20) size (40,30)\n  junction j at (5,5)\n  text "t" at (1,2)\n  line l (0,0) -> h 10\n}\n',
    );
    const a = shapeOf(s, "a");
    expect(xy(a.origin)).toEqual([100, 50]);
    expect(rc(a.localRect())).toEqual([10, 20, 40, 30]);
    const r = a.anchor("right", null);
    expect(r === null ? null : xy(r)).toEqual([150, 85]);
    expect(a.anchor("nope", null)).toBe(null);
    const g = shapeOf(s, "g");
    expect(xy(g.origin)).toEqual([0, 0]);
    expect(xy(s.junctions[0]!.origin)).toEqual([100, 50]);
    expect(xy(s.texts[0]!.origin)).toEqual([100, 50]);
    expect(xy(s.lines[0]!.origin)).toEqual([100, 50]);
  });

  test("placement_graph_records_at_references_and_frames", () => {
    const s = ok(
      "box api at (40,40) size (120,60)\ngroup g at (0,200) size (400,200) {\n  box db at (api.left.x, 20) size (100,60)\n}\njunction j at db.bottom+(0,20)\n",
    );
    const e = (f: string, t: string, kind: EdgeKind) => new PlacementEdge(f, t, kind);
    const want = [e("db", "g", "frame"), e("db", "api", "at"), e("j", "db", "at")];
    expect(s.placement.map((p) => [p.from, p.to, p.kind])).toEqual(
      want.map((p) => [p.from, p.to, p.kind]),
    );
  });

  test("placement_graph_survives_cycles", () => {
    const [s] = compile(`${HEAD}box a at b.right size (10,10)\nbox b at a.left size (10,10)\n`);
    expect(s.placement.length).toBe(2);
  });

  test("lines_keep_their_authored_segments", () => {
    const s = ok(
      "box a at (0,0) size (40,40)\nline l a.right -> h 20 -> hv (200,100) -> v 0 radius=5\n",
    );
    const l = s.lines[0]!;
    expect(l.segments.map((g) => g.kind)).toEqual(["h", "hv", "v"]);
    const hv = l.segments[1]!;
    expect([xy(hv.from), xy(hv.to)]).toEqual([
      [60, 20],
      [200, 100],
    ]);
    expect(hv.span.line).toBe(3);
    const flat = (ps: readonly { kind: string; from: Pt; to: Pt }[]) =>
      ps.map((p) => [p.kind, xy(p.from), xy(p.to)]);
    expect(flat(hv.pieces)).toEqual([
      ["line", [60, 20], [200, 20]],
      ["line", [200, 20], [200, 100]],
    ]);
    expect(l.segments[2]!.pieces.length).toBe(0);
    expect(l.pathSeg).toEqual([0, 0, 1, 1, 1]);
    expect(l.path.length).toBe(l.pathSeg.length);
    expect(SegmentKind.name("curve-via")).toBe("curve-via");
  });

  test("vhv_and_hvh_put_the_middle_run_part_way", () => {
    const s = ok(
      "line a (0,0) -> vhv (100,40) -> hvh@0.25 (140,80) -> vhv (140,200) -> hvh (0,200)\n",
    );
    const l = s.lines[0]!;
    expect(l.segments.map((g) => g.kind)).toEqual(["vhv", "hvh", "vhv", "hvh"]);
    const corners = (i: number): [number, number][] => {
      const g = l.segments[i]!;
      return xys([g.from, ...g.pieces.map((p) => p.to)]);
    };
    expect(corners(0)).toEqual([
      [0, 0],
      [0, 20],
      [100, 20],
      [100, 40],
    ]);
    expect(corners(1)).toEqual([
      [100, 40],
      [110, 40],
      [110, 80],
      [140, 80],
    ]);
    expect(corners(2)).toEqual([
      [140, 80],
      [140, 200],
    ]);
    expect(corners(3)).toEqual([
      [140, 200],
      [0, 200],
    ]);
    expect(SegmentKind.name("vhv")).toBe("vhv");
  });

  test("literals_are_recorded_in_their_own_frame", () => {
    const s = ok(
      "group g at (100,50) size (300,200) {\n  box a at (10,20) size (40,30)\n}\nline l a.right+(5,0) -> h 15 -> (300,95)\n",
    );
    const lits = s.literals.map((l) => [l.owner, l.what, l.value]);
    expect(lits).toEqual([
      ["g", "x", 100],
      ["g", "y", 50],
      ["g", "width", 300],
      ["g", "height", 200],
      ["a", "x", 10],
      ["a", "y", 20],
      ["a", "width", 40],
      ["a", "height", 30],
      ["l", "dx", 5],
      ["l", "dy", 0],
      ["l", "h", 15],
      ["l", "x", 300],
      ["l", "y", 95],
    ]);
    expect(s.literals[4]!.span.line).toBe(3);
  });

  test("line_endpoints_know_whether_they_are_literal", () => {
    const s = ok(
      "box a at (0,0) size (40,40)\nline m (100,100) -> a.left\nline n a.top -> y -20\nline o a.bottom -> x a.right\nline p a.right -> hv (80,80)\n",
    );
    const flags = s.lines.map((l) => [l.startLiteral, l.endLiteral]);
    expect(flags).toEqual([
      [true, false],
      [false, true],
      [false, false],
      [false, true],
    ]);
  });

  test("boxes_and_groups_accept_allow_and_reason", () => {
    const s = ok(
      'box a at (0,0) size (10,10) allow=overlap reason="badge"\ngroup g at (0,0) size (50,50) allow=spill:a {\n}\n',
    );
    const a = shapeOf(s, "a");
    expect(a.allow.length).toBe(1);
    expect(a.allow[0]!.code).toBe("overlap");
    expect(a.allow[0]!.other).toBe(null);
    expect([a.allow[0]!.span.col, a.allow[0]!.span.endCol]).toEqual([35, 42]);
    expect(a.reason).toBe("badge");
    const g = shapeOf(s, "g");
    expect(g.allow[0]!.other).toBe("a");
    expect(errors("box a at (0,0) size (10,10) allow=overlpa\n")[0]!.code).toBe("attr");
  });

  test("backward_placement_chains_obey_the_depth_limit", () => {
    const chain = (n: number) => {
      let body = "box b0 at (0,0) size (1,1)\n";
      for (let i = 1; i <= n; i++) {
        body += `box b${i} at b${i - 1}.right size (1,1) pin=left\n`;
      }
      return body;
    };
    const s = ok(chain(256));
    expect(rect(s, "b256").x).toBe(256.0);
    const d = errors(chain(600));
    expect(codesOf(d), dbg(d)).toEqual(["placement-cycle"]);
    expect(d[0]!.message).toBe(
      "placement chain from `b600` to `b0` is 600 references deep; the limit is 256",
    );
    expect(d[0]!.span.line).toBe(602);
  });

  test("long_placement_cycles_name_the_cycle", () => {
    const n = 300;
    let body = "";
    for (let i = 0; i < n; i++) {
      body += `box b${i} at b${(i + 1) % n}.right size (1,1)\n`;
    }
    const d = errors(body);
    expect(codesOf(d), dbg(d)).toEqual(["placement-cycle"]);
    expect(d[0]!.message).toBe(
      "placement cycle of 300 shapes: b0 -> b1 -> b2 -> b3 -> … -> b299 -> b0",
    );
    expect([d[0]!.span.line, d[0]!.span.col]).toEqual([2, 11]);
  });

  test("shapes_leading_into_a_cycle_are_not_part_of_it", () => {
    const d = errors(
      "box p at a.right size (10,10)\nbox a at b.right size (10,10)\nbox b at a.left size (10,10)\n",
    );
    expect(codesOf(d), dbg(d)).toEqual(["placement-cycle"]);
    expect(d[0]!.message).toBe("placement cycle: a -> b -> a");
  });

  test("style_sets_the_shape", () => {
    const s = ok(
      "style dot { shape=ellipse fill=#ff0000 }\n" +
        "box a at (0,0) size (20,20) style=dot\n" +
        "box b at (40,0) size (20,20)\n",
    );
    const a = shapeOf(s, "a");
    expect(a.shape).toBe("ellipse");
    expect(a.outline).toBe("path");
    expect(a.form !== null).toBe(true);
    expect(a.style.fill).toBe("#ff0000");
    expect(shapeOf(s, "b").shape).toBe("rect");
  });

  test("inline_shape_overrides_the_style_shape", () => {
    const s = ok(
      "style dot { shape=ellipse }\n" +
        "box a at (0,0) size (20,20) style=dot shape=diamond\n" +
        "box b at (40,0) size (20,20) style=dot shape=rect\n",
    );
    expect(shapeOf(s, "a").shape).toBe("diamond");
    expect(shapeOf(s, "b").shape).toBe("rect");
    expect(shapeOf(s, "b").outline).toBe("rect");
  });

  test("style_shape_can_name_a_user_shape", () => {
    const s = ok(
      'shape tri {\n  path "M 0.5 0 L 1 1 L 0 1 Z"\n}\n' +
        "style t { shape=tri }\n" +
        "box a at (0,0) size (40,40) style=t\n",
    );
    expect(shapeOf(s, "a").shape).toBe("tri");
    expect(shapeOf(s, "a").outlinePolygon().length).toBe(3);
  });

  test("style_shape_rounded_defaults_the_radius", () => {
    const s = ok(
      "style r { shape=rounded }\nstyle r4 { shape=rounded radius=4 }\n" +
        "box a at (0,0) size (40,40) style=r\n" +
        "box b at (50,0) size (40,40) style=r4\n" +
        "box c at (100,0) size (40,40) style=r radius=2\n",
    );
    expect(shapeOf(s, "a").style.radius).toBe(8.0);
    expect(shapeOf(s, "a").outline).toBe("rounded");
    expect(shapeOf(s, "b").style.radius).toBe(4.0);
    expect(shapeOf(s, "c").style.radius).toBe(2.0);
  });

  test("builtin_style_override_may_set_a_shape", () => {
    let s = ok("style service { shape=cylinder }\nbox a at (0,0) size (40,40) style=service\n");
    expect(shapeOf(s, "a").shape).toBe("cylinder");
    expect(shapeOf(s, "a").style.fill).toBe("#e8f0fe");
    s = ok("box a at (0,0) size (40,40) style=datastore\n");
    expect(shapeOf(s, "a").shape).toBe("rect");
  });

  test("unknown_shape_in_a_style_suggests", () => {
    let d = errors("style s { shape=elipse }\nbox a at (0,0) size (40,40) style=s\n");
    expect(codesOf(d), dbg(d)).toEqual(["unknown-shape"]);
    expect([d[0]!.span.line, d[0]!.span.col]).toEqual([2, 11]);
    expect(d[0]!.help[0]).toBe("did you mean `ellipse`?");
    d = errors('style s { shape="ellipse" }\n');
    expect(codesOf(d), dbg(d)).toEqual(["attr"]);
  });

  test("group_with_a_shaped_style_is_an_error", () => {
    const [s, d] = compile(
      `${HEAD}style dot { shape=ellipse stroke=#ff0000 }\ngroup g at (0,0) size (100,100) style=dot {\n}\n`,
    );
    expect(codesOf(d), dbg(d)).toEqual(["attr"]);
    expect(d[0]!.span.line).toBe(3);
    expect(d[0]!.message).toBe("this style sets `shape`, but groups take no shape");
    expect(d[0]!.help).toEqual(["use a style without `shape`, or drop `shape` from it"]);
    const g = shapeOf(s, "g");
    expect(g.shape).toBe("rect");
    expect(g.style.stroke).toBe("#ff0000");
  });

  test("boundary_style_takes_no_shape", () => {
    const [s, d] = compile(
      `${HEAD}style boundary { shape=ellipse stroke=#ff0000 }\ngroup g at (0,0) size (100,100) {\n}\nbox a at (200,0) size (20,20) style=boundary\n`,
    );
    expect(codesOf(d), dbg(d)).toEqual(["attr"]);
    expect(d[0]!.span.line).toBe(2);
    expect(
      d[0]!.message.startsWith("the `boundary` style can't set `shape`"),
      d[0]!.message,
    ).toBe(true);
    expect(shapeOf(s, "g").shape).toBe("rect");
    expect(shapeOf(s, "g").style.stroke).toBe("#ff0000");
    expect(shapeOf(s, "a").shape).toBe("rect");
  });

  test("line_style_ignores_shape", () => {
    const s = ok("style s { shape=ellipse stroke=#00ff00 }\nline l (0,0) -> h 100 style=s\n");
    expect(s.lines[0]!.style.stroke).toBe("#00ff00");
  });

  test("canvas_without_size_fits_the_drawing_plus_a_margin", () => {
    let [s, d] = compile('diagram "t"\nbox a at (10,10) size (100,50)\n');
    expect(d.length, dbg(d)).toBe(0);
    expect(s.size).toEqual([111.0 + 20.0, 61.0 + 20.0]);
    [s] = compile('diagram "t" margin=0\nline l (10,10) -> h 100 end=arrow\n');
    expect(s.size).toEqual([114.0, 14.0]);
    [s] = compile('diagram "t" margin=5\ntext "note" at (50,100)\n');
    const t = s.texts[0]!.rect;
    expect(s.size).toEqual([Math.ceil(t.right()) + 5.0, Math.ceil(t.bottom()) + 5.0]);
  });

  test("canvas_fits_one_axis", () => {
    let [s, d] = compile(
      'diagram "t" size (300, auto)\nbox a at (10,10) size (100,50) stroke=none\n',
    );
    expect(d.length, dbg(d)).toBe(0);
    expect(s.size).toEqual([300.0, 81.0]);
    [s] = compile('diagram "t" size (auto, 300)\nbox a at (10,10) size (100,50)\n');
    expect(s.size).toEqual([131.0, 300.0]);
  });

  test("margin_needs_a_fitted_canvas", () => {
    let [s, d] = compile('diagram "t" size (300,200) margin=10\n');
    expect(codesOf(d), dbg(d)).toEqual(["attr"]);
    expect(s.size).toEqual([300.0, 200.0]);
    [, d] = compile('diagram "t" margin=-1\n');
    expect(codesOf(d), dbg(d)).toEqual(["attr"]);
  });

  test("style_size_applies_to_boxes_and_groups", () => {
    const s = ok(
      "style node { size=(120,60) fill=#eeeeee }\nstyle wide { size=(300,200) }\n" +
        "style tall { size=(auto, 60) }\n" +
        'box a "A" at (0,0) style=node\n' +
        'box b "B" at (0,100) size (80,40) style=node\n' +
        'box c "Payments API" at (0,200) style=tall\n' +
        "group g at (400,0) style=wide {\n}\n" +
        "line l (0,400) -> h 100 style=node\n",
    );
    const size = (id: string) => [rect(s, id).w, rect(s, id).h];
    expect(size("a")).toEqual([120.0, 60.0]);
    expect(size("b"), "inline size wins").toEqual([80.0, 40.0]);
    expect(size("c"), "an auto axis fits").toEqual([108.0, 60.0]);
    expect(size("g")).toEqual([300.0, 200.0]);
  });

  test("style_size_errors", () => {
    const [, d] = compile(
      `${HEAD}style boundary { size=(100,100) }\nstyle s { size=12 }\nstyle z { size=(0,10) }\n` +
        "style a { size=(auto,40) }\ngroup g at (0,0) style=a {\n}\ngroup k at (0,0) {\n}\n",
    );
    expect(codesOf(d), dbg(d)).toEqual(["attr", "attr", "size", "size", "size"]);
    expect(d[0]!.message.startsWith("the `boundary` style can't set `size`")).toBe(true);
    expect(d[1]!.help).toEqual(["for the text size, write `font-size=N`"]);
    expect(d[3]!.message).toBe("group `g` has no size to fit: it has no children or label");
  });

  test("groups_without_a_size_fit_their_children", () => {
    const s = ok(
      'group outer "Outer" at (20,100) padding=(16,10) {\n  ' +
        'box a "A" at (16,30) size (80,40)\n  ' +
        "group inner at (120,30) {\n    box b at (10,30) size (60,30)\n  }\n  " +
        "line ab a.right -> inner.left end=arrow\n}\n" +
        "box c at outer.tr+(40,0) size (20,20)\n" +
        "group w at (400,20) size (200, auto) {\n  junction j at (10,80) dot\n}\n",
    );
    expect(rc(rect(s, "inner"))).toEqual([140, 130, 79, 69]);
    expect(rc(rect(s, "outer"))).toEqual([20, 100, 216, 110]);
    expect(rc(rect(s, "c"))).toEqual([276, 100, 20, 20]);
    expect(rc(rect(s, "w"))).toEqual([400, 20, 200, 91]);
  });

  test("fitted_groups_hold_their_title_and_round_to_the_grid", () => {
    let s = ok('group g "A long group title" at (0,0) {\n}\n');
    const r = rect(s, "g");
    expect(r.w > 100.0 && r.h > 20.0, fmt(rc(r))).toBe(true);
    s = ok("group g at (0,0) {\n  box a at (-30,-30) size (40,40)\n}\n");
    expect(rc(rect(s, "g"))).toEqual([0, 0, 19, 19]);
    const [s2, d] = compile(
      'diagram "t" size (800,600) grid=10\ngroup g at (0,0) {\n  box a at (10,10) size (41,20)\n}\n',
    );
    expect(d.length, dbg(d)).toBe(0);
    expect(rc(rect(s2, "g"))).toEqual([0, 0, 60, 40]);
  });

  test("fitted_groups_with_a_bottom_title_reserve_it_below_the_children", () => {
    let s = ok('group g "G" at (0,0) title=bottom {\n  box a at (10,10) size (40,40)\n}\n');
    expect(rc(rect(s, "g"))).toEqual([0, 0, 59, 85]);
    s = ok('group g "G" at (0,0) title=bottom {\n}\n');
    expect(rect(s, "g").h).toBe(34.0);
  });

  test("children_cannot_be_placed_against_a_fitted_group", () => {
    let d = errors(
      "group g at (0,0) {\n  box a at (0,0) size (10,10)\n  box b at g.right size (10,10)\n}\n",
    );
    expect(codesOf(d), dbg(d)).toEqual(["placement-cycle"]);
    expect(d[0]!.message).toBe("placement cycle: b -> g -> b");
    expect(d[0]!.help).toEqual([
      "`g` is fitted to its children, so they can't be placed against its size; give `g` a size: `size (w,h)`",
    ]);
    d = errors(
      "group g at (0,0) {\n  box a at (0,0) size (10,10)\n  line l a.right -> g.right\n}\n",
    );
    expect(d[0]!.message).toBe("placement cycle: l -> g -> l");
    const s = ok(
      "group g at (0,0) size (100,auto) {\n  box a at (0,0) size (10,10)\n}\n" +
        "box b at g.br+(10,0) size (10,10)\n",
    );
    expect(rc(rect(s, "b"))).toEqual([110, 19, 10, 10]);
  });

  test("a_fitted_group_pins_on_its_top_left_edges", () => {
    const d = errors("group g at (100,100) pin=center {\n  box a at (0,0) size (10,10)\n}\n");
    expect(codesOf(d), dbg(d)).toEqual(["attr"]);
    expect(d[0]!.message).toBe("`pin=center` moves as `g` grows to fit its children");
    const s = ok(
      "group g at (100,100) size (40,auto) pin=top {\n  box a at (0,0) size (10,10)\n}\n",
    );
    expect(rc(rect(s, "g"))).toEqual([80, 100, 40, 19]);
    expect(rc(rect(s, "a"))).toEqual([80, 100, 10, 10]);
  });

  test("common_attribute_slips_get_the_fix_as_help", () => {
    const ab = "box a at (0,0) size (10,10)\nbox b at (100,0) size (10,10)\n";
    const cases: [string, string][] = [
      [
        'box c label="C" at (0,0) size (10,10)\n',
        'the label is the quoted string after the id, with no `label=`: `box <id> "C" …`',
      ],
      [
        'group g at (0,0) size (50,50) label="G" {\n}\n',
        'the label is the quoted string after the id, with no `label=`: `group <id> "G" …`',
      ],
      [
        `${ab}line l a.right -> b.left arrow=end\n`,
        "arrowheads are line ends: write `end=arrow`, or use the `arrow` statement",
      ],
      [
        `${ab}line l a.right -> b.left arrow=both\n`,
        "arrowheads are line ends: write `start=arrow end=arrow`, or use the `arrow` statement",
      ],
      [
        'box c "C" at (0,0) width=10\n',
        "the size is a clause: write `size (w,h)`, without `=`",
      ],
      [`${ab}line l a.right -> b.left dashed\n`, "write `stroke-style=dashed`"],
      [
        `${ab}line l a.right -> b.left style=dotted\n`,
        "a stroke pattern is `stroke-style=dotted`",
      ],
      ['box c "C" at (0,0) icon=server\n', 'quote it: `icon="server"`'],
      ['box c "C" at (0,0) pin=middle\n', "did you mean `center`?"],
      [`${ab}line l a.middle -> b.left\n`, "did you mean `center`?"],
      [
        'box c "C" at (0,0) icon="oval"\n',
        "no icon is called `oval`; the built-in shape `ellipse` matches it: `shape=ellipse`",
      ],
    ];
    for (const [body, help] of cases) {
      const d = errors(body);
      expect(d.length, `${body}: ${dbg(d)}`).toBe(1);
      expect(d[0]!.help, `${body}: ${dbg(d)}`).toEqual([help]);
    }
  });

  test("unknown_shape_suggests_by_keyword_and_lists_the_builtins", () => {
    let d = errors('box a "A" at (0,0) shape=database\n');
    expect(d[0]!.help[0], dbg(d)).toBe("did you mean `cylinder`?");
    expect(d[0]!.help[1]!.startsWith("`database` is a bundled icon")).toBe(true);
    expect(d[0]!.help[2]!.startsWith("built-in shapes: `rect`, `rounded`, `cylinder`")).toBe(true);
    d = errors('box a "A" at (0,0) shape=server\n');
    expect(d[0]!.help.length, dbg(d)).toBe(2);
    expect(
      d[0]!.help[0]!.startsWith(
        '`server` is a bundled icon: put it on a box with `icon="server"`',
      ),
    ).toBe(true);
  });
});
