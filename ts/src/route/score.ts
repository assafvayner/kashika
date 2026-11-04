//! Scores probe lines with the same checks `kashika check` runs.

import { checkDeferred, suppresses as checkSuppresses } from "../check/index.js";
import { type Ctx } from "../check/ctx.js";
import { type Axis, freeIntervals } from "../check/free.js";
import { type Diagnostic, code, fact } from "../diag/index.js";
import { Pt, TOLERANCE, length } from "../geom/index.js";
import { type Line, type Scene } from "../model/index.js";
import { Corridor, Crossing, Transit } from "./index.js";

export interface Score {
  /// Every finding that counts, suppressed ones included.
  findings: Diagnostic[];
  crossed: Crossing[];
  transited: Transit[];
  near: number;
  length: number;
}

/// Scores `probe` with the findings `kashika check` reports for it, suppressed ones included.
/// Findings that also name `ignore`, the line being rerouted, don't count. `null` means the
/// probe enters a box.
export function score(scene: Scene, probe: Line, ignore: string | null): Score | null {
  const mine: Diagnostic[] = checkDeferred(scene)
    .filter((d) => d.code !== code.UNUSED_ALLOW)
    .filter((d) => involves(d, probe.id) && !(ignore !== null && involves(d, ignore)));
  if (mine.some((d) => d.code === code.THROUGH)) {
    return null;
  }
  const crossed: Crossing[] = [];
  for (const d of mine.filter((d) => d.code === code.CROSS)) {
    const p = at(d, fact.AT);
    if (p === null) continue;
    const line = d.elements.map((e) => e.id).find((id) => id !== probe.id) ?? "";
    crossed.push(new Crossing(line, p));
  }
  const transited: Transit[] = [];
  for (const d of mine.filter((d) => d.code === code.GROUP_TRANSIT)) {
    const group = d.elements.find((e) => e.kind === "group");
    if (group === undefined) continue;
    const p = at(d, fact.ENTRY);
    if (p === null) continue;
    transited.push(new Transit(group.id, p));
  }
  const near = mine.filter((d) => d.code === code.NEAR || d.code === code.SHARED_SEGMENT).length;
  return { findings: mine, crossed, transited, near, length: length(probe.path) };
}

/// Whether the `allow=` entry `entry` (`code` or `code:other`) on the line `owner` suppresses
/// `d`, as `kashika check` decides.
export function suppresses(entry: string, owner: string, d: Diagnostic): boolean {
  const i = entry.indexOf(":");
  const [c, other] = i < 0 ? [entry, null] : [entry.slice(0, i), entry.slice(i + 1)];
  return checkSuppresses(c, other, owner, d);
}

function involves(d: Diagnostic, id: string): boolean {
  return d.elements.some((e) => e.id === id);
}

/// The point `fact` of `d`, which its lint always writes.
function at(d: Diagnostic, f: string): Pt | null {
  const p = d.point(f);
  return p === null ? null : new Pt(p[0], p[1]);
}

/// The free interval, as `kashika inspect --corridors` reports it, that holds a lane at
/// `value` running across `span`. Lines in `skip` aren't obstacles. `null` when the lane
/// doesn't keep `clearance`.
export function corridor(
  ctx: Ctx,
  axis: Axis,
  value: number,
  span: [number, number],
  skip: readonly string[],
): Corridor | null {
  const within: [number, number] =
    axis === "x" ? [0.0, ctx.scene.size[0]] : [0.0, ctx.scene.size[1]];
  const found = freeIntervals(ctx, axis, span, within, skip).find(
    ([lo, hi]) => lo - TOLERANCE <= value && value <= hi + TOLERANCE,
  );
  return found === undefined ? null : new Corridor(axis, [found[0], found[1]]);
}
