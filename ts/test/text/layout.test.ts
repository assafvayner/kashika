import { describe, expect, test } from "vitest";
import { Rect } from "../../src/geom/index.js";
import { type Block, measure, stack, stackSize } from "../../src/text/index.js";

function m(s: string, size: number): Block {
  const r = measure(s, size, false);
  if (!r.ok) throw new Error(r.error.toString());
  return r.value;
}

describe("layout", () => {
  test("place_at_centers_vertically", () => {
    const b = m("API", 14);
    const [lines, r] = b.placeAt(100, 50, "center");
    expect(Math.abs(r.x - (100 - b.width() / 2))).toBeLessThan(1e-9);
    expect(r.y).toBe(41.25);
    expect(r.h).toBe(17.5);
    expect(Math.abs(lines[0]!.baseline - (41.25 + 0.98876953125 * 14))).toBeLessThan(1e-9);
  });

  test("place_at_left_and_right", () => {
    const b = m("API", 14);
    expect(b.placeAt(10, 0, "left")[1].x).toBe(10);
    const r = b.placeAt(10, 0, "right")[1];
    expect(Math.abs(r.right() - 10)).toBeLessThan(1e-9);
  });

  test("multi_line_blocks_align_each_line", () => {
    const b = m("Postgres\nAPI", 14);
    const [lines, r] = b.place(0, 100, 0, "right");
    expect(Math.abs(lines[0]!.x + lines[0]!.width - 100)).toBeLessThan(1e-9);
    expect(Math.abs(lines[1]!.x + lines[1]!.width - 100)).toBeLessThan(1e-9);
    expect(lines[1]!.baseline - lines[0]!.baseline).toBe(17.5);
    expect(Math.abs(r.w - b.width())).toBeLessThan(1e-9);
    expect(r.h).toBe(35);
  });

  test("stack_aligns_items_in_the_area", () => {
    const a = m("Payments API", 14);
    const s = m("v2.4.1", 11.2);
    const area = new Rect(8, 8, 184, 104);
    const out = stack(
      [
        [a, "left"],
        [s, "right"],
      ],
      area,
      "middle",
      2,
    );
    const h = 17.5 + 14 + 2;
    expect(stackSize([a, s], 2)[1]).toBe(h);
    expect(out[0]![1].y).toBe(8 + (104 - h) / 2);
    expect(out[0]![1].x).toBe(8);
    expect(out[1]![1].y).toBe(out[0]![1].y + 17.5 + 2);
    expect(Math.abs(out[1]![1].right() - 192)).toBeLessThan(1e-9);
    const top = stack([[a, "center"]], area, "top", 2);
    expect(top[0]![1].y).toBe(8);
    const bottom = stack([[a, "center"]], area, "bottom", 2);
    expect(bottom[0]![1].bottom()).toBe(112);
  });
});
