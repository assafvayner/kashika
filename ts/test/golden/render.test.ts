import { existsSync, mkdtempSync, readFileSync, rmSync, statSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

import { type Json } from "../../src/diag/json.js";
import { diagnostics } from "../../src/geom/dump.js";
import { jpegSize, kashikaAsync, pngSize, webpSize } from "../support/cli.js";
import { Case, GOLDEN, type Output, checkOutputs, discover, textOutput } from "../support/golden.js";
import { DIVERGENT, USER_SVG, WEBP } from "./divergent.js";

const UNSUPPORTED_WEBP = "webp output is not supported by the TypeScript build";

function args(c: Case, out: string): [string[], Record<string, string | null>] {
  const a = ["render", "input.kash", "-o", out];
  const env: Record<string, string | null> = { KASHIKA_ICON_PATH: null };
  if (c.iconPath !== null) env["KASHIKA_ICON_PATH"] = join(c.dir, c.iconPath);
  for (const p of c.include) a.push("--include", join(c.dir, p));
  if (c.force) a.push("--force");
  if (c.debug) a.push("--debug");
  if (c.allowPath) a.push("--allow-path");
  return [a, env];
}

function size(ext: string, b: Uint8Array): [number, number] {
  return ext === "png" ? pngSize(b) : ext === "jpg" ? jpegSize(b) : webpSize(b);
}

/// Runs `kashika render` in the case directory for `out.svg` and each `// raster:` output,
/// like `run_render` in the Rust harness. Rasters are compared by their dimensions; WebP,
/// which the TypeScript build cannot write, must fail with its tool error.
async function runRender(c: Case): Promise<[Output[], string[]]> {
  const tmp = mkdtempSync(join(tmpdir(), "kashika-golden-"));
  try {
    const out: Output[] = [];
    const skip: string[] = [];
    for (const ext of ["svg", ...c.raster]) {
      const path = join(tmp, `out.${ext}`);
      const [a, env] = args(c, path);
      if (ext === "svg") a.push("--format", "json", "--max", "1000", "--show-suppressed");
      const run = await kashikaAsync(a, { cwd: c.dir, env });
      if (ext === "webp") {
        expect(run.code, run.stderr).toBe(3);
        expect(run.stderr).toContain(UNSUPPORTED_WEBP);
        skip.push("out.webp");
        continue;
      }
      const ok = run.code === 0 || run.code === 1 || (c.force && run.code === 2);
      if (!ok || !(statSync(path, { throwIfNoEntry: false })?.isFile() ?? false)) {
        throw new Error(
          `${c.name}: \`kashika render -o out.${ext}\` exited ${run.code} without writing output\nstdout:\n${run.stdout}\nstderr:\n${run.stderr}`,
        );
      }
      if (ext === "svg") {
        const report = JSON.parse(run.stdout) as { diagnostics?: Json[] };
        out.push(textOutput("diagnostics.txt", diagnostics(report.diagnostics ?? [])));
        out.push({ name: "out.svg", bytes: new Uint8Array(readFileSync(path)) });
      } else {
        const expected = join(c.dir, `out.${ext}`);
        expect(existsSync(expected), `out.${ext} is missing`).toBe(true);
        expect(size(ext, new Uint8Array(readFileSync(path)))).toEqual(
          size(ext, new Uint8Array(readFileSync(expected))),
        );
        skip.push(`out.${ext}`);
      }
    }
    return [out, skip];
  } finally {
    rmSync(tmp, { recursive: true, force: true });
  }
}

const dirs = discover(join(GOLDEN, "render"));

describe("golden render cases", () => {
  it("finds the cases", () => {
    expect(dirs.length).toBe(62);
  });
  it("lists only existing cases as divergent", () => {
    const names = new Set(dirs.map((d) => Case.load(GOLDEN, d).name));
    expect([...DIVERGENT.keys()].filter((n) => !names.has(n))).toEqual([]);
  });
  it("lists every WebP case as divergent", () => {
    const webp = dirs.map((d) => Case.load(GOLDEN, d)).filter((c) => c.raster.includes("webp"));
    expect(webp.filter((c) => DIVERGENT.get(c.name) !== WEBP).map((c) => c.name)).toEqual([]);
  });
});

describe.concurrent("golden render outputs", () => {
  for (const dir of dirs) {
    const c = Case.load(GOLDEN, dir);
    const reason = DIVERGENT.get(c.name);
    it(reason === undefined ? c.name : `${c.name} (divergent: ${reason})`, async () => {
      const [outputs, skip] = await runRender(c);
      if (reason !== USER_SVG) {
        checkOutputs(c, outputs, skip);
        return;
      }
      checkOutputs(
        c,
        outputs.filter((o) => o.name !== "out.svg"),
        [...skip, "out.svg"],
      );
      const svg = outputs.find((o) => o.name === "out.svg")!;
      const want = new Uint8Array(readFileSync(join(c.dir, "out.svg")));
      if (svg.bytes.length === want.length && svg.bytes.every((b, i) => b === want[i])) {
        throw new Error(`${c.name} matches out.svg: remove it from divergent.ts`);
      }
    });
  }
});
