import { type Diagnostic, type Pair, type Span, fact } from "../diag/index.js";
import { type PathSeg, Pt, Rect, fmtNum as n, line, pointAt } from "../geom/index.js";
import { ANCHORS, Run, type Scene } from "../model/index.js";
import { outline } from "../text/index.js";
import { esc, pathData, renderSvg } from "./svg.js";

const ERROR = "#dc2626";
const WARN = "#ea580c";
const GUIDE = "#2563eb";
const RULER = "#94a3b8";

/// What to draw over the diagram. The default draws nothing, leaving the SVG unchanged.
export interface Overlay {
  /// Red markers at each error (`render --force`).
  errorMarkers: boolean;
  /// Rulers, ids, anchor dots and markers for every diagnostic (`render --debug`).
  debug: boolean;
}

export const Overlay = {
  default(): Overlay {
    return { errorMarkers: false, debug: false };
  },
};

export function renderSvgWith(scene: Scene, diags: Iterable<Diagnostic>, o: Overlay): string {
  let svg = renderSvg(scene);
  if (!o.errorMarkers && !o.debug) {
    return svg;
  }
  const end = svg.lastIndexOf("</svg>");
  if (end < 0) throw new Error("render_svg closes the root element");
  svg = svg.slice(0, end);
  svg += '<g id="kashika-overlay">\n';
  const halo = scene.background ?? "#ffffff";
  if (o.debug) {
    svg += rulers(scene.size, halo);
    svg += guides(scene, halo);
  }
  let unplaced = 0;
  for (const d of diags) {
    if (!(o.debug || d.severity === "error")) continue;
    const m = locate(scene, d);
    if (m !== null) {
      svg += mark(d, m, halo);
    } else {
      unplaced += 1;
      const text = `${d.span.line}:${d.span.col} ${tag(d)}`;
      svg += label(text, new Pt(4.0, 12.0 * unplaced), color(d), halo);
    }
  }
  svg += "</g>\n</svg>\n";
  return svg;
}

export type Mark =
  | { kind: "point"; p: Pt }
  | { kind: "rect"; r: Rect }
  | { kind: "path"; path: PathSeg[] };

function color(d: Diagnostic): string {
  switch (d.severity) {
    case "error":
      return ERROR;
    case "warn":
      return WARN;
  }
}

function tag(d: Diagnostic): string {
  const sev = d.severity === "error" ? "error" : "warn";
  return `${sev}[${d.code}]`;
}

export function locate(scene: Scene, d: Diagnostic): Mark | null {
  return carried(d) ?? fromSpan(scene, d.span);
}

/// Geometry the diagnostic carries itself: a point fact, else its first element's segment or
/// bounding box.
export function carried(d: Diagnostic): Mark | null {
  const pt = ([x, y]: Pair): Pt | null =>
    Number.isFinite(x) && Number.isFinite(y) ? new Pt(x, y) : null;
  for (const key of [fact.AT, fact.ENTRY]) {
    const p = d.point(key);
    if (p !== null) {
      return { kind: "point", p: new Pt(p[0], p[1]) };
    }
  }
  const e = d.elements[0];
  if (e === undefined) return null;
  const from = e.from === null ? null : pt(e.from);
  const to = e.to === null ? null : pt(e.to);
  if (from !== null && to !== null) {
    return { kind: "path", path: [line(from, to)] };
  }
  if (e.bbox === null) return null;
  const [pa, pb] = e.bbox;
  const a = pt(pa);
  if (a === null) return null;
  const b = pt(pb);
  if (b === null) return null;
  return { kind: "rect", r: new Rect(a.x, a.y, b.x - a.x, b.y - a.y) };
}

function before(a: [number, number], b: [number, number]): boolean {
  return a[0] < b[0] || (a[0] === b[0] && a[1] <= b[1]);
}

export function contains(outer: Span, inner: Span): boolean {
  return (
    before([outer.line, outer.col], [inner.line, inner.col]) &&
    !before([outer.endLine, outer.endCol], [inner.line, inner.col])
  );
}

/// The innermost scene element whose statement contains the diagnostic's start.
export function fromSpan(scene: Scene, span: Span): Mark | null {
  let best: [Span, Mark] | null = null;
  const pick = (s: Span, m: Mark) => {
    const later =
      best !== null
        ? s.line > best[0].line || (s.line === best[0].line && s.col > best[0].col)
        : true;
    if (contains(s, span) && later) {
      best = [s, m];
    }
  };
  for (const s of scene.shapes) {
    pick(s.span, { kind: "rect", r: s.rect });
  }
  for (const l of scene.lines) {
    pick(l.span, { kind: "path", path: [...l.path] });
  }
  for (const j of scene.junctions) {
    pick(j.span, { kind: "point", p: j.at });
  }
  for (const t of scene.texts) {
    pick(t.span, { kind: "point", p: t.at });
  }
  return (best as [Span, Mark] | null)?.[1] ?? null;
}

export function mark(d: Diagnostic, m: Mark, halo: string): string {
  const c = color(d);
  let out = "";
  let at: Pt;
  switch (m.kind) {
    case "point":
      out += `<circle cx="${n(m.p.x)}" cy="${n(m.p.y)}" r="6" fill="none" stroke="${c}" stroke-width="2"/>\n`;
      at = m.p.add(new Pt(8.0, -8.0));
      break;
    case "rect": {
      const r = m.r;
      out += `<rect x="${n(r.x - 3.0)}" y="${n(r.y - 3.0)}" width="${n(r.w + 6.0)}" height="${n(r.h + 6.0)}" fill="none" stroke="${c}" stroke-width="2" stroke-dasharray="4 3"/>\n`;
      at = new Pt(r.x - 3.0, r.y - 6.0);
      break;
    }
    case "path":
      out += `<path d="${pathData(m.path)}" fill="none" stroke="${c}" stroke-opacity="0.5" stroke-width="6"/>\n`;
      at = pointAt(m.path, 0.5).add(new Pt(6.0, -6.0));
      break;
  }
  out += label(tag(d), at, c, halo);
  return out;
}

/// Draws `text` as glyph outlines, so the overlay needs no fonts either, with a halo in the
/// canvas color.
export function label(text: string, at: Pt, color: string, halo: string): string {
  const runs = [new Run(text, "mono")];
  const d = outline(runs, 10.0, at.x, at.y);
  if (d === "") {
    return "";
  }
  return `<path aria-label="${esc(text)}" d="${d}" fill="${color}" stroke="${halo}" stroke-width="3" stroke-linejoin="round" paint-order="stroke"/>\n`;
}

export function tick(v: number): number {
  if (v % 100.0 === 0.0) {
    return 10.0;
  } else if (v % 50.0 === 0.0) {
    return 6.0;
  } else {
    return 3.0;
  }
}

export function rulers([w, h]: readonly [number, number], halo: string): string {
  const d: string[] = [];
  for (let i = 0; i <= Math.trunc(w / 10.0); i++) {
    const x = i * 10.0;
    d.push(`M ${n(x)} 0 V ${n(tick(x))}`);
  }
  for (let i = 0; i <= Math.trunc(h / 10.0); i++) {
    const y = i * 10.0;
    d.push(`M 0 ${n(y)} H ${n(tick(y))}`);
  }
  let out = `<path d="${d.join(" ")}" fill="none" stroke="${RULER}" stroke-width="1"/>\n`;
  for (let i = 1; i <= Math.trunc(w / 100.0); i++) {
    const x = i * 100.0;
    out += label(n(x), new Pt(x + 2.0, 20.0), RULER, halo);
  }
  for (let i = 1; i <= Math.trunc(h / 100.0); i++) {
    const y = i * 100.0;
    out += label(n(y), new Pt(12.0, y - 2.0), RULER, halo);
  }
  return out;
}

function dot(p: Pt): string {
  return `<circle cx="${n(p.x)}" cy="${n(p.y)}" r="1.5" fill="${GUIDE}"/>\n`;
}

export function guides(scene: Scene, halo: string): string {
  let out = "";
  for (const s of scene.shapes) {
    out += label(s.id, new Pt(s.rect.x, s.rect.y - 3.0), GUIDE, halo);
    for (const a of ANCHORS) {
      const p = s.anchor(a, null);
      if (p !== null) {
        out += dot(p);
      }
    }
  }
  for (const l of scene.lines) {
    if (l.path.length > 0) {
      out += label(l.id, pointAt(l.path, 0.5).add(new Pt(4.0, 12.0)), GUIDE, halo);
    }
  }
  for (const j of scene.junctions) {
    out += dot(j.at);
    out += label(j.id, j.at.add(new Pt(4.0, -4.0)), GUIDE, halo);
  }
  return out;
}
