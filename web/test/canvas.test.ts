import { fireEvent, render } from "@testing-library/svelte";
import { tick } from "svelte";
import { describe, expect, it } from "vitest";

import { Document } from "../src/document/store.svelte.js";
import Diagnostics from "../src/ui/Diagnostics.svelte";
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

  it("zooms with the wheel and fit resets", async () => {
    const doc = new Document(engine, SRC);
    const { container, component } = render(Canvas, { doc });
    const stage = container.querySelector<HTMLElement>(".stage")!;
    const scale = () => Number(/scale\(([\d.]+)\)/.exec(stage.style.transform)![1]);
    await fireEvent.wheel(container.querySelector(".canvas")!, { deltaY: -200 });
    expect(scale()).toBeGreaterThan(1);
    component.fit();
    await tick();
    expect(scale()).toBe(1);
  });

  it("does not select after a drag", async () => {
    const doc = new Document(engine, SRC);
    const { container } = render(Canvas, { doc });
    const el = container.querySelector("svg")!;
    Object.defineProperty(el, "getScreenCTM", { value: () => new DOMMatrix() });
    const canvas = container.querySelector(".canvas")!;
    await fireEvent.mouseDown(canvas, { clientX: 50, clientY: 30 });
    await fireEvent.mouseMove(canvas, { clientX: 60, clientY: 40 });
    await fireEvent.mouseUp(canvas, { clientX: 60, clientY: 40 });
    await fireEvent.click(el, { clientX: 50, clientY: 30 });
    expect(doc.selection).toBeNull();
  });
});

describe("Diagnostics", () => {
  it("counts unsuppressed items and lists them when opened", async () => {
    const span = { line: 1, col: 1, end_line: 1, end_col: 2 };
    const diagnostics = [
      { severity: "error" as const, code: "e1", message: "bad", span },
      { severity: "warn" as const, code: "w1", message: "meh", span },
      { severity: "warn" as const, code: "w2", message: "hidden", span, suppressed: true },
    ];
    const { container, getByText } = render(Diagnostics, { diagnostics });
    expect(getByText("1 error")).toBeTruthy();
    expect(getByText("1 warning")).toBeTruthy();
    await fireEvent.click(container.querySelector(".toggle")!);
    expect(container.querySelectorAll("li")).toHaveLength(2);
  });
});
