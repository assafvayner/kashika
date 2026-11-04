import { Diagnostic, Element, type Span } from "../../diag/diagnostic.js";
import { Float } from "../../diag/json.js";
import { fmax, round } from "../../diag/rust.js";
import { fact } from "../../diag/codes.js";
import {
  type Pt,
  Rect,
  endDir,
  fmtNum,
  insideRuns,
  pointAt,
  polygonsOverlap,
  rectPolygon,
  segFrom,
  segTo,
  startDir,
} from "../../geom/index.js";
import {
  type Label,
  LabelSide,
  type Line,
  Marker,
  ShapeKind,
  type Text,
  type TitlePos,
} from "../../model/index.js";
import { type Ctx } from "../ctx.js";
import { arrFact, bbox, bboxFact, lineEl, shapeEl, textEl } from "../element.js";
import { type Sink } from "../sink.js";
import { Grid } from "../spatial.js";
import {
  bandAt,
  belowBox,
  labelBox,
  labelBoxAt,
  textBox,
  titleBand,
  titleBox,
} from "../text_box.js";
import { debugStr } from "../../syntax/index.js";

interface Subject {
  el: Element;
  span: Span;
  what: string;
  rect: Rect;
  /// For a line label: its line (index into `ctx.lines`) and the label.
  label: [number, Label] | null;
  /// For the label under a `text-area=below` shape: that shape's id.
  shape: string | null;
  /// For a free `text`: the text.
  text: Text | null;
}

/// `label-collision`: a line label, text or the label under a `text-area=below` shape
/// intersects a box, an image, a group title or border, another label or text, a line other
/// than its own, or (for a line label) its own line's markers; or a line crosses a group title.
export function labelCollision(ctx: Ctx, out: Sink): void {
  const subs = subjects(ctx);
  const grid = Grid.new(subs.map((s) => s.rect));
  subs.forEach((s, i) => {
    const hits = obstacles(
      ctx,
      s.rect,
      s.label === null ? null : [s.label[0], !LabelSide.isOn(s.label[1].side)],
      s.shape,
    );
    for (const j of grid.near(s.rect, 1.0)) {
      if (j >= i) break;
      const o = subs[j]!;
      if (s.rect.intersect(o.rect) !== null) {
        hits.push([o.what, o.el]);
      }
    }
    if (hits.length === 0) {
      return;
    }
    const shapeHit = hits.some(([, el]) => el.kind === "box" || el.kind === "image");
    let clear: [number[], string] | null = null;
    if (s.label !== null) {
      const [li, label] = s.label;
      const ts = clearPositions(ctx, subs, grid, i, li, label);
      const help =
        ts.length === 0
          ? noClearPosition(ctx, ctx.lines[li]!.line, label, shapeHit)
          : describeClear(ctx.lines[li]!.line.id, ts);
      clear = [ts, help];
    }
    const intoBlock = s.text === null ? null : textInBox(ctx, s.text);
    for (const [other, el] of hits) {
      const d = Diagnostic.warn("label-collision", s.span, `${s.what} collides with ${other}`)
        .withElement(s.el)
        .withElement(el)
        .withFact("bbox", bboxFact(s.rect));
      if (clear !== null) {
        d.withFact(
          "clear_t",
          clear[0].map((t) => new Float(t)),
        ).withHelp(clear[1]);
      }
      if (intoBlock !== null && el.id === intoBlock[0]) {
        d.withHelp(intoBlock[1]);
      }
      const id = s.shape;
      if (id !== null && el.kind === "line" && leavesBottom(ctx, el.id, id)) {
        d.withHelp(
          `move the line to another side of '${id}', or give '${id}' an \`icon=\` with the text inside the box instead of below`,
        );
      }
      out.push(d);
    }
  });
  titlesCrossed(ctx, out);
}

/// Whether line `line` starts or ends on shape `shape`, nearer its bottom than its other sides.
function leavesBottom(ctx: Ctx, line: string, shape: string): boolean {
  const l = ctx.lines.find((l) => l.line.id === line);
  if (l === undefined) {
    return false;
  }
  const s = ctx.shapes.find((s) => s.shape.id === shape);
  if (s === undefined) {
    return false;
  }
  const r = s.shape.rect;
  return [l.start, l.end].some((p) => {
    const bottom = r.bottom() - p.y;
    return (
      ctx.attached(p).includes(shape) &&
      bottom < p.y - r.y &&
      bottom < p.x - r.x &&
      bottom < r.right() - p.x
    );
  });
}

/// Lines that cross a group's title text.
function titlesCrossed(ctx: Ctx, out: Sink): void {
  for (const g of ctx.groups()) {
    const t = titleBox(g.shape);
    if (t === null) {
      continue;
    }
    const poly = rectPolygon(t);
    for (const i of ctx.linesNear(t, 0.0)) {
      const l = ctx.lines[i]!;
      let hit: [Pt, number] | null = null;
      for (const [p, q, k] of l.pieces) {
        const r = insideRuns(p, q, poly)[0];
        if (r !== undefined) {
          hit = [r[0], k];
          break;
        }
      }
      if (hit === null) continue;
      const [entry, k] = hit;
      const seg = l.segment(k);
      const span = l.line.segments[seg]?.span ?? l.line.span;
      const band = titleBand(g.shape);
      const inside =
        g.shape.title === "top"
          ? `below y=${fmtNum(band === null ? t.bottom() : band.bottom())}`
          : `above y=${fmtNum(band === null ? t.y : band.y)}`;
      const side = entry.x < g.shape.rect.center().x ? "right" : "left";
      const other: TitlePos = g.shape.title === "top" ? "bottom" : "top";
      const otherBand = rectPolygon(bandAt(g.shape, other));
      const flip =
        band !== null && l.pieces.every(([p, q]) => insideRuns(p, q, otherBand).length === 0);
      const d = Diagnostic.warn(
        "label-collision",
        span,
        `${l.line.id} crosses the title of group '${g.shape.id}'`,
      )
        .withElement(lineEl(l.line, seg))
        .withElement(shapeEl(g.shape))
        .withFact("bbox", bboxFact(t))
        .withFact(fact.ENTRY, arrFact(entry))
        .withHelp(
          `the title of '${g.shape.id}' covers x ${fmtNum(t.x)}..${fmtNum(t.right())}, y ${fmtNum(t.y)}..${fmtNum(t.bottom())}; route ${l.line.id} around it, e.g. inside the group ${inside} or outside it`,
        )
        .withHelp(
          `or move the title away from the line: \`text-align=${side}\` on '${g.shape.id}'`,
        );
      if (flip) {
        d.withHelp(
          `or move the title to the ${other} edge: \`title=${other}\` on '${g.shape.id}'`,
        );
      }
      out.push(d);
    }
  }
}

function subjects(ctx: Ctx): Subject[] {
  const out: Subject[] = [];
  ctx.lines.forEach((l, li) => {
    for (const lab of l.line.labels) {
      const rect = labelBox(lab);
      const el = Element.new(l.line.id, "label", lab.span);
      el.bbox = bbox(rect);
      out.push({
        el,
        span: lab.span,
        what: `label "${lab.text}" of ${l.line.id}`,
        rect,
        label: [li, lab],
        shape: null,
        text: null,
      });
    }
  });
  for (const t of ctx.scene.texts) {
    out.push({
      el: textEl(t),
      span: t.span,
      what: `text '${t.displayId()}'`,
      rect: textBox(t),
      label: null,
      shape: null,
      text: t,
    });
  }
  for (const s of ctx.boxes()) {
    const rect = belowBox(s.shape);
    if (rect === null) {
      continue;
    }
    const el = Element.new(s.shape.id, "label", s.shape.span);
    el.bbox = bbox(rect);
    out.push({
      el,
      span: s.shape.span,
      what: `the label under ${ShapeKind.name(s.shape.kind)} '${s.shape.id}'`,
      rect,
      label: null,
      shape: s.shape.id,
      text: null,
    });
  }
  return out;
}

/// Boxes, images, group titles, lines (except shape `ownShape`, and line `own` unless its
/// label sits beside it) and the markers of line `own` that `rect` intersects. `own` is the
/// line's index and whether the label sits beside the line rather than on it.
function obstacles(
  ctx: Ctx,
  rect: Rect,
  own: [number, boolean] | null,
  ownShape: string | null,
): [string, Element][] {
  const poly = rectPolygon(rect);
  const out: [string, Element][] = [];
  for (const s of ctx.boxesNear(rect, 0.0)) {
    if (ownShape !== s.shape.id && polygonsOverlap(poly, s.poly)) {
      out.push([`${ShapeKind.name(s.shape.kind)} '${s.shape.id}'`, shapeEl(s.shape)]);
    }
  }
  for (const g of ctx.groupsNear(rect, 0.0)) {
    const t = titleBox(g.shape);
    if (t !== null && rect.intersect(t) !== null) {
      out.push([`the title of group '${g.shape.id}'`, shapeEl(g.shape)]);
    }
    if (rect.intersect(g.shape.rect) !== null && !g.shape.rect.containsRect(rect)) {
      out.push([`the border of group '${g.shape.id}'`, shapeEl(g.shape)]);
    }
  }
  for (const i of ctx.linesNear(rect, 0.0)) {
    const l = ctx.lines[i]!;
    const isOwnOn = own !== null && own[0] === i && !own[1];
    if (!isOwnOn && l.pieces.some(([p, q]) => insideRuns(p, q, poly).length > 0)) {
      out.push([`line ${l.line.id}`, lineEl(l.line, null)]);
    }
  }
  if (own !== null) {
    const l = ctx.lines[own[0]]!;
    for (const [end, name, marker] of markers(l.line)) {
      if (polygonsOverlap(poly, marker)) {
        const seg = end === "start" ? 0 : l.lastSegment();
        out.push([`the ${end} ${name} of ${l.line.id}`, lineEl(l.line, seg)]);
      }
    }
  }
  return out;
}

/// The outlines of a line's start and end markers, as the renderer draws them:
/// (end, marker name, polygon).
function markers(l: Line): [string, string, Pt[]][] {
  const first = l.path[0];
  const last = l.path[l.path.length - 1];
  if (first === undefined || last === undefined) {
    return [];
  }
  const w = l.style.strokeWidth;
  const ends: [string, Marker, Pt, Pt][] = [
    ["start", l.start, segFrom(first), startDir(first).mul(-1.0)],
    ["end", l.end, segTo(last), endDir(last)],
  ];
  const out: [string, string, Pt[]][] = [];
  for (const [end, m, tip, dir] of ends) {
    const er = () => l.erParts(end === "start").hull();
    let name: string;
    let poly: Pt[];
    switch (m) {
      case "none":
        continue;
      case "arrow":
      case "open":
        [name, poly] = ["arrowhead", Marker.outline(m, tip, dir, w)];
        break;
      case "triangle":
        [name, poly] = ["triangle", Marker.outline(m, tip, dir, w)];
        break;
      case "diamond":
      case "hollow-diamond":
        [name, poly] = ["diamond", Marker.outline(m, tip, dir, w)];
        break;
      case "dot":
        [name, poly] = ["dot", Marker.outline(m, tip, dir, w)];
        break;
      case "circle":
        [name, poly] = ["circle", Marker.outline(m, tip, dir, w)];
        break;
      case "one":
        [name, poly] = ["bar", er()];
        break;
      case "zero-one":
        [name, poly] = ["bar and circle", er()];
        break;
      case "many":
      case "one-many":
      case "zero-many":
        [name, poly] = ["crow's foot", er()];
        break;
    }
    out.push([end, name, poly]);
  }
  return out;
}

/// `t` values (steps of 0.05) where label `i` on line `li` would collide with nothing and,
/// for a label beside its line, where that side fits the segment.
function clearPositions(
  ctx: Ctx,
  subs: Subject[],
  grid: Grid,
  i: number,
  li: number,
  label: Label,
): number[] {
  const l = ctx.lines[li]!;
  const out: number[] = [];
  for (let k = 1; k < 20; k++) {
    const t = k / 20.0;
    if (!LabelSide.fits(label.side, l.line.path, t).ok) {
      continue;
    }
    const r = labelBoxAt(label, pointAt(l.line.path, t));
    if (
      obstacles(ctx, r, [li, !LabelSide.isOn(label.side)], null).length === 0 &&
      grid.near(r, 1.0).every((j) => j === i || r.intersect(subs[j]!.rect) === null)
    ) {
      out.push(t);
    }
  }
  return out;
}

/// For a free text whose anchor point is inside a box: the box's id, and the help to move the
/// text into that box's block, in box-local coordinates.
function textInBox(ctx: Ctx, t: Text): [string, string] | null {
  const at = new Rect(t.at.x, t.at.y, 0.0, 0.0);
  const s = ctx
    .boxesNear(at, 0.0)
    .find((s) => s.shape.kind === "box" && s.shape.rect.containsRect(at));
  if (s === undefined) {
    return null;
  }
  const r = s.shape.rect;
  const id = s.shape.id;
  const help = `to put the text inside '${id}', move it into its text block, in box-local coordinates: \`box ${id} … { text ${debugStr(t.content)} at (${fmtNum(t.at.x - r.x)},${fmtNum(t.at.y - r.y)}) }\``;
  return [id, help];
}

/// The help for a line label with no clear position: how long a run it needs.
function noClearPosition(ctx: Ctx, l: Line, label: Label, shapeHit: boolean): string {
  const w = l.style.strokeWidth;
  const len = (m: Marker) => (m === "open" ? Marker.length("arrow", w) : Marker.length(m, w));
  const fallback = `no clear label position on ${l.id}; move the line or the obstacle`;
  let found: [Pt, Pt] | null = null;
  for (const s of l.path) {
    if (s.kind === "line" && onSegment(label.at, s.from, s.to)) {
      found = [s.from, s.to];
      break;
    }
  }
  if (found === null) {
    return fallback;
  }
  const [from, to] = found;
  const horizontal = Math.abs(to.x - from.x) >= Math.abs(to.y - from.y);
  const [extent, dim] = horizontal ? [label.rect.w, "wide"] : [label.rect.h, "tall"];
  const ends = fmax(len(l.start), len(l.end));
  const need = Math.ceil(extent + 2.0 * ends + ctx.scene.clearance);
  const run = from.dist(to);
  if (!LabelSide.isOn(label.side) || run >= need) {
    return fallback;
  }
  let help = `no clear label position on ${l.id}: the label is ${fmtNum(Math.ceil(extent))}px ${dim} and its segment ${fmtNum(round(run))}px long; give it about ${fmtNum(need)}px clear of boxes and arrowheads (widen the gap), shorten it or break it with \`\\n\``;
  if (!shapeHit) {
    const side = (["above", "left"] as LabelSide[]).find(
      (s) => LabelSide.fits(s, l.path, label.t).ok,
    );
    if (side !== undefined) {
      help += `, or put it beside the line with \`label-side=${LabelSide.name(side)}\``;
    }
  }
  return help;
}

function onSegment(p: Pt, a: Pt, b: Pt): boolean {
  const len = a.dist(b);
  return len > 0.0 && Math.abs(p.dist(a) + p.dist(b) - len) <= 0.5;
}

function describeClear(line: string, ts: number[]): string {
  const ranges: [number, number][] = [];
  for (const t of ts) {
    const r = ranges[ranges.length - 1];
    if (r !== undefined && Math.abs(t - r[1] - 0.05) < 1e-9) {
      r[1] = t;
    } else {
      ranges.push([t, t]);
    }
  }
  const parts = ranges.map(([a, b]) =>
    a === b ? `label@${fmtNum(a)}` : `label@${fmtNum(a)}..${fmtNum(b)}`,
  );
  return `clear label positions on ${line}: ${parts.join(", ")}`;
}
