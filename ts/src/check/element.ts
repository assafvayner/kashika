import { Element, type Pair } from "../diag/diagnostic.js";
import { Float, type Json } from "../diag/json.js";
import { Pt, Rect, round2 } from "../geom/index.js";
import { type Junction, type Line, type Shape, ShapeKind, type Text } from "../model/index.js";
import { textBox } from "./text_box.js";

export function arr(p: Pt): Pair {
  return [round2(p.x), round2(p.y)];
}

export function bbox(r: Rect): [Pair, Pair] {
  return [arr(r.origin()), arr(new Pt(r.right(), r.bottom()))];
}

/// `arr` as a fact value, its numbers serialized as floats.
export function arrFact(p: Pt): Json {
  return pairFact(arr(p));
}

/// `bbox` as a fact value, its numbers serialized as floats.
export function bboxFact(r: Rect): Json {
  return bbox(r).map(pairFact);
}

export function pairFact(p: Pair): Json {
  return [new Float(p[0]), new Float(p[1])];
}

export function shapeEl(s: Shape): Element {
  const e = Element.new(s.id, ShapeKind.name(s.kind), s.span);
  e.bbox = bbox(s.rect);
  if (s.parent !== null) {
    e.frame = s.parent;
    e.localBbox = bbox(s.localRect());
  }
  return e;
}

/// A line element; `seg` is an index into `l.segments`.
export function lineEl(l: Line, seg: number | null): Element {
  const e = Element.new(l.id, "line", l.span);
  const s = seg === null ? undefined : l.segments[seg];
  if (seg !== null && s !== undefined) {
    e.segment = seg + 1;
    e.from = arr(s.from);
    e.to = arr(s.to);
  }
  return e;
}

export function junctionEl(j: Junction): Element {
  const e = Element.new(j.id, "junction", j.span);
  e.bbox = bbox(new Rect(j.at.x, j.at.y, 0.0, 0.0));
  return e;
}

export function textEl(t: Text): Element {
  const e = Element.new(t.displayId(), "text", t.span);
  e.bbox = bbox(textBox(t));
  return e;
}
