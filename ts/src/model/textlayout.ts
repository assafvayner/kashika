import { Diagnostic, type Span } from "../diag/diagnostic.js";
import { Float, type Json } from "../diag/json.js";
import { displayF64, fmax } from "../diag/rust.js";
import { type Pt, Rect, TOLERANCE, fmtNum } from "../geom/index.js";
import { type Attr, type BlockText, type Node } from "../syntax/index.js";
import {
  type Block,
  type HAlign,
  type PlacedLine,
  type VAlign,
  measure,
  stack,
  stackSize,
} from "../text/index.js";
import * as attrs from "./attrs.js";
import { Attrs, LIMIT } from "./attrs.js";
import { Align, Fit, type ShapeKind, TitlePos } from "./scene.js";
import * as shapes from "./shapes.js";
import { type Padding, type Style, type StyleProps } from "./style.js";

/// Attributes a box or group accepts for its text, besides the style keys.
export const SHAPE_TEXT_KEYS: readonly string[] = ["sublabel", "gap"];
export const BLOCK_ITEM_KEYS: readonly string[] = [
  "font-size",
  "text-align",
  "color",
  "muted",
  "size",
  "align",
];
const SUBLABEL_SCALE = 0.8;
const DEFAULT_GAP = 2.0;

export type TextRole = "label" | "sublabel" | "stacked" | "absolute";

/// One laid-out piece of text inside a box or group.
export class TextItem {
  constructor(
    public role: TextRole,
    /// The source text, with markup.
    public content: string,
    public size: number,
    public color: string,
    public bold: boolean,
    public rect: Rect,
    public lines: PlacedLine[],
    public span: Span,
  ) {}

  toJson(): Json {
    return {
      role: this.role,
      content: this.content,
      size: new Float(this.size),
      color: this.color,
      bold: this.bold,
      rect: this.rect.toJson(),
      lines: this.lines.map((l) => l.toJson()),
      span: this.span.toJson(),
    };
  }
}

export class ShapeTextProps {
  constructor(
    public sublabel: [string, Span] | null,
    public align: Align,
    public valign: VAlign,
    public gap: number,
    public title: TitlePos,
  ) {}

  static default(): ShapeTextProps {
    return new ShapeTextProps(null, "center", "middle", DEFAULT_GAP, "top");
  }
}

export function shapeTextProps(
  a: Attrs,
  style: StyleProps,
  group: boolean,
  diags: Diagnostic[],
): ShapeTextProps {
  const sx = a.get("sublabel");
  let sublabel: [string, Span] | null = null;
  if (sx !== null) {
    const s = attrs.text(sx, diags);
    sublabel = s === null ? null : [s, sx.span];
  }
  const align: Align = style.textAlign !== null ? style.textAlign : group ? "left" : "center";
  const title = style.title ?? TitlePos.default();
  let valign: VAlign;
  if (group) {
    const x = a.get("text-valign");
    if (x !== null) {
      diags.push(
        Diagnostic.error(
          "attr",
          x.span,
          "a group's title sits at its top or bottom edge; `text-valign` applies to boxes only",
        ).withHelp("remove `text-valign`; `title=bottom` puts the title at the bottom"),
      );
    }
    valign = title === "top" ? "top" : "bottom";
  } else {
    valign = style.textValign ?? "middle";
  }
  const gx = a.get("gap");
  const g = gx === null ? null : attrs.numIn(gx, "NonNegative", diags);
  const gap = g ?? DEFAULT_GAP;
  return new ShapeTextProps(sublabel, align, valign, gap, title);
}

/// A shape's text area (§6): the shape's own text area (the whole box for `rect` and
/// `rounded`) inset by `padding`.
export function textArea(shape: string, custom: Rect | null, rect: Rect, padding: Padding): Rect {
  const unit = custom ?? shapes.unitTextArea(shape, rect.w, rect.h);
  const r = unit === null ? rect : rect.mapRect(unit);
  return new Rect(
    r.x + padding.x,
    r.y + padding.y,
    fmax(r.w - 2.0 * padding.x, 0.0),
    fmax(r.h - 2.0 * padding.y, 0.0),
  );
}

/// The smallest box size, at least `rect`'s, whose text area holds `need` (width, height).
function requiredSize(
  shape: string,
  custom: Rect | null,
  need: readonly [number, number],
  padding: Padding,
  rect: Rect,
): [number, number] {
  const [w, h] = shapes.requiredSize(
    shape,
    custom,
    [need[0] + 2.0 * padding.x, need[1] + 2.0 * padding.y],
    [rect.w, rect.h],
  );
  return [fmax(w, rect.w), fmax(h, rect.h)];
}

export function halign(a: Align): HAlign {
  switch (a) {
    case "left":
      return "left";
    case "center":
      return "center";
    case "right":
      return "right";
  }
}

/// Measures `text`, reporting unclosed markup as a `parse` error at `span`.
export function measureAt(
  text: string,
  size: number,
  bold: boolean,
  span: Span,
  diags: Diagnostic[],
): Block | null {
  const r = measure(text, size, bold);
  if (!r.ok) {
    diags.push(
      Diagnostic.error("parse", span, r.error.toString()).withHelp(
        "close the marker, or write `\\*` or `\\`` for a literal one",
      ),
    );
    return null;
  }
  return r.value;
}

/// A text `font-size=` (or `size=`): greater than 0 and at most `LIMIT`, else a `size` error.
export function textSize(x: Attr, diags: Diagnostic[]): number | null {
  const v = attrs.num(x, diags);
  if (v === null) return null;
  if (v <= 0.0 || v > LIMIT) {
    diags.push(
      Diagnostic.error(
        "size",
        x.span,
        `text size must be greater than 0 and at most ${displayF64(LIMIT)}`,
      ),
    );
    return null;
  }
  return v;
}

/// A text `color=`, which can't be `none`.
export function textColor(x: Attr, diags: Diagnostic[]): string | null {
  const c = attrs.color(x, diags);
  if (c === null) return null;
  if (c === "none") {
    diags.push(attrs.typeErr(x, "a color like `#1a73e8` or a CSS color name"));
    return null;
  }
  return c;
}

export interface ShapeText {
  area: Rect;
  items: TextItem[];
  titleBand: number;
}

export interface ShapeInput {
  id: string;
  node: Node;
  kind: ShapeKind;
  /// The `shape` name, `rect` or `rounded` for groups.
  shape: string;
  /// Unit text area of a custom shape; `null` for built-in shapes.
  customTextArea: Rect | null;
  /// `text-area=below`: the text sits in a band `padding` under the box.
  textBelow: boolean;
  /// The natural size of an SVG-file shape drawn with `fit=contain`: its text area, and
  /// the band under it, follow the drawing instead of the box.
  contain: [number, number] | null;
  /// How far an icon pushes the text area from the left and from the top edge.
  iconInset: [number, number];
  rect: Rect;
  style: Style;
  props: ShapeTextProps;
  /// Canvas position of each block item's `at`, in block order (`null` for stacked items
  /// and for points that failed to resolve).
  blockAt: readonly (Pt | null)[];
}

interface Pending {
  role: TextRole;
  content: string;
  block: Block;
  color: string;
  bold: boolean;
  align: Align;
  span: Span;
}

/// A shape's measured text: the stacked flow (the label and sublabel, else the stacked
/// block items) and the absolute block items with their block index.
interface Measured {
  flow: Pending[];
  absolute: [number, Pending][];
  /// The first stacked block item, when a label or sublabel is there too.
  conflict: Span | null;
}

function measureShape(s: ShapeInput, diags: Diagnostic[]): Measured {
  const st = s.style;
  const p = s.props;
  const bold = st.fontWeight === "bold";
  const head: Pending[] = [];
  const label = s.node.label;
  if (label !== null) {
    const block = measureAt(label, st.fontSize, bold, s.node.span, diags);
    if (block !== null) {
      head.push({
        role: "label",
        content: label,
        block,
        color: st.color,
        bold,
        align: p.align,
        span: s.node.span,
      });
    }
  }
  if (p.sublabel !== null) {
    const [sub, span] = p.sublabel;
    const block = measureAt(sub, st.fontSize * SUBLABEL_SCALE, false, span, diags);
    if (block !== null) {
      head.push({
        role: "sublabel",
        content: sub,
        block,
        color: st.muted,
        bold: false,
        align: p.align,
        span,
      });
    }
  }

  const stacked: Pending[] = [];
  const absolute: [number, Pending][] = [];
  (s.node.block ?? []).forEach((item, i) => {
    const pending = blockItem(item, st, p.align, diags);
    if (pending === null) {
      return;
    }
    if (item.at !== null) {
      absolute.push([i, pending]);
    } else {
      stacked.push(pending);
    }
  });
  const hasHead = s.node.label !== null || p.sublabel !== null;
  const conflict = hasHead && stacked.length > 0 ? stacked[0]!.span : null;
  return {
    flow: hasHead ? head : stacked,
    absolute,
    conflict,
  };
}

/// The size a box needs to hold its text, as `label-overflow` reports it; `null` when it
/// has no text to fit, or its text sits under it (`text-area=below`). `s.rect` holds the
/// axes that are already fixed, and 0 for the others.
export function fitSize(s: ShapeInput): [number, number] | null {
  if (s.textBelow) {
    return null;
  }
  const m = measureShape(s, []);
  const blocks: Block[] = m.flow.map((x) => x.block);
  const sizes: [number, number][] = [];
  if (blocks.length > 0) {
    sizes.push(stackSize(blocks, s.props.gap));
  }
  for (const [, x] of m.absolute) {
    sizes.push([x.block.width(), x.block.height()]);
  }
  if (sizes.length === 0) return null;
  const need = sizes.reduce((a, b) => [fmax(a[0], b[0]), fmax(a[1], b[1])]);
  return requiredBox(s, withInset(s, need));
}

/// `need` plus the room an icon takes from the text area.
function withInset(s: ShapeInput, need: readonly [number, number]): [number, number] {
  const inset: readonly [number, number] = s.textBelow ? [0.0, 0.0] : s.iconInset;
  return [need[0] + inset[0], need[1] + inset[1]];
}

/// The box size, in whole pixels, whose text area holds `need`.
function requiredBox(s: ShapeInput, need: readonly [number, number]): [number, number] {
  const [w, h] =
    s.contain !== null
      ? containedSize(s.contain, need, s.style.padding, s.rect)
      : requiredSize(s.shape, s.customTextArea, need, s.style.padding, s.rect);
  return [Math.ceil(w), Math.ceil(h)];
}

/// The smallest box, at least `rect`'s, whose contained drawing of `natural` size holds
/// `need` plus `padding`. An axis that `rect` leaves at 0 follows the drawing's aspect
/// ratio from the other axis, so a fitted box hugs its drawing.
function containedSize(
  natural: readonly [number, number],
  need: readonly [number, number],
  padding: Padding,
  rect: Rect,
): [number, number] {
  const [nw, nh] = natural;
  const [w, h] = [need[0] + 2.0 * padding.x, need[1] + 2.0 * padding.y];
  if (nw <= 0.0 || nh <= 0.0) {
    return [fmax(w, rect.w), fmax(h, rect.h)];
  }
  const ratio = nw / nh;
  const [dw, dh] = [fmax(w, h * ratio), fmax(h, w / ratio)];
  const fw = rect.w > 0.0 ? fmax(dw, rect.w) : fmax(dw, rect.h * ratio);
  const fh = rect.h > 0.0 ? fmax(dh, rect.h) : fmax(dh, rect.w / ratio);
  return [fw, fh];
}

/// Lays out a shape's label, sublabel and text block, and reports `text-conflict`,
/// `label-overflow` and markup errors.
export function layoutShape(s: ShapeInput, diags: Diagnostic[]): ShapeText {
  const st = s.style;
  const p = s.props;
  const drawn = s.contain === null ? s.rect : Fit.place("contain", s.contain, s.rect);
  let area = textArea(s.shape, s.customTextArea, drawn, st.padding);
  const m = measureShape(s, diags);
  const absolute: [Pending, Pt][] = [];
  for (const [i, x] of m.absolute) {
    const at = s.blockAt[i];
    if (at !== undefined && at !== null) {
      absolute.push([x, at]);
    }
  }
  if (m.conflict !== null) {
    diags.push(
      Diagnostic.error(
        "text-conflict",
        m.conflict,
        `\`${s.id}\` has a label or sublabel and stacked text block items`,
      ).withHelp(
        "move the label into the block as its first `text` item, or give every block item its own `at`",
      ),
    );
  }
  const flow = m.flow;
  if (s.textBelow) {
    area = belowArea(drawn, flow, st, p.gap);
  } else {
    const [dx, dy] = s.iconInset;
    area = new Rect(area.x + dx, area.y + dy, fmax(area.w - dx, 0.0), fmax(area.h - dy, 0.0));
  }

  const items: TextItem[] = [];
  let titleBand = 0.0;
  if (flow.length > 0) {
    const blocks: Block[] = flow.map((x) => x.block);
    const need = stackSize(blocks, p.gap);
    if (need[0] > area.w + TOLERANCE || need[1] > area.h + TOLERANCE) {
      diags.push(overflow(s, "its text", need, area, flow[0]!.span));
    }
    const input: [Block, HAlign][] = flow.map((x) => [x.block, halign(x.align)]);
    const placed = stack(input, area, p.valign, p.gap);
    flow.forEach((x, i) => {
      const [lines, rect] = placed[i]!;
      items.push(item(x, rect, lines));
    });
    if (s.kind === "group") {
      titleBand = st.padding.y + need[1];
    }
  }
  for (const [x, at] of absolute) {
    const need: [number, number] = [x.block.width(), x.block.height()];
    if (need[0] > area.w + TOLERANCE || need[1] > area.h + TOLERANCE) {
      diags.push(overflow(s, "a block item", need, area, x.span));
    }
    const [lines, rect] = x.block.placeAt(at.x, at.y, halign(x.align));
    items.push(item(x, rect, lines));
  }
  return { area, items, titleBand };
}

/// The band under a `text-area=below` shape: `padding` below the bottom edge of `rect` (the
/// drawing), as tall as the stacked text (one line when there is none) and as wide as the
/// drawing or the text, whichever is wider, centered on the drawing.
function belowArea(rect: Rect, flow: readonly Pending[], st: Style, gap: number): Rect {
  const blocks: Block[] = flow.map((x) => x.block);
  const [w0, h] = blocks.length === 0 ? [0.0, st.fontSize * 1.25] : stackSize(blocks, gap);
  const w = fmax(w0, rect.w);
  return new Rect(rect.x + (rect.w - w) / 2.0, rect.bottom() + st.padding.y, w, h);
}

function item(x: Pending, rect: Rect, lines: PlacedLine[]): TextItem {
  return new TextItem(x.role, x.content, x.block.size(), x.color, x.bold, rect, lines, x.span);
}

function blockItem(
  item: BlockText,
  st: Style,
  align: Align,
  diags: Diagnostic[],
): Pending | null {
  const a = new Attrs(item.attrs, "text block item", BLOCK_ITEM_KEYS, diags);
  const sx = attrs.aliased(a, "font-size", "size", diags);
  const size = (sx === null ? null : textSize(sx, diags)) ?? st.fontSize;
  const mx = a.get("muted");
  const muted = mx !== null && attrs.flag(mx, diags);
  const cx = a.get("color");
  const color = (cx === null ? null : textColor(cx, diags)) ?? (muted ? st.muted : st.color);
  const ax = attrs.aliased(a, "text-align", "align", diags);
  const aw = ax === null ? null : attrs.word(ax, Align.NAMES, diags);
  const al = (aw === null ? null : Align.fromName(aw)) ?? align;
  const bold = st.fontWeight === "bold";
  const block = measureAt(item.content, size, bold, item.span, diags);
  if (block === null) return null;
  return {
    role: item.at !== null ? "absolute" : "stacked",
    content: item.content,
    block,
    color,
    bold,
    align: al,
    span: item.span,
  };
}

function overflow(
  s: ShapeInput,
  what: string,
  need0: readonly [number, number],
  area: Rect,
  span: Span,
): Diagnostic {
  const need = withInset(s, need0);
  const size = requiredBox(s, need);
  const [dw, dh] = [need[0] - area.w, need[1] - area.h];
  const wide = dw > TOLERANCE;
  const tall = dh > TOLERANCE;
  const short =
    wide && !tall
      ? `too narrow by ${fmtNum(dw)}px`
      : !wide && tall
        ? `too short by ${fmtNum(dh)}px`
        : `too small: ${fmtNum(fmax(dw, 0.0))}px narrower, ${fmtNum(fmax(dh, 0.0))}px shorter`;
  return Diagnostic.error(
    "label-overflow",
    span,
    `${what} needs ${fmtNum(need[0])}x${fmtNum(need[1])}px but the text area of \`${s.id}\` is ${fmtNum(area.w)}x${fmtNum(area.h)}px (${short})`,
  ).withHelp(
    `make \`${s.id}\` at least size (${displayF64(size[0])},${displayF64(size[1])}), or shorten the text`,
  );
}
