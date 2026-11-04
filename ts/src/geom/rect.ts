import { Float, type Json } from "../diag/json.js";
import { fmax, fmin } from "../diag/rust.js";
import { Pt, TOLERANCE } from "./point.js";

export class Rect {
  constructor(
    readonly x: number,
    readonly y: number,
    readonly w: number,
    readonly h: number,
  ) {}

  static new(x: number, y: number, w: number, h: number): Rect {
    return new Rect(x, y, w, h);
  }

  static default(): Rect {
    return new Rect(0, 0, 0, 0);
  }

  origin(): Pt {
    return new Pt(this.x, this.y);
  }

  center(): Pt {
    return new Pt(this.x + this.w / 2, this.y + this.h / 2);
  }

  right(): number {
    return this.x + this.w;
  }

  bottom(): number {
    return this.y + this.h;
  }

  /// Maps a point in unit coordinates (0..1 across the rect) to this rect.
  mapUnit(p: Pt): Pt {
    return new Pt(this.x + p.x * this.w, this.y + p.y * this.h);
  }

  /// Maps a rect in unit coordinates to this rect.
  mapRect(u: Rect): Rect {
    return new Rect(this.x + u.x * this.w, this.y + u.y * this.h, u.w * this.w, u.h * this.h);
  }

  /// The rect with corners `a` and `b`.
  static fromPoints(a: Pt, b: Pt): Rect {
    return new Rect(fmin(a.x, b.x), fmin(a.y, b.y), Math.abs(a.x - b.x), Math.abs(a.y - b.y));
  }

  /// The bounding box of `pts`, or the default rect when there are none.
  static bounding(pts: Iterable<Pt>): Rect {
    const it = pts[Symbol.iterator]();
    const first = it.next();
    if (first.done) {
      return Rect.default();
    }
    let lo = first.value;
    let hi = first.value;
    for (let n = it.next(); !n.done; n = it.next()) {
      const p = n.value;
      lo = new Pt(fmin(lo.x, p.x), fmin(lo.y, p.y));
      hi = new Pt(fmax(hi.x, p.x), fmax(hi.y, p.y));
    }
    return new Rect(lo.x, lo.y, hi.x - lo.x, hi.y - lo.y);
  }

  /// The smallest rect holding both.
  union(o: Rect): Rect {
    const x0 = fmin(this.x, o.x);
    const y0 = fmin(this.y, o.y);
    const x1 = fmax(this.right(), o.right());
    const y1 = fmax(this.bottom(), o.bottom());
    return new Rect(x0, y0, x1 - x0, y1 - y0);
  }

  /// The overlap of the two, when it is more than `TOLERANCE` in both directions.
  intersect(o: Rect): Rect | null {
    const x0 = fmax(this.x, o.x);
    const y0 = fmax(this.y, o.y);
    const x1 = fmin(this.right(), o.right());
    const y1 = fmin(this.bottom(), o.bottom());
    return x1 - x0 > TOLERANCE && y1 - y0 > TOLERANCE ? new Rect(x0, y0, x1 - x0, y1 - y0) : null;
  }

  /// Whether `inner` lies within this rect, touching allowed.
  containsRect(inner: Rect): boolean {
    return (
      inner.x >= this.x - TOLERANCE &&
      inner.y >= this.y - TOLERANCE &&
      inner.right() <= this.right() + TOLERANCE &&
      inner.bottom() <= this.bottom() + TOLERANCE
    );
  }

  eq(o: Rect): boolean {
    return this.x === o.x && this.y === o.y && this.w === o.w && this.h === o.h;
  }

  toJson(): Json {
    return { x: new Float(this.x), y: new Float(this.y), w: new Float(this.w), h: new Float(this.h) };
  }
}
