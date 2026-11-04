import { type Element } from "../diag/diagnostic.js";
import { Float, type Json } from "../diag/json.js";
import { displayF64, fclamp, totalCmp } from "../diag/rust.js";
import {
  type PathSeg,
  Pt,
  Rect,
  TOLERANCE,
  boundaryNearest,
  fmtNum,
  pieces,
  pointInPolygon,
  round2,
  segFrom,
  segTo,
} from "../geom/index.js";
import { type Junction, type Line, type Scene, type Shape } from "../model/index.js";
import { junctionEl, lineEl, shapeEl, textEl } from "./element.js";
import { polygon } from "./outline.js";
import { Grid } from "./spatial.js";
import { belowBox, labelBox, textBox } from "./text_box.js";

export interface ShapeGeom {
  shape: Shape;
  poly: Pt[];
}

export class LineGeom {
  constructor(
    public line: Line,
    /// Straight pieces of the rendered path, curves flattened: (from, to, index into `line.path`).
    public pieces: [Pt, Pt, number][],
    public start: Pt,
    public end: Pt,
  ) {}

  /// Index into `line.segments` of the authored segment that drew `line.path[pathIndex]`.
  segment(pathIndex: number): number {
    return this.line.pathSeg[pathIndex] ?? 0;
  }

  lastSegment(): number {
    return Math.max(this.line.segments.length - 1, 0);
  }

  straight(pathIndex: number): boolean {
    return this.line.path[pathIndex]?.kind === "line";
  }

  straightEnds(pathIndex: number): [Pt, Pt] | null {
    const s: PathSeg | undefined = this.line.path[pathIndex];
    return s !== undefined && s.kind === "line" ? [s.from, s.to] : null;
  }
}

/// Something drawn on top that a free lane must keep clear of besides boxes and lines: a
/// junction, a text, a line label or the label under a `text-area=below` shape.
export interface Mark {
  rect: Rect;
  /// The id of the junction, text, line or shape it belongs to.
  owner: string;
}

function firstIds<T>(items: readonly T[], id: (t: T) => string): Map<string, number> {
  const m = new Map<string, number>();
  items.forEach((t, i) => {
    const k = id(t);
    if (!m.has(k)) m.set(k, i);
  });
  return m;
}

/// The scene plus geometry shared by every lint.
export class Ctx {
  private constructor(
    public scene: Scene,
    public shapes: ShapeGeom[],
    public lines: LineGeom[],
    /// Bounding boxes of each shape (outline and rect) and line (pieces and ends), and grids over them.
    public shapeBoxes: Rect[],
    public lineBoxes: Rect[],
    private shapeGrid: Grid,
    private lineGrid: Grid,
    /// Index into `scene.shapes` of the first shape with each id.
    private shapeIds: Map<string, number>,
    /// Index into `lines` of the first line with each id.
    private lineIds: Map<string, number>,
    /// Index into `scene.lines`, `scene.junctions` and `scene.texts` of the first of each id.
    private sceneLineIds: Map<string, number>,
    private junctionIds: Map<string, number>,
    private textIds: Map<string, number>,
    private junctionGrid: Grid,
    private marks: Mark[],
    private markGrid: Grid,
  ) {}

  static new(scene: Scene): Ctx {
    const shapes: ShapeGeom[] = scene.shapes.map((s) => ({ shape: s, poly: polygon(s) }));
    const lines: LineGeom[] = scene.lines
      .filter((l) => l.path.length > 0)
      .map(
        (l) =>
          new LineGeom(l, pieces(l.path), segFrom(l.path[0]!), segTo(l.path[l.path.length - 1]!)),
      );
    const shapeBoxes: Rect[] = shapes.map((s) => {
      const r = s.shape.rect;
      const corners = [r.origin(), new Pt(r.right(), r.bottom())];
      return extent([...s.poly, ...corners]);
    });
    const lineBoxes: Rect[] = lines.map((l) => {
      const ends = [l.start, l.end];
      return extent([...l.pieces.flatMap(([p, q]) => [p, q]), ...ends]);
    });
    const shapeIds = firstIds(scene.shapes, (s) => s.id);
    const lineIds = firstIds(lines, (l) => l.line.id);
    const sceneLineIds = firstIds(scene.lines, (l) => l.id);
    const junctionIds = firstIds(scene.junctions, (j) => j.id);
    const textIds = firstIds(scene.texts, (t) => t.displayId());
    const junctionBoxes = scene.junctions.map((j) => new Rect(j.at.x, j.at.y, 0.0, 0.0));
    const ms = marks(scene);
    const markBoxes = ms.map((m) => m.rect);
    return new Ctx(
      scene,
      shapes,
      lines,
      shapeBoxes,
      lineBoxes,
      Grid.new(shapeBoxes),
      Grid.new(lineBoxes),
      shapeIds,
      lineIds,
      sceneLineIds,
      junctionIds,
      textIds,
      Grid.new(junctionBoxes),
      ms,
      Grid.new(markBoxes),
    );
  }

  /// Indices into `shapes`, ascending, of the shapes whose outline may lie within `margin` of `r`.
  shapesNear(r: Rect, margin: number): number[] {
    return this.shapeGrid.near(r, margin + 1.0);
  }

  /// Indices into `lines`, ascending, of the lines whose pieces may lie within `margin` of `r`.
  linesNear(r: Rect, margin: number): number[] {
    return this.lineGrid.near(r, margin + 1.0);
  }

  /// Boxes and images whose outline may lie within `margin` of `r`, in `boxes()` order.
  boxesNear(r: Rect, margin: number): ShapeGeom[] {
    return this.shapesNear(r, margin)
      .map((i) => this.shapes[i]!)
      .filter((s) => isBox(s.shape));
  }

  /// Groups whose outline may lie within `margin` of `r`, in `groups()` order.
  groupsNear(r: Rect, margin: number): ShapeGeom[] {
    return this.shapesNear(r, margin)
      .map((i) => this.shapes[i]!)
      .filter((s) => s.shape.kind === "group");
  }

  /// Boxes and images on the normal layer: the opaque shapes that lines and labels must
  /// stay clear of. `layer=under` and `layer=over` shapes are never obstacles.
  boxes(): ShapeGeom[] {
    return this.shapes.filter((s) => isBox(s.shape));
  }

  groups(): ShapeGeom[] {
    return this.shapes.filter((s) => s.shape.kind === "group");
  }

  /// Boxes that `p` lies on or inside: the boxes a line ending at `p` attaches to.
  attached(p: Pt): string[] {
    return this.boxesNear(new Rect(p.x, p.y, 0.0, 0.0), TOLERANCE)
      .filter((s) => pointInPolygon(p, s.poly) || boundaryNearest(p, s.poly)[0] <= TOLERANCE)
      .map((s) => s.shape.id);
  }

  /// Junctions within TOLERANCE of `p`, in file order.
  junctionsAt(p: Pt): Junction[] {
    return this.junctionsNear(new Rect(p.x, p.y, 0.0, 0.0), TOLERANCE).filter(
      (j) => j.at.dist(p) <= TOLERANCE,
    );
  }

  /// Junctions that may lie within `margin` of `r`, in file order.
  junctionsNear(r: Rect, margin: number): Junction[] {
    return this.junctionGrid.near(r, margin + 1.0).map((i) => this.scene.junctions[i]!);
  }

  isJunction(p: Pt): boolean {
    return this.junctionsAt(p).length > 0;
  }

  /// Junctions, texts and labels whose box may lie within `margin` of `r`, in `marks()` order.
  marksNear(r: Rect, margin: number): Mark[] {
    return this.markGrid.near(r, margin + 1.0).map((i) => this.marks[i]!);
  }

  shape(id: string): Shape | null {
    const i = this.shapeIndex(id);
    return i === null ? null : this.scene.shapes[i]!;
  }

  /// Index into `shapes` of the first shape with id `id`.
  shapeIndex(id: string): number | null {
    return this.shapeIds.get(id) ?? null;
  }

  line(id: string): LineGeom | null {
    const i = this.lineIds.get(id);
    return i === undefined ? null : this.lines[i]!;
  }

  /// True when `anc` is a parent, grandparent, ... of shape `id`.
  isAncestor(anc: string, id: string): boolean {
    let cur = this.shape(id)?.parent ?? null;
    while (cur !== null) {
      if (cur === anc) {
        return true;
      }
      cur = this.shape(cur)?.parent ?? null;
    }
    return false;
  }

  /// An element for any id in the scene.
  element(id: string): Element | null {
    const s = this.shape(id);
    if (s !== null) {
      return shapeEl(s);
    }
    const li = this.sceneLineIds.get(id);
    if (li !== undefined) {
      return lineEl(this.scene.lines[li]!, null);
    }
    const ji = this.junctionIds.get(id);
    if (ji !== undefined) {
      return junctionEl(this.scene.junctions[ji]!);
    }
    const ti = this.textIds.get(id);
    return ti === undefined ? null : textEl(this.scene.texts[ti]!);
  }
}

/// Junctions, then texts, line labels and the labels under `text-area=below` shapes.
function marks(scene: Scene): Mark[] {
  const out: Mark[] = [];
  for (const j of scene.junctions) {
    out.push({ rect: new Rect(j.at.x, j.at.y, 0.0, 0.0), owner: j.id });
  }
  for (const t of scene.texts) {
    out.push({ rect: textBox(t), owner: t.id ?? "" });
  }
  for (const l of scene.lines) {
    for (const lab of l.labels) {
      out.push({ rect: labelBox(lab), owner: l.id });
    }
  }
  for (const s of scene.shapes.filter(isBox)) {
    const rect = belowBox(s);
    if (rect !== null) {
      out.push({ rect, owner: s.id });
    }
  }
  return out;
}

/// A box or image on the normal layer.
export function isBox(s: Shape): boolean {
  return s.kind !== "group" && s.layer === "normal";
}

export function fmtPt(p: Pt): string {
  return `(${fmtNum(p.x)},${fmtNum(p.y)})`;
}

/// The bounding box of `pts`, or a NaN box when any of them is not finite.
function extent(pts: Pt[]): Rect {
  if (pts.every((p) => Number.isFinite(p.x) && Number.isFinite(p.y))) {
    return Rect.bounding(pts);
  }
  return new Rect(NaN, NaN, 0.0, 0.0);
}

export function lineBbox(l: LineGeom): Rect {
  return Rect.bounding(l.pieces.flatMap(([p, q]) => [p, q]));
}

export type Overflow = [string, number][];

/// How far `inner` sticks out of `outer` on each side, ignoring anything within TOLERANCE.
export function overflow(outer: Rect, inner: Rect): Overflow {
  const sides: [string, number][] = [
    ["left", outer.x - inner.x],
    ["top", outer.y - inner.y],
    ["right", inner.right() - outer.right()],
    ["bottom", inner.bottom() - outer.bottom()],
  ];
  return sides.filter(([, d]) => d > TOLERANCE).map(([s, d]) => [s, round2(d)]);
}

/// e.g. "right by 20px, bottom by 5px"
export function describeOverflow(over: Overflow): string {
  return over.map(([s, d]) => `${s} by ${fmtNum(d)}px`).join(", ");
}

export function overflowFact(over: Overflow): Json {
  const m = new Map<string, Json>();
  for (const [s, d] of over) m.set(s, new Float(d));
  return m;
}

function sideDistances(p: Pt, r: Rect): [string, number][] {
  return [
    ["top", Math.abs(p.y - r.y)],
    ["bottom", Math.abs(p.y - r.bottom())],
    ["left", Math.abs(p.x - r.x)],
    ["right", Math.abs(p.x - r.right())],
  ];
}

/// The bounding-box side nearest to `p`, and `p`'s position along it (0..1).
export function sideAnchor(s: Shape, p: Pt): [string, number] {
  const r = s.anchorFrame();
  const sides = sideDistances(p, r);
  let best = sides[0]!;
  for (const c of sides.slice(1)) {
    if (totalCmp(c[1], best[1]) < 0) best = c;
  }
  const side = best[0];
  const t = side === "top" || side === "bottom" ? (p.x - r.x) / r.w : (p.y - r.y) / r.h;
  return [side, Number.isFinite(t) ? fclamp(t, 0.0, 1.0) : 0.5];
}

/// The bounding-box side nearest to `p`, and `p`'s position along it (0..1). On shapes
/// with a form, edge anchors project onto the outline, so the nearest side whose anchor
/// lands back on `p` wins.
export function outlineAnchor(s: Shape, p: Pt): [string, number] {
  const nearest = sideAnchor(s, p);
  if (s.form === null) {
    return nearest;
  }
  const r = s.anchorFrame();
  const sides = sideDistances(p, r);
  sides.sort((a, b) => totalCmp(a[1], b[1]));
  for (const [side] of sides) {
    let t = side === "top" || side === "bottom" ? (p.x - r.x) / r.w : (p.y - r.y) / r.h;
    t = Number.isFinite(t) ? fclamp(t, 0.0, 1.0) : 0.5;
    const q = s.anchor(side, t);
    if (q !== null && q.dist(p) <= 0.5) {
      return [side, t];
    }
  }
  return nearest;
}

/// `id.side` or `id.side@t` for the anchor of `s` at outline point `p`.
export function anchorText(s: Shape, p: Pt): string {
  const [side, t0] = outlineAnchor(s, p);
  const t = round2(t0);
  return t === 0.5 ? `${s.id}.${side}` : `${s.id}.${side}@${displayF64(t)}`;
}
