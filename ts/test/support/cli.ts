import { execFile, spawnSync } from "node:child_process";
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { onTestFinished } from "vitest";

export const BIN = join(import.meta.dirname, "../../bin/kashika.js");
export const ROOT = join(import.meta.dirname, "../../..");
export const CLI_CRATE = join(ROOT, "crates/kashika-cli");

export function fixture(name: string): string {
  return join(CLI_CRATE, "tests/fixtures", name);
}

export interface Run {
  code: number;
  stdout: string;
  stderr: string;
}

export interface RunOpts {
  cwd?: string;
  /// Variables to set; `null` removes one.
  env?: Record<string, string | null>;
  input?: string;
}

function environment(env: RunOpts["env"]): NodeJS.ProcessEnv {
  const out: NodeJS.ProcessEnv = { ...process.env };
  for (const [k, v] of Object.entries(env ?? {})) {
    if (v === null) delete out[k];
    else out[k] = v;
  }
  return out;
}

/// Runs `kashika` with `args`, like `assert_cmd`'s `Command::cargo_bin("kashika")`; stdin is
/// `input`, or empty.
export function kashika(args: readonly string[], o: RunOpts = {}): Run {
  const r = spawnSync(process.execPath, [BIN, ...args], {
    cwd: o.cwd,
    env: environment(o.env),
    input: o.input ?? "",
    encoding: "utf8",
    maxBuffer: 256 * 1024 * 1024,
  });
  if (r.error) throw r.error;
  return { code: r.status ?? -1, stdout: r.stdout, stderr: r.stderr };
}

/// `kashika` without blocking, for tests that run concurrently.
export function kashikaAsync(args: readonly string[], o: RunOpts = {}): Promise<Run> {
  return new Promise((resolve, reject) => {
    const child = execFile(
      process.execPath,
      [BIN, ...args],
      { cwd: o.cwd, env: environment(o.env), encoding: "utf8", maxBuffer: 256 * 1024 * 1024 },
      (error, stdout, stderr) => {
        if (error !== null && typeof error.code !== "number") {
          reject(error);
          return;
        }
        resolve({ code: error === null ? 0 : (error.code as number), stdout, stderr });
      },
    );
    child.stdin?.end(o.input ?? "");
  });
}

/// A temporary directory removed when the test finishes.
export function tempdir(): string {
  const d = mkdtempSync(join(tmpdir(), "kashika-test-"));
  onTestFinished(() => rmSync(d, { recursive: true, force: true }));
  return d;
}

export function pngSize(b: Uint8Array): [number, number] {
  const v = new DataView(b.buffer, b.byteOffset, b.byteLength);
  return [v.getUint32(16), v.getUint32(20)];
}

export function jpegSize(b: Uint8Array): [number, number] {
  let i = 2;
  while (i + 9 < b.length) {
    if (b[i] !== 0xff) throw new Error(`no marker at ${i}`);
    const marker = b[i + 1]!;
    const u = (j: number) => (b[j]! << 8) | b[j + 1]!;
    if (marker >= 0xc0 && marker <= 0xc3) {
      return [u(i + 7), u(i + 5)];
    }
    i += 2 + u(i + 2);
  }
  throw new Error("no SOF marker");
}

export function webpSize(b: Uint8Array): [number, number] {
  const tag = String.fromCharCode(...b.subarray(12, 16));
  if (tag !== "VP8L") throw new Error(`not lossless WebP: ${tag}`);
  const bits = (b[21]! | (b[22]! << 8) | (b[23]! << 16) | (b[24]! << 24)) >>> 0;
  return [(bits & 0x3fff) + 1, ((bits >>> 14) & 0x3fff) + 1];
}
