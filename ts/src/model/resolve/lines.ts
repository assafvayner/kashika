import { Diagnostic, type Span } from "../../diag/diagnostic.js";
import { displayF64 } from "../../diag/rust.js";
import {
  type PathSeg,
  Pt,
  TOLERANCE,
  cubic,
  isAxisAligned,
  line,
  pointAt,
  quad,
  roundCornersTagged,
} from "../../geom/index.js";
import type * as ast from "../../syntax/ast.js";
import { Entry, type Resolver } from "../resolve.js";
import { type Label, LabelSide, Line, Orientation, Segment } from "../scene.js";
import * as textlayout from "../textlayout.js";
import { finite } from "./checks.js";
import { evalPoint, evalTarget, frameOrigin } from "./placement.js";
import * as record from "./record.js";

export function buildLine(
  r: Resolver,
  idx: number,
  l: ast.Line,
  parent: string | null,
): Line | null {
  const origin = frameOrigin(r, idx);
  if (origin === null) return null;
  const start = evalPoint(r, l.start, origin);
  let failed = start === null;
  let cur = start ?? origin;
  const raw: [PathSeg, number][] = [];
  const segments: Segment[] = [];
  for (const seg of l.segs) {
    if (failed) {
      evalSegPoints(r, seg, origin);
      continue;
    }
    const from = cur;
    const segs = expand(r, seg, cur, origin);
    if (segs === null) {
      failed = true;
      continue;
    }
    const pieces: PathSeg[] = [];
    for (const s of segs) {
      cur = s.to;
      if (s.kind === "line" && s.from.dist(s.to) <= 1e-9) {
        continue;
      }
      raw.push([s, segments.length]);
      pieces.push(s);
    }
    segments.push(new Segment(record.segmentKind(seg.kind), seg.span, from, cur, pieces));
  }
  if (failed) {
    return null;
  }
  if (raw.length === 0) {
    r.diags.push(Diagnostic.error("zero-length", l.span, "line has zero length"));
    return null;
  }
  const p = Entry.takeLine(r.entries[idx]!);
  if (p === null) return null;
  if (p.ortho) {
    for (const [s, i] of raw) {
      if (!isAxisAligned(s)) {
        const d = p.orthoInherited
          ? Diagnostic.error(
              "not-ortho",
              segments[i]!.span,
              "segment is not horizontal or vertical, and the diagram sets `ortho`",
            ).withHelp(
              "use `hv`/`vh` elbows or `x`/`y` targets, or `ortho=false` on this line for a deliberate diagonal or curve",
            )
          : Diagnostic.error(
              "not-ortho",
              segments[i]!.span,
              "segment is not horizontal or vertical on a line marked `ortho`",
            ).withHelp("use `hv`/`vh` elbows or `x`/`y` targets");
        r.diags.push(d);
      }
    }
  }
  const rounded = p.radius > 0.0 ? roundCornersTagged(raw, p.radius) : raw;
  const path = rounded.map(([s]) => s);
  const pathSeg = rounded.map(([, i]) => i);
  const bold = p.style.fontWeight === "bold";
  const labels = p.labels.filter((lab) => {
    const block = textlayout.measureAt(lab.text, p.style.fontSize, bold, lab.span, r.diags);
    if (block === null) {
      return false;
    }
    lab.at = pointAt(path, lab.t);
    const fits = LabelSide.fits(p.labelSide, path, lab.t);
    if (fits.ok) {
      lab.side = p.labelSide;
    } else {
      r.diags.push(sideMismatch(p.labelSide, fits.error, lab, p.labelSideSpan));
      lab.side = "on";
    }
    const center = lab.at.add(
      LabelSide.offset(lab.side, block.width(), block.height(), p.style.strokeWidth),
    );
    [lab.lines, lab.rect] = block.placeAt(center.x, center.y, "center");
    return true;
  });
  const [id, named] = l.id !== null ? [l.id.name, true] : [`line@L${l.span.line}`, false];
  return new Line({
    id,
    named,
    path,
    segments,
    pathSeg,
    startLiteral: record.isLiteral(l.start),
    endLiteral: record.endIsLiteral(l.segs),
    start: p.start,
    end: p.end,
    style: p.style,
    labels,
    labelSide: p.labelSide,
    ortho: p.ortho,
    allow: p.allow,
    reason: p.reason,
    parent,
    origin,
    span: l.span,
  });
}

/// Evaluates a segment's points only for their diagnostics, once an earlier part of
/// the line has failed and the current point is unknown.
function evalSegPoints(r: Resolver, seg: ast.Seg, origin: Pt): void {
  const k = seg.kind;
  switch (k.kind) {
    case "To":
    case "Hv":
    case "Vh":
      evalPoint(r, k.value, origin);
      break;
    case "Vhv":
    case "Hvh":
    case "Curve":
      evalPoint(r, k.to, origin);
      break;
    case "X":
    case "Y":
      if (k.value.kind === "Point") evalPoint(r, k.value.value, origin);
      break;
    case "CurveVia":
      for (const p of [k.c1, k.c2, k.to]) {
        evalPoint(r, p, origin);
      }
      break;
    case "H":
    case "V":
      break;
  }
}

export function expand(r: Resolver, seg: ast.Seg, cur: Pt, origin: Pt): PathSeg[] | null {
  const k = seg.kind;
  switch (k.kind) {
    case "To": {
      const p = evalPoint(r, k.value, origin);
      return p === null ? null : [line(cur, p)];
    }
    case "H": {
      const x = cur.x + k.value;
      if (!finite(r, [x], seg.span)) return null;
      return [line(cur, new Pt(x, cur.y))];
    }
    case "V": {
      const y = cur.y + k.value;
      if (!finite(r, [y], seg.span)) return null;
      return [line(cur, new Pt(cur.x, y))];
    }
    case "X": {
      const x = evalTarget(r, k.value, origin, "X");
      if (x === null) return null;
      if (!finite(r, [x], seg.span)) return null;
      return [line(cur, new Pt(x, cur.y))];
    }
    case "Y": {
      const y = evalTarget(r, k.value, origin, "Y");
      if (y === null) return null;
      if (!finite(r, [y], seg.span)) return null;
      return [line(cur, new Pt(cur.x, y))];
    }
    case "Hv": {
      const p = evalPoint(r, k.value, origin);
      if (p === null) return null;
      const corner = new Pt(p.x, cur.y);
      return [line(cur, corner), line(corner, p)];
    }
    case "Vh": {
      const p = evalPoint(r, k.value, origin);
      if (p === null) return null;
      const corner = new Pt(cur.x, p.y);
      return [line(cur, corner), line(corner, p)];
    }
    case "Vhv": {
      const p = evalPoint(r, k.to, origin);
      if (p === null) return null;
      if (Math.abs(p.x - cur.x) <= 1e-9) {
        return [line(cur, p)];
      }
      const y = cur.y + (p.y - cur.y) * (k.t ?? 0.5);
      const [a, b] = [new Pt(cur.x, y), new Pt(p.x, y)];
      return [line(cur, a), line(a, b), line(b, p)];
    }
    case "Hvh": {
      const p = evalPoint(r, k.to, origin);
      if (p === null) return null;
      if (Math.abs(p.y - cur.y) <= 1e-9) {
        return [line(cur, p)];
      }
      const x = cur.x + (p.x - cur.x) * (k.t ?? 0.5);
      const [a, b] = [new Pt(x, cur.y), new Pt(x, p.y)];
      return [line(cur, a), line(a, b), line(b, p)];
    }
    case "Curve": {
      const to = evalPoint(r, k.to, origin);
      if (to === null) return null;
      if (!finite(r, [k.amount], seg.span)) return null;
      if (k.amount < 0.0) {
        r.diags.push(
          Diagnostic.error(
            "degenerate-curve",
            seg.span,
            "curve amount must not be negative; use `left`/`right` to choose the side",
          ),
        );
        return null;
      }
      if (cur.dist(to) <= TOLERANCE) {
        r.diags.push(
          Diagnostic.error(
            "degenerate-curve",
            seg.span,
            "curve starts and ends at the same point",
          ).withHelp("use `curve via (c1) (c2) <point>` for loops"),
        );
        return null;
      }
      const d = to.sub(cur).norm();
      // Screen-left of the travel direction in y-down coordinates.
      const normal = k.side === "Left" ? new Pt(d.y, -d.x) : new Pt(-d.y, d.x);
      const ctrl = cur.lerp(to, 0.5).add(normal.mul(2.0 * k.amount));
      return [quad(cur, ctrl, to)];
    }
    case "CurveVia": {
      const c1 = evalPoint(r, k.c1, origin);
      const c2 = evalPoint(r, k.c2, origin);
      const to = evalPoint(r, k.to, origin);
      if (c1 === null || c2 === null || to === null) return null;
      if ([c1, c2, to].every((p) => p.dist(cur) <= TOLERANCE)) {
        r.diags.push(Diagnostic.error("degenerate-curve", seg.span, "curve has zero size"));
        return null;
      }
      return [cubic(cur, c1, c2, to)];
    }
  }
}

function sideMismatch(side: LabelSide, found: Orientation, lab: Label, span: Span): Diagnostic {
  const n = LabelSide.needs(side);
  const need = n === null ? "" : Orientation.name(n);
  let help: string;
  switch (found) {
    case "Horizontal":
      help = "use `label-side=above` or `label-side=below`";
      break;
    case "Vertical":
      help = "use `label-side=left` or `label-side=right`";
      break;
    case "Diagonal":
    case "Curve":
      help = `use \`label-side=on\`, or move \`label@t\` onto a straight ${need} segment`;
      break;
  }
  return Diagnostic.error(
    "attr",
    span,
    `\`label-side=${LabelSide.name(side)}\` needs a ${need} segment, but the label "${lab.text}" at t=${displayF64(lab.t)} sits on a ${Orientation.name(found)} segment`,
  ).withHelp(help);
}
