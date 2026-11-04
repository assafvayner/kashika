import { fmax } from "../diag/rust.js";
import {
  type PathSeg,
  type Pt,
  endDir,
  fmtNum as n,
  segFrom,
  segTo,
  segTrim,
  startDir,
} from "../geom/index.js";
import {
  type ErParts,
  type Label,
  LabelSide,
  type Line,
  Marker,
  type Scene,
  type Shape,
  type Style,
  type Text,
} from "../model/index.js";
import { Defs, body } from "./shapes.js";
import { lines } from "./text.js";

type Overlay = { kind: "label"; line: Line; label: Label } | { kind: "text"; text: Text };

export function renderSvg(scene: Scene): string {
  const [w, h] = scene.size;
  const defs = new Defs(scene, ID_HASH);
  let out = `<svg xmlns="http://www.w3.org/2000/svg" width="${n(w)}" height="${n(h)}" viewBox="0 0 ${n(w)} ${n(h)}">\n`;
  out += defs.write();
  if (scene.background !== null) {
    out += `<rect width="100%" height="100%" fill="${scene.background}"/>\n`;
  }
  const order = scene.paintOrder();
  const over = order.filter((i) => scene.shapes[i]!.layer === "over");
  const before = order.filter((i) => scene.shapes[i]!.layer !== "over");
  for (const i of before) {
    out += shape(scene.shapes[i]!, defs);
  }
  for (const j of scene.junctions.filter((j) => j.dot)) {
    out += `<circle cx="${n(j.at.x)}" cy="${n(j.at.y)}" r="3" fill="#334155"/>\n`;
  }
  scene.lines.forEach((l, i) => {
    out += line(l, i, scene.size, defs.id());
  });
  const overlays: [[number, number], Overlay][] = [];
  for (const l of scene.lines) {
    for (const lab of l.labels) {
      overlays.push([[l.span.line, l.span.col], { kind: "label", line: l, label: lab }]);
    }
  }
  for (const t of scene.texts) {
    overlays.push([[t.span.line, t.span.col], { kind: "text", text: t }]);
  }
  overlays.sort(([a], [b]) => (a[0] !== b[0] ? a[0] - b[0] : a[1] - b[1]));
  for (const [, o] of overlays) {
    if (o.kind === "label") {
      out += lines(o.label.lines, o.line.style.fontSize, o.line.style.color);
    } else {
      out += lines(o.text.lines, o.text.size, o.text.color);
    }
  }
  for (const i of over) {
    out += shape(scene.shapes[i]!, defs);
  }
  out += "</svg>\n";
  const hash = fnv1a32(new TextEncoder().encode(out)).toString(16).padStart(8, "0");
  return out.split(ID_HASH).join(hash);
}

/// Stands in for the per-diagram id hash until the document is complete. `esc` drops
/// control characters, so no user text can contain it.
const ID_HASH = "\u0001ID\u0001";

/// FNV-1a over the finished document, whose numbers are already rounded: ids stay
/// deterministic across platforms, and differ between diagrams so several SVGs can be
/// inlined in one HTML page.
export function fnv1a32(bytes: Uint8Array): number {
  let h = 0x811c9dc5;
  for (const b of bytes) {
    h = Math.imul(h ^ b, 0x01000193) >>> 0;
  }
  return h;
}

/// Escapes `s` for an attribute value, dropping characters XML 1.0 forbids.
export function esc(s: string): string {
  let out = "";
  for (const c of s) {
    const cp = c.codePointAt(0)!;
    if (c === "&") out += "&amp;";
    else if (c === "<") out += "&lt;";
    else if (c === ">") out += "&gt;";
    else if (c === '"') out += "&quot;";
    else if (c === "\t" || c === "\n" || c === "\r") out += c;
    else if (cp <= 0x1f || cp === 0xfffe || cp === 0xffff) continue;
    else out += c;
  }
  return out;
}

export function strokeAttrs(style: Style): string {
  let s = `stroke="${style.stroke}" stroke-width="${n(style.strokeWidth)}"`;
  switch (style.strokeStyle) {
    case "solid":
      break;
    case "dashed":
      s += ' stroke-dasharray="6 4"';
      break;
    case "dotted":
      s += ' stroke-dasharray="1.5 3" stroke-linecap="round"';
      break;
  }
  return s;
}

function shape(s: Shape, defs: Defs): string {
  let out = body(s, defs);
  for (const item of s.text) {
    out += lines(item.lines, item.size, item.color);
  }
  return out;
}

/// Shortens the end segments so the stroke stops at each marker's base.
export function trim(path: PathSeg[], start: Marker, end: Marker, strokeWidth: number): void {
  const [a, b] = [Marker.length(start, strokeWidth), Marker.length(end, strokeWidth)];
  if (path.length === 0) return;
  if (path.length === 1) {
    path[0] = segTrim(path[0]!, a, b);
    return;
  }
  const last = path.length - 1;
  path[0] = segTrim(path[0]!, a, 0.0);
  path[last] = segTrim(path[last]!, 0.0, b);
}

export function pathData(path: readonly PathSeg[]): string {
  let d = "";
  const first = path[0];
  if (first !== undefined) {
    const p = segFrom(first);
    d += `M ${n(p.x)} ${n(p.y)}`;
  }
  for (const seg of path) {
    switch (seg.kind) {
      case "line":
        d += ` L ${n(seg.to.x)} ${n(seg.to.y)}`;
        break;
      case "quad":
        d += ` Q ${n(seg.ctrl.x)} ${n(seg.ctrl.y)} ${n(seg.to.x)} ${n(seg.to.y)}`;
        break;
      case "cubic":
        d += ` C ${n(seg.c1.x)} ${n(seg.c1.y)} ${n(seg.c2.x)} ${n(seg.c2.y)} ${n(seg.to.x)} ${n(seg.to.y)}`;
        break;
    }
  }
  return d;
}

/// Each marker of `l` with its tip and the direction it points: the end first, then the start.
export function ends(l: Line): [Marker, Pt, Pt, ErParts][] {
  const first = l.path[0];
  const last = l.path[l.path.length - 1];
  if (first === undefined || last === undefined) {
    return [];
  }
  return [
    [l.end, segTo(last), endDir(last), l.erParts(false)],
    [l.start, segFrom(first), startDir(first).mul(-1.0), l.erParts(true)],
  ];
}

/// Opens a group masked so the line and its markers are cut out under its own labels that
/// sit on it (not beside it), inside its hollow ends and inside the open circles of its
/// `zero-*` ends, leaving whatever is beneath visible.
/// Returns the markup and whether a group was opened.
export function lineMask(
  l: Line,
  i: number,
  [w, h]: readonly [number, number],
  id: string,
): [string, boolean] {
  const sw = l.style.strokeWidth;
  const hollow: [Marker, Pt, Pt][] = ends(l)
    .filter(([m]) => Marker.isHollow(m))
    .map(([m, tip, dir]) => [m, tip, dir]);
  const circles: [Pt, number][] = ends(l).flatMap(([, , , parts]) =>
    parts.circle !== null ? [parts.circle] : [],
  );
  const masked = l.labels.filter((lab) => LabelSide.isOn(lab.side));
  if (masked.length === 0 && circles.length === 0 && hollow.length === 0) {
    return ["", false];
  }
  const maskId = `kashika-${id}-mask-${i}`;
  let out = `<mask id="${maskId}" maskUnits="userSpaceOnUse" x="0" y="0" width="${n(w)}" height="${n(h)}"><rect width="${n(w)}" height="${n(h)}" fill="#fff"/>`;
  for (const [m, tip, dir] of hollow) {
    const round = Marker.round(m, tip, dir, sw);
    if (round !== null) {
      const [c, r] = round;
      out += `<circle cx="${n(c.x)}" cy="${n(c.y)}" r="${n(fmax(r - sw / 2.0, 0.0))}" fill="#000"/>`;
    } else {
      out += `<polygon points="${points(Marker.outline(m, tip, dir, sw))}" fill="#000" stroke="#fff" stroke-width="${n(sw)}" stroke-linejoin="round"/>`;
    }
  }
  for (const lab of masked) {
    out += `<rect x="${n(lab.rect.x)}" y="${n(lab.rect.y)}" width="${n(lab.rect.w)}" height="${n(lab.rect.h)}" fill="#000"/>`;
  }
  for (const [c, r] of circles) {
    out += `<circle cx="${n(c.x)}" cy="${n(c.y)}" r="${n(fmax(r - sw / 2.0, 0.0))}" fill="#000"/>`;
  }
  out += `</mask>\n<g mask="url(#${maskId})">\n`;
  return [out, true];
}

export function line(l: Line, i: number, size: readonly [number, number], id: string): string {
  const [mask, masked] = lineMask(l, i, size, id);
  let out = mask;
  const path = [...l.path];
  trim(path, l.start, l.end, l.style.strokeWidth);
  out += `<path d="${pathData(path)}" fill="none" stroke-linejoin="round" ${strokeAttrs(l.style)}/>\n`;
  for (const [m, tip, dir, parts] of ends(l)) {
    out += marker(m, tip, dir, parts, l.style);
  }
  if (masked) {
    out += "</g>\n";
  }
  return out;
}

/// Draws a marker whose tip sits on `tip`, pointing along `dir`.
export function marker(m: Marker, tip: Pt, dir: Pt, parts: ErParts, style: Style): string {
  const outline = Marker.outline(m, tip, dir, style.strokeWidth);
  switch (m) {
    case "none":
      return "";
    case "arrow":
    case "diamond":
      return `<polygon points="${points(outline)}" fill="${style.stroke}"/>\n`;
    case "open":
      return `<polyline points="${points([outline[1]!, outline[0]!, outline[2]!])}" fill="none" stroke="${style.stroke}" stroke-width="${n(style.strokeWidth)}" stroke-linejoin="round"/>\n`;
    case "triangle":
    case "hollow-diamond":
      return `<polygon points="${points(outline)}" fill="none" stroke="${style.stroke}" stroke-width="${n(style.strokeWidth)}" stroke-linejoin="round"/>\n`;
    case "dot":
    case "circle": {
      const round = Marker.round(m, tip, dir, style.strokeWidth);
      if (round === null) {
        return "";
      }
      const [c, r] = round;
      const paint =
        m === "dot"
          ? `fill="${style.stroke}"`
          : `fill="none" stroke="${style.stroke}" stroke-width="${n(style.strokeWidth)}"`;
      return `<circle cx="${n(c.x)}" cy="${n(c.y)}" r="${n(r)}" ${paint}/>\n`;
    }
    case "one":
    case "many":
    case "one-many":
    case "zero-one":
    case "zero-many": {
      const d = parts.strokes
        .map(([a, b]) => `M ${n(a.x)} ${n(a.y)} L ${n(b.x)} ${n(b.y)}`)
        .join(" ");
      let out = `<path d="${d}" fill="none" stroke="${style.stroke}" stroke-width="${n(style.strokeWidth)}" stroke-linecap="round"/>\n`;
      if (parts.circle !== null) {
        const [c, r] = parts.circle;
        out += `<circle cx="${n(c.x)}" cy="${n(c.y)}" r="${n(r)}" fill="none" stroke="${style.stroke}" stroke-width="${n(style.strokeWidth)}"/>\n`;
      }
      return out;
    }
  }
}

export function points(ps: readonly Pt[]): string {
  return ps.map((p) => `${n(p.x)},${n(p.y)}`).join(" ");
}
