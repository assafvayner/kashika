import type { Data, Sfnt } from "./tables.js";

/// The GSUB features HarfBuzz enables by default for horizontal left-to-right text with
/// the default shaper. `frac`, `numr` and `dnom` only apply around a fraction slash, so
/// they are left out.
const DEFAULT_FEATURES = [
  "rvrn",
  "ltra",
  "ltrm",
  "rand",
  "ccmp",
  "locl",
  "rlig",
  "calt",
  "clig",
  "liga",
  "rclt",
];

const LIGATURE = 4;
const EXTENSION = 7;

export interface Ligature {
  glyph: number;
  components: number[];
}

/// A ligature substitution subtable: the ligature sets of each covered first glyph.
export interface LigatureSubst {
  sets: Map<number, Ligature[]>;
}

export interface Lookup {
  flag: number;
  subtables: LigatureSubst[];
}

function tag(d: Data, o: number): string {
  return String.fromCharCode(d.u8(o), d.u8(o + 1), d.u8(o + 2), d.u8(o + 3));
}

function coverage(d: Data): number[] {
  const format = d.u16(0);
  const out: number[] = [];
  if (format === 1) {
    const n = d.u16(2);
    for (let i = 0; i < n; i++) out.push(d.u16(4 + i * 2));
  } else if (format === 2) {
    const n = d.u16(2);
    for (let i = 0; i < n; i++) {
      const r = 4 + i * 6;
      const start = d.u16(r);
      const end = d.u16(r + 2);
      for (let g = start; g <= end; g++) out.push(g);
    }
  } else {
    throw new Error(`unsupported coverage format ${format}`);
  }
  return out;
}

function ligatureSubst(d: Data): LigatureSubst {
  const cov = coverage(d.slice(d.u16(2)));
  const n = d.u16(4);
  const sets = new Map<number, Ligature[]>();
  for (let i = 0; i < n && i < cov.length; i++) {
    const set = d.slice(d.u16(6 + i * 2));
    const ligs: Ligature[] = [];
    const m = set.u16(0);
    for (let k = 0; k < m; k++) {
      const lig = set.slice(set.u16(2 + k * 2));
      const count = lig.u16(2);
      const components: number[] = [];
      for (let c = 1; c < count; c++) components.push(lig.u16(2 + c * 2));
      ligs.push({ glyph: lig.u16(0), components });
    }
    sets.set(cov[i]!, ligs);
  }
  return { sets };
}

function subtable(type: number, d: Data): LigatureSubst {
  if (type === EXTENSION) return subtable(d.u16(2), d.slice(d.u32(4)));
  if (type === LIGATURE) return ligatureSubst(d);
  throw new Error(`unsupported GSUB lookup type ${type}`);
}

/// The lookups that HarfBuzz's default features select from the `DFLT` script's default
/// language system, in lookup-list order. Only ligature substitution (type 4, also inside
/// extension lookups) is read: it is the only kind the bundled emoji font uses.
export function defaultLookups(font: Sfnt): Lookup[] {
  const gsub = font.table("GSUB");
  if (gsub === null) return [];
  const scripts = gsub.slice(gsub.u16(4));
  const features = gsub.slice(gsub.u16(6));
  const lookups = gsub.slice(gsub.u16(8));
  let langSys: Data | null = null;
  for (let i = 0; i < scripts.u16(0); i++) {
    if (tag(scripts, 2 + i * 6) !== "DFLT") continue;
    const script = scripts.slice(scripts.u16(2 + i * 6 + 4));
    if (script.u16(0) !== 0) langSys = script.slice(script.u16(0));
  }
  if (langSys === null) return [];
  const featureIndices: number[] = [];
  const required = langSys.u16(2);
  if (required !== 0xffff) featureIndices.push(required);
  for (let i = 0; i < langSys.u16(4); i++) featureIndices.push(langSys.u16(6 + i * 2));
  const selected = new Set<number>();
  for (const fi of featureIndices) {
    const rec = 2 + fi * 6;
    if (fi !== required && !DEFAULT_FEATURES.includes(tag(features, rec))) continue;
    const feature = features.slice(features.u16(rec + 4));
    for (let k = 0; k < feature.u16(2); k++) selected.add(feature.u16(4 + k * 2));
  }
  return [...selected]
    .sort((a, b) => a - b)
    .map((li) => {
      const lookup = lookups.slice(lookups.u16(2 + li * 2));
      const type = lookup.u16(0);
      const subtables: LigatureSubst[] = [];
      for (let k = 0; k < lookup.u16(4); k++) {
        subtables.push(subtable(type, lookup.slice(lookup.u16(6 + k * 2))));
      }
      return { flag: lookup.u16(2), subtables };
    });
}
