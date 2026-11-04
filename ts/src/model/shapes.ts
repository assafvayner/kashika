import { asUsize, displayF64, fmax, fmin, hypot, round } from "../diag/rust.js";
import { type PathSeg, Rect, parseUnitPath } from "../geom/index.js";

/// Builds a unit-path shape for a `w`×`h` box: path data, outline subpath (1-based) and
/// unit text area.
export type ShapeSource = (w: number, h: number) => [string, number, Rect];

/// A built-in `shape=`.
export interface BuiltinShape {
  readonly name: string;
  /// Search keywords for `kashika icons --search`.
  readonly keywords: readonly string[];
  /// The unit path, outline subpath (1-based) and unit text area on a `w`×`h` box;
  /// `null` for `rect` and `rounded`, which use native outlines.
  readonly source: ShapeSource | null;
  /// The outline subpath is used only for anchors and checks, and is not drawn.
  readonly hiddenOutline: boolean;
}

const n = displayF64;

/// Largest cylinder cap height in px.
const CAP = 20.0;
/// Largest `doc` wave depth in px.
const WAVE = 12.0;
/// Largest `cloud` bump height in px.
const BUMP = 24.0;
/// Largest `person` head radius in px.
const HEAD = 18.0;
/// Largest `parallelogram` slant in px.
const SLANT = 24.0;
/// Largest `predefined-process` bar inset in px.
const BAR = 12.0;
/// Largest `octagon` corner cut in px.
const CUT = 24.0;
/// Largest `note` fold in px.
const FOLD = 16.0;
/// Largest `package` tab height and width in px.
const TAB: [number, number] = [12.0, 56.0];
/// Largest `queue` cap width in px.
const SIDE_CAP = 12.0;

/// A shape in unit coordinates, stretched onto each box.
export interface UnitShape {
  subpaths: PathSeg[][];
  /// 0-based index of the subpath used for anchors and checks.
  outline: number;
  /// Whether the outline subpath is left out of the drawing.
  hiddenOutline: boolean;
  /// Unit rect of the text area, before padding.
  textArea: Rect;
}

function cylinder(_w: number, h: number): [string, number, Rect] {
  const c = fmin(CAP, h / 4.0) / h;
  const b = 1.0 - c;
  return [
    `M 0 ${n(c)} A 0.5 ${n(c)} 0 0 1 1 ${n(c)} L 1 ${n(b)} A 0.5 ${n(c)} 0 0 1 0 ${n(b)} Z ` +
      `M 1 ${n(c)} A 0.5 ${n(c)} 0 0 1 0 ${n(c)}`,
    1,
    new Rect(0.0, 2.0 * c, 1.0, 1.0 - 2.0 * c),
  ];
}

function diamond(_w: number, _h: number): [string, number, Rect] {
  return ["M 0.5 0 L 1 0.5 L 0.5 1 L 0 0.5 Z", 1, new Rect(0.22, 0.28, 0.56, 0.44)];
}

function ellipse(_w: number, _h: number): [string, number, Rect] {
  const k = 0.5 - 0.5 * Math.SQRT1_2;
  return [
    "M 0 0.5 A 0.5 0.5 0 0 1 1 0.5 A 0.5 0.5 0 0 1 0 0.5 Z",
    1,
    new Rect(k, k, Math.SQRT1_2, Math.SQRT1_2),
  ];
}

function hexagon(w: number, h: number): [string, number, Rect] {
  const nn = fmin(h / 4.0, w / 4.0) / w;
  const m = 1.0 - nn;
  return [
    `M ${n(nn)} 0 L ${n(m)} 0 L 1 0.5 L ${n(m)} 1 L ${n(nn)} 1 L 0 0.5 Z`,
    1,
    new Rect(nn, 0.0, 1.0 - 2.0 * nn, 1.0),
  ];
}

function doc(_w: number, h: number): [string, number, Rect] {
  const d = fmin(WAVE, 0.15 * h) / h;
  const b = 1.0 - d;
  const c = b - d;
  return [
    `M 0 0 L 1 0 L 1 ${n(b)} Q 0.75 ${n(c)} 0.5 ${n(b)} Q 0.25 1 0 ${n(b)} Z`,
    1,
    new Rect(0.0, 0.0, 1.0, b - d / 2.0),
  ];
}

/// A flat-bottomed cloud: bumps of at most `BUMP` px along the top and as many
/// side lobes as fit, so stretching adds bumps instead of distorting them.
function cloud(w: number, h: number): [string, number, Rect] {
  const c = fmin(fmin(BUMP, 0.3 * h), 0.175 * w);
  const s = fmin(fmin(0.15 * w, (h - c) / 2.0), BUMP - 4.0);
  let d = `M ${n(s / w)} 1`;
  const arcs = (
    from: [number, number],
    to: [number, number],
    bulge: number,
    count: number,
  ): void => {
    const cnt = fmax(round(count), 1.0);
    const len = hypot(to[0] - from[0], to[1] - from[1]) / cnt;
    const r = (len * len / 4.0 + bulge * bulge) / (2.0 * bulge);
    const large = bulge > len / 2.0 ? 1 : 0;
    const steps = asUsize(cnt);
    for (let i = 1; i <= steps; i++) {
      const t = i / cnt;
      const x = from[0] + (to[0] - from[0]) * t;
      const y = from[1] + (to[1] - from[1]) * t;
      d += ` A ${n(r / w)} ${n(r / h)} 0 ${large} 1 ${n(x / w)} ${n(y / h)}`;
    }
  };
  const side = h - c;
  arcs([s, h], [s, c], s, side / (2.5 * s));
  arcs([s, c], [w - s, c], c, fmax((w - 2.0 * s) / (2.5 * c), 2.0));
  arcs([w - s, c], [w - s, h], s, side / (2.5 * s));
  d += " Z";
  const area = new Rect(s / w, c / h, 1.0 - (2.0 * s) / w, (h - 1.5 * c) / h);
  return [d, 1, area];
}

/// A round head of radius at most `HEAD` px over a body with rounded shoulders that
/// spans the box; the text sits in the body. The third subpath, not drawn, is the
/// silhouette of both joined by a neck, for anchors and checks.
function person(w: number, h: number): [string, number, Rect] {
  const r = fmin(fmin(HEAD, 0.2 * h), 0.22 * w);
  const top = fmin(2.0 * r + fmax(0.25 * r, 3.0), h);
  const k = fmin(fmin(0.6 * (h - top), w / 2.0), 1.6 * r);
  const cx = w / 2.0;
  const nk = 0.6 * r;
  const p = (x: number, y: number) => `${n(x / w)} ${n(y / h)}`;
  const head = (rx: number, ry: number) => `${n(rx / w)} ${n(ry / h)}`;
  const shoulders = (neck: string) =>
    `M ${p(0.0, h)} L ${p(0.0, top + k)} A ${head(k, k)} 0 0 1 ${p(k, top)} ${neck}L ${p(w - k, top)} A ${head(k, k)} 0 0 1 ${p(w, top + k)} L ${p(w, h)} Z`;
  const d = `M ${p(cx + r, r)} A ${head(r, r)} 0 1 1 ${p(cx - r, r)} A ${head(r, r)} 0 1 1 ${p(cx + r, r)} Z ${shoulders("")} ${shoulders(
    `L ${p(cx - nk, top)} L ${p(cx - nk, 1.8 * r)} A ${head(r, r)} 0 1 1 ${p(cx + nk, 1.8 * r)} L ${p(cx + nk, top)} `,
  )}`;
  const i = 0.3 * k;
  const area = new Rect(i / w, (top + i) / h, 1.0 - (2.0 * i) / w, (h - top - i) / h);
  return [d, 3, area];
}

/// Formats a px point on a `w`×`h` box as a unit-path point.
function px(w: number, h: number): (x: number, y: number) => string {
  return (x, y) => `${n(x / w)} ${n(y / h)}`;
}

/// A px rect on a `w`×`h` box as a unit rect.
function unitRect(w: number, h: number, r: Rect): Rect {
  return new Rect(r.x / w, r.y / h, r.w / w, r.h / h);
}

function parallelogram(w: number, h: number): [string, number, Rect] {
  const p = px(w, h);
  const s = fmin(fmin(SLANT, h / 2.0), w / 4.0);
  const d = `M ${p(s, 0.0)} L ${p(w, 0.0)} L ${p(w - s, h)} L ${p(0.0, h)} Z`;
  return [d, 1, unitRect(w, h, new Rect(s, 0.0, w - 2.0 * s, h))];
}

function predefinedProcess(w: number, _h: number): [string, number, Rect] {
  const i = fmin(BAR, w / 8.0) / w;
  const m = 1.0 - i;
  return [
    `M 0 0 L 1 0 L 1 1 L 0 1 Z M ${n(i)} 0 L ${n(i)} 1 M ${n(m)} 0 L ${n(m)} 1`,
    1,
    new Rect(i, 0.0, 1.0 - 2.0 * i, 1.0),
  ];
}

/// A rect whose corner radius `r` is half its shorter side. The text area's corners lie on
/// the corner circles, `0.1 × r` in from the top and bottom.
function stadium(w: number, h: number): [string, number, Rect] {
  const p = px(w, h);
  const r = fmin(w, h) / 2.0;
  const arc = (x: number, y: number) => ` A ${n(r / w)} ${n(r / h)} 0 0 1 ${p(x, y)}`;
  let d = `M ${p(r, 0.0)}`;
  const wide = w - r > r;
  const tall = h - r > r;
  if (wide) {
    d += ` L ${p(w - r, 0.0)}`;
  }
  d += arc(w, r);
  if (tall) {
    d += ` L ${p(w, h - r)}`;
  }
  d += arc(w - r, h);
  if (wide) {
    d += ` L ${p(r, h)}`;
  }
  d += arc(0.0, h - r);
  if (tall) {
    d += ` L ${p(0.0, r)}`;
  }
  d += arc(r, 0.0);
  d += " Z";
  const [a, b] = [0.57 * r, 0.1 * r];
  return [d, 1, unitRect(w, h, new Rect(a, b, w - 2.0 * a, h - 2.0 * b))];
}

function triangle(_w: number, _h: number): [string, number, Rect] {
  return ["M 0.5 0 L 1 1 L 0 1 Z", 1, new Rect(0.25, 0.5, 0.5, 0.5)];
}

function octagon(w: number, h: number): [string, number, Rect] {
  const p = px(w, h);
  const c = fmin(CUT, 0.29 * fmin(w, h));
  const d = `M ${p(c, 0.0)} L ${p(w - c, 0.0)} L ${p(w, c)} L ${p(w, h - c)} L ${p(w - c, h)} L ${p(c, h)} L ${p(0.0, h - c)} L ${p(0.0, c)} Z`;
  return [d, 1, unitRect(w, h, new Rect(c / 2.0, c / 2.0, w - c, h - c))];
}

/// A sheet with its top-right corner folded down; the fold is the second subpath.
function note(w: number, h: number): [string, number, Rect] {
  const p = px(w, h);
  const f = fmin(fmin(FOLD, w / 4.0), h / 4.0);
  const d = `M ${p(0.0, 0.0)} L ${p(w - f, 0.0)} L ${p(w, f)} L ${p(w, h)} L ${p(0.0, h)} Z M ${p(w - f, 0.0)} L ${p(w - f, f)} L ${p(w, f)}`;
  return [d, 1, unitRect(w, h, new Rect(0.0, f / 2.0, w - f / 2.0, h - f / 2.0))];
}

/// A folder: a tab on the top left over the body, which holds the text.
function pkg(w: number, h: number): [string, number, Rect] {
  const p = px(w, h);
  const t = fmin(TAB[0], h / 5.0);
  const tw = fmin(TAB[1], 0.4 * w);
  const d = `M ${p(0.0, 0.0)} L ${p(tw, 0.0)} L ${p(tw, t)} L ${p(w, t)} L ${p(w, h)} L ${p(0.0, h)} Z M ${p(0.0, t)} L ${p(tw, t)}`;
  return [d, 1, unitRect(w, h, new Rect(0.0, t, w, h - t))];
}

/// A cylinder on its side: caps `min(SIDE_CAP, h/5, w/4)` px wide, the right one drawn whole.
function queue(w: number, h: number): [string, number, Rect] {
  const p = px(w, h);
  const c = fmin(fmin(SIDE_CAP, h / 5.0), w / 4.0);
  const rad = `${n(c / w)} 0.5`;
  const d = `M ${p(c, 0.0)} L ${p(w - c, 0.0)} A ${rad} 0 0 1 ${p(w - c, h)} L ${p(c, h)} A ${rad} 0 0 1 ${p(c, 0.0)} Z M ${p(w - c, 0.0)} A ${rad} 0 0 0 ${p(w - c, h)}`;
  return [d, 1, unitRect(w, h, new Rect(c, 0.0, w - 3.0 * c, h))];
}

/// Every built-in shape, in the order docs and `kashika icons` list them.
export const SHAPES: readonly BuiltinShape[] = [
  { name: "rect", keywords: ["box", "rectangle", "square"], source: null, hiddenOutline: false },
  { name: "rounded", keywords: ["process", "step"], source: null, hiddenOutline: false },
  {
    name: "cylinder",
    keywords: ["db", "database", "storage", "datastore", "sql"],
    source: cylinder,
    hiddenOutline: false,
  },
  { name: "diamond", keywords: ["decision", "condition"], source: diamond, hiddenOutline: false },
  { name: "ellipse", keywords: ["circle", "oval", "state"], source: ellipse, hiddenOutline: false },
  { name: "hexagon", keywords: ["service", "component"], source: hexagon, hiddenOutline: false },
  { name: "doc", keywords: ["document", "file", "page"], source: doc, hiddenOutline: false },
  { name: "cloud", keywords: ["internet", "saas", "external"], source: cloud, hiddenOutline: false },
  {
    name: "person",
    keywords: ["user", "users", "people", "human", "account", "actor"],
    source: person,
    hiddenOutline: true,
  },
  {
    name: "parallelogram",
    keywords: ["io", "input", "output", "data"],
    source: parallelogram,
    hiddenOutline: false,
  },
  {
    name: "predefined-process",
    keywords: ["subroutine", "subprocess", "procedure", "function", "call"],
    source: predefinedProcess,
    hiddenOutline: false,
  },
  {
    name: "stadium",
    keywords: ["terminator", "terminal", "start", "end", "pill", "capsule"],
    source: stadium,
    hiddenOutline: false,
  },
  {
    name: "triangle",
    keywords: ["warning", "delta", "pyramid"],
    source: triangle,
    hiddenOutline: false,
  },
  { name: "octagon", keywords: ["stop", "halt", "block"], source: octagon, hiddenOutline: false },
  {
    name: "note",
    keywords: ["comment", "annotation", "memo", "sticky"],
    source: note,
    hiddenOutline: false,
  },
  {
    name: "package",
    keywords: ["folder", "module", "namespace"],
    source: pkg,
    hiddenOutline: false,
  },
  {
    name: "queue",
    keywords: ["fifo", "mq", "buffer", "pipe", "topic"],
    source: queue,
    hiddenOutline: false,
  },
];

/// The built-in shape called `name`.
export function builtinShape(name: string): BuiltinShape | null {
  return SHAPES.find((s) => s.name === name) ?? null;
}

/// Every built-in shape name, in `SHAPES` order.
export function shapeNames(): string[] {
  return SHAPES.map((s) => s.name);
}

/// The unit-path definition of a built-in shape for a `w`×`h` box: path data, outline
/// subpath (1-based) and unit text area. `null` for `rect`/`rounded` and unknown names.
export function builtinSource(name: string, w: number, h: number): [string, number, Rect] | null {
  const s = builtinShape(name);
  if (s === null || s.source === null) return null;
  return s.source(w, h);
}

export function builtin(name: string, w: number, h: number): UnitShape | null {
  const shape = builtinShape(name);
  if (shape === null || shape.source === null) return null;
  const [d, outline, textArea] = shape.source(w, h);
  const parsed = parseUnitPath(d);
  if (!parsed.ok) throw new Error("built-in shapes are valid unit paths");
  return {
    subpaths: parsed.value,
    outline: outline - 1,
    hiddenOutline: shape.hiddenOutline,
    textArea,
  };
}

/// The unit text area of `shape` on a `w`×`h` box; `null` for `rect`/`rounded`.
export function unitTextArea(shape: string, w: number, h: number): Rect | null {
  const s = builtinSource(shape, w, h);
  return s === null ? null : s[2];
}

/// The smallest box size in whole pixels, at least `min`, whose unit text area holds
/// `need` (width, height). `custom` is the unit text area of a custom shape. Each axis is found by
/// bisection on the shape's own text area, so no shape needs a hand-written inverse.
export function requiredSize(
  shape: string,
  custom: Rect | null,
  need: readonly [number, number],
  min: readonly [number, number],
): [number, number] {
  const area = (w: number, h: number): [number, number] => {
    const u = custom ?? unitTextArea(shape, w, h) ?? new Rect(0.0, 0.0, 1.0, 1.0);
    return [u.w * w, u.h * h];
  };
  let w = fmax(min[0], need[0]);
  let h = fmax(min[1], need[1]);
  for (let i = 0; i < 64; i++) {
    const h2 = least(h, (h) => area(w, h)[1] >= need[1] - 1e-9);
    const w2 = least(w, (w) => area(w, h2)[0] >= need[0] - 1e-9);
    const done = w2 === w && h2 === h;
    [w, h] = [w2, h2];
    if (done) {
      break;
    }
  }
  return [w, h];
}

/// The least whole number `x >= lo` for which `ok` holds, assuming `ok` is monotonic.
function least(lo: number, ok: (x: number) => boolean): number {
  let a = Math.ceil(lo);
  if (ok(a)) {
    return a;
  }
  let b = fmax(a, 1.0) * 2.0;
  while (!ok(b)) {
    if (b > 1e9) {
      return b;
    }
    a = b;
    b *= 2.0;
  }
  while (b - a > 1.0) {
    const m = Math.floor((a + b) / 2.0);
    if (ok(m)) {
      b = m;
    } else {
      a = m;
    }
  }
  return b;
}
