import type { Elem } from "../engine/types.js";

/** The id of the innermost element whose box contains (x, y) in canvas coordinates, or null. */
export function hitTest(elements: readonly Elem[], x: number, y: number): string | null {
  let best: Elem | null = null;
  for (const e of elements) {
    const [[x1, y1], [x2, y2]] = e.bbox;
    if (x < x1 || x > x2 || y < y1 || y > y2) continue;
    if (best === null || e.depth > best.depth) best = e;
  }
  return best?.id ?? null;
}
