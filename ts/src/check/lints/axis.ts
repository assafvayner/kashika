import { Diagnostic } from "../../diag/diagnostic.js";
import { Float } from "../../diag/json.js";
import { fmax, fmin, toRadians } from "../../diag/rust.js";
import { Pt, TOLERANCE, boundaryNearest, fmtNum, round2 } from "../../geom/index.js";
import { type Shape } from "../../model/index.js";
import { type Ctx, fmtPt, outlineAnchor } from "../ctx.js";
import { lineEl } from "../element.js";
import { type Sink } from "../sink.js";

/// The largest angle off horizontal or vertical that still reads as a slip: 5°.
const MAX_SLANT_DEG = 5.0;

function toDegrees(v: number): number {
  return v * (180.0 / Math.PI);
}

/// How far a straight piece from `from` to `to` is off the nearest axis, when it is off by
/// more than TOLERANCE but within 5°: (offset in px, angle in degrees, horizontal).
export function slant(from: Pt, to: Pt): [number, number, boolean] | null {
  const [dx, dy] = [Math.abs(to.x - from.x), Math.abs(to.y - from.y)];
  const [major, minor] = [fmax(dx, dy), fmin(dx, dy)];
  return minor > TOLERANCE && minor <= major * Math.tan(toRadians(MAX_SLANT_DEG))
    ? [minor, toDegrees(Math.atan2(minor, major)), dx >= dy]
    : null;
}

/// `near-axis`: a straight segment on a line without `ortho` is within 5° of horizontal or
/// vertical but not on it, so it renders as a visible slant.
export function nearAxis(ctx: Ctx, out: Sink): void {
  for (const l of ctx.lines.filter((l) => !l.line.ortho)) {
    l.line.path.forEach((s, i) => {
      if (s.kind !== "line") {
        return;
      }
      const { from, to } = s;
      const sl = slant(from, to);
      if (sl === null) {
        return;
      }
      const [off, deg, horizontal] = sl;
      const seg = l.segment(i);
      const span = l.line.segments[seg]?.span ?? l.line.span;
      const [axis, coord] = horizontal ? ["y", "horizontal"] : ["x", "vertical"];
      const d = Diagnostic.warn(
        "near-axis",
        span,
        `segment ${seg + 1} of ${l.line.id} is ${fmtNum(off)}px (${fmtNum(deg)}°) off ${coord}, from ${fmtPt(from)} to ${fmtPt(to)}`,
      )
        .withElement(lineEl(l.line, seg))
        .withFact("offset", new Float(round2(off)))
        .withFact("degrees", new Float(round2(deg)));
      const level = (keep: Pt, mv: Pt) =>
        horizontal ? new Pt(mv.x, keep.y) : new Pt(keep.x, mv.y);
      const value = (p: Pt) => fmtNum(horizontal ? p.y : p.x);
      const fixes: string[] = [];
      if (i + 1 === l.line.path.length) {
        const a = anchorAt(ctx, to, level(from, to));
        if (a !== null) {
          fixes.push(`end at \`${a}\` to keep ${axis}=${value(from)}`);
        }
      }
      if (i === 0) {
        const a = anchorAt(ctx, from, level(to, from));
        if (a !== null) {
          fixes.push(`start at \`${a}\` to keep ${axis}=${value(to)}`);
        }
      }
      const or = fixes.length === 0 ? "" : "or ";
      fixes.push(
        `${or}move one end ${fmtNum(off)}px in ${axis} so both ends share it, or use \`hv\`/\`vh\` (or an \`x\`/\`y\` target) for a deliberate elbow`,
      );
      for (const f of fixes) {
        d.withHelp(f);
      }
      out.push(d);
    });
  }
}

/// An anchor for `q` on the box that `at` attaches to, when `q` lies on its outline: `id.side@t`
/// when that lands within 0.25px of `q`, otherwise `id.side+(dx,dy)`.
export function anchorAt(ctx: Ctx, at: Pt, q: Pt): string | null {
  const id = ctx.attached(at)[0];
  if (id === undefined) {
    return null;
  }
  const geom = ctx.boxes().find((s) => s.shape.id === id);
  if (geom === undefined) {
    return null;
  }
  if (boundaryNearest(q, geom.poly)[0] > TOLERANCE) {
    return null;
  }
  return anchorText(geom.shape, q);
}

function anchorText(s: Shape, q: Pt): string {
  const [side, t0] = outlineAnchor(s, q);
  const t = round2(t0);
  const p = s.anchor(side, t);
  if (p !== null && p.dist(q) <= 0.25) {
    return t === 0.5 ? `${s.id}.${side}` : `${s.id}.${side}@${fmtNum(t)}`;
  }
  const c = s.anchor(side, null) ?? q;
  return `${s.id}.${side}+(${fmtNum(q.x - c.x)},${fmtNum(q.y - c.y)})`;
}
