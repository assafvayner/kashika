import { existsSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";

import { lines } from "../../src/diag/rust.js";
import { Options } from "../../src/model/index.js";

export const GOLDEN = join(import.meta.dirname, "../../../tests/golden");

/// Every directory under `dir` holding an `input.kash`; case directories are not searched further.
export function caseDirs(dir: string, out: string[] = []): string[] {
  if (existsSync(join(dir, "input.kash"))) {
    out.push(dir);
    return out;
  }
  for (const name of readdirSync(dir).sort()) {
    const p = join(dir, name);
    if (statSync(p).isDirectory()) caseDirs(p, out);
  }
  return out;
}

export interface Directives {
  allowPath: boolean;
  iconPath: string | null;
  include: string[];
}

/// The `// key: value` lines at the top of `input.kash`, as the Rust golden harness reads them.
export function directives(src: string): Directives {
  const d: Directives = { allowPath: false, iconPath: null, include: [] };
  const text = src.startsWith("﻿") ? src.slice(1) : src;
  for (const line of lines(text)) {
    if (!line.startsWith("// ")) break;
    const body = line.slice(3);
    const at = body.indexOf(": ");
    if (at < 0) continue;
    const key = body.slice(0, at);
    if (!/^[a-z-]*$/.test(key)) continue;
    const values = body
      .slice(at + 2)
      .split(/[, ]/)
      .filter((v) => v !== "");
    if (key === "flags" && values.includes("allow-path")) d.allowPath = true;
    if (key === "icon-path") d.iconPath = values[0] ?? null;
    if (key === "include") d.include.push(...values);
  }
  return d;
}

/// The `Options` the Rust golden harness compiles the case in `dir` with.
export function caseOptions(dir: string, src: string): Options {
  const d = directives(src);
  return new Options({
    path: join(dir, "input.kash"),
    allowPath: d.allowPath,
    iconPath: d.iconPath === null ? [] : [join(dir, d.iconPath)],
    include: d.include.map((p) => join(dir, p)),
  });
}
