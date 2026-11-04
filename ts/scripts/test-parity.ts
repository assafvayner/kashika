/// Lists the Rust tests (`#[test]` functions under `crates/*/tests` and `crates/*/src`) that
/// have no TypeScript test of the same name under `ts/test`. Exits 1 when any is missing.

import { readFileSync, readdirSync, statSync } from "node:fs";
import { join, relative } from "node:path";

const TS = join(import.meta.dirname, "..");
const ROOT = join(TS, "..");

/// Rust files whose tests are not ported, with the reason.
const SKIP_FILES: ReadonlyMap<string, string> = new Map([
  ["crates/kashika-cli/tests/packaging.rs", "checks the crates.io package contents"],
  ["crates/kashika-cli/tests/golden/coverage.rs", "golden coverage map, not ported"],
]);

/// Crates whose tests another task ports.
const SKIP_CRATES: ReadonlyMap<string, string> = new Map([
  ["crates/kashika-route", "ported with the route library (ts/src/route, ts/test/route)"],
]);

function walk(dir: string, ext: string, out: string[] = []): string[] {
  for (const name of readdirSync(dir).sort()) {
    const p = join(dir, name);
    const s = statSync(p);
    if (s.isDirectory()) {
      if (name !== "node_modules" && name !== "target") walk(p, ext, out);
    } else if (name.endsWith(ext)) {
      out.push(p);
    }
  }
  return out;
}

interface RustTest {
  name: string;
  file: string;
  debugOnly: boolean;
}

/// Every `#[test]` function in `src`, and whether it only runs with `debug_assertions`.
function rustTests(file: string): RustTest[] {
  const src = readFileSync(file, "utf8");
  const out: RustTest[] = [];
  const re = /#\[test\]((?:\s*(?:#\[[^\]]*\]|\/\/[^\n]*))*)\s*(?:pub\s+)?(?:async\s+)?fn\s+(\w+)/g;
  for (const m of src.matchAll(re)) {
    out.push({
      name: m[2]!,
      file: relative(ROOT, file),
      debugOnly: m[1]!.includes("debug_assertions"),
    });
  }
  return out;
}

/// The names of `test(...)`/`it(...)` calls, with modifiers such as `.skip`, `.concurrent`
/// or `.skipIf(cond)`.
function tsTests(file: string): string[] {
  const src = readFileSync(file, "utf8");
  const re =
    /\b(?:test|it)(?:\.(?:skip|only|concurrent|sequential|todo|fails)|\.(?:skipIf|runIf)\([^)]*\))*\(\s*(["'`])((?:\\.|(?!\1).)*)\1/g;
  return [...src.matchAll(re)].map((m) => m[2]!);
}

const crates = join(ROOT, "crates");
const rust: RustTest[] = [];
for (const c of readdirSync(crates).sort()) {
  for (const sub of ["src", "tests"]) {
    const dir = join(crates, c, sub);
    if (statSync(dir, { throwIfNoEntry: false })?.isDirectory()) {
      for (const f of walk(dir, ".rs")) rust.push(...rustTests(f));
    }
  }
}

const ported = new Set(walk(join(TS, "test"), ".ts").flatMap(tsTests));

const skipped = new Map<string, number>();
const missing: RustTest[] = [];
for (const t of rust) {
  const crate = t.file.split("/").slice(0, 2).join("/");
  const why = SKIP_FILES.get(t.file) ?? SKIP_CRATES.get(crate) ?? (t.debugOnly ? "debug_assertions only" : null);
  if (why !== null) {
    skipped.set(why, (skipped.get(why) ?? 0) + 1);
    continue;
  }
  if (!ported.has(t.name)) missing.push(t);
}

console.log(`${rust.length} Rust tests, ${ported.size} TypeScript test names`);
for (const [why, n] of skipped) console.log(`skipped ${n}: ${why}`);
if (missing.length === 0) {
  console.log("every other Rust test has a TypeScript test of the same name");
} else {
  console.log(`${missing.length} Rust tests have no TypeScript test of the same name:`);
  for (const t of missing) console.log(`  ${t.file}: ${t.name}`);
  process.exitCode = 1;
}
