import type { Data, Sfnt } from "./tables.js";

export type Rgba = [number, number, number, number];

/// A COLRv0 layer: its glyph and palette index (`0xFFFF` is the text color).
export interface ColorLayer {
  glyph: number;
  paletteIndex: number;
}

/// The COLRv0 base glyphs and layers, and `CPAL` palette 0.
export class Colr {
  private readonly colr: Data | null;
  private readonly palette: Rgba[] = [];

  constructor(font: Sfnt) {
    this.colr = font.table("COLR");
    const cpal = font.table("CPAL");
    if (cpal === null || cpal.u16(4) === 0) return;
    const entries = cpal.u16(2);
    const records = cpal.u32(8);
    const first = cpal.u16(12);
    for (let i = 0; i < entries; i++) {
      const at = records + (first + i) * 4;
      if (at + 4 > cpal.length) break;
      this.palette.push([cpal.u8(at + 2), cpal.u8(at + 1), cpal.u8(at), cpal.u8(at + 3)]);
    }
  }

  /// The layers of `gid` in paint order, or `null` when it is not a color glyph.
  layers(gid: number): ColorLayer[] | null {
    const c = this.colr;
    if (c === null) return null;
    const count = c.u16(2);
    const bases = c.u32(4);
    const layers = c.u32(8);
    if (bases === 0 || layers === 0) return null;
    let lo = 0;
    let hi = count;
    while (lo < hi) {
      const i = Math.floor((lo + hi) / 2);
      const rec = bases + i * 6;
      const g = c.u16(rec);
      if (g < gid) {
        lo = i + 1;
      } else if (g > gid) {
        hi = i;
      } else {
        const first = c.u16(rec + 2);
        const n = c.u16(rec + 4);
        const out: ColorLayer[] = [];
        const total = c.u16(12);
        for (let k = first; k < first + n && k < total; k++) {
          out.push({ glyph: c.u16(layers + k * 4), paletteIndex: c.u16(layers + k * 4 + 2) });
        }
        return out;
      }
    }
    return null;
  }

  /// Palette 0's color at `index`.
  color(index: number): Rgba | null {
    return this.palette[index] ?? null;
  }
}
