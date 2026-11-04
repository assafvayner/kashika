import { expect, test } from "vitest";

import { Diagnostic, Span } from "../../src/diag/index.js";
import { compile } from "../../src/model/index.js";
import { check } from "../../src/check/index.js";
import { Overlay, renderSvg, renderSvgWith } from "../../src/render/index.js";

const SRC =
  'diagram "o" size (300,200)\n' +
  'box a "A" at (10,10) size (40,20) colr=#fff\n' +
  'box b "B" at (100,100) size (40,20)\n' +
  "line ab a.right -> hv b.top\n" +
  "line bad a.rigth -> b.left\n";

function warnOnLine4(): Diagnostic {
  return Diagnostic.warn("near", Span.new(4, 1, 4, 5), "test warning");
}

const MARKERS: Overlay = { errorMarkers: true, debug: false };
const DEBUG: Overlay = { errorMarkers: false, debug: true };

function count(s: string, needle: string): number {
  return s.split(needle).length - 1;
}

test("no_overlay_is_the_plain_svg", () => {
  const [scene, diags] = compile(SRC);
  expect(renderSvgWith(scene, diags, Overlay.default())).toBe(renderSvg(scene));
});

test("error_markers_frame_elements_and_list_the_rest", () => {
  const [scene, diags] = compile(SRC);
  const s = renderSvgWith(scene, diags, MARKERS);
  expect(s).toContain('<rect x="7" y="7" width="46" height="26" fill="none" stroke="#dc2626"');
  expect(s).toContain('aria-label="error[attr]"');
  expect(s).toContain('error[unknown-anchor]"');
  expect(s).toContain('<g id="kashika-overlay">');
  expect(s.endsWith("</g>\n</svg>\n")).toBe(true);
});

test("warnings_are_marked_only_in_debug", () => {
  const [scene, diags] = compile(SRC);
  diags.push(warnOnLine4());
  expect(renderSvgWith(scene, diags, MARKERS)).not.toContain("#ea580c");
  const s = renderSvgWith(scene, diags, DEBUG);
  expect(s).toContain('stroke="#ea580c" stroke-opacity="0.5"');
  expect(s).toContain('aria-label="warn[near]"');
});

test("overlay_is_deterministic", () => {
  const [scene, diags] = compile(SRC);
  const o: Overlay = { errorMarkers: true, debug: true };
  expect(renderSvgWith(scene, diags, o)).toBe(renderSvgWith(scene, diags, o));
});

test("debug_draws_rulers_ids_and_anchor_dots", () => {
  const [scene, diags] = compile(SRC);
  const s = renderSvgWith(scene, diags, DEBUG);
  expect(s).toContain("M 100 0 V 10");
  expect(s).toContain("M 0 100 H 10");
  for (const label of ["100", "a", "b", "ab"]) {
    expect(s).toContain(`aria-label="${label}"`);
  }
  expect(count(s, 'r="1.5" fill="#2563eb"')).toBe(18);
});

test("debug_marks_a_through_warning_at_its_entry_point", () => {
  const src =
    'diagram "t" size (300,100)\n' +
    "box a at (10,30) size (40,40)\n" +
    "box w at (120,20) size (40,60)\n" +
    "box b at (240,30) size (40,40)\n" +
    "line ab a.right -> b.left\n";
  const [scene] = compile(src);
  const d = check(scene).find((d) => d.code === "through");
  expect(d).toBeDefined();
  const s = renderSvgWith(scene, [d!], DEBUG);
  expect(s).toContain('<circle cx="120" cy="50" r="6" fill="none" stroke="#ea580c"');
});

test("debug_text_halo_uses_the_background", () => {
  const halo = (head: string, color: string): boolean => {
    const [scene, diags] = compile(`${head}\nbox a "A" at (10,10) size (80,40)\n`);
    expect(diags).toEqual([]);
    const s = renderSvgWith(scene, diags, DEBUG);
    const overlay = s.slice(s.indexOf("kashika-overlay"));
    const want = `stroke="${color}" stroke-width="3"`;
    return overlay.includes(want) && count(overlay, 'stroke-width="3"') === count(overlay, want);
  };
  expect(halo('diagram "o" size (300,200)', "#ffffff")).toBe(true);
  expect(halo('diagram "o" size (300,200) background=#0f172a', "#0f172a")).toBe(true);
});
