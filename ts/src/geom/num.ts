import { displayF64, round } from "../diag/rust.js";

/// `v` rounded to 2 decimals, never `-0`.
export function round2(v: number): number {
  const r = round(v * 100) / 100;
  return r === 0 ? 0 : r;
}

/// `v` as a person would write it: at most 2 decimals, no trailing `.0` and never `-0`.
/// Values too large to round are written as they are.
export function fmtNum(v: number): string {
  const r = round2(v);
  return displayF64(Number.isFinite(r) ? r : v);
}
