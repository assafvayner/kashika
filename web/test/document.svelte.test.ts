import { flushSync } from "svelte";
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

  it("prefers the later sibling at equal depth", () => {
    const sib: Elem[] = [
      {
        id: "x",
        kind: "box",
        parent: null,
        depth: 0,
        bbox: [
          [0, 0],
          [50, 50],
        ],
      },
      {
        id: "y",
        kind: "box",
        parent: null,
        depth: 0,
        bbox: [
          [25, 25],
          [75, 75],
        ],
      },
    ];
    expect(hitTest(sib, 30, 30)).toBe("y");
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

  it("unescapes the title", () => {
    expect(new Document(engine, 'diagram "a \\"b\\"" size (10,10)\n').title).toBe('a "b"');
  });

  it("keeps a selection that still exists", () => {
    const doc = new Document(engine, SRC);
    doc.selection = "a";
    doc.set(SRC + 'box b "B" at (220,30) size (60,40)\n', "user");
    expect(doc.selection).toBe("a");
  });

  it("ignores an unchanged source", () => {
    const doc = new Document(engine, SRC);
    doc.set(SRC, "user");
    expect(doc.canUndo).toBe(false);
    expect(doc.version).toBe(0);
  });

  it("clears the selection on undo when its element disappears", () => {
    const doc = new Document(engine, SRC);
    doc.set(SRC + 'box b "B" at (220,30) size (60,40)\n', "user");
    doc.selection = "b";
    doc.undo();
    expect(doc.selection).toBeNull();
  });

  it("caps history at 100 entries", () => {
    const doc = new Document(engine, SRC);
    for (let i = 1; i <= 105; i++) doc.set(SRC + `// ${i}\n`, "user");
    let n = 0;
    while (doc.canUndo) {
      doc.undo();
      n++;
    }
    expect(n).toBe(100);
  });

  it("makes canUndo reactive", () => {
    const doc = new Document(engine, SRC);
    const seen: boolean[] = [];
    const stop = $effect.root(() => {
      $effect(() => {
        seen.push(doc.canUndo);
      });
    });
    flushSync();
    doc.set(SRC + "// x\n", "user");
    flushSync();
    doc.undo();
    flushSync();
    stop();
    expect(seen).toEqual([false, true, false]);
  });
});
