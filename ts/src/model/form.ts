import { type Json } from "../diag/json.js";
import { type Result, fmax, fmin } from "../diag/rust.js";
import {
  type PathSeg,
  Pt,
  Rect,
  arcToCubics,
  clipToRect,
  flatten,
  horizontalHits,
  line,
  segMap,
  segToJson,
  verticalHits,
} from "../geom/index.js";
import { resolve } from "./anchor.js";
import { type Asset, Fit, type Shape, cornerRadius } from "./scene.js";
import { type UnitShape } from "./shapes.js";

/// The geometry of a non-rectangular shape, in canvas coordinates.
export class Form {
  constructor(
    /// Subpaths to fill and stroke; empty for SVG-file shapes, which draw their image instead.
    public draw: PathSeg[][],
    /// Closed outline polygon, flattened, used for anchors and checks.
    public outline: Pt[],
    /// The rect anchors resolve on when it is not the box: the drawing of an SVG-file
    /// shape with `fit=contain` whose aspect ratio differs from the box's.
    public frame: Rect | null,
  ) {}

  static fromUnit(u: UnitShape, r: Rect): Form {
    const draw: PathSeg[][] = u.subpaths.map((sp) => sp.map((s) => segMap(s, (p) => r.mapUnit(p))));
    const outline = closed(flatten(draw[u.outline]!));
    if (u.hiddenOutline) {
      draw.splice(u.outline, 1);
    }
    return new Form(draw, outline, null);
  }

  /// An SVG-file shape: nothing to draw (the image draws instead) and an outline that is
  /// the drawing's rect, or its convex hull, as `fit` places it on the box and cropped
  /// to the box.
  static fromSvg(asset: Asset, hull: boolean, fit: Fit, r: Rect): Form {
    const placed = Fit.place(fit, asset.natural, r);
    let outline: Pt[];
    if (hull) {
      const pts: Pt[] = asset.unitHull.map((p) => placed.mapUnit(p));
      outline = fit === "cover" ? clipToRect(pts, r) : pts;
    } else {
      outline = corners(placed.intersect(r) ?? placed);
    }
    const differs = Math.abs(placed.w - r.w) > 1e-9 || Math.abs(placed.h - r.h) > 1e-9;
    const frame = fit === "contain" && differs ? placed : null;
    return new Form([], outline, frame);
  }

  toJson(): Json {
    const o: { [key: string]: Json } = {
      draw: this.draw.map((sp) => sp.map(segToJson)),
      outline: this.outline.map((p) => p.toJson()),
    };
    if (this.frame !== null) o["frame"] = this.frame.toJson();
    return o;
  }
}

/// The shape's outline as a closed polygon in canvas coordinates, for anchors and checks.
/// Rounded corners and curves are flattened to `FLATTEN_TOLERANCE`.
export function outlinePolygon(s: Shape): Pt[] {
  if (s.form !== null) {
    return [...s.form.outline];
  }
  switch (s.outline) {
    case "rounded":
      return roundedPolygon(s.rect, s.cornerRadius());
    case "rect":
    case "path":
      return corners(s.rect);
  }
}

/// The rect anchors resolve on: the drawing of a contained SVG-file shape, else the box.
export function anchorFrame(s: Shape): Rect {
  return s.form?.frame ?? s.rect;
}

/// How much `fit=stretch` changes the aspect ratio of an image or SVG-file shape:
/// |box ratio / natural ratio - 1|. `null` for other fits, shapes without an image,
/// and files that failed to load.
export function imageDistortion(s: Shape, assets: readonly Asset[]): number | null {
  const img = s.image !== null && s.image.fit === "stretch" ? s.image : null;
  if (img === null) return null;
  const asset = assets[img.asset];
  if (asset === undefined) return null;
  const [nw, nh] = asset.natural;
  if (nw <= 0.0 || nh <= 0.0 || img.rect.h <= 0.0) {
    return null;
  }
  return Math.abs(img.rect.w / img.rect.h / (nw / nh) - 1.0);
}

/// Resolves an anchor on a placed box or group (§4). On shapes with a `form`, edge
/// anchors are moved onto the outline along their axis.
export function resolveAnchor(
  form: Form | null,
  rect: Rect,
  radius: number,
  owner: string,
  name: string,
  t: number | null,
): Result<Pt, [string, string | null]> {
  if (form !== null) {
    const r = resolve(form.frame ?? rect, 0.0, owner, name, t);
    return r.ok ? { ok: true, value: project(form.outline, name, r.value) } : r;
  }
  return resolve(rect, cornerRadius(radius, rect), owner, name, t);
}

/// Moves a bbox anchor point onto the outline along its axis: `top`/`bottom` move
/// vertically, `left`/`right` horizontally. Other anchors, and lines that miss the
/// outline, keep the bbox point.
export function project(poly: readonly Pt[], anchor: string, p: Pt): Pt {
  const pick = (v: number[], max: boolean): number | null => {
    const f = max ? fmax : fmin;
    return v.length === 0 ? null : v.reduce(f);
  };
  let v: number | null;
  switch (anchor) {
    case "top":
      v = pick(verticalHits(poly, p.x), false);
      return v === null ? p : new Pt(p.x, v);
    case "bottom":
      v = pick(verticalHits(poly, p.x), true);
      return v === null ? p : new Pt(p.x, v);
    case "left":
      v = pick(horizontalHits(poly, p.y), false);
      return v === null ? p : new Pt(v, p.y);
    case "right":
      v = pick(horizontalHits(poly, p.y), true);
      return v === null ? p : new Pt(v, p.y);
    default:
      return p;
  }
}

export function corners(r: Rect): Pt[] {
  return [
    new Pt(r.x, r.y),
    new Pt(r.right(), r.y),
    new Pt(r.right(), r.bottom()),
    new Pt(r.x, r.bottom()),
  ];
}

export function roundedPolygon(r: Rect, rad: number): Pt[] {
  if (rad <= 0.0) {
    return corners(r);
  }
  const { x, y, w, h } = r;
  const p = (a: number, b: number) => new Pt(a, b);
  const arcs: [Pt, Pt][] = [
    [p(x + w - rad, y), p(x + w, y + rad)],
    [p(x + w, y + h - rad), p(x + w - rad, y + h)],
    [p(x + rad, y + h), p(x, y + h - rad)],
    [p(x, y + rad), p(x + rad, y)],
  ];
  const segs: PathSeg[] = [];
  let cur = p(x + rad, y);
  for (const [a, b] of arcs) {
    if (cur.dist(a) > 1e-9) {
      segs.push(line(cur, a));
    }
    segs.push(...arcToCubics(a, rad, rad, 0.0, false, true, b));
    cur = b;
  }
  return closed(flatten(segs));
}

/// Drops the repeated first point of a flattened closed path.
function closed(pts: Pt[]): Pt[] {
  if (pts.length > 1 && pts[0]!.dist(pts[pts.length - 1]!) <= 1e-9) {
    pts.pop();
  }
  return pts;
}
