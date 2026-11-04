import { expect, it } from "vitest";

import { Ctx, check, metrics, suppress } from "../../src/check/index.js";
import { anchorText, overflow } from "../../src/check/ctx.js";
import { polygon } from "../../src/check/outline.js";
import { ports } from "../../src/check/ports.js";
import {
  labelBox,
  labelBoxAt,
  textBox,
  titleBand,
  titleBox,
} from "../../src/check/text_box.js";
import { Diagnostic, Element, Span } from "../../src/diag/index.js";
import { Pt, Rect, boundaryNearest, pointInPolygon } from "../../src/geom/index.js";
import { scene, sceneSrc } from "./common/mod.js";

function diag(code: string, ids: string[]): Diagnostic {
  return ids.reduce(
    (d, id) => d.withElement(Element.new(id, "line", Span.default())),
    Diagnostic.warn(code, Span.default(), "x"),
  );
}

it("clean_scene_has_no_findings", () => {
  expect(check(scene("box a at (10,10) size (100,60)\n"))).toEqual([]);
});

it("line_allow_suppresses_matching_findings_and_keeps_the_reason", () => {
  const s = scene(
    'line a (0,0) -> h 10 allow=cross:b reason="bus"\nline b (5,-5) -> v 10\nline c (0,20) -> h 10\n',
  );
  const ds = [diag("cross", ["b", "a"]), diag("cross", ["c", "a"]), diag("cross", ["b", "c"])];
  suppress(s, ds);
  expect(ds[0]!.suppressed).toBe(true);
  expect(ds[0]!.reason).toBe("bus");
  expect(ds[1]!.suppressed).toBe(false);
  expect(ds[2]!.suppressed).toBe(false);
});

it("shape_and_diagram_allow", () => {
  const s = sceneSrc(
    'diagram "t" size (100,100) allow=near\nbox a at (0,0) size (10,10) allow=overlap\nline l (0,50) -> h 10 allow=cross\n',
  );
  const ds = [
    diag("near", ["l"]),
    diag("overlap", ["a", "z"]),
    diag("cross", ["l", "m"]),
    diag("through", ["l", "a"]),
  ];
  suppress(s, ds);
  expect(ds.map((d) => d.suppressed)).toEqual([true, true, true, false]);
});

it("metrics_count_crossings_and_wire", () => {
  const s = scene("line a (0,0) -> h 10\nline b (0,20) -> h 30\n");
  const m = metrics(s, [diag("cross", ["a", "b"]), diag("near", ["a"])]);
  expect([m.crossings, m.wirePx]).toEqual([1, 40]);
});

it("outlines_and_attachment", () => {
  const s = scene("box r at (0,0) size (100,60)\nbox q at (200,0) size (100,60) shape=rounded\n");
  const ctx = Ctx.new(s);
  expect(polygon(s.shapes[0]!).length).toBe(4);
  const q = polygon(s.shapes[1]!);
  expect(pointInPolygon(new Pt(200.5, 0.5), q)).toBe(false);
  expect(boundaryNearest(new Pt(200.0, 30.0), q)[0]).toBeLessThanOrEqual(0.5);
  expect(ctx.attached(new Pt(100.0, 30.0))).toEqual(["r"]);
  expect(ctx.attached(new Pt(50.0, 30.0))).toEqual(["r"]);
  expect(ctx.attached(new Pt(150.0, 30.0))).toEqual([]);
});

it("rounded_outline_stays_within_a_quarter_pixel_of_the_arc", () => {
  const s = scene("box q at (0,0) size (100,60) radius=20\n");
  const q = polygon(s.shapes[0]!);
  const [c, r] = [new Pt(20.0, 20.0), 20.0];
  for (let i = 0; i + 1 < q.length; i++) {
    const [a, b] = [q[i]!, q[i + 1]!];
    if (!(a.x < 20.0 && a.y < 20.0)) continue;
    const mid = new Pt((a.x + b.x) / 2.0, (a.y + b.y) / 2.0);
    expect(r - mid.dist(c)).toBeLessThanOrEqual(0.25);
  }
});

it("rect_and_anchor_helpers", () => {
  const s = scene("box a at (100,100) size (100,60)\n");
  const a = s.shapes[0]!;
  expect(anchorText(a, new Pt(100.0, 115.0))).toBe("a.left@0.25");
  expect(anchorText(a, new Pt(150.0, 160.0))).toBe("a.bottom");
  expect(overflow(new Rect(0.0, 0.0, 100.0, 100.0), new Rect(-5.0, 10.0, 120.0, 10.0))).toEqual([
    ["left", 5.0],
    ["right", 15.0],
  ]);
});

it("text_boxes_are_the_measured_rects", () => {
  const s = scene('text "abc" at (100,50) size=10\nline l (0,300) -> h 200 label@0.25="hi"\n');
  const t = s.texts[0]!;
  expect(textBox(t)).toEqual(t.rect);
  expect(t.rect.w > 0.0 && t.rect.h > 0.0).toBe(true);
  expect(Math.abs(t.rect.center().x - 100.0)).toBeLessThan(1e-9);
  expect(Math.abs(t.rect.center().y - 50.0)).toBeLessThan(1e-9);

  const l = s.lines[0]!;
  const lab = l.labels[0]!;
  expect(labelBox(lab)).toEqual(lab.rect);
  expect(Math.abs(lab.rect.center().x - 50.0)).toBeLessThan(1e-9);
  const moved = labelBoxAt(lab, new Pt(150.0, 300.0));
  expect([moved.w, moved.h]).toEqual([lab.rect.w, lab.rect.h]);
  expect(Math.abs(moved.center().x - 150.0)).toBeLessThan(1e-9);
  expect(Math.abs(moved.center().y - 300.0)).toBeLessThan(1e-9);
});

it("group_title_band_and_box", () => {
  let s = scene(
    'group g "VPC" at (0,100) size (200,100) {\n}\ngroup u at (300,100) size (200,100) {\n}\nbox b "x" at (0,300) size (100,60)\n',
  );
  let g = s.shapes[0]!;
  let band = titleBand(g)!;
  expect(band).toEqual(new Rect(0.0, 100.0, 200.0, g.titleBand));
  expect(g.titleBand).toBeGreaterThan(g.style.padding.y);
  const title = titleBox(g)!;
  expect(title.x >= band.x && title.bottom() <= band.bottom() + 1e-9).toBe(true);
  expect(title.y).toBe(100.0 + g.style.padding.y);
  expect(titleBand(s.shapes[1]!)).toBeNull();
  expect(titleBox(s.shapes[1]!)).toBeNull();
  expect(titleBand(s.shapes[2]!)).toBeNull();
  expect(titleBox(s.shapes[2]!)).toBeNull();
  s = scene('group g "VPC" at (0,100) size (200,100) title=bottom {\n}\n');
  g = s.shapes[0]!;
  band = titleBand(g)!;
  expect(band).toEqual(new Rect(0.0, 200.0 - g.titleBand, 200.0, g.titleBand));
  expect(Math.abs(titleBox(g)!.bottom() - (200.0 - g.style.padding.y))).toBeLessThan(1e-9);
});

it("ports_list_endpoints_on_outlines", () => {
  const s = scene(
    "box a at (100,100) size (100,60)\njunction j at (0,0)\nline l a.top -> v -50 end=arrow\nline m j -> a.left@0.25\n",
  );
  const p = ports(Ctx.new(s));
  const got = p.map((x) => [x.shape, x.side, x.t, x.line, x.end, x.marker]);
  expect(got).toEqual([
    ["a", "top", 0.5, "l", "start", false],
    ["a", "left", 0.25, "m", "end", false],
  ]);
});
