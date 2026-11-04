import type { Data, Sfnt } from "./tables.js";

const UNICODE = 0;
const ISO = 2;
const WINDOWS = 3;

const ENCODING_MS_SYMBOL = 0;
const ENCODING_MS_UNICODE_CS = 1;
const ENCODING_APPLE_ID_UNICODE_32 = 4;
const ENCODING_APPLE_ID_VARIANT_SELECTOR = 5;
const ENCODING_MS_ID_UCS_4 = 10;

const NONE = 0;
const UNICODE_BMP = 1;
const UNICODE_FULL = 2;
const SYMBOL = 3;

function mapFormat4(t: Data, cp: number): number | null {
  if (cp > 0xffff) return null;
  const segCount = t.u16(6) / 2;
  const endCodes = 14;
  const startCodes = endCodes + segCount * 2 + 2;
  const idDeltas = startCodes + segCount * 2;
  const idRangeOffsets = idDeltas + segCount * 2;
  const glyphIdArray = idRangeOffsets + segCount * 2;
  let lo = 0;
  let hi = segCount;
  while (lo < hi) {
    const i = Math.floor((lo + hi) / 2);
    const startCode = t.u16(startCodes + i * 2);
    if (cp < startCode) {
      hi = i;
    } else if (cp > t.u16(endCodes + i * 2)) {
      lo = i + 1;
    } else {
      const delta = t.i16(idDeltas + i * 2);
      const rangeOffset = t.u16(idRangeOffsets + i * 2);
      if (rangeOffset === 0) return (cp + delta) & 0xffff;
      let offset = rangeOffset / 2 + (cp - startCode);
      offset = Math.max(0, offset - (segCount - i));
      const at = glyphIdArray + offset * 2;
      if (at + 2 > t.length) return null;
      const gid = t.u16(at);
      return gid !== 0 ? (gid + delta) & 0xffff : null;
    }
  }
  return null;
}

function mapFormat12(t: Data, cp: number): number | null {
  const n = t.u32(12);
  let lo = 0;
  let hi = n;
  while (lo < hi) {
    const i = Math.floor((lo + hi) / 2);
    const g = 16 + i * 12;
    const start = t.u32(g);
    if (cp < start) {
      hi = i;
    } else if (cp > t.u32(g + 4)) {
      lo = i + 1;
    } else {
      return (t.u32(g + 8) + (cp - start)) >>> 0;
    }
  }
  return null;
}

/// skrifa's `Charmap`: one format 4 or 12 subtable, preferring a symbol mapping, then a
/// full Unicode repertoire, then the BMP, and among equals the last record.
export class Charmap {
  private readonly subtable: Data | null = null;
  private readonly format: number = 0;
  private readonly isSymbol: boolean = false;

  constructor(font: Sfnt) {
    const cmap = font.table("cmap");
    if (cmap === null) return;
    let kind = NONE;
    const n = cmap.u16(2);
    for (let i = n - 1; i >= 0; i--) {
      const rec = 4 + i * 8;
      const platform = cmap.u16(rec);
      const encoding = cmap.u16(rec + 2);
      const sub = cmap.slice(cmap.u32(rec + 4));
      const format = sub.u16(0);
      const supported = format === 4 || format === 12;
      let k = NONE;
      if (platform === UNICODE && encoding === ENCODING_APPLE_ID_VARIANT_SELECTOR) {
        continue;
      } else if (platform === WINDOWS && encoding === ENCODING_MS_SYMBOL) {
        k = SYMBOL;
      } else if (
        (platform === WINDOWS && encoding === ENCODING_MS_ID_UCS_4) ||
        (platform === UNICODE && encoding === ENCODING_APPLE_ID_UNICODE_32)
      ) {
        k = UNICODE_FULL;
      } else if (
        platform === ISO ||
        platform === UNICODE ||
        (platform === WINDOWS && encoding === ENCODING_MS_UNICODE_CS)
      ) {
        k = UNICODE_BMP;
      }
      if (supported && k > kind) {
        kind = k;
        this.subtable = sub;
        this.format = format;
        this.isSymbol = k === SYMBOL;
      }
    }
  }

  /// The glyph for `cp`, or `null` when it maps to nothing or to `.notdef`.
  map(cp: number): number | null {
    const g = this.mapImpl(cp);
    if (g === null && this.isSymbol && cp <= 0xff) return this.mapImpl(cp + 0xf000);
    return g;
  }

  private mapImpl(cp: number): number | null {
    if (this.subtable === null) return null;
    const g = this.format === 4 ? mapFormat4(this.subtable, cp) : mapFormat12(this.subtable, cp);
    return g === null || g === 0 ? null : g;
  }
}
