import { mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { expect } from "vitest";

import { check } from "../../../src/check/index.js";
import { type Diagnostic } from "../../../src/diag/index.js";
import { Options, type Scene, compile, compileWith } from "../../../src/model/index.js";

export const HEAD = 'diagram "t" size (800,600)\n';

/// Compiles a full source file, asserting that it has no errors.
export function sceneSrc(src: string): Scene {
  const [s, d] = compile(src);
  expect(d).toEqual([]);
  return s;
}

export function scene(body: string): Scene {
  return sceneSrc(`${HEAD}${body}`);
}

/// The findings with `code` that `check` reports for `body`.
export function lint(code: string, body: string): Diagnostic[] {
  return check(scene(body)).filter((d) => d.code === code);
}

/// A PNG header with the given natural size; enough for size probing.
export function png(w: number, h: number): Uint8Array {
  const b = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0, 0, 0, 13, 0x49, 0x48, 0x44, 0x52];
  const be = (v: number) => [(v >>> 24) & 0xff, (v >>> 16) & 0xff, (v >>> 8) & 0xff, v & 0xff];
  b.push(...be(w), ...be(h), 8, 6, 0, 0, 0, 0, 0, 0, 0);
  return Uint8Array.from(b);
}

/// A fresh temporary directory, like `tempfile::tempdir`.
export function tempdir(): string {
  return mkdtempSync(join(tmpdir(), "kashika-check-"));
}

/// Like `lint`, for a body whose files live in `dir`. Writes `logo.png` (200x100) there.
export function lintIn(dir: string, code: string, body: string): Diagnostic[] {
  writeFileSync(join(dir, "logo.png"), png(200, 100));
  const opts = new Options({ path: join(dir, "t.kash") });
  const [s, d] = compileWith(`${HEAD}${body}`, opts);
  expect(d).toEqual([]);
  return check(s).filter((x) => x.code === code);
}
