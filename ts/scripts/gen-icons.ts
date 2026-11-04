import { execFileSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const ts = join(dirname(fileURLToPath(import.meta.url)), "..");
const repo = join(ts, "..");
const iconsDir = join(repo, "crates/kashika-model/icons");

/// Parses `name = ["a", "b"]` like `build.rs`; blank lines and `#` comments give `null`.
function parseLine(raw: string): [string, string[]] | null {
  const line = raw.trim();
  if (line === "" || line.startsWith("#")) return null;
  const eq = line.indexOf("=");
  if (eq < 0) throw new Error(`icons/keywords.toml: expected \`name = [...]\`: ${line}`);
  const list = line.slice(eq + 1).trim();
  if (!list.startsWith("[") || !list.endsWith("]")) {
    throw new Error(`icons/keywords.toml: expected a [...] list: ${line}`);
  }
  const words = list
    .slice(1, -1)
    .split(",")
    .map((w) => w.trim().replace(/^"+|"+$/g, ""))
    .filter((w) => w !== "");
  return [line.slice(0, eq).trim(), words];
}

const toml = readFileSync(join(iconsDir, "keywords.toml"), "utf8");
const keywords = toml
  .split("\n")
  .map((l) => (l.endsWith("\r") ? l.slice(0, -1) : l))
  .map(parseLine)
  .filter((x): x is [string, string[]] => x !== null);

const out = execFileSync("cargo", ["run", "-q", "-p", "kashika-model", "--example", "dump_icons"], {
  cwd: repo,
  encoding: "utf8",
  maxBuffer: 256 * 1024 * 1024,
  stdio: ["ignore", "pipe", "inherit"],
});
const dumped = JSON.parse(out) as Record<string, Record<string, unknown>>;
const bundled: Record<string, unknown> = {};
for (const [name, entry] of Object.entries(dumped)) {
  bundled[name] = { ...entry, source: readFileSync(join(iconsDir, `${name}.svg`), "utf8") };
}
writeFileSync(join(ts, "src/model/icons/bundled.json"), JSON.stringify(bundled, null, 2) + "\n");

let kw = "export const ICON_KEYWORDS: readonly (readonly [string, readonly string[]])[] = [\n";
for (const [name, words] of keywords) {
  kw += `  [${JSON.stringify(name)}, [${words.map((w) => JSON.stringify(w)).join(", ")}]],\n`;
}
kw += "];\n";
writeFileSync(join(ts, "src/model/icons/keywords.ts"), kw);
