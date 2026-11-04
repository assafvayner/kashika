import { describe, expect, it } from "vitest";

import { systemPrompt } from "../src/agent/prompt.js";
import { TOOL_DEFS, runTool } from "../src/agent/tools.js";
import { Document } from "../src/document/store.svelte.js";
import { engine } from "./wasm.js";

const SRC = `diagram "t" size (300,200)
box a "A" at (20,20) size (80,40)
box b "B" at (200,20) size (80,40)
`;

describe("systemPrompt", () => {
  it("carries the skill, the core guide, and no CLI commands", () => {
    const p = systemPrompt(engine);
    expect(p).toContain("no auto-layout");
    expect(p).toContain("## Topics");
    expect(p).not.toMatch(/`kashika (check|render|route|inspect|ref|measure|icons)/);
    expect(p).toContain("write_source");
  });
});

describe("runTool", () => {
  const doc = () => new Document(engine, SRC);
  it("write_source replaces the document and reports", () => {
    const d = doc();
    const out = runTool(engine, d, "write_source", { source: SRC + "line ab a.right -> b.left\n" });
    expect(d.source).toContain("line ab");
    expect(out).toContain("0 errors, 0 warnings");
  });
  it("read_source, check, inspect, route, measure, ref, icons", () => {
    const d = doc();
    expect(runTool(engine, d, "read_source", {})).toBe(SRC);
    expect(runTool(engine, d, "check", {})).toContain("0 errors");
    expect(runTool(engine, d, "inspect", { ids: ["a"] })).toContain("a");
    expect(runTool(engine, d, "route", { from: "a.right", to: "b.left" })).toMatch(/^#1/);
    expect(runTool(engine, d, "measure", { text: "Hello" })).toMatch(/width=/);
    expect(runTool(engine, d, "ref", { topic: "through" })).toMatch(/^# through/);
    expect(runTool(engine, d, "icons", { search: "database" })).toContain("cylinder");
  });
  it("reports engine errors and unknown tools as text", () => {
    const d = doc();
    expect(runTool(engine, d, "inspect", { ids: ["zz"] })).toMatch(/^error: unknown id/);
    expect(runTool(engine, d, "nope", {})).toMatch(/^error: unknown tool/);
    expect(runTool(engine, d, "write_source", {})).toMatch(/^error: /);
  });
  it("every tool has a definition", () => {
    expect(TOOL_DEFS.map((t) => t.name).sort()).toEqual([
      "check",
      "icons",
      "inspect",
      "measure",
      "read_source",
      "ref",
      "route",
      "write_source",
    ]);
  });
});
