import { Pt, Rect } from "../../geom/index.js";
import type * as ast from "../../syntax/ast.js";
import { LibShape } from "../library.js";
import { Entry, Item, type Resolver } from "../resolve.js";
import {
  ImageRef,
  Junction,
  type Line,
  type Scene,
  Shape,
  type ShapeKind,
  Text,
  unnamedTextId,
} from "../scene.js";
import * as textlayout from "../textlayout.js";
import { buildLine } from "./lines.js";
import { evalPoint, frameOrigin, place } from "./placement.js";
import { contained, drawnShape, formOf, nodeIconInset, svgShape } from "./props.js";
import { literal, recordEdges, recordLine, recordNode, recordPoint } from "./record.js";

/// An entry's scene item, built before `build` adds it to the scene.
export type Built =
  | { kind: "Shape"; value: Shape }
  | { kind: "Junction"; value: Junction }
  | { kind: "Text"; value: Text }
  | { kind: "Line"; value: Line };

export function build(r: Resolver, scene: Scene): void {
  for (let idx = 0; idx < r.entries.length; idx++) {
    const entry = r.entries[idx]!;
    if (r.skipped.has(idx) || (entry.parent !== null && r.skipped.has(entry.parent))) {
      r.skipped.add(idx);
      continue;
    }
    const item = entry.item;
    ensureBuilt(r, idx);
    const built = r.built.get(idx) ?? null;
    if (built !== null) r.built.set(idx, null);
    switch (item.kind) {
      case "Box":
      case "Group": {
        const n = item.kind === "Box" ? item.value : item.value.node;
        recordEdges(r, idx, n.at);
        if (built !== null && built.kind === "Shape") {
          scene.shapes.push(built.value);
          recordNode(r, n);
        }
        break;
      }
      case "Image": {
        const i = item.value;
        recordEdges(r, idx, i.at);
        if (built !== null && built.kind === "Shape") {
          scene.shapes.push(built.value);
          recordPoint(r, i.id.name, i.at);
          literal(r, i.id.name, "width", i.size[0], i.span);
          literal(r, i.id.name, "height", i.size[1], i.span);
        }
        break;
      }
      case "Junction": {
        const j = item.value;
        recordEdges(r, idx, j.at);
        if (built !== null && built.kind === "Junction") {
          scene.junctions.push(built.value);
          recordPoint(r, j.id.name, j.at);
        }
        break;
      }
      case "Text": {
        const t = item.value;
        if (built !== null && built.kind === "Text") {
          scene.texts.push(built.value);
          const owner = t.id === null ? unnamedTextId(t.span.line) : t.id.name;
          recordPoint(r, owner, t.at);
        }
        break;
      }
      case "Line": {
        if (built !== null && built.kind === "Line") {
          const id = built.value.id;
          scene.lines.push(built.value);
          recordLine(r, id, item.value);
        }
        break;
      }
    }
  }
  scene.placement = r.edges;
  r.edges = [];
  scene.literals = r.literals;
  r.literals = [];
}

/// Builds an entry's scene item once; a fitted group builds its children early.
export function ensureBuilt(r: Resolver, idx: number): void {
  if (r.built.has(idx)) {
    return;
  }
  const built = make(r, idx);
  r.built.set(idx, built);
}

function make(r: Resolver, idx: number): Built | null {
  if (r.skipped.has(idx)) {
    return null;
  }
  const entry = r.entries[idx]!;
  const [item, depth] = [entry.item, entry.depth];
  const pid = entry.parent === null ? null : Item.id(r.entries[entry.parent]!.item);
  const parent = pid === null ? null : pid.name;
  switch (item.kind) {
    case "Box": {
      const s = makeShape(r, idx, item.value, "box", parent, depth);
      return s === null ? null : { kind: "Shape", value: s };
    }
    case "Group": {
      const s = makeShape(r, idx, item.value.node, "group", parent, depth);
      return s === null ? null : { kind: "Shape", value: s };
    }
    case "Image": {
      const s = makeImage(r, idx, item.value);
      return s === null ? null : { kind: "Shape", value: s };
    }
    case "Junction": {
      const j = item.value;
      const rect = place(r, idx);
      if (rect === null) return null;
      const dot = Entry.dot(r.entries[idx]!);
      const origin = frameOrigin(r, idx) ?? Pt.default();
      return {
        kind: "Junction",
        value: new Junction(j.id.name, rect.origin(), dot, parent, origin, j.span),
      };
    }
    case "Text": {
      const t = item.value;
      const origin = frameOrigin(r, idx);
      if (origin === null) return null;
      const at = evalPoint(r, t.at, origin);
      if (at === null) return null;
      const p = Entry.text(r.entries[idx]!);
      if (p === null) return null;
      const block = textlayout.measureAt(t.content, p.size, false, t.span, r.diags);
      if (block === null) return null;
      const [lines, rect] = block.placeAt(at.x, at.y, textlayout.halign(p.align));
      return {
        kind: "Text",
        value: new Text({
          id: t.id === null ? null : t.id.name,
          content: p.content,
          at,
          size: p.size,
          color: p.color,
          align: p.align,
          rect,
          lines,
          parent,
          origin,
          span: t.span,
        }),
      };
    }
    case "Line": {
      const l = buildLine(r, idx, item.value, parent);
      return l === null ? null : { kind: "Line", value: l };
    }
  }
}

function makeShape(
  r: Resolver,
  idx: number,
  n: ast.Node,
  kind: ShapeKind,
  parent: string | null,
  depth: number,
): Shape | null {
  const rect = place(r, idx);
  if (rect === null) return null;
  const origin = frameOrigin(r, idx) ?? Pt.default();
  const blockAt: (Pt | null)[] = (n.block ?? []).map((item) =>
    item.at === null ? null : evalPoint(r, item.at, rect.origin()),
  );
  const p = Entry.node(r.entries[idx]!);
  if (p === null) return null;
  const shape = drawnShape(p);
  const form = formOf(r, shape, rect);
  const svg = svgShape(r, shape);
  const icon = p.icon;
  const iconInset = nodeIconInset(p, kind === "group");
  const lib = r.lib.shapes.get(shape);
  const text = textlayout.layoutShape(
    {
      id: n.id.name,
      node: n,
      kind,
      shape,
      customTextArea: lib === undefined ? null : LibShape.textArea(lib),
      textBelow: svg !== null && svg.below,
      contain: contained(r, shape),
      iconInset,
      rect,
      style: p.style,
      props: p.text,
      blockAt,
    },
    r.diags,
  );
  let iconRef: ImageRef | null = null;
  if (icon !== null) {
    const pad = p.style.padding;
    const s = icon.size;
    const lh = p.style.fontSize * 1.25;
    let ir: Rect;
    if (kind === "group") {
      let top: number;
      if (p.text.title === "top") {
        top = rect.y + pad.y;
      } else if (text.titleBand > 0.0) {
        top = rect.bottom() - text.titleBand;
      } else {
        top = rect.bottom() - pad.y - lh;
      }
      ir = new Rect(rect.x + pad.x, top + (lh - s) / 2.0, s, s);
    } else if (icon.pos === "Left") {
      ir = new Rect(rect.x + pad.x, rect.center().y - s / 2.0, s, s);
    } else {
      ir = new Rect(rect.center().x - s / 2.0, rect.y + pad.y, s, s);
    }
    iconRef = new ImageRef(icon.asset, ir, "contain");
  }
  return new Shape({
    id: n.id.name,
    kind,
    outline: p.outline,
    form,
    shape,
    image: svg === null ? null : new ImageRef(svg.asset, rect, svg.fit),
    icon: iconRef,
    label: n.label,
    rect,
    parent,
    depth,
    layer: p.layer,
    style: p.style.clone(),
    textArea: text.area,
    textBelow: svg !== null && svg.below,
    text: text.items,
    titleBand: text.titleBand,
    title: p.text.title,
    allow: [...p.allow],
    reason: p.reason,
    origin,
    span: n.span,
  });
}

function makeImage(r: Resolver, idx: number, i: ast.Image): Shape | null {
  const rect = place(r, idx);
  if (rect === null) return null;
  const origin = frameOrigin(r, idx) ?? Pt.default();
  const p = Entry.node(r.entries[idx]!);
  if (p === null) return null;
  const props = p.image;
  if (props === null) return null;
  return new Shape({
    id: i.id.name,
    kind: "image",
    outline: "rect",
    form: null,
    shape: "image",
    image: props.asset === null ? null : new ImageRef(props.asset, rect, props.fit),
    icon: null,
    label: null,
    rect,
    parent: null,
    depth: 0,
    layer: p.layer,
    style: p.style.clone(),
    textArea: rect,
    textBelow: false,
    text: [],
    titleBand: 0.0,
    title: "top",
    allow: [...p.allow],
    reason: p.reason,
    origin,
    span: i.span,
  });
}

export function name(r: Resolver, idx: number): string {
  const item = r.entries[idx]!.item;
  const id = Item.id(item);
  if (id !== null) return id.name;
  if (item.kind === "Text") return unnamedTextId(item.value.span.line);
  return `line@L${Item.span(item).line}`;
}
