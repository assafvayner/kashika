import { expect, it } from "vitest";

import { check } from "../../src/check/index.js";
import { type Diagnostic } from "../../src/diag/index.js";
import { lint, sceneSrc } from "./common/mod.js";

const CROSSING =
  "line a (0,50) -> h 100\nline b (50,0) -> v 100\nline c (0,70) -> h 100\n";

function crosses(src: string): Diagnostic[] {
  return check(sceneSrc(src)).filter((d) => d.code === "cross");
}

it("diagram_allow_silences_a_code_everywhere_without_a_reason", () => {
  const d = crosses(`diagram "t" size (200,200) allow=cross\n${CROSSING}`);
  expect(d.length).toBe(2);
  expect(d.every((d) => d.suppressed && d.reason === null)).toBe(true);
  const near = check(sceneSrc(`diagram "t" size (200,200) allow=near\n${CROSSING}`));
  expect(near.some((d) => d.code === "cross" && !d.suppressed)).toBe(true);
});

it("diagram_allow_keeps_the_reason_an_element_gives", () => {
  const src = `diagram "t" size (200,200) allow=cross\n${CROSSING.replace(
    "line a (0,50) -> h 100",
    'line a (0,50) -> h 100 allow=cross:b reason="bus"',
  )}`;
  const d = crosses(src);
  expect(d.every((d) => d.suppressed)).toBe(true);
  const reasons = d.map((d) => d.reason);
  expect(reasons).toContain("bus");
  expect(reasons).toContain(null);
});

it("allow_code_other_silences_only_findings_naming_the_other", () => {
  const d = lint(
    "cross",
    CROSSING.replace("line b (50,0) -> v 100", 'line b (50,0) -> v 100 allow=cross:a reason="tap"'),
  );
  expect(d.length).toBe(2);
  const withA = d.find((d) => d.elements.some((e) => e.id === "a"))!;
  expect(withA.suppressed).toBe(true);
  expect(withA.reason).toBe("tap");
  const withC = d.find((d) => d.elements.some((e) => e.id === "c"))!;
  expect(withC.suppressed).toBe(false);
  expect(withC.reason).toBeNull();
});

it("allow_code_other_never_matches_the_owner_itself", () => {
  const d = lint(
    "cross",
    CROSSING.replace("line b (50,0) -> v 100", "line b (50,0) -> v 100 allow=cross:b"),
  );
  expect(d.every((d) => !d.suppressed)).toBe(true);
});

it("allow_code_other_on_a_box", () => {
  const body =
    'box card at (0,0) size (100,100)\nbox badge at (80,80) size (40,40) allow=overlap:card reason="badge"\nbox other at (110,110) size (40,40)\n';
  const d = lint("overlap", body);
  expect(d.length).toBe(2);
  const flags = d.map((d): [boolean, string | null] => [d.suppressed, d.reason]);
  expect(flags).toContainEqual([true, "badge"]);
  expect(flags).toContainEqual([false, null]);
});
