import { Charmap } from "./cmap.js";
import { type Lookup, defaultLookups } from "./gsub.js";
import type { Sfnt } from "./tables.js";

/// A glyph to draw, in font units: its advance and its offset from the pen.
export interface Glyph {
  id: number;
  advance: number;
  dx: number;
  dy: number;
}

/// HarfBuzz's `is_default_ignorable`, which differs from Unicode's property: the Hangul
/// fillers, U+FFA0, U+180F and the shorthand format controls are left visible.
export function hbIsDefaultIgnorable(cp: number): boolean {
  const plane = cp >> 16;
  if (plane === 0) {
    switch (cp >> 8) {
      case 0x00:
        return cp === 0x00ad;
      case 0x03:
        return cp === 0x034f;
      case 0x06:
        return cp === 0x061c;
      case 0x17:
        return cp >= 0x17b4 && cp <= 0x17b5;
      case 0x18:
        return cp >= 0x180b && cp <= 0x180e;
      case 0x20:
        return (
          (cp >= 0x200b && cp <= 0x200f) ||
          (cp >= 0x202a && cp <= 0x202e) ||
          (cp >= 0x2060 && cp <= 0x206f)
        );
      case 0xfe:
        return (cp >= 0xfe00 && cp <= 0xfe0f) || cp === 0xfeff;
      case 0xff:
        return cp >= 0xfff0 && cp <= 0xfff8;
      default:
        return false;
    }
  }
  if (plane === 0x01) return cp >= 0x1d173 && cp <= 0x1d17a;
  if (plane === 0x0e) return cp >= 0xe0000 && cp <= 0xe0fff;
  return false;
}

/// The marks (general category M) that can reach the emoji shaper: the keycap and the
/// emoji font's selectors, and the non-spacing marks among the default-ignorable code
/// points a segment can hold.
function isUnicodeMark(cp: number): boolean {
  return cp === 0x20e3 || isNonspacingMark(cp);
}

function isNonspacingMark(cp: number): boolean {
  return (
    cp === 0x034f ||
    (cp >= 0x17b4 && cp <= 0x17b5) ||
    (cp >= 0x180b && cp <= 0x180f) ||
    (cp >= 0xfe00 && cp <= 0xfe0f) ||
    (cp >= 0xe0100 && cp <= 0xe01ef)
  );
}

type Script = "arabic" | "hangul" | "khmer" | "mongolian";

/// The script of the characters an emoji segment can hold that are not Common, Inherited
/// or unassigned: the default-ignorable code points of those scripts.
function script(cp: number): Script | null {
  if (cp === 0x061c) return "arabic";
  if (cp === 0x115f || cp === 0x1160 || cp === 0x3164 || cp === 0xffa0) return "hangul";
  if (cp === 0x17b4 || cp === 0x17b5) return "khmer";
  if (cp >= 0x180b && cp <= 0x180f) return "mongolian";
  return null;
}

interface Info {
  cp: number;
  glyph: number;
  ignorable: boolean;
  zwj: boolean;
  zwnj: boolean;
  hidden: boolean;
  substituted: boolean;
}

function info(cmap: Charmap, cp: number): Info {
  const ignorable = cp >= 0x80 && hbIsDefaultIgnorable(cp);
  return {
    cp,
    glyph: cmap.map(cp) ?? 0,
    ignorable,
    zwj: ignorable && cp === 0x200d,
    zwnj: ignorable && cp === 0x200c,
    hidden:
      ignorable &&
      ((cp >= 0x180b && cp <= 0x180d) ||
        cp === 0x180f ||
        (cp >= 0xe0020 && cp <= 0xe007f) ||
        cp === 0x034f),
    substituted: false,
  };
}

function isDefaultIgnorable(i: Info): boolean {
  return i.ignorable && !i.substituted;
}

/// Whether HarfBuzz's GSUB input matcher may step over `i` (`SKIP_MAYBE`): a visible
/// default-ignorable that is not ZWNJ. ZWJ is skipped too, since every default feature
/// joins automatically.
function maySkip(i: Info): boolean {
  return isDefaultIgnorable(i) && !i.zwnj && !i.hidden;
}

/// The positions of `components` after `start`, matched like HarfBuzz's skipping
/// iterator: a glyph that matches is taken even when it could be skipped.
function matchInput(buf: Info[], start: number, components: number[]): number[] | null {
  const positions = [start];
  let j = start;
  for (const c of components) {
    for (;;) {
      j += 1;
      if (j >= buf.length) return null;
      const g = buf[j]!;
      if (g.glyph === c) break;
      if (!maySkip(g)) return null;
    }
    positions.push(j);
  }
  return positions;
}

function applyLookup(lookup: Lookup, buf: Info[]): Info[] {
  if (lookup.flag !== 0) throw new Error(`unsupported GSUB lookup flag ${lookup.flag}`);
  const out: Info[] = [];
  let i = 0;
  outer: while (i < buf.length) {
    const cur = buf[i]!;
    for (const sub of lookup.subtables) {
      const set = sub.sets.get(cur.glyph);
      if (set === undefined) continue;
      for (const lig of set) {
        const positions = matchInput(buf, i, lig.components);
        if (positions === null) continue;
        out.push({ ...cur, glyph: lig.glyph, substituted: true });
        const end = positions[positions.length - 1]!;
        for (let k = i + 1; k <= end; k++) {
          if (!positions.includes(k)) out.push(buf[k]!);
        }
        i = end + 1;
        continue outer;
      }
    }
    out.push(cur);
    i += 1;
  }
  return out;
}

/// What `rustybuzz::shape` does with the emoji face, left to right and with no features
/// set: map each character through the cmap (a variation selector separately, since the
/// font has no format 14 subtable), make a combining grapheme joiner between two
/// characters skippable (no character here has a nonzero combining class), apply the
/// default GSUB lookups and take advances from `hmtx`. The font has no GPOS and its COLRv0
/// glyphs have no extents, so the fallback mark positioning zeroes the advance of every
/// non-spacing mark after a base, moving it back by that advance. Default-ignorable
/// glyphs left over are deleted, as the font has no space glyph to show them with.
export class Shaper {
  private readonly cmap: Charmap;
  private readonly lookups: Lookup[];

  constructor(private readonly font: Sfnt) {
    this.cmap = new Charmap(font);
    this.lookups = defaultLookups(font);
    if (this.cmap.map(0x20) !== null) throw new Error("emoji face has a space glyph");
  }

  shape(text: string): Glyph[] {
    let buf: Info[] = [];
    for (const c of text) buf.push(info(this.cmap, c.codePointAt(0)!));
    const sc = buf.map((i) => script(i.cp)).find((s) => s !== null) ?? null;
    for (let i = 1; i + 1 < buf.length; i++) {
      if (buf[i]!.cp === 0x034f) buf[i]!.hidden = false;
    }
    if (sc !== "khmer") {
      for (const lookup of this.lookups) buf = applyLookup(lookup, buf);
    }
    const fallbackPosition = sc !== "hangul" && sc !== "khmer";
    const out: Glyph[] = [];
    let base = false;
    for (const i of buf) {
      const g: Glyph = { id: i.glyph, advance: this.font.advanceWidth(i.glyph) ?? 0, dx: 0, dy: 0 };
      const mark = isUnicodeMark(i.cp);
      if (fallbackPosition && mark && base && isNonspacingMark(i.cp)) {
        g.dx -= g.advance;
        g.advance = 0;
      }
      base ||= !mark;
      if (!isDefaultIgnorable(i)) out.push(g);
    }
    return out;
  }
}
