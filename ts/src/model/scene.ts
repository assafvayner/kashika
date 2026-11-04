import { type Span } from "../diag/diagnostic.js";
import { Float, type Json } from "../diag/json.js";
import { type Result, err, fclamp, fmax, fmin, ok, sum } from "../diag/rust.js";
import {
  type PathSeg,
  Pt,
  Rect,
  TOLERANCE,
  convexHull,
  endDir,
  segLength,
  segToJson,
  segTrim,
  startDir,
} from "../geom/index.js";
import { type PlacedLine } from "../text/index.js";
import { type Form, anchorFrame, imageDistortion, outlinePolygon, resolveAnchor } from "./form.js";
import { type Style } from "./style.js";
import { type TextItem } from "./textlayout.js";

function opt<T>(v: T | null, f: (v: T) => Json): Json {
  return v === null ? null : f(v);
}

function float(v: number): Float {
  return new Float(v);
}

export interface SceneFields {
  title: string;
  size: [number, number];
  grid: number | null;
  clearance: number;
  /// The canvas color; `null` leaves the canvas transparent.
  background: string | null;
  allow: Allow[];
  shapes: Shape[];
  junctions: Junction[];
  texts: Text[];
  lines: Line[];
  /// Embedded files (images, icons, SVG shapes), in first-use order.
  assets: Asset[];
  /// Who is placed relative to whom: `at` references and group frames.
  placement: PlacementEdge[];
  /// Every number the author wrote as a position, distance or size.
  literals: Literal[];
  /// Whether each canvas axis was fitted to the drawing rather than given in `size`.
  fitted: [boolean, boolean];
  /// `diagram … ortho`: lines are `ortho` unless they say `ortho=false`.
  ortho: boolean;
}

export interface Scene extends SceneFields {}

export class Scene {
  constructor(f: SceneFields) {
    Object.assign(this, f);
  }

  /// Indices into `shapes` in paint order: top-level `layer=under` shapes, each group followed
  /// by its own `layer=under` children, the other boxes and images, then `layer=over` shapes.
  /// Junction dots, lines, line labels and texts paint before the `layer=over` shapes.
  paintOrder(): number[] {
    const groups: string[] = this.shapes.filter((g) => g.kind === "group").map((g) => g.id);
    const under: [number, string | null][] = [];
    this.shapes.forEach((s, i) => {
      if (s.layer === "under") {
        under.push([i, s.parent !== null && groups.includes(s.parent) ? s.parent : null]);
      }
    });
    const underIn = (parent: string | null) => under.filter(([, p]) => p === parent).map(([i]) => i);
    const order = underIn(null);
    this.shapes.forEach((g, i) => {
      if (g.kind === "group") {
        order.push(i);
        order.push(...underIn(g.id));
      }
    });
    for (const layer of ["normal", "over"] as const) {
      this.shapes.forEach((s, i) => {
        if (s.kind !== "group" && s.layer === layer) order.push(i);
      });
    }
    return order;
  }

  toJson(): Json {
    const o: { [key: string]: Json } = {
      title: this.title,
      size: [float(this.size[0]), float(this.size[1])],
      grid: opt(this.grid, float),
      clearance: float(this.clearance),
      background: this.background,
      allow: this.allow.map((a) => a.toJson()),
      shapes: this.shapes.map((s) => s.toJson()),
      junctions: this.junctions.map((j) => j.toJson()),
      texts: this.texts.map((t) => t.toJson()),
      lines: this.lines.map((l) => l.toJson()),
      assets: this.assets.map((a) => a.toJson()),
      placement: this.placement.map((p) => p.toJson()),
      literals: this.literals.map((l) => l.toJson()),
    };
    if (this.ortho) o["ortho"] = true;
    return o;
  }
}

export type ShapeKind = "box" | "group" | "image";

export const ShapeKind = {
  /// `box`, `group` or `image`.
  name(k: ShapeKind): string {
    return k;
  },
};

/// `layer=`: when a box or image paints, and whether checks treat it as an obstacle.
/// `under` paints before everything else in its parent: a top-level shape before every
/// group, a group's child right after that group's own fill and border; `over` after
/// everything else, texts and line labels included. Neither is an obstacle.
export type Layer = "under" | "normal" | "over";

export const Layer = {
  default(): Layer {
    return "normal";
  },

  isNormal(l: Layer): boolean {
    return l === "normal";
  },

  cmp(a: Layer, b: Layer): number {
    const rank = { under: 0, normal: 1, over: 2 };
    return rank[a] - rank[b];
  },
};

/// `path` is a built-in, custom or SVG shape; `Shape.form` holds the geometry.
export type Outline = "rect" | "rounded" | "path";

export interface ShapeFields {
  id: string;
  kind: ShapeKind;
  outline: Outline;
  /// `rect`, `rounded`, a built-in shape, a defined or imported shape (`aws.s3`), or `image`.
  shape: string;
  /// Outline and drawing for `path` shapes.
  form: Form | null;
  /// The drawing of an `image` or a `shape from` SVG.
  image: ImageRef | null;
  icon: ImageRef | null;
  label: string | null;
  rect: Rect;
  parent: string | null;
  depth: number;
  layer: Layer;
  style: Style;
  /// The bounding box inset by `padding` (and by the shape's own reduction).
  textArea: Rect;
  /// `text-area=below`: the text sits in a band under the box instead of inside it.
  textBelow: boolean;
  /// Label, sublabel and text block items, laid out.
  text: TextItem[];
  /// Height of a group's reserved title band from its top edge (its bottom edge with
  /// `title=bottom`); 0 for boxes and unlabeled groups.
  titleBand: number;
  title: TitlePos;
  allow: Allow[];
  reason: string | null;
  /// Canvas position of the frame this element's literals are written in.
  origin: Pt;
  span: Span;
}

export interface Shape extends ShapeFields {}

export class Shape {
  constructor(f: ShapeFields) {
    Object.assign(this, f);
  }

  cornerRadius(): number {
    return cornerRadius(this.style.radius, this.rect);
  }

  /// The bounding box in the frame the shape was written in.
  localRect(): Rect {
    return new Rect(
      this.rect.x - this.origin.x,
      this.rect.y - this.origin.y,
      this.rect.w,
      this.rect.h,
    );
  }

  /// Canvas position of an anchor, or `null` when the name or `t` is invalid.
  anchor(name: string, t: number | null): Pt | null {
    const r = resolveAnchor(this.form, this.rect, this.style.radius, this.id, name, t);
    return r.ok ? r.value : null;
  }

  /// The shape's outline as a closed polygon in canvas coordinates, for anchors and checks.
  /// Rounded corners and curves are flattened to `FLATTEN_TOLERANCE`.
  outlinePolygon(): Pt[] {
    return outlinePolygon(this);
  }

  /// The rect anchors resolve on: the drawing of a contained SVG-file shape, else the box.
  anchorFrame(): Rect {
    return anchorFrame(this);
  }

  /// How much `fit=stretch` changes the aspect ratio of an image or SVG-file shape:
  /// |box ratio / natural ratio - 1|. `null` for other fits, shapes without an image,
  /// and files that failed to load.
  imageDistortion(assets: readonly Asset[]): number | null {
    return imageDistortion(this, assets);
  }

  toJson(): Json {
    const o: { [key: string]: Json } = {
      id: this.id,
      kind: this.kind,
      outline: this.outline,
      shape: this.shape,
      form: opt(this.form, (f) => f.toJson()),
      image: opt(this.image, (i) => i.toJson()),
      icon: opt(this.icon, (i) => i.toJson()),
      label: this.label,
      rect: this.rect.toJson(),
      parent: this.parent,
      depth: this.depth,
    };
    if (!Layer.isNormal(this.layer)) o["layer"] = this.layer;
    o["style"] = this.style.toJson();
    o["text_area"] = this.textArea.toJson();
    o["text_below"] = this.textBelow;
    o["text"] = this.text.map((t) => t.toJson());
    o["title_band"] = float(this.titleBand);
    if (!TitlePos.isTop(this.title)) o["title"] = this.title;
    o["allow"] = this.allow.map((a) => a.toJson());
    o["reason"] = this.reason;
    o["origin"] = this.origin.toJson();
    o["span"] = this.span.toJson();
    return o;
  }
}

/// `at`: `from`'s `at` references `to`. `frame`: `from` is written inside group `to`.
export type EdgeKind = "at" | "frame";

export const EdgeKind = {
  name(k: EdgeKind): string {
    return k;
  },
};

export class PlacementEdge {
  constructor(
    public from: string,
    public to: string,
    public kind: EdgeKind,
  ) {}

  eq(o: PlacementEdge): boolean {
    return this.from === o.from && this.to === o.to && this.kind === o.kind;
  }

  toJson(): Json {
    return { from: this.from, to: this.to, kind: this.kind };
  }
}

/// A number the author wrote as a position, distance or size, in its own frame.
export class Literal {
  constructor(
    /// Id of the element the number belongs to (`line@L<n>`/`text@L<n>` when unnamed).
    public owner: string,
    /// `x`, `y`, `dx`, `dy`, `h`, `v`, `width` or `height`.
    public what: string,
    public value: number,
    /// The enclosing point, segment or statement.
    public span: Span,
  ) {}

  toJson(): Json {
    return { owner: this.owner, what: this.what, value: float(this.value), span: this.span.toJson() };
  }
}

export function cornerRadius(radius: number, rect: Rect): number {
  return fmax(fmin(fmin(radius, rect.w / 2), rect.h / 2), 0);
}

export class Junction {
  constructor(
    public id: string,
    public at: Pt,
    public dot: boolean,
    public parent: string | null,
    public origin: Pt,
    public span: Span,
  ) {}

  toJson(): Json {
    return {
      id: this.id,
      at: this.at.toJson(),
      dot: this.dot,
      parent: this.parent,
      origin: this.origin.toJson(),
      span: this.span.toJson(),
    };
  }
}

export type Align = "left" | "center" | "right";

export const Align = {
  NAMES: ["left", "center", "right"] as readonly string[],

  fromName(name: string): Align | null {
    return (["left", "center", "right"] as const).find((x) => Align.name(x) === name) ?? null;
  },

  name(a: Align): string {
    return a;
  },
};

/// `title=`: the edge of a group its title and reserved title band sit along.
export type TitlePos = "top" | "bottom";

export const TitlePos = {
  NAMES: ["top", "bottom"] as readonly string[],

  default(): TitlePos {
    return "top";
  },

  fromName(name: string): TitlePos | null {
    return (["top", "bottom"] as const).find((x) => TitlePos.name(x) === name) ?? null;
  },

  name(t: TitlePos): string {
    return t;
  },

  isTop(t: TitlePos): boolean {
    return t === "top";
  },
};

export interface TextFields {
  id: string | null;
  content: string;
  at: Pt;
  size: number;
  color: string;
  align: Align;
  rect: Rect;
  lines: PlacedLine[];
  parent: string | null;
  origin: Pt;
  span: Span;
}

export interface Text extends TextFields {}

export class Text {
  constructor(f: TextFields) {
    Object.assign(this, f);
  }

  /// The declared id, or `text@L<n>` for unnamed texts.
  displayId(): string {
    return this.id ?? unnamedTextId(this.span.line);
  }

  toJson(): Json {
    return {
      id: this.id,
      content: this.content,
      at: this.at.toJson(),
      size: float(this.size),
      color: this.color,
      align: this.align,
      rect: this.rect.toJson(),
      lines: this.lines.map((l) => l.toJson()),
      parent: this.parent,
      origin: this.origin.toJson(),
      span: this.span.toJson(),
    };
  }
}

/// The id an unnamed text on source line `line` goes by.
export function unnamedTextId(line: number): string {
  return `text@L${line}`;
}

export type Marker =
  | "none"
  | "arrow"
  | "open"
  | "triangle"
  | "diamond"
  | "hollow-diamond"
  | "dot"
  | "circle"
  | "one"
  | "many"
  | "one-many"
  | "zero-one"
  | "zero-many";

/// The strokes of a crow's-foot end: bars and prongs as (from, to), and the open circle of
/// the `zero-*` ends as (center, radius).
export class ErParts {
  constructor(
    public tip: Pt,
    public strokes: [Pt, Pt][],
    public circle: [Pt, number] | null,
  ) {}

  static default(): ErParts {
    return new ErParts(Pt.default(), [], null);
  }

  /// The convex hull of the tip, the bars, the prongs and the circle's bounding square, or
  /// nothing when there are no parts.
  hull(): Pt[] {
    if (this.strokes.length === 0) {
      return [];
    }
    const pts: Pt[] = this.strokes.flatMap(([a, b]) => [a, b]);
    pts.push(this.tip);
    if (this.circle !== null) {
      const [c, r] = this.circle;
      pts.push(
        c.add(new Pt(r, r)),
        c.add(new Pt(r, -r)),
        c.add(new Pt(-r, r)),
        c.add(new Pt(-r, -r)),
      );
    }
    return convexHull(pts);
  }
}

const DOT_RADIUS = 3.0;
const CIRCLE_RADIUS = 4.0;
const ARROW_LEN = 10.0;
const DIAMOND_LEN = 12.0;
const HALF_WIDTH = 4.0;
const TRIANGLE_LEN = 12.0;
const TRIANGLE_HALF_WIDTH = 6.0;
const FOOT_LEN = 12.0;
const ER_HALF_WIDTH = 6.0;
const BAR_AT = 8.0;
const ER_GAP = 4.0;
const BASE_STROKE_WIDTH = 1.5;

const MARKER_ALL: readonly Marker[] = [
  "none",
  "arrow",
  "open",
  "triangle",
  "diamond",
  "hollow-diamond",
  "dot",
  "circle",
  "one",
  "many",
  "one-many",
  "zero-one",
  "zero-many",
];

export const Marker = {
  ALL: MARKER_ALL,

  /// The `start=`/`end=` names, in `ALL` order.
  NAMES: MARKER_ALL.map((m) => m) as readonly string[],

  fromName(name: string): Marker | null {
    return MARKER_ALL.find((m) => Marker.name(m) === name) ?? null;
  },

  name(m: Marker): string {
    return m;
  },

  /// The factor every marker dimension is multiplied by on a line of `strokeWidth`: 1 up
  /// to the default width, then in proportion to it.
  scale(strokeWidth: number): number {
    return fmax(strokeWidth / BASE_STROKE_WIDTH, 1.0);
  },

  /// Whether the marker is an unfilled closed outline, with the line masked inside it.
  isHollow(m: Marker): boolean {
    return m === "triangle" || m === "hollow-diamond" || m === "circle";
  },

  /// How far the line stroke stops short of the tip on a line of `strokeWidth`.
  /// Crow's-foot ends sit on the line and don't trim it.
  length(m: Marker, strokeWidth: number): number {
    let v: number;
    switch (m) {
      case "arrow":
        v = ARROW_LEN;
        break;
      case "triangle":
        v = TRIANGLE_LEN;
        break;
      case "diamond":
      case "hollow-diamond":
        v = DIAMOND_LEN;
        break;
      case "circle":
        v = 2.0 * CIRCLE_RADIUS;
        break;
      default:
        v = 0.0;
    }
    return Marker.scale(strokeWidth) * v;
  },

  /// How far the marker reaches to either side of a line of `strokeWidth`.
  halfWidth(m: Marker, strokeWidth: number): number {
    let v: number;
    switch (m) {
      case "none":
        v = 0.0;
        break;
      case "dot":
        v = DOT_RADIUS;
        break;
      case "circle":
        v = CIRCLE_RADIUS;
        break;
      case "arrow":
      case "open":
      case "diamond":
      case "hollow-diamond":
        v = HALF_WIDTH;
        break;
      case "triangle":
        v = TRIANGLE_HALF_WIDTH;
        break;
      case "one":
      case "many":
      case "one-many":
      case "zero-one":
      case "zero-many":
        v = ER_HALF_WIDTH;
        break;
    }
    return Marker.scale(strokeWidth) * v;
  },

  /// The bars, prongs and circle of a crow's-foot end on a line of `strokeWidth`.
  /// `back(d)` is the point `d` back along the line from the tip and the line's direction
  /// there, toward the tip; the tip is `back(0)`. Bars cross the line where they sit, so
  /// they stay on a curve. Empty for the other markers.
  erParts(m: Marker, strokeWidth: number, back: (d: number) => [Pt, Pt]): ErParts {
    const k = Marker.scale(strokeWidth);
    const across = (d: number): [Pt, Pt] => {
      const [p, dir] = back(d);
      const u = dir.norm();
      return [p, new Pt(-u.y, u.x).mul(ER_HALF_WIDTH * k)];
    };
    const bar = (d: number): [Pt, Pt] => {
      const [p, perp] = across(d * k);
      return [p.add(perp), p.sub(perp)];
    };
    const foot = (): [Pt, Pt][] => {
      const [tip, perp] = across(0.0);
      const apex = back(FOOT_LEN * k)[0];
      return [
        [apex, tip.add(perp)],
        [apex, tip],
        [apex, tip.sub(perp)],
      ];
    };
    const circle = (d: number): [Pt, number] | null => [
      back((d + ER_GAP + CIRCLE_RADIUS) * k)[0],
      CIRCLE_RADIUS * k,
    ];
    let strokes: [Pt, Pt][];
    let circ: [Pt, number] | null;
    switch (m) {
      case "one":
        [strokes, circ] = [[bar(BAR_AT)], null];
        break;
      case "many":
        [strokes, circ] = [foot(), null];
        break;
      case "one-many": {
        const s = foot();
        s.push(bar(FOOT_LEN + ER_GAP));
        [strokes, circ] = [s, null];
        break;
      }
      case "zero-one":
        [strokes, circ] = [[bar(BAR_AT)], circle(BAR_AT)];
        break;
      case "zero-many":
        [strokes, circ] = [foot(), circle(FOOT_LEN)];
        break;
      default:
        [strokes, circ] = [[], null];
    }
    return new ErParts(back(0.0)[0], strokes, circ);
  },

  /// The circle of a `dot` (centered on the tip) or a `circle` end (touching the tip) on a
  /// line of `strokeWidth`, as (center, radius). `null` for the other markers.
  round(m: Marker, tip: Pt, dir: Pt, strokeWidth: number): [Pt, number] | null {
    const k = Marker.scale(strokeWidth);
    switch (m) {
      case "dot":
        return [tip, DOT_RADIUS * k];
      case "circle": {
        const r = CIRCLE_RADIUS * k;
        return [tip.sub(dir.norm().mul(r)), r];
      }
      default:
        return null;
    }
  },

  /// Vertices of the head with its tip on `tip` pointing along `dir`, on a line of
  /// `strokeWidth`: the tip first, then the rest in drawing order. For crow's-foot ends on
  /// a straight end segment, the convex hull of their parts; for `dot` and `circle`, the
  /// circle's bounding square. Empty for `none`.
  outline(m: Marker, tip: Pt, dir: Pt, strokeWidth: number): Pt[] {
    const k = Marker.scale(strokeWidth);
    const u = dir.norm();
    const perp = new Pt(-u.y, u.x).mul(Marker.halfWidth(m, strokeWidth));
    switch (m) {
      case "arrow":
      case "open":
      case "triangle": {
        const len = m === "triangle" ? TRIANGLE_LEN : ARROW_LEN;
        const base = tip.sub(u.mul(len * k));
        return [tip, base.add(perp), base.sub(perp)];
      }
      case "diamond":
      case "hollow-diamond": {
        const len = DIAMOND_LEN * k;
        const mid = tip.sub(u.mul(len / 2.0));
        return [tip, mid.add(perp), tip.sub(u.mul(len)), mid.sub(perp)];
      }
      case "dot":
      case "circle": {
        const rd = Marker.round(m, tip, dir, strokeWidth);
        if (rd === null) {
          return [];
        }
        const [c, r] = rd;
        return [
          c.add(new Pt(-r, -r)),
          c.add(new Pt(r, -r)),
          c.add(new Pt(r, r)),
          c.add(new Pt(-r, r)),
        ];
      }
      case "none":
        return [];
      case "one":
      case "many":
      case "one-many":
      case "zero-one":
      case "zero-many":
        return Marker.erParts(m, strokeWidth, (d) => [tip.sub(u.mul(d)), u]).hull();
    }
  },
};

/// Where a line's labels sit: on the line, masking it, or beside it (`label-side=`).
export type LabelSide = "on" | "above" | "below" | "left" | "right";

/// The direction of the path piece under a label's point.
export type Orientation = "Horizontal" | "Vertical" | "Diagonal" | "Curve";

export const Orientation = {
  name(o: Orientation): string {
    switch (o) {
      case "Horizontal":
        return "horizontal";
      case "Vertical":
        return "vertical";
      case "Diagonal":
        return "diagonal";
      case "Curve":
        return "curved";
    }
  },

  of(seg: PathSeg): Orientation {
    if (seg.kind === "line") {
      if (Math.abs(seg.from.y - seg.to.y) <= TOLERANCE) return "Horizontal";
      if (Math.abs(seg.from.x - seg.to.x) <= TOLERANCE) return "Vertical";
      return "Diagonal";
    }
    return "Curve";
  },
};

/// The orientations of the path pieces at fraction `t` of the arc length: one, or two
/// at a vertex.
export function orientationsAt(path: readonly PathSeg[], t: number): Orientation[] {
  const lens: number[] = path.map(segLength);
  const target = sum(lens) * fclamp(t, 0.0, 1.0);
  let start = 0.0;
  const out: Orientation[] = [];
  path.forEach((seg, i) => {
    const len = lens[i]!;
    if (target >= start - TOLERANCE && target <= start + len + TOLERANCE) {
      out.push(Orientation.of(seg));
    }
    start += len;
  });
  return out;
}

const LABEL_SIDES: readonly LabelSide[] = ["on", "above", "below", "left", "right"];

export const LabelSide = {
  NAMES: ["on", "above", "below", "left", "right"] as readonly string[],

  default(): LabelSide {
    return "on";
  },

  fromName(name: string): LabelSide | null {
    return LABEL_SIDES.find((s) => LabelSide.name(s) === name) ?? null;
  },

  name(s: LabelSide): string {
    return s;
  },

  isOn(s: LabelSide): boolean {
    return s === "on";
  },

  /// The segment orientation this side needs; `null` for `on`, which fits anywhere.
  needs(s: LabelSide): Orientation | null {
    switch (s) {
      case "on":
        return null;
      case "above":
      case "below":
        return "Horizontal";
      case "left":
      case "right":
        return "Vertical";
    }
  },

  /// Checks that a label at fraction `t` of `path` can sit on this side. On a mismatch,
  /// returns the orientation of the piece under the point (an axis-aligned one at a vertex).
  fits(s: LabelSide, path: readonly PathSeg[], t: number): Result<null, Orientation> {
    const need = LabelSide.needs(s);
    if (need === null) {
      return ok(null);
    }
    const found = orientationsAt(path, t);
    if (found.includes(need)) {
      return ok(null);
    }
    return err(
      found.find((o) => o === "Horizontal" || o === "Vertical") ?? found[0] ?? "Curve",
    );
  },

  /// From the point on the line to the center of a `w`×`h` label on this side: its near
  /// edge sits `strokeWidth / 2 + 4` from the line.
  offset(s: LabelSide, w: number, h: number, strokeWidth: number): Pt {
    const gap = strokeWidth / 2.0 + LABEL_GAP;
    switch (s) {
      case "on":
        return new Pt(0.0, 0.0);
      case "above":
        return new Pt(0.0, -(gap + h / 2.0));
      case "below":
        return new Pt(0.0, gap + h / 2.0);
      case "left":
        return new Pt(-(gap + w / 2.0), 0.0);
      case "right":
        return new Pt(gap + w / 2.0, 0.0);
    }
  },
};

/// The space between a line's stroke and a label placed beside it.
export const LABEL_GAP = 4.0;

export interface LabelFields {
  t: number;
  text: string;
  /// The point on the path at `t`; the label's center when it sits on the line.
  at: Pt;
  rect: Rect;
  lines: PlacedLine[];
  /// Where this label sits: the line's `label-side`, or `on` when that side does not
  /// fit the segment under `at`.
  side: LabelSide;
  span: Span;
}

export interface Label extends LabelFields {}

export class Label {
  constructor(f: LabelFields) {
    Object.assign(this, f);
  }

  toJson(): Json {
    const o: { [key: string]: Json } = {
      t: float(this.t),
      text: this.text,
      at: this.at.toJson(),
      rect: this.rect.toJson(),
      lines: this.lines.map((l) => l.toJson()),
    };
    if (!LabelSide.isOn(this.side)) o["side"] = this.side;
    o["span"] = this.span.toJson();
    return o;
  }
}

export class Allow {
  constructor(
    public code: string,
    public other: string | null,
    /// The entry in the source, or the whole `allow=` when it can't be told apart.
    public span: Span,
  ) {}

  eq(o: Allow): boolean {
    return this.code === o.code && this.other === o.other && this.span.eq(o.span);
  }

  toJson(): Json {
    return { code: this.code, other: this.other };
  }
}

export interface LineFields {
  /// The declared id, or `line@L<n>` for unnamed lines.
  id: string;
  named: boolean;
  path: PathSeg[];
  /// The authored segments, in order.
  segments: Segment[];
  /// For each piece of `path`, the index into `segments` that drew it.
  pathSeg: number[];
  /// Whether the start point was written as a literal or mixed point rather than an anchor or junction.
  startLiteral: boolean;
  /// Whether the last segment ended on a literal: `h`/`v`, a numeric `x`/`y`, or a literal target point.
  endLiteral: boolean;
  start: Marker;
  end: Marker;
  style: Style;
  labels: Label[];
  labelSide: LabelSide;
  ortho: boolean;
  allow: Allow[];
  reason: string | null;
  parent: string | null;
  origin: Pt;
  span: Span;
}

export interface Line extends LineFields {}

export class Line {
  constructor(f: LineFields) {
    Object.assign(this, f);
  }

  /// The crow's-foot parts of the start (`start`) or end marker, laid along the end
  /// segment. Past that segment they continue straight along its far end.
  erParts(start: boolean): ErParts {
    const [m, seg] = start
      ? [this.start, this.path[0]]
      : [this.end, this.path[this.path.length - 1]];
    if (seg === undefined) {
      return ErParts.default();
    }
    const len = segLength(seg);
    return Marker.erParts(m, this.style.strokeWidth, (d) => {
      if (start) {
        if (d < len) {
          const s = segTrim(seg, d, 0.0);
          return [s.from, startDir(s).mul(-1.0)];
        } else {
          const u = endDir(seg);
          return [seg.to.add(u.mul(d - len)), u.mul(-1.0)];
        }
      } else if (d < len) {
        const s = segTrim(seg, 0.0, d);
        return [s.to, endDir(s)];
      } else {
        const u = startDir(seg);
        return [seg.from.sub(u.mul(d - len)), u];
      }
    });
  }

  toJson(): Json {
    const o: { [key: string]: Json } = {
      id: this.id,
      named: this.named,
      path: this.path.map(segToJson),
      segments: this.segments.map((s) => s.toJson()),
      path_seg: [...this.pathSeg],
      start_literal: this.startLiteral,
      end_literal: this.endLiteral,
      start: this.start,
      end: this.end,
      style: this.style.toJson(),
      labels: this.labels.map((l) => l.toJson()),
    };
    if (!LabelSide.isOn(this.labelSide)) o["label_side"] = this.labelSide;
    o["ortho"] = this.ortho;
    o["allow"] = this.allow.map((a) => a.toJson());
    o["reason"] = this.reason;
    o["parent"] = this.parent;
    o["origin"] = this.origin.toJson();
    o["span"] = this.span.toJson();
    return o;
  }
}

export type Fit = "contain" | "cover" | "stretch";

export const Fit = {
  NAMES: ["contain", "cover", "stretch"] as readonly string[],

  fromName(name: string): Fit | null {
    return (["contain", "cover", "stretch"] as const).find((x) => Fit.name(x) === name) ?? null;
  },

  name(f: Fit): string {
    return f;
  },

  /// Where a drawing of `natural` size lands when fitted to `r`, centered: inside `r` for
  /// `contain`, covering it (and cropped to it when drawn) for `cover`, `r` for `stretch`.
  place(f: Fit, natural: readonly [number, number], r: Rect): Rect {
    const [nw, nh] = natural;
    if (f === "stretch" || nw <= 0.0 || nh <= 0.0) {
      return r;
    }
    const [sx, sy] = [r.w / nw, r.h / nh];
    const s = f === "contain" ? fmin(sx, sy) : fmax(sx, sy);
    const [w, h] = [nw * s, nh * s];
    return new Rect(r.x + (r.w - w) / 2.0, r.y + (r.h - h) / 2.0, w, h);
  },
};

/// A placed asset: which file, where, and how it fills its rect.
export class ImageRef {
  constructor(
    /// Index into `Scene.assets`.
    public asset: number,
    public rect: Rect,
    public fit: Fit,
  ) {}

  eq(o: ImageRef): boolean {
    return this.asset === o.asset && this.rect.eq(o.rect) && this.fit === o.fit;
  }

  toJson(): Json {
    return { asset: this.asset, rect: this.rect.toJson(), fit: this.fit };
  }
}

export interface AssetFields {
  /// The path as written in the source, or `builtin:<name>` for a bundled icon.
  path: string;
  mime: string;
  /// Natural size in px (SVG: its width/height; raster: pixels).
  natural: [number, number];
  /// The bytes embedded in output: sanitized SVG, or the raster file unchanged.
  bytes: Uint8Array;
  /// Convex hull of the drawing in unit coordinates (the unit square for rasters).
  unitHull: Pt[];
  /// The color an SVG icon's `currentColor` resolved to: the box's text color.
  currentColor: string | null;
  /// Elements of an SVG file that embedding lost, as (element name, count).
  dropped: [string, number][];
}

export interface Asset extends AssetFields {}

export class Asset {
  constructor(f: AssetFields) {
    Object.assign(this, f);
  }

  toJson(): Json {
    return {
      path: this.path,
      mime: this.mime,
      natural: [float(this.natural[0]), float(this.natural[1])],
    };
  }
}

export type SegmentKind =
  | "to"
  | "h"
  | "v"
  | "x"
  | "y"
  | "hv"
  | "vh"
  | "vhv"
  | "hvh"
  | "curve"
  | "curve-via";

export const SegmentKind = {
  name(k: SegmentKind): string {
    return k;
  },
};

/// One authored `-> ...` segment of a line.
export class Segment {
  constructor(
    public kind: SegmentKind,
    public span: Span,
    public from: Pt,
    public to: Pt,
    /// The geometry this segment drew before corner rounding, without zero-length pieces.
    public pieces: PathSeg[],
  ) {}

  toJson(): Json {
    return {
      kind: this.kind,
      span: this.span.toJson(),
      from: this.from.toJson(),
      to: this.to.toJson(),
      pieces: this.pieces.map(segToJson),
    };
  }
}
