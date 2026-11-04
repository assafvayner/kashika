import { readFileSync, readdirSync, statSync } from "node:fs";
import { join, relative } from "node:path";

import { lines } from "../../src/diag/rust.js";

/// Every file name the harness writes into a case directory.
export const OUTPUTS: readonly string[] = [
  "ast.txt",
  "scene.json",
  "diagnostics.txt",
  "out.svg",
  "out.png",
  "out.jpg",
  "out.webp",
];

const RASTERS: readonly string[] = ["png", "jpg", "webp"];

export type Suite = "parse" | "render";

export const GOLDEN = join(import.meta.dirname, "../../../tests/golden");

export function goldenRoot(): string {
  return GOLDEN;
}

function isFile(p: string): boolean {
  return statSync(p, { throwIfNoEntry: false })?.isFile() ?? false;
}

/// Every directory under `root` holding an `input.kash`, sorted by name. Case directories
/// are not searched further, so auxiliary subdirectories (`lib/`, `icons/`) are never cases.
export function discover(root: string): string[] {
  const found: string[] = [];
  const stack = [root];
  while (stack.length > 0) {
    const dir = stack.pop()!;
    if (isFile(join(dir, "input.kash"))) {
      found.push(dir);
      continue;
    }
    let names: string[];
    try {
      names = readdirSync(dir);
    } catch {
      continue;
    }
    for (const n of names) {
      const p = join(dir, n);
      if (statSync(p, { throwIfNoEntry: false })?.isDirectory()) stack.push(p);
    }
  }
  return found.sort((a, b) => cmpComponents(a, b));
}

function cmpComponents(a: string, b: string): number {
  const x = a.split("/");
  const y = b.split("/");
  for (let i = 0; i < Math.min(x.length, y.length); i++) {
    if (x[i] !== y[i]) return x[i]! < y[i]! ? -1 : 1;
  }
  return x.length - y.length;
}

export class Case {
  spec: string[] = [];
  dumpAst = false;
  dumpScene = false;
  force = false;
  debug = false;
  allowPath = false;
  raster: string[] = [];
  iconPath: string | null = null;
  /// `--include` files, relative to the case directory.
  include: string[] = [];
  /// The first comment line that is not a directive.
  description = "";

  private constructor(
    /// Path relative to `tests/golden`, with `/` separators, e.g. `parse/lexical/bom`.
    readonly name: string,
    readonly dir: string,
    readonly suite: Suite,
    readonly src: string,
  ) {}

  /// Loads the case in `dir`, or throws with the reason it is malformed.
  static load(root: string, dir: string): Case {
    const name = relative(root, dir).split("\\").join("/");
    const top = name.split("/")[0];
    let suite: Suite;
    if (top === "parse") suite = "parse";
    else if (top === "render") suite = "render";
    else throw new Error(`${name}: cases live under parse/ or render/`);
    let src: string;
    try {
      src = readFileSync(join(dir, "input.kash"), "utf8");
    } catch (e) {
      throw new Error(`${name}: cannot read input.kash: ${String(e)}`);
    }
    const c = new Case(name, dir, suite, src);
    c.readDirectives();
    return c;
  }

  /// Reads the `// key: value` lines at the top of `input.kash`. A leading comment line
  /// whose text before `:` is one lowercase word is a directive and must be known.
  private readDirectives(): void {
    let src = this.src;
    while (src.startsWith("\u{feff}")) src = src.slice(1);
    for (const line of lines(src)) {
      if (!line.startsWith("// ")) break;
      const text = line.slice(3);
      const at = text.indexOf(": ");
      const key = at < 0 ? null : text.slice(0, at);
      if (key === null || !/^[a-z-]*$/.test(key)) {
        if (this.description === "") this.description = text;
        continue;
      }
      const values = text
        .slice(at + 2)
        .split(/[, ]/)
        .filter((v) => v !== "");
      const suite = this.suite;
      if (key === "spec") {
        this.spec = values;
      } else if (key === "dump" && suite === "parse") {
        for (const v of values) {
          if (v === "ast") this.dumpAst = true;
          else if (v === "scene") this.dumpScene = true;
          else throw new Error(`${this.name}: unknown dump \`${v}\``);
        }
      } else if (key === "flags") {
        for (const v of values) {
          if (v === "allow-path") this.allowPath = true;
          else if (v === "force" && suite === "render") this.force = true;
          else if (v === "debug" && suite === "render") this.debug = true;
          else throw new Error(`${this.name}: unknown flag \`${v}\``);
        }
      } else if (key === "raster" && suite === "render") {
        for (const v of values) {
          if (!RASTERS.includes(v)) throw new Error(`${this.name}: unknown raster \`${v}\``);
        }
        this.raster = values;
      } else if (key === "icon-path") {
        this.iconPath = values[0] ?? null;
      } else if (key === "include") {
        this.include.push(...values);
      } else {
        throw new Error(`${this.name}: unknown directive \`${key}\` for this suite`);
      }
    }
    if (this.spec.length === 0) {
      throw new Error(`${this.name}: input.kash must start with \`// spec: <sections>\``);
    }
  }
}
