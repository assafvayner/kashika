import { describe, expect, test } from "vitest";
import { type Diagnostic } from "../../src/diag/diagnostic.js";
import { displayF64, fmax, fmin } from "../../src/diag/rust.js";
import {
  type PathSeg,
  Pt,
  Rect,
  boundaryNearest,
  flatten,
  horizontalHits,
  pointInPolygon,
} from "../../src/geom/index.js";
import {
  SHAPES,
  type Scene,
  type Shape,
  builtinSource,
  compile,
  shapeNames,
} from "../../src/model/index.js";

const HEAD = 'diagram "t" size (800,600)\n';

function ok(body: string): Scene {
  const [scene, diags] = compile(`${HEAD}${body}`);
  expect(diags, JSON.stringify(diags.map((d) => [d.code, d.message]))).toEqual([]);
  return scene;
}

function errors(body: string): Diagnostic[] {
  return compile(`${HEAD}${body}`)[1];
}

function shape(s: Scene, id: string): Shape {
  const x = s.shapes.find((x) => x.id === id);
  if (x === undefined) throw new Error(`no shape ${id}`);
  return x;
}

function start(s: Scene, id: string): Pt {
  return s.lines.find((l) => l.id === id)!.path[0]!.from;
}

function close(a: Pt, b: Pt): boolean {
  return a.dist(b) <= 0.3;
}

function firstSeg(s: Scene, id: string): PathSeg {
  return s.lines.find((l) => l.id === id)!.path[0]!;
}
void firstSeg;

const dump = (v: unknown): string => JSON.stringify(v);

/// The built-in shapes drawn from unit paths: all but `rect` and `rounded`.
function pathShapes(): string[] {
  return SHAPES.filter((s) => s.source !== null).map((s) => s.name);
}

const BIG = 'diagram "t" size (4000,4000)\n';

describe("shapes", () => {
  test("rect_outline_polygon_and_text_area", () => {
    const s = ok("box a at (10,20) size (100,60)\n");
    const a = shape(s, "a");
    expect(a.shape).toBe("rect");
    expect(a.form).toBeNull();
    expect(a.image === null && a.icon === null).toBe(true);
    expect(a.outlinePolygon()).toEqual([
      new Pt(10.0, 20.0),
      new Pt(110.0, 20.0),
      new Pt(110.0, 80.0),
      new Pt(10.0, 80.0),
    ]);
    expect(a.textArea).toEqual(new Rect(18.0, 28.0, 84.0, 44.0));
    expect(s.assets.length).toBe(0);
  });

  test("rounded_outline_polygon_follows_the_corners", () => {
    const s = ok("box r at (0,0) size (100,60) shape=rounded\n");
    const r = shape(s, "r");
    expect(r.outline).toBe("rounded");
    expect(r.shape).toBe("rounded");
    const poly = r.outlinePolygon();
    expect(poly.length > 8).toBe(true);
    expect(
      poly.some((p) => p.x === 0.0 && p.y === 0.0),
      "corner is cut",
    ).toBe(false);
    for (const p of poly) {
      expect(p.x >= -1e-9 && p.x <= 100.0 + 1e-9 && p.y >= -1e-9 && p.y <= 60.0 + 1e-9).toBe(true);
    }
    const corner = new Pt(8.0, 8.0);
    const nearest = poly.map((p) => p.dist(corner)).reduce((a, b) => fmin(a, b), Infinity);
    expect(Math.abs(nearest - 8.0) < 0.3, String(nearest)).toBe(true);
  });

  test("radius_alone_makes_a_rounded_rect", () => {
    const s = ok("box r at (0,0) size (100,60) radius=4\n");
    const r = shape(s, "r");
    expect(r.outline).toBe("rounded");
    expect(r.shape).toBe("rounded");
    expect(r.outlinePolygon().length > 8).toBe(true);
  });

  test("group_outline_polygon_is_its_rect", () => {
    const s = ok('group g "G" at (0,0) size (200,100) {\n}\n');
    const g = shape(s, "g");
    expect(g.outlinePolygon().length).toBe(4);
  });

  test("every_builtin_shape_resolves_inside_its_box", () => {
    for (const name of pathShapes()) {
      const s = ok(`box a "A" at (0,0) size (160,100) shape=${name}\n`);
      const a = shape(s, "a");
      expect(a.outline, name).toBe("path");
      expect(a.shape).toBe(name);
      const f = a.form!;
      expect(f.outline.length >= 3, name).toBe(true);
      expect(
        f.outline.every((p) => p.x >= -0.01 && p.x <= 160.01 && p.y >= -0.01 && p.y <= 100.01),
        `${name} outline leaves its box`,
      ).toBe(true);
      const ta = a.textArea;
      expect(
        ta.w > 0.0 && ta.h > 0.0 && ta.x >= 0.0 && ta.right() <= 160.0 && ta.bottom() <= 100.0,
        `${name}: ${dump(ta)}`,
      ).toBe(true);
      expect(a.outlinePolygon()).toEqual(f.outline);
      const label = a.text[0]!.rect;
      expect(
        Math.abs(label.center().x - ta.center().x) < 1e-9 &&
          Math.abs(label.center().y - ta.center().y) < 1e-9,
        `${name}: label is centered in the shape's text area`,
      ).toBe(true);
    }
  });

  test("diamond_anchors_are_axis_projected", () => {
    const s = ok(
      "box d at (0,0) size (120,80) shape=diamond\n" +
        "line l d.right@0.25 -> h 40\nline m d.top@0.25 -> v -40\n" +
        "line n d.tl -> v -10\nline c d.center -> h 1\n",
    );
    expect(start(s, "l")).toEqual(new Pt(90.0, 20.0));
    expect(start(s, "m")).toEqual(new Pt(30.0, 20.0));
    expect(start(s, "n"), "corners stay on the bbox").toEqual(new Pt(0.0, 0.0));
    expect(start(s, "c")).toEqual(new Pt(60.0, 40.0));
  });

  test("cylinder_top_follows_the_cap", () => {
    const s = ok(
      "box c at (0,0) size (120,80) shape=cylinder\n" +
        "line a c.top -> v -20\nline b c.top@0.25 -> v -20\nline e c.bottom -> v 20\n",
    );
    expect(close(start(s, "a"), new Pt(60.0, 0.0))).toBe(true);
    expect(close(start(s, "b"), new Pt(30.0, 20.0 - 20.0 * Math.sqrt(0.75)))).toBe(true);
    expect(close(start(s, "e"), new Pt(60.0, 80.0))).toBe(true);
    expect(start(s, "b").x, "a vertical line into top stays vertical").toBe(30.0);
  });

  test("person_anchors_reach_the_head_and_body", () => {
    const s = ok(
      "box p at (0,0) size (120,120) shape=person\n" +
        "line a p.top -> v -20\nline b p.left@0.9 -> h -20\n" +
        "line c p.right@0.9 -> h 20\nline e p.bottom -> v 20\n",
    );
    expect(close(start(s, "a"), new Pt(60.0, 0.0))).toBe(true);
    expect(close(start(s, "b"), new Pt(0.0, 108.0))).toBe(true);
    expect(close(start(s, "c"), new Pt(120.0, 108.0))).toBe(true);
    expect(close(start(s, "e"), new Pt(60.0, 120.0))).toBe(true);
  });

  test("builtin_text_areas", () => {
    const s = ok(
      "box d at (0,0) size (120,80) shape=diamond\nbox c at (200,0) size (120,80) shape=cylinder\n",
    );
    const d = shape(s, "d").textArea;
    expect(
      close(new Pt(d.x, d.y), new Pt(34.4, 30.4)) && close(new Pt(d.w, d.h), new Pt(51.2, 19.2)),
      dump(d),
    ).toBe(true);
    expect(shape(s, "c").textArea).toEqual(new Rect(208.0, 48.0, 104.0, 24.0));
  });

  test("builtin_text_areas_stay_inside_the_outline", () => {
    for (const name of pathShapes()) {
      for (const [w, h] of [
        [120, 80],
        [60, 160],
        [240, 80],
        [200, 200],
        [160, 50],
        [40, 40],
        [600, 90],
      ] as [number, number][]) {
        const s = ok(`box a at (0,0) size (${w},${h}) shape=${name} padding=0\n`);
        const a = shape(s, "a");
        const poly = a.form!.outline;
        const t = a.textArea;
        // A cylinder's text area runs down to the bottom of the box, past its bottom cap's corners.
        const bottom = name === "cylinder" ? t.y : t.bottom();
        for (let i = 0; i <= 20; i++) {
          const f = i / 20;
          for (const p of [
            new Pt(t.x + f * t.w, t.y),
            new Pt(t.x + f * t.w, bottom),
            new Pt(t.x, t.y + f * (bottom - t.y)),
            new Pt(t.right(), t.y + f * (bottom - t.y)),
          ]) {
            expect(
              pointInPolygon(p, poly) || boundaryNearest(p, poly)[0] < 0.5,
              `${name} ${w}x${h}: ${dump(p)} of ${dump(t)} is outside the outline`,
            ).toBe(true);
          }
        }
      }
    }
  });

  test("tall_hexagons_keep_half_their_width_for_text", () => {
    const s = ok("box hx at (0,0) size (60,160) shape=hexagon padding=0\n");
    expect(shape(s, "hx").textArea).toEqual(new Rect(15.0, 0.0, 30.0, 160.0));
  });

  test("pixel_features_do_not_stretch", () => {
    const s = ok(
      "box d at (0,0) size (100,200) shape=doc padding=0\n" +
        "box p at (200,0) size (300,300) shape=person padding=0\n" +
        "box q at (500,300) size (300,100) shape=person padding=0\n" +
        "box c at (0,300) size (400,200) shape=cloud padding=0\n",
    );
    expect(Math.abs(shape(s, "d").textArea.h - 182.0) < 1e-9, "the wave is 12px deep").toBe(true);
    const p = shape(s, "p").form!.outline;
    const across = horizontalHits(p, 18.0);
    const [l, r] = across.reduce<[number, number]>(
      ([l, r], x) => [fmin(l, x), fmax(r, x)],
      [Number.MAX_VALUE, -Number.MAX_VALUE],
    );
    expect(Math.abs(r - l - 36.0) < 0.5, `the head is a circle of radius 18: ${dump(across)}`).toBe(
      true,
    );
    const head = (id: string): [number, number, number] => {
      const f = shape(s, id).form!;
      expect(f.draw.length, "the outline subpath is not drawn").toBe(2);
      const pts = flatten(f.draw[0]!);
      const [x0, x1, y0, y1] = pts.reduce<[number, number, number, number]>(
        ([a, b, c, d], p) => [fmin(a, p.x), fmax(b, p.x), fmin(c, p.y), fmax(d, p.y)],
        [Number.MAX_VALUE, -Number.MAX_VALUE, Number.MAX_VALUE, -Number.MAX_VALUE],
      );
      return [x1 - x0, y1 - y0, y0];
    };
    for (const [id, top] of [
      ["p", 0.0],
      ["q", 300.0],
    ] as [string, number][]) {
      const [w, h, y] = head(id);
      expect(
        Math.abs(w - 36.0) < 0.5 && Math.abs(h - 36.0) < 0.5 && Math.abs(y - top) < 1e-9,
        `${id}: the head stays a circle of radius 18 at the top: ${w}x${h} at y=${y}`,
      ).toBe(true);
    }
    expect(Math.abs(shape(s, "c").textArea.y - 324.0) < 1e-9, "bumps are 24px high").toBe(true);
  });

  test("flowchart_and_uml_features_are_capped_in_pixels", () => {
    for (const [name, want] of [
      ["parallelogram", new Rect(24.0, 0.0, 352.0, 300.0)],
      ["predefined-process", new Rect(12.0, 0.0, 376.0, 300.0)],
      ["octagon", new Rect(12.0, 12.0, 376.0, 276.0)],
      ["note", new Rect(0.0, 8.0, 392.0, 292.0)],
      ["package", new Rect(0.0, 12.0, 400.0, 288.0)],
      ["queue", new Rect(12.0, 0.0, 364.0, 300.0)],
      ["stadium", new Rect(85.5, 15.0, 229.0, 270.0)],
    ] as [string, Rect][]) {
      const s = ok(`box a at (0,0) size (400,300) shape=${name} padding=0\n`);
      const t = shape(s, "a").textArea;
      expect(
        close(new Pt(t.x, t.y), new Pt(want.x, want.y)) &&
          close(new Pt(t.w, t.h), new Pt(want.w, want.h)),
        `${name}: ${dump(t)}`,
      ).toBe(true);
    }
  });

  test("fitted_sizes_hold_their_text_and_are_minimal", () => {
    for (const name of shapeNames()) {
      for (const size of [9, 12, 14, 20, 28]) {
        for (const label of ["Q", "Payments API", "Two\\nlines here"]) {
          const [s, d] = compile(`${BIG}box a "${label}" at (0,0) shape=${name} font-size=${size}\n`);
          expect(d, `${name} ${size} ${label}: ${dump(d)}`).toEqual([]);
          const a = shape(s, "a");
          const need = a.text[0]!.rect;
          const [w, h] = [a.rect.w, a.rect.h];
          const area = (w: number, h: number): Rect => {
            const [s] = compile(
              `${BIG}box a "${label}" at (0,0) size (${displayF64(w)},${displayF64(h)}) shape=${name} font-size=${size}\n`,
            );
            return shape(s, "a").textArea;
          };
          const at = area(w, h);
          expect(
            at.w + 1e-9 >= need.w && at.h + 1e-9 >= need.h,
            `${name} ${size} ${label}: ${dump(at)} at (${w},${h}) does not hold ${dump(need)}`,
          ).toBe(true);
          expect(
            area(w - 1.0, h).w < need.w && area(w, h - 1.0).h < need.h,
            `${name} ${size} ${label}: (${w},${h}) is not minimal`,
          ).toBe(true);
        }
      }
    }
  });

  test("anchors_on_placed_shapes_use_the_outline", () => {
    const s = ok(
      "box d at (0,0) size (120,80) shape=diamond\nbox e at d.right size (40,40) pin=left\n",
    );
    expect(shape(s, "e").rect).toEqual(new Rect(120.0, 20.0, 40.0, 40.0));
  });

  test("unknown_shape_suggests_a_name", () => {
    const d = errors("box a at (0,0) size (10,10) shape=cylindr\n");
    expect(d.length, dump(d)).toBe(1);
    expect(d[0]!.code).toBe("unknown-shape");
    expect(d[0]!.help[0]).toBe("did you mean `cylinder`?");
  });

  test("label_overflow_suggests_a_size_that_fits_the_shape", () => {
    for (const name of pathShapes()) {
      const d = errors(`box a "A rather long label" at (0,0) size (60,30) shape=${name}\n`);
      expect(d.length, `${name}: ${dump(d)}`).toBe(1);
      expect(d[0]!.code, name).toBe("label-overflow");
      const help = d[0]!.help[0]!;
      const part = help.split("size (")[1];
      if (part === undefined) throw new Error(`${name}: ${help}`);
      const size = part.split(")")[0]!;
      const fixed = errors(`box a "A rather long label" at (0,0) size (${size}) shape=${name}\n`);
      expect(fixed, `${name} at (${size}): ${dump(fixed)}`).toEqual([]);
    }
  });

  test("builtin_source_is_exposed_for_ref", () => {
    const [d, outline, ta] = builtinSource("diamond", 120.0, 80.0)!;
    expect(d).toBe("M 0.5 0 L 1 0.5 L 0.5 1 L 0 0.5 Z");
    expect(outline).toBe(1);
    expect(ta.eq(new Rect(0.22, 0.28, 0.56, 0.44))).toBe(true);
    expect(builtinSource("rect", 1.0, 1.0)).toBeNull();
  });

  test("custom_shapes_stretch_to_the_box", () => {
    const s = ok(
      'shape tri {\n  path "M 0.5 0 L 1 1 L 0 1 Z  M 0.2 0.8 L 0.8 0.8"\n  text-area (0.25,0.5,0.5,0.5)\n}\n' +
        "box t at (100,100) size (200,100) shape=tri\nline l t.left -> h -20\n",
    );
    const t = shape(s, "t");
    expect(t.shape).toBe("tri");
    expect(t.outline).toBe("path");
    expect(t.form!.draw.length).toBe(2);
    expect(start(s, "l")).toEqual(new Pt(150.0, 150.0));
    expect(t.textArea).toEqual(new Rect(158.0, 158.0, 84.0, 34.0));
  });

  test("outline_picks_the_subpath", () => {
    const s = ok(
      'shape two {\n  path "M 0 0 L 1 0 L 1 0.2 L 0 0.2 Z M 0 0.5 L 1 0.5 L 1 1 L 0 1 Z"\n  outline 2\n}\n' +
        "box b at (0,0) size (100,100) shape=two\nline l b.top -> v -10\n",
    );
    expect(start(s, "l")).toEqual(new Pt(50.0, 50.0));
  });

  test("custom_shape_without_text_area_uses_the_whole_box", () => {
    const s = ok(
      'shape tri {\n  path "M 0.5 0 L 1 1 L 0 1 Z"\n}\nbox t at (0,0) size (100,50) shape=tri\n',
    );
    expect(shape(s, "t").textArea).toEqual(new Rect(8.0, 8.0, 84.0, 34.0));
  });

  test("custom_shape_label_overflow_uses_its_text_area", () => {
    const def =
      'shape tri {\n  path "M 0.5 0 L 1 1 L 0 1 Z"\n  text-area (0.25,0.5,0.5,0.5)\n}\n';
    const d = errors(`${def}box a "A rather long label" at (0,0) size (60,30) shape=tri\n`);
    expect(d.length, dump(d)).toBe(1);
    expect(d[0]!.code).toBe("label-overflow");
    const help = d[0]!.help[0]!;
    const part = help.split("size (")[1];
    if (part === undefined) throw new Error(help);
    const size = part.split(")")[0]!;
    const fixed = errors(`${def}box a "A rather long label" at (0,0) size (${size}) shape=tri\n`);
    expect(fixed, `at (${size}): ${dump(fixed)}`).toEqual([]);
  });

  test("custom_shape_errors", () => {
    for (const [body, code, line] of [
      ['shape s {\n  path "M 0 0 L 2 0"\n}\n', "parse", 3],
      ['shape s {\n  path "m 0 0 l 1 0"\n}\n', "parse", 3],
      ['shape s {\n  path "M 0 0 L 1 0"\n  outline 2\n}\n', "parse", 4],
      ['shape s {\n  path "M 0 0 L 1 0"\n  text-area (0.5,0.5,0.6,0.1)\n}\n', "parse", 4],
      ['shape cloud {\n  path "M 0 0 L 1 0"\n}\n', "duplicate-style", 2],
      [
        'shape s {\n  path "M 0 0 L 1 0"\n}\nshape s {\n  path "M 0 0 L 1 1"\n}\n',
        "duplicate-style",
        5,
      ],
      [
        'group g at (0,0) size (10,10) {\n  shape s {\n    path "M 0 0 L 1 0"\n  }\n}\n',
        "parse",
        3,
      ],
    ] as [string, string, number][]) {
      const d = errors(body);
      expect(
        d.map((x) => [x.code, x.span.line]),
        `${body}: ${dump(d)}`,
      ).toEqual([[code, line]]);
    }
  });

  test("unknown_custom_shape_suggests_a_definition", () => {
    const d = errors(
      'shape tri {\n  path "M 0 0 L 1 0 L 0 1 Z"\n}\nbox b at (0,0) size (10,10) shape=tr\n',
    );
    expect(d.length, dump(d)).toBe(1);
    expect(d[0]!.code).toBe("unknown-shape");
    expect(d[0]!.help[0]).toBe("did you mean `tri`?");
  });
});
