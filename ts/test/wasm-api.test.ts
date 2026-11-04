import { readFileSync } from "node:fs";
import { join } from "node:path";
import { expect, test } from "vitest";

import { render, renderJson, version } from "../src/wasm-api.js";

const CLEAN = 'diagram "t" size (200,100)\nbox a "A" at (20,20) size (80,40)\n';
const BROKEN =
  'diagram "t" size (200,100)\nbox a "A" at (20,20) size (80,40) icon="missing.svg"\n';
const ROOT = join(import.meta.dirname, "../..");

test("clean_source_renders_svg", () => {
  const r = render(CLEAN, false);
  expect([r.errors, r.warnings]).toEqual([0, 0]);
  expect(r.svg!.startsWith("<svg")).toBe(true);
});

test("file_references_are_not_found", () => {
  const r = render(BROKEN, false);
  expect(r.errors).toBe(1);
  expect(r.diagnostics[0]!.code).toBe("file");
  expect(r.svg).toBeNull();
});

test("force_renders_despite_errors", () => {
  const r = render(BROKEN, true);
  expect(r.errors).toBe(1);
  expect(r.svg).not.toBeNull();
});

test("library_has_no_svg", () => {
  const r = render('style s { fill="#ffffff" }\n', true);
  expect(r.svg).toBeNull();
});

test("json_has_the_report_fields", () => {
  const s = renderJson(BROKEN, false);
  const v = JSON.parse(s);
  expect(v.svg).toBeNull();
  expect(v.errors).toBe(1);
  expect(v.warnings).toBe(0);
  expect(v.diagnostics[0].code).toBe("file");
  expect(v.diagnostics[0].span.line).toBe(2);
  expect(s.startsWith('{"svg":null,"errors":1,"warnings":0,"diagnostics":[{"severity":"error","code":"file","message":')).toBe(true);
});

test("matches_the_cli_render_of_the_upload_example", () => {
  const src = readFileSync(join(ROOT, "examples/upload/upload.kash"), "utf8");
  const svg = render(src, false).svg!;
  expect(svg).toBe(readFileSync(join(ROOT, "examples/upload/upload.svg"), "utf8"));
});

test("diagnostics_are_sorted_when_there_is_no_scene", () => {
  const r = render('box a "A" at (0,0) size (10,10)\nbox b\n', false);
  const lines = r.diagnostics.map((d) => d.span.line);
  expect(lines.every((l, i) => i === 0 || lines[i - 1]! <= l)).toBe(true);
});

test("version is the tool version", () => {
  expect(version()).toBe("0.1.0-dev0");
});
