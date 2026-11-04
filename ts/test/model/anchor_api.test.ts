import { describe, expect, test } from "vitest";
import { Pt } from "../../src/geom/index.js";
import { compile } from "../../src/model/index.js";

describe("anchor_api", () => {
  test("shape_anchor_matches_the_resolver", () => {
    const [scene, diags] = compile(
      'diagram "a" size (200,100)\nbox r at (10,10) size (100,60) shape=rounded\n',
    );
    expect(diags, JSON.stringify(diags)).toEqual([]);
    const r = scene.shapes[0]!;
    expect(r.anchor("top", null)).toEqual(new Pt(60.0, 10.0));
    expect(r.anchor("left", null)).toEqual(new Pt(10.0, 40.0));
    expect(r.anchor("tl", null)).toEqual(new Pt(10.0, 10.0));
    expect(r.anchor("bottom-right", null)).toEqual(new Pt(110.0, 70.0));
    expect(r.anchor("nope", null)).toBeNull();
  });
});
