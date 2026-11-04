import { Diagnostic, type Element, type Span } from "../../diag/diagnostic.js";
import { Float } from "../../diag/json.js";
import { fmax, fmin, round } from "../../diag/rust.js";
import { type Pt, Rect, pointInPolygon, round2 } from "../../geom/index.js";
import { type PlacedLine, type Shape } from "../../model/index.js";
import { onlyEmoji } from "../../text/index.js";
import { type Ctx } from "../ctx.js";
import { lineEl, shapeEl, textEl } from "../element.js";
import { type Sink } from "../sink.js";
import { textBox } from "../text_box.js";

type Rgb = [number, number, number];
type Rgba = [number, number, number, number];

const MIN_RATIO = 3.0;
const WHITE: Rgb = [1.0, 1.0, 1.0];

/// `low-contrast`: the WCAG contrast ratio between text and the fill behind it is below 3:1.
export function lowContrast(ctx: Ctx, out: Sink): void {
  const rank: number[] = new Array<number>(ctx.shapes.length).fill(0);
  ctx.scene.paintOrder().forEach((i, r) => {
    rank[i] = r;
  });
  let texts: number | null = null;
  ctx.shapes.forEach((s, i) => {
    if (s.shape.layer !== "over") {
      texts = texts === null ? rank[i]! : Math.max(texts, rank[i]!);
    }
  });
  const bd = (p: Pt, last: string | null) => {
    let stop: number | null;
    if (last !== null) {
      const i = ctx.shapeIndex(last);
      stop = i === null ? null : rank[i]!;
    } else {
      stop = texts;
    }
    return backdrop(ctx, rank, p, stop);
  };
  for (const s of ctx.shapes) {
    const sh = s.shape;
    for (const item of sh.text) {
      if (emojiOnly(item.lines)) {
        continue;
      }
      let what: string;
      switch (item.role) {
        case "label":
          what = `label "${item.content}"`;
          break;
        case "sublabel":
          what = `sublabel "${item.content}"`;
          break;
        case "stacked":
        case "absolute":
          what = `text "${item.content}"`;
          break;
      }
      const bg = bd(item.rect.center(), sh.id);
      check(out, item.color, bg, shapeEl(sh), item.span, `${what} of '${sh.id}'`);
    }
    const icon = sh.icon;
    if (icon === null) continue;
    const color = ctx.scene.assets[icon.asset]!.currentColor;
    if (color === null) {
      continue;
    }
    const bg = bd(icon.rect.center(), sh.id);
    const d = finding(color, bg, shapeEl(sh), sh.span, `icon of '${sh.id}'`);
    if (d !== null) {
      out.push(
        d.withHelp(
          "set the shape's `color`, which the icon is drawn in, to contrast with the `fill` behind it",
        ),
      );
    }
  }
  for (const t of ctx.scene.texts) {
    if (emojiOnly(t.lines)) {
      continue;
    }
    const bg = bd(textBox(t).center(), null);
    check(out, t.color, bg, textEl(t), t.span, `text '${t.displayId()}'`);
  }
  for (const l of ctx.lines) {
    for (const lab of l.line.labels) {
      if (emojiOnly(lab.lines)) {
        continue;
      }
      const what = `label "${lab.text}" of ${l.line.id}`;
      const bg = bd(lab.rect.center(), null);
      check(out, l.line.style.color, bg, lineEl(l.line, null), lab.span, what);
    }
  }
}

/// Emoji keep their own colors, so text that is only emoji is not judged.
function emojiOnly(lines: readonly PlacedLine[]): boolean {
  return onlyEmoji(lines.flatMap((l) => l.runs));
}

function check(out: Sink, color: string, bg: Rgb, el: Element, span: Span, what: string): void {
  const d = finding(color, bg, el, span, what);
  if (d !== null) {
    out.push(d.withHelp("darken the text `color` or lighten the `fill` behind it"));
  }
}

/// Rust's `{:.1}`: the exact value rounded to one decimal, ties to even.
function fixed1(v: number): string {
  const x20 = v * 20;
  if (Number.isInteger(x20) && Math.abs(x20) % 2 === 1) {
    const lo = Math.floor(v * 10);
    const even = lo % 2 === 0 ? lo : lo + 1;
    return (even / 10).toFixed(1);
  }
  return v.toFixed(1);
}

function finding(
  color: string,
  bg: Rgb,
  el: Element,
  span: Span,
  what: string,
): Diagnostic | null {
  const fg = rgba(color);
  if (fg === null) return null;
  const r = ratio(over(fg, bg), bg);
  if (r >= MIN_RATIO) {
    return null;
  }
  return Diagnostic.warn(
    "low-contrast",
    span,
    `${what} has contrast ${fixed1(r)}:1 against its background (minimum 3:1)`,
  )
    .withElement(el)
    .withFact("ratio", new Float(round2(r)))
    .withFact("color", color)
    .withFact("background", hex(bg));
}

/// The canvas with every fill containing `p` painted on top in paint order (`rank`, from
/// `Scene.paintOrder`), up to and including the shape ranked `stop`; `null` stops before
/// any shape. A transparent canvas counts as white. Images and SVG-file shapes draw their
/// file instead of a fill, so they add nothing.
function backdrop(ctx: Ctx, rank: readonly number[], p: Pt, stop: number | null): Rgb {
  const canvas = ctx.scene.background === null ? null : rgba(ctx.scene.background);
  let c: Rgb = canvas === null ? WHITE : over(canvas, WHITE);
  if (stop === null) return c;
  const near = ctx
    .shapesNear(new Rect(p.x, p.y, 0.0, 0.0), 0.0)
    .filter((i) => rank[i]! <= stop)
    .sort((a, b) => rank[a]! - rank[b]!);
  for (const i of near) {
    const s = ctx.shapes[i]!;
    if (paintsFill(s.shape) && pointInPolygon(p, s.poly)) {
      const fill = rgba(s.shape.style.fill);
      if (fill !== null) {
        c = over(fill, c);
      }
    }
  }
  return c;
}

function paintsFill(s: Shape): boolean {
  return s.kind !== "image" && s.image === null && (s.form === null || s.form.draw.length > 0);
}

/// `#rgb`, `#rrggbb` or `#rrggbbaa` as 0..1 channels; `null` for `none` or anything else.
function rgba(c: string): Rgba | null {
  if (!c.startsWith("#")) return null;
  const h = c.slice(1);
  if (!/^[\x00-\x7f]*$/.test(h)) return null;
  let v: number[];
  if (h.length === 3) {
    if (!/^[0-9a-fA-F]{3}$/.test(h)) return null;
    v = [...h].map((ch) => parseInt(ch, 16) * 17);
  } else if (h.length === 6 || h.length === 8) {
    v = [];
    for (let i = 0; i < h.length; i += 2) {
      const pair = h.slice(i, i + 2);
      if (!/^\+?[0-9a-fA-F]+$/.test(pair)) return null;
      v.push(parseInt(pair, 16));
    }
  } else {
    return null;
  }
  return [v[0]! / 255.0, v[1]! / 255.0, v[2]! / 255.0, v[3] === undefined ? 1.0 : v[3] / 255.0];
}

function over(top: Rgba, under: Rgb): Rgb {
  return [0, 1, 2].map((i) => top[i]! * top[3] + under[i]! * (1.0 - top[3])) as Rgb;
}

function luminance(c: Rgb): number {
  const lin = (v: number) => (v <= 0.03928 ? v / 12.92 : Math.pow((v + 0.055) / 1.055, 2.4));
  return 0.2126 * lin(c[0]) + 0.7152 * lin(c[1]) + 0.0722 * lin(c[2]);
}

function ratio(a: Rgb, b: Rgb): number {
  const [la, lb] = [luminance(a), luminance(b)];
  return (fmax(la, lb) + 0.05) / (fmin(la, lb) + 0.05);
}

function hex(c: Rgb): string {
  const b = (v: number) => Math.min(Math.max(round(v * 255.0), 0), 255);
  return `#${c.map((v) => b(v).toString(16).padStart(2, "0")).join("")}`;
}
