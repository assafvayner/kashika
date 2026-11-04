import { expect, it } from "vitest";

import {
  Ctx,
  type Omitted,
  checkBounded,
  checkDeferred,
  metrics,
} from "../../src/check/index.js";
import { type Diagnostic, sort } from "../../src/diag/index.js";
import { type Scene } from "../../src/model/index.js";
import { scene } from "./common/mod.js";

/// Five horizontal lines crossing five vertical ones: 25 crossings, plus a line along a box.
function grid(): Scene {
  let body = "box a at (600,100) size (80,60)\nline n (560,163) -> h 150\n";
  for (let i = 0; i < 5; i++) {
    const at = 50 + 60 * i;
    body += `line h${i} (20,${at}) -> h 300\n`;
    body += `line v${i} (${at},20) -> v 300\n`;
  }
  return scene(body);
}

function all(_: Diagnostic): boolean {
  return true;
}

it("a_roomy_limit_reports_what_check_deferred_does", () => {
  const s = grid();
  const b = checkBounded(Ctx.new(s), 1000, all);
  expect(b.omitted).toEqual([]);
  expect(b.diags).toEqual(checkDeferred(s));
  expect(b.metrics(s)).toEqual(metrics(s, b.diags));
});

it("past_the_limit_findings_are_counted_and_the_first_in_order_kept", () => {
  const s = grid();
  const b = checkBounded(Ctx.new(s), 2, all);
  const crosses = checkDeferred(s).filter((d) => d.code === "cross");
  expect(crosses.length).toBe(25);
  sort(crosses);
  const kept = b.diags.filter((d) => d.code === "cross");
  expect(kept).toEqual(crosses.slice(0, 2));
  expect(b.omitted).toEqual([{ code: "cross", suppressed: false, count: 23 } satisfies Omitted]);
  expect(b.diags.some((d) => d.code === "near")).toBe(true);
  expect(b.found("cross")).toBe(25);
  expect(b.metrics(s).crossings).toBe(25);
});

it("a_zero_limit_only_counts", () => {
  const s = grid();
  const b = checkBounded(Ctx.new(s), 0, all);
  expect(b.diags).toEqual([]);
  const total = b.omitted.reduce((n, o) => n + o.count, 0);
  expect(total).toBe(checkDeferred(s).length);
});

it("the_filter_drops_findings_before_the_limit_but_metrics_count_them", () => {
  const s = grid();
  const onlyH0 = (d: Diagnostic) => d.elements.some((e) => e.id === "h0");
  const b = checkBounded(Ctx.new(s), 1000, onlyH0);
  expect(b.diags.length).toBe(5);
  expect(b.diags.every(onlyH0)).toBe(true);
  expect(b.metrics(s).crossings).toBe(25);
});

it("suppressed_findings_have_their_own_limit", () => {
  const s = scene(
    "line a (0,50) -> h 100 allow=cross\nline b (50,0) -> v 100\nline c (60,0) -> v 100\n" +
      "line d (0,80) -> h 100\n",
  );
  const b = checkBounded(Ctx.new(s), 1, all);
  const kept = b.diags.filter((d) => d.code === "cross").map((d) => d.suppressed);
  expect(kept.length).toBe(2);
  expect(kept.includes(true) && kept.includes(false)).toBe(true);
  expect(b.omitted).toEqual([
    { code: "cross", suppressed: false, count: 1 },
    { code: "cross", suppressed: true, count: 1 },
  ]);
});
