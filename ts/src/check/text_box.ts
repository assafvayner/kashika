import { type Pt, Rect } from "../geom/index.js";
import { type Label, type Shape, type Text, type TitlePos } from "../model/index.js";

export function textBox(t: Text): Rect {
  return t.rect;
}

export function labelBox(label: Label): Rect {
  return label.rect;
}

/// The box `label` would occupy if it were centered on `at` instead.
export function labelBoxAt(label: Label, at: Pt): Rect {
  const r = label.rect;
  return new Rect(r.x + at.x - label.at.x, r.y + at.y - label.at.y, r.w, r.h);
}

function unionOf(rects: Rect[]): Rect | null {
  let acc: Rect | null = null;
  for (const r of rects) acc = acc === null ? r : acc.union(r);
  return acc;
}

/// The label of a `text-area=below` shape, which sits under the shape instead of inside it.
export function belowBox(s: Shape): Rect | null {
  if (!s.textBelow) {
    return null;
  }
  return unionOf(s.text.filter((t) => t.role !== "absolute").map((t) => t.rect));
}

/// The reserved band at the top (or, with `title=bottom`, the bottom) of a group with a
/// title: padding plus the title's height.
export function titleBand(g: Shape): Rect | null {
  return g.kind === "group" && g.titleBand > 0.0 ? bandAt(g, g.title) : null;
}

/// Where `g`'s title band would sit with `title=pos`.
export function bandAt(g: Shape, pos: TitlePos): Rect {
  const y = pos === "top" ? g.rect.y : g.rect.bottom() - g.titleBand;
  return new Rect(g.rect.x, y, g.rect.w, g.titleBand);
}

/// Where a group's title is drawn: the union of its label, sublabel and stacked block items.
export function titleBox(g: Shape): Rect | null {
  if (g.kind !== "group") {
    return null;
  }
  return unionOf(g.text.filter((t) => t.role !== "absolute").map((t) => t.rect));
}
