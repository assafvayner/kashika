import { cmpStr, fixedDigits, scientificDigits, shortestDigits } from "./rust.js";

/// An `f64` in a serialized value: serde_json writes it as a float (`5.0`), where a plain
/// integral `number` is an integer (`5`).
export class Float {
  constructor(readonly v: number) {}
}

export type Json =
  | null
  | boolean
  | number
  | string
  | Float
  | Json[]
  | Map<string, Json>
  | { [key: string]: Json };

export type JsonObject = Map<string, Json> | { [key: string]: Json };

export function isObject(v: Json | undefined): v is JsonObject {
  return (
    v !== null &&
    v !== undefined &&
    typeof v === "object" &&
    !Array.isArray(v) &&
    !(v instanceof Float)
  );
}

/// The object's entries sorted by key, as serde_json's `BTreeMap`-backed `Map` keeps them.
export function entries(o: JsonObject): [string, Json][] {
  const es = o instanceof Map ? [...o.entries()] : Object.entries(o);
  return es.sort((a, b) => cmpStr(a[0], b[0]));
}

/// `v[key]` like serde_json's `Value` index: `null` when `v` is not an object or lacks it.
export function get(v: Json | undefined, key: string | number): Json {
  if (typeof key === "number") return Array.isArray(v) ? (v[key] ?? null) : null;
  if (!isObject(v)) return null;
  return (v instanceof Map ? v.get(key) : v[key]) ?? null;
}

/// serde_json's `f64` output: exponent form (`1e+16`, `1e-6`) below 1e-5 and from 1e16,
/// `.0` on integral values, and `null` for non-finite values.
export function ryuF64(v: number): string {
  if (!Number.isFinite(v)) return "null";
  if (v === 0) return Object.is(v, -0) ? "-0.0" : "0.0";
  const { neg, digits, exp } = shortestDigits(v);
  const sign = neg ? "-" : "";
  if (exp < -5 || exp >= 16) return sign + scientificDigits(digits, exp, true);
  const s = fixedDigits(digits, exp);
  return sign + (s.includes(".") ? s : `${s}.0`);
}

export function jsonString(s: string): string {
  let out = '"';
  for (const c of s) {
    switch (c) {
      case '"':
        out += '\\"';
        break;
      case "\\":
        out += "\\\\";
        break;
      case "\n":
        out += "\\n";
        break;
      case "\r":
        out += "\\r";
        break;
      case "\t":
        out += "\\t";
        break;
      case "\b":
        out += "\\b";
        break;
      case "\f":
        out += "\\f";
        break;
      default: {
        const cp = c.codePointAt(0)!;
        out += cp < 0x20 ? `\\u${cp.toString(16).padStart(4, "0")}` : c;
      }
    }
  }
  return `${out}"`;
}

export function numberString(v: number): string {
  return Number.isSafeInteger(v) ? String(v) : ryuF64(v);
}

/// Compact JSON like `serde_json::to_string` of a `Value`.
export function toJsonString(v: Json): string {
  if (v === null) return "null";
  if (typeof v === "boolean") return v ? "true" : "false";
  if (typeof v === "number") return numberString(v);
  if (typeof v === "string") return jsonString(v);
  if (v instanceof Float) return ryuF64(v.v);
  if (Array.isArray(v)) return `[${v.map(toJsonString).join(",")}]`;
  return `{${entries(v)
    .map(([k, x]) => `${jsonString(k)}:${toJsonString(x)}`)
    .join(",")}}`;
}
