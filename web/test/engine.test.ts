import { describe, expect, it } from "vitest";

import { EngineError } from "../src/engine/types.js";
import { engine } from "./wasm.js";

const SRC = `diagram "t" size (300,200)
box a "A" at (20,20) size (80,40)
box b "B" at (200,20) size (80,40)
line ab a.right -> b.left
`;

describe("engine", () => {
  it("renders with elements", () => {
    const r = engine.render(SRC);
    expect(r.svg).toMatch(/^<svg/);
    expect(r.errors).toBe(0);
    expect(r.elements.map((e) => e.id)).toEqual(["a", "b"]);
  });

  it("force renders broken input", () => {
    const r = engine.render(
      'diagram "t" size (100,100)\nbox a "A" at (0,0) size (80,40) icon="x.svg"\n',
    );
    expect(r.errors).toBe(1);
    expect(r.svg).toContain("kashika-overlay");
  });

  it("checks", () => {
    const c = engine.check(SRC);
    expect(c.errors).toBe(0);
    expect(c.text).toContain("0 errors");
  });

  it("inspects, routes, measures", () => {
    expect(engine.inspect(SRC, ["a"])).toContain("a");
    expect(() => engine.inspect(SRC, ["zz"])).toThrow(EngineError);
    expect(engine.route(SRC, "a.right", "b.left")).toMatch(/^#1/);
    expect(() => engine.route(SRC, "a.right", "zz.left")).toThrow(/error\[/);
    expect(engine.measure("ab").lines).toBe(1);
  });

  it("formats, refs, icons", () => {
    expect(engine.format('diagram "t"  size (1,1)\n')).toBe('diagram "t" size (1, 1)\n');
    expect(engine.ref()).toContain("## Topics");
    expect(engine.ref("through")).toMatch(/^# through/);
    expect(() => engine.ref("nope")).toThrow(/unknown ref topic/);
    expect(engine.icons("database").some((i) => i.name === "cylinder")).toBe(true);
    expect(engine.version()).toMatch(/^\d/);
  });
});
