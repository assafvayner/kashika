import { type Dirent, readFileSync, readdirSync, statSync } from "node:fs";
import { extname, join, parse as parsePath, relative } from "node:path";

import { Float, type Json, toJsonString } from "../diag/json.js";
import { cmpStr, displayF64 } from "../diag/rust.js";
import { BUNDLED_ICONS, ICON_KEYWORDS, SHAPES, probe } from "../model/index.js";
import { bundledIcon } from "../model/icons.js";
import { parse } from "../syntax/index.js";
import { type Format, iconPath, print, eprint, toolError } from "./main.js";
import * as term from "./term.js";

export const SCHEMA = "kashika.icons/1";

export interface IconsArgs {
  /// Directories to scan (default: the KASHIKA_ICON_PATH entries)
  dirs: string[];
  search: string | null;
  format: Format;
}

function isDir(p: string): boolean {
  try {
    return statSync(p).isDirectory();
  } catch {
    return false;
  }
}

function isFile(p: string): boolean {
  try {
    return statSync(p).isFile();
  } catch {
    return false;
  }
}

export function run(a: IconsArgs): number {
  const dirs = a.dirs.length === 0 ? iconPath() : a.dirs;
  const bad = dirs.find((d) => !isDir(d));
  if (bad !== undefined) {
    return toolError(a.format, SCHEMA, `${bad} is not a directory`);
  }
  const entries = list(dirs, a.search);
  print(a.format === "text" ? term.text(text(entries)) : json(entries));
  if (a.search !== null && entries.length === 0) {
    eprint(`kashika: no icons or shapes match \`${term.line(a.search)}\`\n`);
    return 1;
  }
  return 0;
}

export interface Entry {
  name: string;
  path: string;
  size: [number, number] | null;
  library: string;
  matchedKeyword: string | null;
  keywords: readonly string[];
}

/// Bundled icons and built-in shapes first, then every image and `.kash` shape under `dirs`, in sorted order.
export function list(dirs: readonly string[], search: string | null): Entry[] {
  let out: Entry[] = BUNDLED_ICONS.map((name) => {
    const size = probe(new TextEncoder().encode(bundledIcon(name)!.source), true);
    return {
      name,
      path: `builtin:${name}`,
      size: size.ok ? size.value : null,
      library: "kashika",
      matchedKeyword: null,
      keywords: keywords(name),
    };
  });
  out.push(
    ...SHAPES.map((s) => ({
      name: s.name,
      path: `builtin:shape:${s.name}`,
      size: null,
      library: "kashika",
      matchedKeyword: null,
      keywords: s.keywords,
    })),
  );
  for (const dir of dirs) {
    const files: string[] = [];
    walk(dir, files);
    files.sort(cmpPath);
    for (const f of files) {
      entries(dir, f, out);
    }
  }
  if (search !== null) {
    const t = search.toLowerCase();
    out = out.filter((e) => {
      if (e.name.toLowerCase().includes(t) || e.path.toLowerCase().includes(t)) {
        return true;
      }
      e.matchedKeyword = e.keywords.find((k) => k.includes(t)) ?? null;
      return e.matchedKeyword !== null;
    });
  }
  return out;
}

/// `Path` order: component by component.
function cmpPath(a: string, b: string): number {
  const ca = a.split("/").filter((c) => c !== "");
  const cb = b.split("/").filter((c) => c !== "");
  for (let i = 0; i < Math.min(ca.length, cb.length); i++) {
    const c = cmpStr(ca[i]!, cb[i]!);
    if (c !== 0) return c;
  }
  return ca.length - cb.length;
}

function keywords(name: string): readonly string[] {
  return ICON_KEYWORDS.find(([n]) => n === name)?.[1] ?? [];
}

function walk(dir: string, out: string[]): void {
  let rd: Dirent[];
  try {
    rd = readdirSync(dir, { withFileTypes: true });
  } catch {
    return;
  }
  for (const e of rd) {
    const p = join(dir, e.name);
    if (e.name.startsWith(".")) {
      continue;
    }
    if (e.isDirectory()) {
      walk(p, out);
    } else if (!e.isSymbolicLink() || isFile(p)) {
      out.push(p);
    }
  }
}

function entries(dir: string, f: string, out: Entry[]): void {
  const rel = relative(dir, f).replaceAll("\\", "/");
  const library = dir;
  const ext = extname(f).slice(1).toLowerCase();
  const stem = parsePath(f).name;
  switch (ext) {
    case "svg":
    case "png":
    case "jpg":
    case "jpeg": {
      let size: [number, number] | null = null;
      try {
        const r = probe(new Uint8Array(readFileSync(f)), ext === "svg");
        if (r.ok) size = r.value;
      } catch {
        size = null;
      }
      out.push({ name: stem, path: rel, size, library, matchedKeyword: null, keywords: [] });
      break;
    }
    case "kash": {
      let src: string;
      try {
        src = new TextDecoder("utf-8", { fatal: true }).decode(readFileSync(f));
      } catch {
        return;
      }
      const { file } = parse(src);
      for (const stmt of file.stmts) {
        if (stmt.kind === "Shape") {
          out.push({
            name: stmt.value.name.name,
            path: rel,
            size: null,
            library,
            matchedKeyword: null,
            keywords: [],
          });
        }
      }
      break;
    }
    default:
  }
}

function pad(s: string, width: number): string {
  const n = Array.from(s).length;
  return n >= width ? s : s + " ".repeat(width - n);
}

export function text(entries: readonly Entry[]): string {
  let out = "";
  for (const e of entries) {
    const size = e.size === null ? "-" : `${displayF64(e.size[0])}x${displayF64(e.size[1])}`;
    const note = e.matchedKeyword === null ? "" : `  (matches "${e.matchedKeyword}")`;
    out += `${pad(e.name, 16)} ${pad(e.path, 32)} ${pad(size, 9)} ${e.library}${note}\n`;
  }
  return out;
}

export function json(entries: readonly Entry[]): string {
  const icons = entries.map((e) => {
    const o: { [key: string]: Json } = {
      name: e.name,
      path: e.path,
      size: e.size === null ? null : [new Float(e.size[0]), new Float(e.size[1])],
      library: e.library,
    };
    if (e.matchedKeyword !== null) o["matched_keyword"] = e.matchedKeyword;
    return o;
  });
  return `${toJsonString({ schema: SCHEMA, icons })}\n`;
}
