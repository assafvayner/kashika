import { describe, expect, it } from "vitest";

import { hitTest } from "../src/document/hit.js";
import { Document } from "../src/document/store.svelte.js";
import type { Elem } from "../src/engine/types.js";
import { engine } from "./wasm.js";

const SRC = `diagram "t" size (300,200)
group g "G" at (10,10) size (200,150) {
  box a "A" at (20,30) size (80,40)
}
`;

describe("hitTest", () => {
  const els: Elem[] = [
    {
      id: "g",
      kind: "group",
      parent: null,
      depth: 0,
      bbox: [
        [10, 10],
        [210, 160],
      ],
    },
    {
      id: "a",
      kind: "box",
      parent: "g",
      depth: 1,
      bbox: [
        [30, 40],
        [110, 80],
      ],
    },
  ];
  it("picks the innermost element", () => {
    expect(hitTest(els, 50, 50)).toBe("a");
    expect(hitTest(els, 150, 100)).toBe("g");
    expect(hitTest(els, 250, 50)).toBeNull();
  });
});

describe("Document", () => {
  it("renders on set and keeps undo history", () => {
    const doc = new Document(engine, SRC);
    expect(doc.result.errors).toBe(0);
    expect(doc.result.elements.map((e) => e.id)).toEqual(["g", "a"]);
    doc.set(SRC + 'box b "B" at (220,30) size (60,40)\n', "agent");
    expect(doc.result.elements.length).toBe(3);
    expect(doc.canUndo).toBe(true);
    doc.undo();
    expect(doc.source).toBe(SRC);
    expect(doc.canUndo).toBe(false);
  });

  it("always has an svg, even with errors", () => {
    const doc = new Document(
      engine,
      'diagram "t" size (100,100)\nbox a "A" at (0,0) size (60,40) icon="x.svg"\n',
    );
    expect(doc.result.errors).toBe(1);
    expect(doc.result.svg).toContain("<svg");
  });

  it("clears a selection that no longer exists", () => {
    const doc = new Document(engine, SRC);
    doc.selection = "a";
    doc.set('diagram "t" size (100,100)\n', "user");
    expect(doc.selection).toBeNull();
  });

  it("titles from the diagram statement", () => {
    expect(new Document(engine, SRC).title).toBe("t");
    expect(new Document(engine, "box a\n").title).toBe("Untitled");
  });
});
