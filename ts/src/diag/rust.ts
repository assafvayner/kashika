export type Result<T, E> = { ok: true; value: T } | { ok: false; error: E };

export function ok<T>(value: T): { ok: true; value: T } {
  return { ok: true, value };
}

export function err<E>(error: E): { ok: false; error: E } {
  return { ok: false, error };
}

export const f32 = Math.fround;

export function chars(s: string): string[] {
  return Array.from(s);
}

/// UTF-8 length in bytes, which is what Rust's `str::len` counts.
export function utf8Len(s: string): number {
  let n = 0;
  for (const c of s) {
    const cp = c.codePointAt(0)!;
    n += cp < 0x80 ? 1 : cp < 0x800 ? 2 : cp < 0x10000 ? 3 : 4;
  }
  return n;
}

/// Rust's `char::is_whitespace` (the Unicode `White_Space` property).
export function isWhitespace(c: string): boolean {
  const cp = c.codePointAt(0)!;
  return (
    (cp >= 0x09 && cp <= 0x0d) ||
    cp === 0x20 ||
    cp === 0x85 ||
    cp === 0xa0 ||
    cp === 0x1680 ||
    (cp >= 0x2000 && cp <= 0x200a) ||
    cp === 0x2028 ||
    cp === 0x2029 ||
    cp === 0x202f ||
    cp === 0x205f ||
    cp === 0x3000
  );
}

export function trimStart(s: string): string {
  const cs = chars(s);
  let i = 0;
  while (i < cs.length && isWhitespace(cs[i]!)) i++;
  return cs.slice(i).join("");
}

export function trimEnd(s: string): string {
  const cs = chars(s);
  let j = cs.length;
  while (j > 0 && isWhitespace(cs[j - 1]!)) j--;
  return cs.slice(0, j).join("");
}

export function trim(s: string): string {
  return trimEnd(trimStart(s));
}

/// Rust's `str::lines`: split on `\n`, drop a trailing `\r` from each line and the empty
/// piece after a final newline.
export function lines(s: string): string[] {
  if (s === "") return [];
  const out = s.split("\n");
  if (out[out.length - 1] === "") out.pop();
  return out.map((l) => (l.endsWith("\r") ? l.slice(0, -1) : l));
}

export function isAsciiDigit(c: string): boolean {
  return c.length === 1 && c >= "0" && c <= "9";
}

export function isAsciiAlphabetic(c: string): boolean {
  return c.length === 1 && ((c >= "a" && c <= "z") || (c >= "A" && c <= "Z"));
}

function sortKey(c: number): number {
  if (c >= 0xd800 && c <= 0xdfff) return c + 0x2000;
  if (c >= 0xe000) return c - 0x800;
  return c;
}

/// Compares like Rust's `str` `Ord`: by UTF-8 bytes, which is code point order.
export function cmpStr(a: string, b: string): number {
  const n = Math.min(a.length, b.length);
  for (let i = 0; i < n; i++) {
    const x = a.charCodeAt(i);
    const y = b.charCodeAt(i);
    if (x !== y) return sortKey(x) < sortKey(y) ? -1 : 1;
  }
  return a.length < b.length ? -1 : a.length > b.length ? 1 : 0;
}

export type TupleItem = number | string | boolean;

/// Lexicographic order of tuples, like Rust's tuple `Ord`.
export function cmpTuple(a: readonly TupleItem[], b: readonly TupleItem[]): number {
  const n = Math.min(a.length, b.length);
  for (let i = 0; i < n; i++) {
    const x = a[i]!;
    const y = b[i]!;
    let c: number;
    if (typeof x === "string" && typeof y === "string") {
      c = cmpStr(x, y);
    } else if (typeof x === "boolean" && typeof y === "boolean") {
      c = x === y ? 0 : x ? 1 : -1;
    } else {
      c = x < y ? -1 : x > y ? 1 : 0;
    }
    if (c !== 0) return c;
  }
  return a.length - b.length < 0 ? -1 : a.length > b.length ? 1 : 0;
}

const bits = new DataView(new ArrayBuffer(8));

function toBits(v: number): bigint {
  bits.setFloat64(0, v);
  return bits.getBigUint64(0);
}

function fromBits(b: bigint): number {
  bits.setBigUint64(0, b);
  return bits.getFloat64(0);
}

/// Rust's `f64::total_cmp`.
export function totalCmp(a: number, b: number): number {
  bits.setFloat64(0, a);
  let x = bits.getBigInt64(0);
  bits.setFloat64(0, b);
  let y = bits.getBigInt64(0);
  x ^= BigInt.asIntN(64, BigInt.asUintN(64, x >> 63n) >> 1n);
  y ^= BigInt.asIntN(64, BigInt.asUintN(64, y >> 63n) >> 1n);
  return x < y ? -1 : x > y ? 1 : 0;
}

/// Rust's `f64::min`: a NaN operand is ignored.
export function fmin(a: number, b: number): number {
  if (Number.isNaN(a)) return b;
  if (Number.isNaN(b)) return a;
  return Math.min(a, b);
}

/// Rust's `f64::max`: a NaN operand is ignored.
export function fmax(a: number, b: number): number {
  if (Number.isNaN(a)) return b;
  if (Number.isNaN(b)) return a;
  return Math.max(a, b);
}

/// Rust's `f64::clamp`.
export function fclamp(v: number, lo: number, hi: number): number {
  if (!(lo <= hi)) throw new Error(`min > max, or either was NaN. min = ${lo}, max = ${hi}`);
  let x = v;
  if (x < lo) x = lo;
  if (x > hi) x = hi;
  return x;
}

/// Rust's `f64::round`: halfway cases away from zero.
export function round(v: number): number {
  const r = Math.round(Math.abs(v));
  return v < 0 || Object.is(v, -0) ? -r : r;
}

/// Rust's saturating `f64 as usize`.
export function asUsize(v: number): number {
  if (Number.isNaN(v) || v <= 0) return 0;
  return Math.min(Math.trunc(v), 18446744073709551615);
}

/// Rust's `f64::to_radians`.
export function toRadians(v: number): number {
  return v * (Math.PI / 180);
}

/// The neutral element of Rust's `f64` `Sum` is `-0.0`.
export function sum(vs: Iterable<number>): number {
  let s = -0;
  for (const v of vs) s += v;
  return s;
}

const SPLIT = 134217729;

function twoProd(a: number, b: number): [number, number] {
  const p = a * b;
  let t = SPLIT * a;
  const ah = t - (t - a);
  const al = a - ah;
  t = SPLIT * b;
  const bh = t - (t - b);
  const bl = b - bh;
  return [p, ah * bh - p + ah * bl + al * bh + al * bl];
}

function twoSum(a: number, b: number): [number, number] {
  const s = a + b;
  const bb = s - a;
  return [s, a - (s - bb) + (b - bb)];
}

function addRoundToOdd(a: number, b: number): number {
  const [s, e] = twoSum(a, b);
  if (e === 0 || (toBits(s) & 1n) === 1n) return s;
  const up = (e > 0) === (s > 0);
  const b0 = toBits(s);
  return fromBits(up ? b0 + 1n : b0 - 1n);
}

/// `a * b + c` rounded once (Boldo and Melquiond's emulation with rounding to odd), for
/// operands whose products neither overflow nor underflow.
function fma(a: number, b: number, c: number): number {
  const [uh, ul] = twoProd(a, b);
  const [th, tl] = twoSum(c, uh);
  return th + addRoundToOdd(tl, ul);
}

/// Rust's `f64::hypot` as macOS libm computes it, `sqrt(fma(lo, lo, hi * hi))` on the
/// magnitudes, which `Math.hypot` does not match bit for bit.
export function hypot(a: number, b: number): number {
  const x = Math.abs(a);
  const y = Math.abs(b);
  if (x === Infinity || y === Infinity) return Infinity;
  if (Number.isNaN(x) || Number.isNaN(y)) return NaN;
  let hi = Math.max(x, y);
  let lo = Math.min(x, y);
  if (lo === 0) return hi;
  let scale = 1;
  if (hi > 2 ** 500) {
    hi *= 2 ** -600;
    lo *= 2 ** -600;
    scale = 2 ** 600;
  } else if (hi < 2 ** -500) {
    hi *= 2 ** 600;
    lo *= 2 ** 600;
    scale = 2 ** -600;
  }
  return Math.sqrt(fma(lo, lo, hi * hi)) * scale;
}

/// Rust's `str::parse::<f64>`.
export function parseF64(s: string): number | null {
  const m = /^([+-]?)(inf|infinity|nan)$/i.exec(s);
  if (m) {
    if (m[2]!.toLowerCase() === "nan") return NaN;
    return m[1] === "-" ? -Infinity : Infinity;
  }
  if (!/^[+-]?(\d+\.?\d*|\.\d+)([eE][+-]?\d+)?$/.test(s)) return null;
  return Number(s);
}

/// The shortest round-trip digits of a finite, nonzero `v` and its decimal exponent.
export function shortestDigits(v: number): { neg: boolean; digits: string; exp: number } {
  const e = Math.abs(v).toExponential();
  const at = e.indexOf("e");
  return {
    neg: v < 0,
    digits: e.slice(0, at).replace(".", ""),
    exp: Number(e.slice(at + 1)),
  };
}

export function fixedDigits(digits: string, exp: number): string {
  if (exp >= digits.length - 1) return digits + "0".repeat(exp - (digits.length - 1));
  if (exp >= 0) return `${digits.slice(0, exp + 1)}.${digits.slice(exp + 1)}`;
  return `0.${"0".repeat(-exp - 1)}${digits}`;
}

export function scientificDigits(digits: string, exp: number, plus: boolean): string {
  const mant = digits.length > 1 ? `${digits[0]}.${digits.slice(1)}` : digits;
  return `${mant}e${exp < 0 ? "-" : plus ? "+" : ""}${Math.abs(exp)}`;
}

function nonFinite(v: number): string {
  return Number.isNaN(v) ? "NaN" : v > 0 ? "inf" : "-inf";
}

/// Rust's `{}` for `f64`: shortest round-trip digits, never in exponent form, and integral
/// values without `.0`.
export function displayF64(v: number): string {
  if (!Number.isFinite(v)) return nonFinite(v);
  if (v === 0) return Object.is(v, -0) ? "-0" : "0";
  const { neg, digits, exp } = shortestDigits(v);
  return (neg ? "-" : "") + fixedDigits(digits, exp);
}

/// Rust's `{:?}` for `f64`: exponent form below 1e-4 and from 1e16, `.0` on integral values.
export function debugF64(v: number): string {
  if (!Number.isFinite(v)) return nonFinite(v);
  if (v === 0) return Object.is(v, -0) ? "-0.0" : "0.0";
  const { neg, digits, exp } = shortestDigits(v);
  const sign = neg ? "-" : "";
  if (exp < -4 || exp >= 16) return sign + scientificDigits(digits, exp, false);
  const s = fixedDigits(digits, exp);
  return sign + (s.includes(".") ? s : `${s}.0`);
}
