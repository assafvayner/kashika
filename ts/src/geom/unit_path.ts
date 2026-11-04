import {
  asUsize,
  chars,
  displayF64,
  err,
  fmax,
  isAsciiAlphabetic,
  isAsciiDigit,
  isWhitespace,
  ok,
  parseF64,
  type Result,
  toRadians,
} from "../diag/rust.js";
import { cubic, line, type PathSeg, quad } from "./path.js";
import { Pt } from "./point.js";

const TAU = 2 * Math.PI;
const FRAC_PI_2 = Math.PI / 2;

type PTok = { kind: "cmd"; c: string } | { kind: "num"; n: number };

function tokens(d: string): Result<PTok[], string> {
  const cs = chars(d);
  const out: PTok[] = [];
  let i = 0;
  while (i < cs.length) {
    const c = cs[i]!;
    if (isWhitespace(c) || c === ",") {
      i += 1;
      continue;
    }
    if (isAsciiAlphabetic(c)) {
      out.push({ kind: "cmd", c });
      i += 1;
      continue;
    }
    const start = i;
    if (c === "-" || c === "+") {
      i += 1;
    }
    while (i < cs.length && (isAsciiDigit(cs[i]!) || cs[i] === ".")) {
      i += 1;
    }
    if (i > start && (cs[i] === "e" || cs[i] === "E")) {
      let j = i + 1;
      if (cs[j] === "-" || cs[j] === "+") {
        j += 1;
      }
      if (j < cs.length && isAsciiDigit(cs[j]!)) {
        while (j < cs.length && isAsciiDigit(cs[j]!)) {
          j += 1;
        }
        i = j;
      }
    }
    const s = cs.slice(start, Math.max(i, start + 1)).join("");
    const n = parseF64(s);
    if (n === null) return err(`bad number \`${s}\` in path`);
    out.push({ kind: "num", n });
    i = Math.max(i, start + 1);
  }
  return ok(out);
}

function take(toks: PTok[], i: { v: number }, n: number, cmd: string): Result<number[], string> {
  const v: number[] = [];
  for (let k = 0; k < n; k++) {
    const t = toks[i.v];
    if (t !== undefined && t.kind === "num") {
      v.push(t.n);
      i.v += 1;
    } else {
      return err(`\`${cmd}\` expects ${n} numbers`);
    }
  }
  return ok(v);
}

function unit(x: number, y: number): Result<Pt, string> {
  const inRange = (v: number) => v >= -1e-9 && v <= 1 + 1e-9;
  if (inRange(x) && inRange(y)) {
    return ok(new Pt(x, y));
  } else {
    return err(
      `path coordinates must be between 0 and 1, found (${displayF64(x)}, ${displayF64(y)})`,
    );
  }
}

function flag(v: number): Result<boolean, string> {
  if (v === 0) {
    return ok(false);
  } else if (v === 1) {
    return ok(true);
  } else {
    return err(`arc flags must be 0 or 1, found ${displayF64(v)}`);
  }
}

class Bail {
  constructor(readonly error: string) {}
}

function must<T>(r: Result<T, string>): T {
  if (!r.ok) throw new Bail(r.error);
  return r.value;
}

/// Parses a unit-coordinate path (`M L H V C Q A Z`, absolute only) into subpaths.
/// Subpaths with no segments (a lone `M`) are dropped, and a path with none left is an
/// error. Arcs become cubic Béziers; `Z` adds the closing line when the subpath isn't closed.
export function parseUnitPath(d: string): Result<PathSeg[][], string> {
  try {
    return ok(parse(d));
  } catch (e) {
    if (e instanceof Bail) return err(e.error);
    throw e;
  }
}

function parse(d: string): PathSeg[][] {
  const toks = must(tokens(d));
  let subs: PathSeg[][] = [];
  let cur: Pt | null = null;
  let start = Pt.default();
  let cmd: string | null = null;
  const i = { v: 0 };
  while (i.v < toks.length) {
    const tok = toks[i.v]!;
    if (tok.kind === "cmd") {
      const c = tok.c;
      i.v += 1;
      if ("mlhvcqaz".includes(c)) {
        throw new Bail(`relative command \`${c}\` is not supported; use \`${c.toUpperCase()}\``);
      }
      if (!"MLHVCQAZ".includes(c)) {
        throw new Bail(`unknown path command \`${c}\`; use M L H V C Q A Z`);
      }
      if (c !== "M" && cur === null) {
        throw new Bail("a path must start with `M`");
      }
      if (c === "Z") {
        const p: Pt = cur!;
        if (p.dist(start) > 1e-12) {
          subs[subs.length - 1]!.push(line(p, start));
        }
        cur = start;
        cmd = null;
        continue;
      }
      cmd = c;
    }
    if (cmd === null) {
      throw new Bail(
        cur === null ? "a path must start with `M`" : "numbers must follow a command",
      );
    }
    const c: string = cmd;
    if (c === "M") {
      const a = must(take(toks, i, 2, c));
      const p = must(unit(a[0]!, a[1]!));
      subs.push([]);
      cur = p;
      start = p;
      cmd = "L";
      continue;
    }
    const from: Pt = cur!;
    let segs: PathSeg[];
    switch (c) {
      case "L": {
        const a = must(take(toks, i, 2, c));
        segs = [line(from, must(unit(a[0]!, a[1]!)))];
        break;
      }
      case "H": {
        const a = must(take(toks, i, 1, c));
        segs = [line(from, must(unit(a[0]!, from.y)))];
        break;
      }
      case "V": {
        const a = must(take(toks, i, 1, c));
        segs = [line(from, must(unit(from.x, a[0]!)))];
        break;
      }
      case "C": {
        const a = must(take(toks, i, 6, c));
        const c1 = must(unit(a[0]!, a[1]!));
        const c2 = must(unit(a[2]!, a[3]!));
        const to = must(unit(a[4]!, a[5]!));
        segs = [cubic(from, c1, c2, to)];
        break;
      }
      case "Q": {
        const a = must(take(toks, i, 4, c));
        const ctrl = must(unit(a[0]!, a[1]!));
        const to = must(unit(a[2]!, a[3]!));
        segs = [quad(from, ctrl, to)];
        break;
      }
      case "A": {
        const a = must(take(toks, i, 7, c));
        const to = must(unit(a[5]!, a[6]!));
        const large = must(flag(a[3]!));
        const sweep = must(flag(a[4]!));
        segs = arcToCubics(from, a[0]!, a[1]!, a[2]!, large, sweep, to);
        break;
      }
      default:
        throw new Error("commands are validated above");
    }
    const last = segs[segs.length - 1];
    if (last !== undefined) {
      cur = last.to;
    }
    subs[subs.length - 1]!.push(...segs);
  }
  subs = subs.filter((s) => s.length > 0);
  if (subs.length === 0) {
    throw new Bail("empty path");
  }
  return subs;
}

/// Converts an SVG elliptical arc (endpoint form, SVG 1.1 F.6.5) to cubic Béziers of at
/// most 90° each. Radii that are too small are scaled up, as SVG requires.
export function arcToCubics(
  from: Pt,
  rx: number,
  ry: number,
  phiDeg: number,
  large: boolean,
  sweep: boolean,
  to: Pt,
): PathSeg[] {
  if (from.dist(to) < 1e-12) {
    return [];
  }
  rx = Math.abs(rx);
  ry = Math.abs(ry);
  if (rx < 1e-12 || ry < 1e-12) {
    return [line(from, to)];
  }
  const phi = toRadians(phiDeg);
  const sin = Math.sin(phi);
  const cos = Math.cos(phi);
  const dx = (from.x - to.x) / 2;
  const dy = (from.y - to.y) / 2;
  const x1p = cos * dx + sin * dy;
  const y1p = -sin * dx + cos * dy;
  const lambda = (x1p * x1p) / (rx * rx) + (y1p * y1p) / (ry * ry);
  if (lambda > 1) {
    const s = Math.sqrt(lambda);
    rx *= s;
    ry *= s;
  }
  const num = rx * rx * ry * ry - rx * rx * y1p * y1p - ry * ry * x1p * x1p;
  const den = rx * rx * y1p * y1p + ry * ry * x1p * x1p;
  let coef = Math.sqrt(fmax(num / den, 0));
  if (large === sweep) {
    coef = -coef;
  }
  const cxp = (coef * rx * y1p) / ry;
  const cyp = (-coef * ry * x1p) / rx;
  const cx = cos * cxp - sin * cyp + (from.x + to.x) / 2;
  const cy = sin * cxp + cos * cyp + (from.y + to.y) / 2;
  const angle = (ux: number, uy: number, vx: number, vy: number) =>
    Math.atan2(ux * vy - uy * vx, ux * vx + uy * vy);
  const ux = (x1p - cxp) / rx;
  const uy = (y1p - cyp) / ry;
  const vx = (-x1p - cxp) / rx;
  const vy = (-y1p - cyp) / ry;
  const theta1 = angle(1, 0, ux, uy);
  let dtheta = angle(ux, uy, vx, vy);
  if (!sweep && dtheta > 0) {
    dtheta -= TAU;
  } else if (sweep && dtheta < 0) {
    dtheta += TAU;
  }
  const n = Math.max(asUsize(Math.ceil(Math.abs(dtheta) / FRAC_PI_2)), 1);
  const step = dtheta / n;
  const k = (4 / 3) * Math.tan(step / 4);
  const at = (t: number): [Pt, Pt] => {
    const st = Math.sin(t);
    const ct = Math.cos(t);
    const p = new Pt(cx + rx * ct * cos - ry * st * sin, cy + rx * ct * sin + ry * st * cos);
    const d = new Pt(-rx * st * cos - ry * ct * sin, -rx * st * sin + ry * ct * cos);
    return [p, d];
  };
  const segs: PathSeg[] = [];
  let prev = from;
  for (let i = 0; i < n; i++) {
    const t1 = theta1 + step * i;
    const [, d1] = at(t1);
    const [p2, d2] = at(t1 + step);
    const end = i === n - 1 ? to : p2;
    segs.push(cubic(prev, prev.add(d1.mul(k)), end.sub(d2.mul(k)), end));
    prev = end;
  }
  return segs;
}
