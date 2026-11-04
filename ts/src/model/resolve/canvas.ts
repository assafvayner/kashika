import { fmax } from "../../diag/rust.js";
import { Rect, flatten } from "../../geom/index.js";
import { type Resolver } from "../resolve.js";
import { type Junction, type Line, Marker, type Scene, type Shape, type Text } from "../scene.js";

/// Radius of a junction's `dot`.
const DOT_RADIUS = 3.0;

/// Sets each `auto` canvas axis to the far edge of everything drawn, plus `margin`.
export function fitCanvas(r: Resolver, scene: Scene): void {
  const [fitW, fitH] = r.canvasFit;
  scene.fitted = [fitW, fitH];
  if (!fitW && !fitH) {
    return;
  }
  const [w, h] = extent(scene);
  if (fitW) {
    scene.size[0] = Math.ceil(fmax(w, 0.0)) + r.margin;
  }
  if (fitH) {
    scene.size[1] = Math.ceil(fmax(h, 0.0)) + r.margin;
  }
}

/// The largest x and y that anything painted reaches.
function extent(scene: Scene): [number, number] {
  const far = Reach.default();
  for (const s of scene.shapes) {
    far.shape(s);
  }
  for (const j of scene.junctions) {
    far.junction(j);
  }
  for (const t of scene.texts) {
    far.text(t);
  }
  for (const l of scene.lines) {
    far.line(l);
  }
  return far.far();
}

/// The largest x and y painted items reach: shapes with their strokes, text and icons
/// (labels under a shape included), junction dots, texts, and lines with their strokes,
/// markers and labels.
export class Reach {
  constructor(
    private x: number,
    private y: number,
  ) {}

  static default(): Reach {
    return new Reach(-Infinity, -Infinity);
  }

  far(): [number, number] {
    return [this.x, this.y];
  }

  private rect(r: Rect, pad: number): void {
    this.x = fmax(this.x, r.right() + pad);
    this.y = fmax(this.y, r.bottom() + pad);
  }

  shape(s: Shape): void {
    this.rect(s.rect, s.style.strokeWidth / 2.0);
    for (const t of s.text) {
      this.rect(t.rect, 0.0);
    }
    if (s.icon !== null) {
      this.rect(s.icon.rect, 0.0);
    }
  }

  junction(j: Junction): void {
    const r = j.dot ? DOT_RADIUS : 0.0;
    this.rect(new Rect(j.at.x, j.at.y, 0.0, 0.0), r);
  }

  text(t: Text): void {
    this.rect(t.rect, 0.0);
  }

  line(l: Line): void {
    const pad = fmax(
      fmax(l.style.strokeWidth / 2.0, Marker.halfWidth(l.start, l.style.strokeWidth)),
      Marker.halfWidth(l.end, l.style.strokeWidth),
    );
    for (const p of flatten(l.path)) {
      this.rect(new Rect(p.x, p.y, 0.0, 0.0), pad);
    }
    for (const lab of l.labels) {
      this.rect(lab.rect, 0.0);
    }
  }
}
