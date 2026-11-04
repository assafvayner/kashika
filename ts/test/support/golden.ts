import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { Options } from "../../src/model/index.js";
import { type Case, GOLDEN, OUTPUTS, discover } from "../golden/case.js";
import { unifiedDiff } from "./diff.js";

export { Case, GOLDEN, discover } from "../golden/case.js";

/// Where mismatching outputs are saved, per case name.
export const ACTUAL = join(import.meta.dirname, "../../.golden-actual");

/// Every case under `tests/golden/<suite>`.
export function suiteDirs(suite: string): string[] {
  return discover(join(GOLDEN, suite));
}

/// The `Options` the Rust golden harness compiles `c` with.
export function caseOptions(c: Case): Options {
  return new Options({
    path: join(c.dir, "input.kash"),
    allowPath: c.allowPath,
    iconPath: c.iconPath === null ? [] : [join(c.dir, c.iconPath)],
    include: c.include.map((p) => join(c.dir, p)),
  });
}

/// One expected file: its name in the case directory and its bytes.
export interface Output {
  name: string;
  bytes: Uint8Array;
}

export function textOutput(name: string, s: string): Output {
  return { name, bytes: new TextEncoder().encode(s) };
}

function utf8(b: Uint8Array): string | null {
  try {
    return new TextDecoder("utf-8", { fatal: true }).decode(b);
  } catch {
    return null;
  }
}

function equal(a: Uint8Array, b: Uint8Array): boolean {
  return a.length === b.length && a.every((x, i) => x === b[i]);
}

/// Compares every output with the case's expected file, and flags expected files the case
/// no longer produces (except those in `skip`); on a mismatch saves the actual one under
/// `.golden-actual/<case>/` and throws with a unified diff. Never writes into `tests/golden`.
export function checkOutputs(
  c: Case,
  outputs: readonly Output[],
  skip: readonly string[] = [],
): void {
  const problems: string[] = [];
  for (const n of OUTPUTS) {
    if (existsSync(join(c.dir, n)) && !outputs.some((o) => o.name === n) && !skip.includes(n)) {
      problems.push(`${n} is stale: this case no longer produces it`);
    }
  }
  for (const o of outputs) {
    const path = join(c.dir, o.name);
    if (!existsSync(path)) {
      problems.push(`${o.name} is missing`);
      continue;
    }
    const expected = new Uint8Array(readFileSync(path));
    if (equal(expected, o.bytes)) continue;
    const dir = join(ACTUAL, c.name);
    mkdirSync(dir, { recursive: true });
    const saved = join(dir, o.name);
    writeFileSync(saved, o.bytes);
    const e = utf8(expected);
    const a = utf8(o.bytes);
    if (e !== null && a !== null) {
      problems.push(`${o.name} differs (actual saved to ${saved}):\n${unifiedDiff(e, a, 3)}`);
    } else {
      problems.push(
        `${o.name} differs: expected ${expected.length} bytes, got ${o.bytes.length} (actual saved to ${saved})`,
      );
    }
  }
  if (problems.length > 0) {
    throw new Error(`${c.name}\n${problems.join("\n")}`);
  }
}
