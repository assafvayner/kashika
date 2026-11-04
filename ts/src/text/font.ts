import { readFileSync } from "node:fs";

import { chars, f32, isWhitespace, sum } from "../diag/rust.js";
import { fmtNum as num } from "../geom/index.js";
import { Charmap } from "./font/cmap.js";
import { Colr } from "./font/colr.js";
import { Glyf, type OutlinePen } from "./font/glyf.js";
import { type Glyph, Shaper } from "./font/shape.js";
import { Data, Sfnt } from "./font/tables.js";
import type { Font, Run } from "./markup.js";

/// Line height as a multiple of the font size.
export const LINE_HEIGHT = 1.25;

const FONT_FILES = [
  "Inter-Regular.ttf",
  "Inter-Bold.ttf",
  "JetBrainsMono-Regular.ttf",
  "TwemojiMozilla.ttf",
];

let files: Uint8Array[] | null = null;

/// The bytes of every bundled font file: Inter regular and bold, JetBrains Mono, then
/// Twemoji Mozilla.
export function fontFiles(): Uint8Array[] {
  files ??= FONT_FILES.map(
    (name) => new Uint8Array(readFileSync(new URL(`./fonts/${name}`, import.meta.url))),
  );
  return files;
}

const FaceId = { Regular: 0, Bold: 1, Mono: 2, Emoji: 3 } as const;
type FaceId = (typeof FaceId)[keyof typeof FaceId];

/// A bundled font, parsed once, with the metrics every measurement needs.
class Face {
  readonly font: Sfnt;
  readonly charmap: Charmap;
  readonly outlines: Glyf;
  readonly colr: Colr;
  readonly unitsPerEm: number;

  constructor(data: Uint8Array) {
    this.font = new Sfnt(new Data(data));
    this.charmap = new Charmap(this.font);
    this.outlines = new Glyf(this.font);
    this.colr = new Colr(this.font);
    this.unitsPerEm = this.font.metrics().unitsPerEm;
  }
}

let faces: Face[] | null = null;

function face(id: FaceId): Face {
  faces ??= fontFiles().map((data) => new Face(data));
  return faces[id]!;
}

let shaper: Shaper | null = null;

function emojiShaper(): Shaper {
  shaper ??= new Shaper(face(FaceId.Emoji).font);
  return shaper;
}

/// `rustybuzz::shape` of `text` on the emoji face, left to right with no features set.
export function shapeEmoji(text: string): Glyph[] {
  return emojiShaper().shape(text);
}

/// The faces a run falls back through, in order: its own, the other text face, then emoji.
function chain(font: Font): [FaceId, FaceId, FaceId] {
  switch (font) {
    case "regular":
      return [FaceId.Regular, FaceId.Mono, FaceId.Emoji];
    case "bold":
      return [FaceId.Bold, FaceId.Mono, FaceId.Emoji];
    case "mono":
      return [FaceId.Mono, FaceId.Regular, FaceId.Emoji];
  }
}

function has(id: FaceId, c: string): boolean {
  return face(id).charmap.map(c.codePointAt(0)!) !== null;
}

const ZWJ = "\u{200D}";
const TEXT_STYLE = "\u{FE0E}";
const KEYCAP = "\u{20E3}";

function isAscii(c: string): boolean {
  return c.codePointAt(0)! < 0x80;
}

function isRegionalIndicator(c: string): boolean {
  const cp = c.codePointAt(0)!;
  return cp >= 0x1f1e6 && cp <= 0x1f1ff;
}

/// Characters that extend an emoji: presentation selectors, the keycap, skin tones and tags.
function extendsEmoji(c: string): boolean {
  const cp = c.codePointAt(0)!;
  return (
    cp === 0xfe0e ||
    cp === 0xfe0f ||
    cp === 0x20e3 ||
    (cp >= 0x1f3fb && cp <= 0x1f3ff) ||
    (cp >= 0xe0020 && cp <= 0xe007f)
  );
}

/// The face for the cluster starting at `chars[i]`, and the index after it. A lone
/// default-ignorable character has no face of its own: it joins the segment before it.
/// A cluster is a single character, or an emoji with its extenders and ZWJ-joined emoji,
/// or a pair of regional indicators. Such a sequence takes the emoji face unless it asks
/// for text presentation; an ASCII digit, `#` or `*` does so only with a keycap.
function cluster(
  cs: string[],
  i: number,
  chain: [FaceId, FaceId, FaceId],
): [FaceId | null, number] {
  const c = cs[i]!;
  if (isDefaultIgnorable(c)) return [null, i + 1];
  let end = i + 1;
  if (has(FaceId.Emoji, c)) {
    if (isRegionalIndicator(c) && end < cs.length && isRegionalIndicator(cs[end]!)) end += 1;
    for (;;) {
      const m = cs[end];
      if (m !== undefined && extendsEmoji(m)) {
        end += 1;
      } else if (
        m === ZWJ &&
        ((n) => n !== undefined && !isAscii(n) && !isDefaultIgnorable(n) && has(FaceId.Emoji, n))(
          cs[end + 1],
        )
      ) {
        end += 2;
      } else {
        break;
      }
    }
  }
  const seq = cs.slice(i + 1, end);
  const emoji = isAscii(c) ? seq.includes(KEYCAP) : seq.length > 0 && !seq.includes(TEXT_STYLE);
  if (emoji) return [FaceId.Emoji, end];
  const id = chain.find((f) => has(f, c)) ?? chain[0];
  return [id, i + 1];
}

/// Splits a run into maximal pieces that one face draws, chosen per cluster from the
/// run's fallback chain.
function segments(run: Run): [FaceId, string][] {
  const ch = chain(run.font);
  const cs = chars(run.text);
  const out: [FaceId, number, number][] = [];
  let i = 0;
  while (i < cs.length) {
    const [id, end] = cluster(cs, i, ch);
    const last = out[out.length - 1];
    if (last !== undefined && (id === null || last[0] === id)) last[2] = end;
    else out.push([id ?? ch[0], i, end]);
    i = end;
  }
  return out.map(([id, from, to]) => [id, cs.slice(from, to).join("")]);
}

/// The glyphs of one segment. Text faces map each character through the cmap, with no
/// kerning or shaping, and skip default-ignorable characters. The emoji face is shaped
/// with its GSUB, which forms the ligatures of emoji sequences.
function glyphs(id: FaceId, text: string): Glyph[] {
  if (id === FaceId.Emoji) return shapeEmoji(text);
  const f = face(id);
  return chars(text)
    .filter((c) => !isDefaultIgnorable(c))
    .map((c) => {
      const g = glyph(f, c);
      return { id: g, advance: advance(f, g), dx: 0, dy: 0 };
    });
}

/// Unicode's `Default_Ignorable_Code_Point` ranges (DerivedCoreProperties.txt): joiners,
/// variation selectors, invisible format and bidi controls, the soft hyphen and fillers.
export const DEFAULT_IGNORABLE: readonly (readonly [number, number])[] = [
  [0x00ad, 0x00ad],
  [0x034f, 0x034f],
  [0x061c, 0x061c],
  [0x115f, 0x1160],
  [0x17b4, 0x17b5],
  [0x180b, 0x180f],
  [0x200b, 0x200f],
  [0x202a, 0x202e],
  [0x2060, 0x206f],
  [0x3164, 0x3164],
  [0xfe00, 0xfe0f],
  [0xfeff, 0xfeff],
  [0xffa0, 0xffa0],
  [0xfff0, 0xfff8],
  [0x1bca0, 0x1bca3],
  [0x1d173, 0x1d17a],
  [0xe0000, 0xe0fff],
];

/// Whether `c` is a Unicode default-ignorable code point (ZWJ, ZWNJ, a variation selector,
/// a zero-width space, the soft hyphen, ...). These measure and draw as nothing.
export function isDefaultIgnorable(c: string): boolean {
  const cp = c.codePointAt(0)!;
  return DEFAULT_IGNORABLE.some(([lo, hi]) => cp >= lo && cp <= hi);
}

/// Scripts and symbol sets with a sample character, in the order `coveredScripts` names
/// them.
const SCRIPTS: readonly (readonly [string, string])[] = [
  ["Latin", "A"],
  ["Greek", "Ω"],
  ["Cyrillic", "Ж"],
  ["Armenian", "Ա"],
  ["Georgian", "ა"],
  ["Hebrew", "א"],
  ["Arabic", "ب"],
  ["Devanagari", "क"],
  ["Thai", "ก"],
  ["Han", "日"],
  ["Kana", "あ"],
  ["Hangul", "한"],
  ["box drawing", "┌"],
  ["emoji", "🚀"],
];

/// The scripts and symbol sets whose sample character every font's fallback chain can
/// draw, such as `["Latin", "Greek", "emoji"]`.
export function coveredScripts(): string[] {
  return SCRIPTS.filter(([, c]) =>
    (["regular", "bold", "mono"] as const).every((f) => chain(f).some((id) => has(id, c))),
  ).map(([name]) => name);
}

function glyph(f: Face, c: string): number {
  return f.charmap.map(c.codePointAt(0)!) ?? 0;
}

function advance(f: Face, g: number): number {
  return f32(f.font.advanceWidth(g) ?? 0);
}

/// The distinct characters, in order of first appearance, that no font in the run's
/// fallback chain has a glyph for. Drawing and measuring use the run's own font's
/// `.notdef` glyph for them. Default-ignorable characters are never missing.
export function missingGlyphs(runs: Iterable<Run>): string[] {
  const out: string[] = [];
  for (const run of runs) {
    for (const [id, text] of segments(run)) {
      for (const c of chars(text)) {
        if (!has(id, c) && !isDefaultIgnorable(c) && !out.includes(c)) out.push(c);
      }
    }
  }
  return out;
}

/// Whether the runs draw something and all of it is emoji, which keep their own colors.
export function onlyEmoji(runs: Iterable<Run>): boolean {
  let emoji = false;
  for (const run of runs) {
    for (const [id, text] of segments(run)) {
      if (id === FaceId.Emoji) {
        emoji = true;
      } else if (chars(text).some((c) => !isWhitespace(c) && !isDefaultIgnorable(c))) {
        return false;
      }
    }
  }
  return emoji;
}

/// Advance width of a run in pixels: the sum of glyph advances, without kerning. Only
/// emoji are shaped.
export function runWidth(run: Run, size: number): number {
  return sum(
    segments(run).map(([id, text]) => {
      const units = sum(glyphs(id, text).map((g) => g.advance));
      return (units * size) / face(id).unitsPerEm;
    }),
  );
}

/// Distance from the top of a line box to its baseline. Inter's ascender-to-descender
/// range is centered in the line box, and every font shares that baseline.
export function baselineOffset(size: number): number {
  const m = face(FaceId.Regular).font.metrics();
  const em = m.unitsPerEm;
  const asc = m.ascent / em;
  const desc = m.descent / em;
  return ((LINE_HEIGHT - (asc - desc)) / 2 + asc) * size;
}

class PathOut implements OutlinePen {
  out = "";
  start: [number, number] = [0, 0];
  last: [number, number] = [0, 0];

  constructor(
    readonly x: number,
    readonly y: number,
    readonly scale: number,
  ) {}

  pt(x: number, y: number): string {
    return `${num(this.x + x * this.scale)} ${num(this.y - y * this.scale)}`;
  }

  moveTo(x: number, y: number): void {
    this.start = [x, y];
    this.last = [x, y];
    this.out += `M${this.pt(x, y)}`;
  }

  lineTo(x: number, y: number): void {
    this.last = [x, y];
    this.out += `L${this.pt(x, y)}`;
  }

  quadTo(x1: number, y1: number, x: number, y: number): void {
    this.last = [x, y];
    this.out += `Q${this.pt(x1, y1)} ${this.pt(x, y)}`;
  }

  curveTo(x1: number, y1: number, x2: number, y2: number, x: number, y: number): void {
    this.last = [x, y];
    this.out += `C${this.pt(x1, y1)} ${this.pt(x2, y2)} ${this.pt(x, y)}`;
  }

  close(): void {
    if (this.last[0] !== this.start[0] || this.last[1] !== this.start[1]) {
      this.out += `L${this.pt(this.start[0], this.start[1])}`;
    }
    this.out += "Z";
  }
}

function draw(id: FaceId, g: number, x: number, y: number, scale: number): string {
  const b = new PathOut(x, y, scale);
  face(id).outlines.draw(g, b);
  return b.out;
}

/// Calls `f` with each glyph of a line, its face, pen position and scale.
function walk(
  runs: readonly Run[],
  size: number,
  x: number,
  baseline: number,
  f: (id: FaceId, g: Glyph, x: number, y: number, scale: number) => void,
): void {
  let pen = x;
  for (const run of runs) {
    for (const [id, text] of segments(run)) {
      const scale = size / face(id).unitsPerEm;
      for (const g of glyphs(id, text)) {
        f(id, g, pen + g.dx * scale, baseline - g.dy * scale, scale);
        pen += g.advance * scale;
      }
    }
  }
}

/// SVG path data for the text-colored glyphs of a line of runs whose pen starts at
/// `(x, baseline)`. Emoji are left out; `colorLayers` draws them.
export function outline(runs: readonly Run[], size: number, x: number, baseline: number): string {
  let out = "";
  walk(runs, size, x, baseline, (id, g, x, y, scale) => {
    if (id !== FaceId.Emoji) out += draw(id, g.id, x, y, scale);
  });
  return out;
}

/// One filled path of a color glyph.
export interface Layer {
  d: string;
  /// The layer's RGBA color, or `null` to use the text color.
  color: [number, number, number, number] | null;
}

/// The emoji of a line, drawn like `outline`, as their color layers in paint order.
export function colorLayers(
  runs: readonly Run[],
  size: number,
  x: number,
  baseline: number,
): Layer[] {
  const out: Layer[] = [];
  walk(runs, size, x, baseline, (id, g, x, y, scale) => {
    if (id !== FaceId.Emoji) return;
    const f = face(id);
    const layers = f.colr.layers(g.id);
    let fills: [number, Layer["color"]][];
    if (layers !== null) {
      fills = [];
      for (const l of layers) {
        if (l.paletteIndex === 0xffff) {
          fills.push([l.glyph, null]);
          continue;
        }
        const c = f.colr.color(l.paletteIndex);
        if (c === null) continue;
        fills.push([l.glyph, [c[0], c[1], c[2], c[3]]]);
      }
    } else {
      fills = [[g.id, null]];
    }
    for (const [layer, color] of fills) {
      const d = draw(id, layer, x, y, scale);
      if (d !== "") out.push({ d, color });
    }
  });
  return out;
}
