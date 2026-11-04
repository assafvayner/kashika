import { closeSync, openSync, readdirSync, readSync, realpathSync, statSync } from "node:fs";
import { isAbsolute, sep } from "node:path";

import { didYouMean } from "../diag/distance.js";
import { cmpStr, err, ok, type Result } from "../diag/rust.js";
import { bundled, BUNDLED_ICONS, ICON_KEYWORDS } from "./icons.js";
import { SHAPES } from "./shapes.js";

/// How `compileWith` finds files.
export class Options {
  /// The input file. Its directory is the project root and the base for relative paths;
  /// `null` (stdin) uses the current directory.
  path: string | null = null;
  /// Allows paths outside the project root (`--allow-path`).
  allowPath = false;
  /// Extra search directories, from `KASHIKA_ICON_PATH`.
  iconPath: string[] = [];
  /// Single files outside the project root that may be read (`--include`). Only the
  /// file itself is allowed, not its directory.
  include: string[] = [];

  constructor(init: Partial<Options> = {}) {
    Object.assign(this, init);
  }
}

export type Found = { kind: "file"; path: string } | { kind: "bundled"; name: string };

export class FileError {
  constructor(
    public message: string,
    public help: string | null = null,
  ) {}
}

export class Loader {
  private constructor(
    private readonly root_: string,
    private readonly main_: string | null,
    private readonly search: string[],
    private readonly include: string[],
    private readonly allowPath: boolean,
  ) {}

  static new(opts: Options): Loader {
    const parent = opts.path === null ? null : pathParent(opts.path);
    const dir = parent !== null && parent !== "" ? parent : ".";
    const root = canonicalize(dir) ?? dir;
    const main = opts.path === null ? null : canonicalize(opts.path);
    const search = opts.iconPath.map(canonicalize).filter((p): p is string => p !== null);
    const include = opts.include.map(canonicalize).filter((p): p is string => p !== null);
    return new Loader(root, main, search, include, opts.allowPath);
  }

  /// The project root: the input file's directory.
  root(): string {
    return this.root_;
  }

  /// The canonical path of the input file, if it has one.
  main(): string | null {
    return this.main_;
  }

  /// Adds the `icons=` directory (relative to the project root) to the front of the search path.
  addIconsDir(dir: string): Result<null, FileError> {
    const u = urlCheck(dir);
    if (!u.ok) return u;
    const c = canonicalize(pathJoin(this.root_, dir));
    if (c === null) return err(new FileError(`icon directory \`${dir}\` not found`));
    if (!isDir(c)) return err(new FileError(`\`${dir}\` is not a directory`));
    const inside = this.checkInside(c, dir);
    if (!inside.ok) return inside;
    this.search.unshift(c);
    return ok(null);
  }

  private checkInside(p: string, written: string): Result<null, FileError> {
    if (
      this.allowPath ||
      pathStartsWith(p, this.root_) ||
      this.search.some((s) => pathStartsWith(p, s)) ||
      this.include.some((f) => f === p)
    ) {
      return ok(null);
    }
    return err(
      new FileError(
        `\`${written}\` is outside the project root`,
        "move it under the diagram's directory, pass `--include <file>` for this file, add its directory to KASHIKA_ICON_PATH, or pass --allow-path",
      ),
    );
  }

  /// Finds `name` relative to `from` (the referencing file's directory), then on the
  /// search path. A hit that escapes the sandbox is an error.
  find(from: string, name: string): Result<Found, FileError> {
    const u = urlCheck(name);
    if (!u.ok) return u;
    for (const dir of this.dirs(from)) {
      const p = pathJoin(dir, name);
      if (isFile(p)) {
        let c: string;
        try {
          c = realpathSync(p);
        } catch (e) {
          return err(new FileError(`cannot read \`${name}\`: ${ioError(e)}`));
        }
        const inside = this.checkInside(c, name);
        if (!inside.ok) return inside;
        return ok({ kind: "file", path: c });
      }
    }
    if (!/[/\\.]/.test(name)) {
      const b = bundled(name);
      if (b !== null) return ok({ kind: "bundled", name: b[0] });
    }
    if (
      this.dirs(from).some((dir) => {
        const p = pathJoin(dir, name);
        return isDir(p) && this.allowed(p);
      })
    ) {
      return err(new FileError(`\`${name}\` is a directory, not a file`));
    }
    return err(new FileError(`file \`${name}\` not found`, this.suggest(from, name)));
  }

  /// Reads a found file, refusing one larger than `max` bytes. The file is opened once
  /// and read through a `max + 1` byte window, so a file that grows after the check
  /// still can't exceed the limit.
  read(found: Found, written: string, max: number): Result<Uint8Array, FileError> {
    if (found.kind === "bundled") {
      const b = bundled(found.name);
      if (b === null) throw new Error("the loader only returns bundled names");
      return ok(new TextEncoder().encode(b[1]));
    }
    const cannot = (e: unknown) => new FileError(`cannot read \`${written}\`: ${ioError(e)}`);
    let fd: number;
    try {
      fd = openSync(found.path, "r");
    } catch (e) {
      return err(cannot(e));
    }
    const limit = max + 1;
    const chunks: Uint8Array[] = [];
    let total = 0;
    try {
      while (total < limit) {
        const buf = new Uint8Array(Math.min(limit - total, 64 * 1024));
        const n = readSync(fd, buf, 0, buf.length, null);
        if (n === 0) break;
        chunks.push(buf.subarray(0, n));
        total += n;
      }
    } catch (e) {
      return err(cannot(e));
    } finally {
      closeSync(fd);
    }
    if (total > max) {
      return err(new FileError(`\`${written}\` is larger than ${showBytes(max)}`));
    }
    const bytes = new Uint8Array(total);
    let at = 0;
    for (const c of chunks) {
      bytes.set(c, at);
      at += c.length;
    }
    return ok(bytes);
  }

  private dirs(from: string): string[] {
    return [from, ...this.search];
  }

  /// Whether `p` exists inside the sandbox.
  private allowed(p: string): boolean {
    const c = canonicalize(p);
    return c !== null && this.checkInside(c, "").ok;
  }

  /// Near names for a missing file: files `find` would accept, never `name` itself.
  private suggest(from: string, name: string): string | null {
    const sub = pathParent(name) ?? "";
    let cands: string[] = [];
    for (const dir of this.dirs(from)) {
      const d = pathJoin(dir, sub);
      let entries: string[];
      try {
        entries = readdirSync(d);
      } catch {
        continue;
      }
      for (const e of entries) {
        const p = pathJoin(d, e);
        if (isFile(p) && this.allowed(p)) cands.push(pathJoin(sub, e));
      }
    }
    if (!name.includes("/")) cands.push(...BUNDLED_ICONS);
    cands = cands.filter((c) => c !== name);
    cands.sort(cmpStr);
    cands = cands.filter((c, i) => i === 0 || c !== cands[i - 1]);
    const s = didYouMean(name, cands);
    return s !== null ? `did you mean \`${s}\`?` : keywordHelp(name);
  }
}

/// For a bare name that is a search keyword: the bundled icon or built-in shape it finds.
function keywordHelp(name: string): string | null {
  if (/[/\\.]/.test(name)) return null;
  const w = name.replace(/[A-Z]/g, (c) => c.toLowerCase());
  const icon = ICON_KEYWORDS.find(([, words]) => words.includes(w));
  if (icon !== undefined) return `did you mean the bundled icon \`${icon[0]}\`?`;
  const s = SHAPES.find((s) => s.keywords.includes(w));
  return s === undefined
    ? null
    : `no icon is called \`${name}\`; the built-in shape \`${s.name}\` matches it: \`shape=${s.name}\``;
}

function showBytes(n: number): string {
  const MIB = 1024 * 1024;
  return n % MIB === 0 ? `${n / MIB} MiB` : `${n} bytes`;
}

function urlCheck(name: string): Result<null, FileError> {
  if (name.includes("://") || name.startsWith("data:")) {
    return err(
      new FileError(
        `\`${name}\` is not a local file; only local files are supported`,
        "download it and refer to it by a relative path",
      ),
    );
  }
  return ok(null);
}

function canonicalize(p: string): string | null {
  try {
    return realpathSync(p);
  } catch {
    return null;
  }
}

function isFile(p: string): boolean {
  try {
    return statSync(p, { throwIfNoEntry: false })?.isFile() ?? false;
  } catch {
    return false;
  }
}

function isDir(p: string): boolean {
  try {
    return statSync(p, { throwIfNoEntry: false })?.isDirectory() ?? false;
  } catch {
    return false;
  }
}

function isSep(c: string): boolean {
  return c === "/" || (sep === "\\" && c === "\\");
}

/// Rust's `Path::join`: an absolute `b` replaces `a`; nothing is normalized.
export function pathJoin(a: string, b: string): string {
  if (isAbsolute(b)) return b;
  if (a === "") return b;
  return isSep(a[a.length - 1]!) ? a + b : a + sep + b;
}

/// Rust's `Path::parent`: `null` for `""` and a root, `""` for a single relative component.
export function pathParent(p: string): string | null {
  let end = p.length;
  while (end > 0 && isSep(p[end - 1]!)) end--;
  if (end === 0) return null;
  let i = end;
  while (i > 0 && !isSep(p[i - 1]!)) i--;
  if (i === 0) return "";
  let j = i;
  while (j > 0 && isSep(p[j - 1]!)) j--;
  return j === 0 ? p.slice(0, 1) : p.slice(0, j);
}

/// Rust's `Path::starts_with` for canonical paths: whole components only.
function pathStartsWith(p: string, base: string): boolean {
  if (p === base) return true;
  return p.startsWith(isSep(base[base.length - 1] ?? "") ? base : base + sep);
}

const OS_ERRORS: Readonly<Record<string, [string, number]>> = {
  ENOENT: ["No such file or directory", 2],
  EACCES: ["Permission denied", 13],
  EISDIR: ["Is a directory", 21],
  ENOTDIR: ["Not a directory", 20],
  ELOOP: ["Too many levels of symbolic links", 62],
  EPERM: ["Operation not permitted", 1],
};

/// An I/O error as Rust's `io::Error` displays it.
function ioError(e: unknown): string {
  const code = (e as NodeJS.ErrnoException).code;
  const known = code === undefined ? undefined : OS_ERRORS[code];
  if (known !== undefined) {
    const errno = (e as NodeJS.ErrnoException).errno;
    return `${known[0]} (os error ${errno !== undefined ? Math.abs(errno) : known[1]})`;
  }
  return e instanceof Error ? e.message : String(e);
}
