import { type Span } from "../../diag/diagnostic.js";
import * as ast from "../../syntax/ast.js";
import { type Point, type SegKind, type Target, offsetDelta, pointBase } from "../../syntax/ast.js";
import { Item, type Resolver } from "../resolve.js";
import { Literal, PlacementEdge, type SegmentKind } from "../scene.js";

/// Records the placement edges of a box, group or junction: its frame (parent group)
/// and every id its `at` point references.
export function recordEdges(r: Resolver, idx: number, at: Point): void {
  const fromId = Item.id(r.entries[idx]!.item);
  if (fromId === null) {
    return;
  }
  const from = fromId.name;
  const p = r.entries[idx]!.parent;
  const parent = p === null ? null : Item.id(r.entries[p]!.item);
  if (parent !== null) {
    r.edges.push(new PlacementEdge(from, parent.name, "frame"));
  }
  for (const ref of pointRefs(at)) {
    const e = new PlacementEdge(from, ref.id.name, "at");
    if (!r.edges.some((x) => x.eq(e))) {
      r.edges.push(e);
    }
  }
}

export function literal(r: Resolver, owner: string, what: string, value: number, span: Span): void {
  r.literals.push(new Literal(owner, what, value, span));
}

export function recordPoint(r: Resolver, owner: string, p: Point): void {
  switch (p.kind) {
    case "Lit":
      if (p.x.kind === "Num") {
        literal(r, owner, "x", p.x.value, p.span);
      }
      if (p.y.kind === "Num") {
        literal(r, owner, "y", p.y.value, p.span);
      }
      break;
    case "Ref":
      if (p.offset !== null) {
        const [dx, dy] = offsetDelta(p.offset);
        literal(r, owner, "dx", dx, p.span);
        literal(r, owner, "dy", dy, p.span);
      }
      break;
    case "Group":
      recordPoint(r, owner, p.inner);
      break;
    case "Shifted":
      recordPoint(r, owner, p.base);
      for (const o of p.offsets) {
        const [dx, dy] = offsetDelta(o);
        literal(r, owner, "dx", dx, p.span);
        literal(r, owner, "dy", dy, p.span);
      }
      break;
  }
}

/// Records a box or group's `at` literals and the axes of its size that are numbers.
export function recordNode(r: Resolver, n: ast.Node): void {
  recordPoint(r, n.id.name, n.at);
  const size = n.size ?? { w: null, h: null };
  if (size.w !== null) {
    literal(r, n.id.name, "width", size.w, n.span);
  }
  if (size.h !== null) {
    literal(r, n.id.name, "height", size.h, n.span);
  }
}

export function recordLine(r: Resolver, owner: string, l: ast.Line): void {
  recordPoint(r, owner, l.start);
  for (const seg of l.segs) {
    recordSegment(r, owner, seg);
  }
}

function recordSegment(r: Resolver, owner: string, seg: ast.Seg): void {
  const k = seg.kind;
  switch (k.kind) {
    case "To":
    case "Hv":
    case "Vh":
      recordPoint(r, owner, k.value);
      break;
    case "Vhv":
    case "Hvh":
    case "Curve":
      recordPoint(r, owner, k.to);
      break;
    case "H":
      literal(r, owner, "h", k.value, seg.span);
      break;
    case "V":
      literal(r, owner, "v", k.value, seg.span);
      break;
    case "X":
      recordTarget(r, owner, "x", k.value, seg.span);
      break;
    case "Y":
      recordTarget(r, owner, "y", k.value, seg.span);
      break;
    case "CurveVia":
      for (const p of [k.c1, k.c2, k.to]) {
        recordPoint(r, owner, p);
      }
      break;
  }
}

function recordTarget(r: Resolver, owner: string, what: string, t: Target, span: Span): void {
  if (t.kind === "Num") {
    literal(r, owner, what, t.value, span);
  } else {
    recordPoint(r, owner, t.value);
  }
}

/// The ids a point references: its base, or the refs in its `x`/`y` coordinates.
export function pointRefs(p: Point): ast.Ref[] {
  const b = pointBase(p);
  switch (b.kind) {
    case "Ref":
      return [b.r];
    case "Lit": {
      const out: ast.Ref[] = [];
      for (const c of [b.x, b.y]) {
        if (c.kind === "Axis") out.push(c.r);
      }
      return out;
    }
    case "Group":
    case "Shifted":
      return [];
  }
}

/// Every point a line names: its start and each segment's points.
export function linePoints(l: ast.Line): Point[] {
  const out: Point[] = [l.start];
  for (const seg of l.segs) {
    const k = seg.kind;
    switch (k.kind) {
      case "To":
      case "Hv":
      case "Vh":
        out.push(k.value);
        break;
      case "Vhv":
      case "Hvh":
      case "Curve":
        out.push(k.to);
        break;
      case "X":
      case "Y":
        if (k.value.kind === "Point") out.push(k.value.value);
        break;
      case "CurveVia":
        out.push(k.c1, k.c2, k.to);
        break;
      case "H":
      case "V":
        break;
    }
  }
  return out;
}

export function segmentKind(k: SegKind): SegmentKind {
  switch (k.kind) {
    case "To":
      return "to";
    case "H":
      return "h";
    case "V":
      return "v";
    case "X":
      return "x";
    case "Y":
      return "y";
    case "Hv":
      return "hv";
    case "Vh":
      return "vh";
    case "Vhv":
      return "vhv";
    case "Hvh":
      return "hvh";
    case "Curve":
      return "curve";
    case "CurveVia":
      return "curve-via";
  }
}

export function isLiteral(p: Point): boolean {
  return pointBase(p).kind !== "Ref";
}

export function endIsLiteral(segs: readonly ast.Seg[]): boolean {
  const last = segs[segs.length - 1];
  if (last === undefined) return true;
  const k = last.kind;
  switch (k.kind) {
    case "To":
    case "Hv":
    case "Vh":
      return isLiteral(k.value);
    case "Vhv":
    case "Hvh":
    case "Curve":
    case "CurveVia":
      return isLiteral(k.to);
    case "X":
    case "Y":
      return k.value.kind !== "Point";
    default:
      return true;
  }
}
