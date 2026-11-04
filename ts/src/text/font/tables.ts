/// A minimal OpenType reader: the table directory and the fixed-layout tables the bundled
/// fonts need, read like skrifa/read-fonts read them.

export class Data {
  readonly view: DataView;

  constructor(
    readonly bytes: Uint8Array,
    readonly offset = 0,
    readonly length = bytes.length - offset,
  ) {
    this.view = new DataView(bytes.buffer, bytes.byteOffset + offset, length);
  }

  slice(offset: number, length = this.length - offset): Data {
    return new Data(this.bytes, this.offset + offset, length);
  }

  u8(o: number): number {
    return this.view.getUint8(o);
  }

  i8(o: number): number {
    return this.view.getInt8(o);
  }

  u16(o: number): number {
    return this.view.getUint16(o);
  }

  i16(o: number): number {
    return this.view.getInt16(o);
  }

  u32(o: number): number {
    return this.view.getUint32(o);
  }
}

export interface Head {
  unitsPerEm: number;
  indexToLocFormat: number;
}

export interface Hhea {
  ascender: number;
  descender: number;
  lineGap: number;
  numberOfHMetrics: number;
}

export interface Os2 {
  fsSelection: number;
  sTypoAscender: number;
  sTypoDescender: number;
  sTypoLineGap: number;
  usWinAscent: number;
  usWinDescent: number;
}

/// `OS/2` `fsSelection` bit 7.
const USE_TYPO_METRICS = 1 << 7;

export class Sfnt {
  readonly tables = new Map<string, Data>();
  readonly head: Head;
  readonly hhea: Hhea | null;
  readonly os2: Os2 | null;
  readonly numGlyphs: number;
  private readonly hmtx: Data | null;

  constructor(readonly data: Data) {
    const numTables = data.u16(4);
    for (let i = 0; i < numTables; i++) {
      const rec = 12 + i * 16;
      const tag = String.fromCharCode(
        data.u8(rec),
        data.u8(rec + 1),
        data.u8(rec + 2),
        data.u8(rec + 3),
      );
      this.tables.set(tag, data.slice(data.u32(rec + 8), data.u32(rec + 12)));
    }
    const head = this.required("head");
    this.head = { unitsPerEm: head.u16(18), indexToLocFormat: head.i16(50) };
    const hhea = this.table("hhea");
    this.hhea =
      hhea === null
        ? null
        : {
            ascender: hhea.i16(4),
            descender: hhea.i16(6),
            lineGap: hhea.i16(8),
            numberOfHMetrics: hhea.u16(34),
          };
    const os2 = this.table("OS/2");
    this.os2 =
      os2 === null
        ? null
        : {
            fsSelection: os2.u16(62),
            sTypoAscender: os2.i16(68),
            sTypoDescender: os2.i16(70),
            sTypoLineGap: os2.i16(72),
            usWinAscent: os2.u16(74),
            usWinDescent: os2.u16(76),
          };
    this.numGlyphs = this.required("maxp").u16(4);
    this.hmtx = this.table("hmtx");
  }

  table(tag: string): Data | null {
    return this.tables.get(tag) ?? null;
  }

  required(tag: string): Data {
    const t = this.table(tag);
    if (t === null) throw new Error(`font has no \`${tag}\` table`);
    return t;
  }

  /// skrifa's `GlyphMetrics::advance_width` for an unscaled, default instance: `null` past
  /// the glyph count, and the last long metric for glyphs past `numberOfHMetrics`.
  advanceWidth(gid: number): number | null {
    if (gid >= this.numGlyphs) return null;
    const n = this.hhea?.numberOfHMetrics ?? 0;
    if (this.hmtx === null || n === 0) return 0;
    return this.hmtx.u16(4 * Math.min(gid, n - 1));
  }

  /// skrifa's `Metrics` line metrics at `Size::unscaled()`, chosen like FreeType: the
  /// typographic metrics when `USE_TYPO_METRICS` is set, else `hhea`, falling back to the
  /// typographic, then the Windows metrics when `hhea`'s are zero.
  metrics(): { unitsPerEm: number; ascent: number; descent: number; leading: number } {
    let ascent = 0;
    let descent = 0;
    let leading = 0;
    const os2 = this.os2;
    let usedTypoMetrics = false;
    if (os2 !== null && (os2.fsSelection & USE_TYPO_METRICS) !== 0) {
      ascent = os2.sTypoAscender;
      descent = os2.sTypoDescender;
      leading = os2.sTypoLineGap;
      usedTypoMetrics = true;
    }
    if (!usedTypoMetrics) {
      if (this.hhea !== null) {
        ascent = this.hhea.ascender;
        descent = this.hhea.descender;
        leading = this.hhea.lineGap;
      }
      if (ascent === 0 && descent === 0 && os2 !== null) {
        if (os2.sTypoAscender !== 0 || os2.sTypoDescender !== 0) {
          ascent = os2.sTypoAscender;
          descent = os2.sTypoDescender;
          leading = os2.sTypoLineGap;
        } else {
          ascent = os2.usWinAscent;
          descent = -os2.usWinDescent;
        }
      }
    }
    return { unitsPerEm: this.head.unitsPerEm, ascent, descent, leading };
  }
}
