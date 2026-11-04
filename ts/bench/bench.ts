import { readFileSync, readdirSync, statSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { join, relative } from "node:path";
import { spawnSync } from "node:child_process";

import { render as tsRender } from "../src/wasm-api.js";

const ROOT = join(import.meta.dirname, "../..");
const WASM_PKG = join(ROOT, "crates/kashika-wasm/pkg/kashika_wasm.js");
const ITERATIONS = Number(process.env.BENCH_ITERATIONS ?? 20);
const WARMUP = 3;

interface Wasm {
  render(src: string, force: boolean): string;
  version(): string;
}

function loadWasm(): Wasm {
  const require = createRequire(import.meta.url);
  try {
    return require(WASM_PKG) as Wasm;
  } catch (e) {
    throw new Error(
      `cannot load ${WASM_PKG}; build it with: wasm-pack build crates/kashika-wasm --target nodejs\n${String(e)}`,
    );
  }
}

function kashFiles(dir: string, out: string[] = []): string[] {
  for (const name of readdirSync(dir).sort()) {
    const p = join(dir, name);
    if (statSync(p).isDirectory()) kashFiles(p, out);
    else if (name.endsWith(".kash")) out.push(p);
  }
  return out;
}

/// Examples and golden inputs that compile to a diagram on both sides without file assets
/// (the wasm build has no file system, so those report `file` errors on both sides anyway).
function corpus(): string[] {
  return [...kashFiles(join(ROOT, "examples")), ...kashFiles(join(ROOT, "tests/golden"))].filter(
    (p) => p.endsWith("input.kash") || relative(ROOT, p).startsWith("examples"),
  );
}

interface Row {
  file: string;
  bytes: number;
  svgBytes: number;
  equal: boolean;
  wasmMedian: number;
  wasmP90: number;
  tsMedian: number;
  tsP90: number;
}

function timeIt(f: () => void): number[] {
  for (let i = 0; i < WARMUP; i++) f();
  const samples: number[] = [];
  for (let i = 0; i < ITERATIONS; i++) {
    const t = process.hrtime.bigint();
    f();
    samples.push(Number(process.hrtime.bigint() - t) / 1e6);
  }
  return samples.sort((a, b) => a - b);
}

function quantile(sorted: number[], q: number): number {
  return sorted[Math.min(sorted.length - 1, Math.floor(q * sorted.length))]!;
}

function fmt(ms: number): string {
  return ms.toFixed(2);
}

/// Time from process start to the first `render` result, in a fresh process per implementation.
function coldStart(which: "wasm" | "ts", file: string): number {
  const script =
    which === "wasm"
      ? `const t=process.hrtime.bigint();const w=require(${JSON.stringify(WASM_PKG)});w.render(require("fs").readFileSync(${JSON.stringify(file)},"utf8"),false);console.log(Number(process.hrtime.bigint()-t)/1e6)`
      : `const t=process.hrtime.bigint();const {render}=await import(${JSON.stringify(join(import.meta.dirname, "../src/wasm-api.ts"))});render(require("fs").readFileSync(${JSON.stringify(file)},"utf8"),false);console.log(Number(process.hrtime.bigint()-t)/1e6)`;
  const args =
    which === "wasm"
      ? ["-e", script]
      : ["--import", "tsx/esm", "--input-type=module", "-e", `const require=(await import("node:module")).createRequire(import.meta.url);${script}`];
  const run = spawnSync(process.execPath, args, { cwd: join(ROOT, "ts"), encoding: "utf8" });
  if (run.status !== 0) throw new Error(`${which} cold start failed:\n${run.stderr}`);
  return Number(run.stdout.trim());
}

function main(): void {
  const wasm = loadWasm();
  const files = corpus();
  const rows: Row[] = [];
  const mismatches: string[] = [];
  let wasmTotal = 0;
  let tsTotal = 0;
  for (const file of files) {
    const src = readFileSync(file, "utf8");
    const want = JSON.parse(wasm.render(src, false)) as { svg: string | null };
    const got = tsRender(src, false);
    if (want.svg === null) continue;
    const equal = got.svg === want.svg;
    if (!equal) mismatches.push(relative(ROOT, file));
    const w = timeIt(() => wasm.render(src, false));
    const t = timeIt(() => tsRender(src, false));
    wasmTotal += quantile(w, 0.5);
    tsTotal += quantile(t, 0.5);
    rows.push({
      file: relative(ROOT, file),
      bytes: Buffer.byteLength(src),
      svgBytes: Buffer.byteLength(want.svg),
      equal,
      wasmMedian: quantile(w, 0.5),
      wasmP90: quantile(w, 0.9),
      tsMedian: quantile(t, 0.5),
      tsP90: quantile(t, 0.9),
    });
  }
  const upload = join(ROOT, "examples/upload/upload.kash");
  const cold = { wasm: coldStart("wasm", upload), ts: coldStart("ts", upload) };

  const lines: string[] = [];
  lines.push("# kashika-wasm vs TypeScript: `render(src, false)`");
  lines.push("");
  lines.push(
    `Node ${process.version}, ${process.platform} ${process.arch}, wasm ${wasm.version()}, ${ITERATIONS} iterations after ${WARMUP} warm-ups, median and p90 wall time in ms.`,
  );
  lines.push("");
  lines.push(`Diagrams compared: ${rows.length}. SVG byte-identical: ${rows.length - mismatches.length}.`);
  if (mismatches.length > 0) {
    lines.push("");
    lines.push("Differing (expected: cases embedding user SVG files, which the TS build does not pass through usvg):");
    for (const m of mismatches) lines.push(`- \`${m}\``);
  }
  lines.push("");
  lines.push("| file | src bytes | svg bytes | wasm median | wasm p90 | ts median | ts p90 | ts/wasm |");
  lines.push("|---|---:|---:|---:|---:|---:|---:|---:|");
  for (const r of rows) {
    lines.push(
      `| \`${r.file}\`${r.equal ? "" : " (differs)"} | ${r.bytes} | ${r.svgBytes} | ${fmt(r.wasmMedian)} | ${fmt(r.wasmP90)} | ${fmt(r.tsMedian)} | ${fmt(r.tsP90)} | ${(r.tsMedian / r.wasmMedian).toFixed(2)} |`,
    );
  }
  lines.push("");
  lines.push(`Sum of medians: wasm ${fmt(wasmTotal)} ms, ts ${fmt(tsTotal)} ms (ratio ${(tsTotal / wasmTotal).toFixed(2)}).`);
  lines.push("");
  lines.push(
    `Cold start (fresh process, load + first render of \`examples/upload/upload.kash\`): wasm ${fmt(cold.wasm)} ms, ts ${fmt(cold.ts)} ms (the TS number includes tsx transpilation of the sources and parsing the four fonts).`,
  );
  lines.push("");
  const report = lines.join("\n");
  writeFileSync(join(import.meta.dirname, "results.md"), report);
  console.log(report);
}

main();
