import { asUsize } from "../diag/rust.js";
import { Pt, type Rect, rectPolygon } from "../geom/index.js";
import { type Shape } from "../model/index.js";

const FRAC_PI_2 = Math.PI / 2;

/// The outline the checks use, as a closed polygon in canvas coordinates: the flattened
/// form outline for built-in, custom and SVG shapes (box or hull), the box for images.
export function polygon(s: Shape): Pt[] {
  if (s.form !== null && s.form.outline.length >= 3) {
    return [...s.form.outline];
  }
  const r = s.cornerRadius();
  if (s.outline === "rounded" && r > 0.0) {
    return rounded(s.rect, r);
  }
  return rectPolygon(s.rect);
}

/// Quarter arcs at each corner, clockwise from the top-left, flattened within 0.25px.
function rounded(b: Rect, r: number): Pt[] {
  const steps = Math.max(asUsize(Math.ceil(FRAC_PI_2 * Math.sqrt(r / 2.0))), 2);
  const corners: [number, number, number][] = [
    [b.x + r, b.y + r, Math.PI],
    [b.right() - r, b.y + r, 1.5 * Math.PI],
    [b.right() - r, b.bottom() - r, 0.0],
    [b.x + r, b.bottom() - r, FRAC_PI_2],
  ];
  const pts: Pt[] = [];
  for (const [cx, cy, a0] of corners) {
    for (let k = 0; k <= steps; k++) {
      const a = a0 + (FRAC_PI_2 * k) / steps;
      pts.push(new Pt(cx + r * Math.cos(a), cy + r * Math.sin(a)));
    }
  }
  return pts;
}
