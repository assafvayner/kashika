import { fireEvent, render } from "@testing-library/svelte";
import { describe, expect, it } from "vitest";

import { Document } from "../src/document/store.svelte.js";
import Canvas from "../src/ui/Canvas.svelte";
import { engine } from "./wasm.js";

const SRC = 'diagram "t" size (300,200)\nbox a "A" at (20,20) size (80,40)\n';

describe("Canvas", () => {
  it("shows the svg and selects on click", async () => {
    const doc = new Document(engine, SRC);
    const { container } = render(Canvas, { doc });
    const svg = container.querySelector("svg");
    expect(svg).not.toBeNull();
    const el = svg!;
    Object.defineProperty(el, "getScreenCTM", { value: () => new DOMMatrix() });
    await fireEvent.click(el, { clientX: 50, clientY: 30 });
    expect(doc.selection).toBe("a");
    await fireEvent.click(el, { clientX: 250, clientY: 150 });
    expect(doc.selection).toBeNull();
  });
});
