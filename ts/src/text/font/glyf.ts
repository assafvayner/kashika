import { f32 } from "../../diag/rust.js";
import type { Data, Sfnt } from "./tables.js";

/// skrifa's `OutlinePen`: coordinates are `f32` font units.
export interface OutlinePen {
  moveTo(x: number, y: number): void;
  lineTo(x: number, y: number): void;
  quadTo(x1: number, y1: number, x: number, y: number): void;
  curveTo(x1: number, y1: number, x2: number, y2: number, x: number, y: number): void;
  close(): void;
}

const ARG_1_AND_2_ARE_WORDS = 0x0001;
const ARGS_ARE_XY_VALUES = 0x0002;
const WE_HAVE_A_SCALE = 0x0008;
const MORE_COMPONENTS = 0x0020;
const WE_HAVE_AN_X_AND_Y_SCALE = 0x0040;
const WE_HAVE_A_TWO_BY_TWO = 0x0080;
const SCALED_COMPONENT_OFFSET = 0x0800;
const UNSCALED_COMPONENT_OFFSET = 0x1000;

const ON_CURVE = 0x01;
const X_SHORT_VECTOR = 0x02;
const Y_SHORT_VECTOR = 0x04;
const REPEAT_FLAG = 0x08;
const X_SAME_OR_POSITIVE = 0x10;
const Y_SAME_OR_POSITIVE = 0x20;

const RECURSION_LIMIT = 32;

/// A loaded outline in font units. skrifa keeps unscaled points in 26.6 fixed point with
/// integral values, so plain integers hold them exactly.
interface Outline {
  xs: number[];
  ys: number[];
  onCurve: boolean[];
  contours: number[];
}

/// `Fixed * Fixed` as font-types computes it, rounding half away from zero.
function fixedMul(a: number, b: number): number {
  const ab = a * b;
  return Math.floor((ab + 0x8000 - (ab < 0 ? 1 : 0)) / 65536);
}

/// FreeType's approximate hypot of two `Fixed` values.
function fixedHypot(a: number, b: number): number {
  const x = Math.abs(a);
  const y = Math.abs(b);
  return x > y ? x + ((3 * y) >> 3) : y + ((3 * x) >> 3);
}

export class Glyf {
  private readonly glyf: Data;
  private readonly loca: Data;
  private readonly longLoca: boolean;
  private readonly numGlyphs: number;

  constructor(font: Sfnt) {
    this.glyf = font.required("glyf");
    this.loca = font.required("loca");
    this.longLoca = font.head.indexToLocFormat !== 0;
    this.numGlyphs = font.numGlyphs;
  }

  /// Whether skrifa has an outline for `gid`: every glyph in range, empty or not.
  has(gid: number): boolean {
    return gid < this.numGlyphs;
  }

  private glyphData(gid: number): Data | null {
    let start: number;
    let end: number;
    if (this.longLoca) {
      start = this.loca.u32(gid * 4);
      end = this.loca.u32(gid * 4 + 4);
    } else {
      start = this.loca.u16(gid * 2) * 2;
      end = this.loca.u16(gid * 2 + 2) * 2;
    }
    if (end <= start) return null;
    return this.glyf.slice(start, end - start);
  }

  /// Draws `gid` unhinted at `Size::unscaled()` the way skrifa's FreeType path style does.
  draw(gid: number, pen: OutlinePen): void {
    if (!this.has(gid)) return;
    const out: Outline = { xs: [], ys: [], onCurve: [], contours: [] };
    this.load(gid, out, 0);
    toPath(out, pen);
  }

  private load(gid: number, out: Outline, depth: number): void {
    if (depth > RECURSION_LIMIT)
      throw new Error(`glyph ${gid} exceeds the composite recursion limit`);
    const g = this.glyphData(gid);
    if (g === null) return;
    const n = g.i16(0);
    if (n >= 0) this.loadSimple(g, n, out);
    else this.loadComposite(g, out, depth);
  }

  private loadSimple(g: Data, contourCount: number, out: Outline): void {
    const base = out.xs.length;
    const ends: number[] = [];
    for (let i = 0; i < contourCount; i++) ends.push(g.u16(10 + i * 2));
    const count = contourCount === 0 ? 0 : ends[contourCount - 1]! + 1;
    const insLen = g.u16(10 + contourCount * 2);
    let p = 12 + contourCount * 2 + insLen;
    const flags: number[] = [];
    while (flags.length < count) {
      const f = g.u8(p++);
      if ((f & REPEAT_FLAG) !== 0) {
        const repeat = Math.min(g.u8(p++) + 1, count - flags.length);
        for (let k = 0; k < repeat; k++) flags.push(f);
      } else {
        flags.push(f);
      }
    }
    let x = 0;
    for (const f of flags) {
      let d = 0;
      if ((f & X_SHORT_VECTOR) !== 0) {
        d = g.u8(p++);
        if ((f & X_SAME_OR_POSITIVE) === 0) d = -d;
      } else if ((f & X_SAME_OR_POSITIVE) === 0) {
        d = g.i16(p);
        p += 2;
      }
      x = (x + d) | 0;
      out.xs.push(x);
    }
    let y = 0;
    for (const f of flags) {
      let d = 0;
      if ((f & Y_SHORT_VECTOR) !== 0) {
        d = g.u8(p++);
        if ((f & Y_SAME_OR_POSITIVE) === 0) d = -d;
      } else if ((f & Y_SAME_OR_POSITIVE) === 0) {
        d = g.i16(p);
        p += 2;
      }
      y = (y + d) | 0;
      out.ys.push(y);
      out.onCurve.push((f & ON_CURVE) !== 0);
    }
    let last = 0;
    for (const e of ends) {
      if (e < last) throw new Error("unordered contour end points in TrueType glyph");
      last = e;
      out.contours.push(e + base);
    }
  }

  private loadComposite(g: Data, out: Outline, depth: number): void {
    let p = 10;
    for (;;) {
      const flags = g.u16(p);
      const glyph = g.u16(p + 2);
      p += 4;
      const words = (flags & ARG_1_AND_2_ARE_WORDS) !== 0;
      if ((flags & ARGS_ARE_XY_VALUES) === 0) {
        throw new Error("composite glyph anchored by point numbers, which no bundled font uses");
      }
      let dx = words ? g.i16(p) : g.i8(p);
      let dy = words ? g.i16(p + 2) : g.i8(p + 1);
      p += words ? 4 : 2;
      let xx = 0x4000;
      let yx = 0;
      let xy = 0;
      let yy = 0x4000;
      if ((flags & WE_HAVE_A_SCALE) !== 0) {
        xx = g.i16(p);
        yy = xx;
        p += 2;
      } else if ((flags & WE_HAVE_AN_X_AND_Y_SCALE) !== 0) {
        xx = g.i16(p);
        yy = g.i16(p + 2);
        p += 4;
      } else if ((flags & WE_HAVE_A_TWO_BY_TWO) !== 0) {
        xx = g.i16(p);
        yx = g.i16(p + 2);
        xy = g.i16(p + 4);
        yy = g.i16(p + 6);
        p += 8;
      }
      const start = out.xs.length;
      this.load(glyph, out, depth + 1);
      const end = out.xs.length;
      const [fxx, fyx, fxy, fyy] = [xx * 4, yx * 4, xy * 4, yy * 4];
      const haveXform =
        (flags & (WE_HAVE_A_SCALE | WE_HAVE_AN_X_AND_Y_SCALE | WE_HAVE_A_TWO_BY_TWO)) !== 0;
      if (haveXform) {
        for (let i = start; i < end; i++) {
          const ux = out.xs[i]!;
          const uy = out.ys[i]!;
          out.xs[i] = (fixedMul(ux, fxx) + fixedMul(uy, fxy)) | 0;
          out.ys[i] = (fixedMul(ux, fyx) + fixedMul(uy, fyy)) | 0;
        }
      }
      if (
        haveXform &&
        (flags & (SCALED_COMPONENT_OFFSET | UNSCALED_COMPONENT_OFFSET)) === SCALED_COMPONENT_OFFSET
      ) {
        dx = fixedMul(dx, fixedHypot(fxx, fxy));
        dy = fixedMul(dy, fixedHypot(fyy, fyx));
      }
      if (dx !== 0 || dy !== 0) {
        for (let i = start; i < end; i++) {
          out.xs[i] = out.xs[i]! + dx;
          out.ys[i] = out.ys[i]! + dy;
        }
      }
      if ((flags & MORE_COMPONENTS) === 0) break;
    }
  }
}

type Pending = { kind: "empty" } | { kind: "quad"; x: number; y: number };

/// skrifa's `to_path` with `PathStyle::FreeType` for quadratic contours. A contour that
/// starts off-curve starts at its last point when that is on-curve (which is then not
/// repeated), else at the midpoint of its last and first points. Consecutive off-curve
/// points imply an on-curve midpoint, and the contour closes back to its start.
function toPath(o: Outline, pen: OutlinePen): void {
  let startIx = 0;
  for (const endIx of o.contours) {
    if (endIx < startIx || endIx >= o.xs.length) throw new Error("contour order");
    const ix: number[] = [];
    for (let i = startIx; i <= endIx; i++) ix.push(i);
    startIx = endIx + 1;
    if (ix.length === 0) continue;
    const first = ix[0]!;
    const last = ix[ix.length - 1]!;
    let points = ix;
    let sx: number;
    let sy: number;
    if (!o.onCurve[first]) {
      if (o.onCurve[last]) {
        sx = o.xs[last]!;
        sy = o.ys[last]!;
        points = ix.slice(0, -1);
      } else {
        sx = (o.xs[last]! + o.xs[first]!) / 2;
        sy = (o.ys[last]! + o.ys[first]!) / 2;
      }
    } else {
      sx = o.xs[first]!;
      sy = o.ys[first]!;
      points = ix.slice(1);
    }
    pen.moveTo(f32(sx), f32(sy));
    let state: Pending = { kind: "empty" };
    const emit = (x: number, y: number, on: boolean) => {
      if (state.kind === "empty") {
        if (!on) state = { kind: "quad", x, y };
        else pen.lineTo(f32(x), f32(y));
      } else if (!on) {
        pen.quadTo(f32(state.x), f32(state.y), f32((state.x + x) / 2), f32((state.y + y) / 2));
        state = { kind: "quad", x, y };
      } else {
        pen.quadTo(f32(state.x), f32(state.y), f32(x), f32(y));
        state = { kind: "empty" };
      }
    };
    for (const i of points) emit(o.xs[i]!, o.ys[i]!, o.onCurve[i]!);
    if (state.kind !== "empty") emit(sx, sy, true);
    pen.close();
  }
}
