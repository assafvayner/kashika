import { randomBytes } from "node:crypto";
import {
  chmodSync,
  closeSync,
  fsyncSync,
  openSync,
  readFileSync,
  readlinkSync,
  realpathSync,
  renameSync,
  statSync,
  unlinkSync,
  writeSync,
} from "node:fs";
import { basename, delimiter, dirname, join } from "node:path";

import { Ctx, checkBounded } from "../check/index.js";
import { type Deferred } from "../check/free.js";
import { type Diagnostic, order } from "../diag/diagnostic.js";
import { didYouMean } from "../diag/distance.js";
import { toJsonString } from "../diag/json.js";
import { type Result, err, ok, parseF64 } from "../diag/rust.js";
import { Rect } from "../geom/index.js";
import { Options, type Scene, compileFile, compileWith } from "../model/index.js";
import { TOOL_VERSION } from "../model/version.js";
import { renderSvgWith } from "../render/index.js";
import { type File } from "../syntax/index.js";
import { type App, type Command, type Matches, parse as parseArgs } from "./args.js";
import * as fmtCmd from "./fmt_cmd.js";
import * as icons from "./icons.js";
import * as inspect from "./inspect.js";
import * as measure from "./measure.js";
import * as output from "./output.js";
import * as refCmd from "./ref_cmd.js";
import * as report from "./report.js";
import * as routeCmd from "./route_cmd.js";
import * as term from "./term.js";

export type Format = "text" | "json";
type FailOn = "error" | "warn" | "never";

export function print(s: string): void {
  process.stdout.write(s);
}

export function eprint(s: string): void {
  process.stderr.write(s);
}

const FORMATS = ["text", "json"] as const;
const FILE_HELP = "Input .kash file, or `-` for stdin";
const STDIN_NAME = "File name to report for stdin input";
const ALLOW_PATH = "Allow file references outside the input file's directory";
const INCLUDE =
  "Allow this one file outside the input file's directory (repeatable; relative to the current directory)";

const usize = (s: string): string | null =>
  /^\+?[0-9]+$/.test(s) ? null : s === "" ? "cannot parse integer from empty string" : "invalid digit found in string";
const f64 = (s: string): string | null => (parseF64(s) === null ? "invalid float literal" : null);
const rectCheck = (s: string): string | null => {
  const r = parseRect(s);
  return r.ok ? null : r.error;
};
const nonNegativeCheck = (s: string): string | null => {
  const r = nonNegative(s);
  return r.ok ? null : r.error;
};
const atLeastOneCheck = (s: string): string | null => {
  const r = atLeastOne(s);
  return r.ok ? null : r.error;
};

const COMMON: Command["opts"] = [
  { long: "format", value: "FORMAT", possible: FORMATS, default: "text" },
  { long: "fail-on", value: "FAIL_ON", possible: ["error", "warn", "never"], default: "warn" },
  {
    long: "max",
    value: "MAX",
    help: "Maximum number of diagnostics to print",
    default: "50",
    check: usize,
  },
  { long: "stdin-name", value: "STDIN_NAME", help: STDIN_NAME },
  { long: "allow-path", help: ALLOW_PATH },
  { long: "include", value: "FILE", help: INCLUDE, multiple: true },
  { long: "show-suppressed", help: "Also list suppressed findings" },
  { long: "summary", help: "Also print the number of findings per diagnostic code" },
  {
    long: "focus",
    value: "FOCUS",
    help: "Only report warnings that involve these ids (comma-separated); errors are always reported",
    multiple: true,
    delimiter: ",",
  },
];

const FILE_ARG = { name: "FILE", help: FILE_HELP, required: true, max: 1 };

export const APP: App = {
  name: "kashika",
  version: TOOL_VERSION,
  about: "Diagrams you place exactly; kashika checks the geometry",
  afterHelp:
    "Exit codes: 0 clean; 1 warnings (check/render with --fail-on warn), `fmt --check` with changes, or `route`/`icons --search` with no result; 2 errors; 3 usage or I/O failure.",
  commands: [
    {
      name: "check",
      aliases: [],
      about: "Check a .kash file and report diagnostics",
      args: [FILE_ARG],
      opts: COMMON,
    },
    {
      name: "render",
      aliases: [],
      about: "Check a .kash file and render it to SVG, PNG, JPEG or WebP",
      args: [FILE_ARG],
      opts: [
        ...COMMON,
        {
          long: "output",
          short: "o",
          value: "OUTPUT",
          required: true,
          help: "Output file (.svg, .png, .jpg, .jpeg or .webp); an existing file is left untouched when there are errors, unless --force",
        },
        { long: "force", help: "Write output even when there are errors" },
        {
          long: "debug",
          help: "Overlay rulers, element ids, anchor dots and diagnostic markers",
        },
      ],
    },
    {
      name: "route",
      aliases: [],
      about: "Suggest ready-to-paste paths between two points; never edits the file",
      args: [
        FILE_ARG,
        {
          name: "REST",
          help: "`<from> <to>` points (e.g. `app1.bottom pg.left@0.25`), then an optional style: corner (default), arc or bezier. With `--line`, the points may be omitted",
          required: false,
          max: 3,
        },
      ],
      opts: [
        { long: "line", value: "LINE", help: "Reroute this existing line, keeping its id and attributes" },
        {
          long: "radius",
          value: "RADIUS",
          help: "Corner radius for the `arc` style",
          default: "8",
          allowHyphen: true,
          check: nonNegativeCheck,
        },
        {
          long: "max",
          value: "MAX",
          help: "Number of candidates to print",
          default: "3",
          check: atLeastOneCheck,
        },
        { long: "format", value: "FORMAT", possible: FORMATS, default: "text" },
        { long: "allow-path", help: ALLOW_PATH },
        { long: "include", value: "FILE", help: INCLUDE, multiple: true },
        { long: "stdin-name", value: "STDIN_NAME", help: STDIN_NAME },
      ],
    },
    {
      name: "measure",
      aliases: [],
      about: "Print the exact width and height of a label, with markup applied",
      args: [
        {
          name: "TEXT",
          help: "Texts to measure, with markup and `\\n` escapes as in a .kash string; one result per text",
          required: true,
          max: Infinity,
        },
      ],
      opts: [
        { long: "size", value: "SIZE", help: "Font size in px", default: "14", check: f64 },
        { long: "weight", value: "WEIGHT", possible: ["normal", "bold"], default: "normal" },
        { long: "format", value: "FORMAT", possible: FORMATS, default: "text" },
      ],
    },
    {
      name: "ref",
      aliases: ["references", "docs"],
      about: "Print the language reference, or one topic of it",
      args: [
        {
          name: "TOPIC",
          help: "A topic (rules, syntax, anchors, paths, sizing, text, shapes, styles, recipes, examples, lints, metrics, cli, limits) or a diagnostic code; without one, the core guide",
          required: false,
          max: 1,
        },
      ],
      opts: [{ long: "json", help: "Wrap the page in a JSON envelope" }],
    },
    {
      name: "icons",
      aliases: [],
      about: "List bundled icons and the importable icons and shapes in directories",
      args: [
        {
          name: "DIRS",
          help: "Directories to scan (default: the KASHIKA_ICON_PATH entries)",
          required: false,
          max: Infinity,
        },
      ],
      opts: [
        {
          long: "search",
          value: "SEARCH",
          help: "Only list entries whose name, path or keyword contains this term; exits 1 when nothing matches",
        },
        { long: "format", value: "FORMAT", possible: FORMATS, default: "text" },
      ],
    },
    {
      name: "inspect",
      aliases: [],
      about:
        "Print the resolved geometry: shapes, anchors, lines, labels and the placement graph",
      args: [
        FILE_ARG,
        { name: "IDS", help: "Only show these ids", required: false, max: Infinity },
      ],
      opts: [
        { long: "format", value: "FORMAT", possible: FORMATS, default: "text" },
        { long: "stdin-name", value: "STDIN_NAME", help: STDIN_NAME },
        { long: "allow-path", help: ALLOW_PATH },
        { long: "include", value: "FILE", help: INCLUDE, multiple: true },
        {
          long: "ports",
          help: "List the endpoints on each shape side and the lines that use them",
        },
        {
          long: "corridors",
          value: "X1,Y1,X2,Y2",
          help: "Free horizontal and vertical intervals inside the rectangle x1,y1,x2,y2; without ids, prints only these",
          allowHyphen: true,
          check: rectCheck,
        },
      ],
    },
    {
      name: "fmt",
      aliases: ["format"],
      about: "Rewrite a .kash file in canonical layout (whitespace only)",
      args: [
        {
          name: "FILE",
          help: "The .kash file to format, or `-` to format stdin to stdout",
          required: true,
          max: 1,
        },
      ],
      opts: [
        {
          long: "check",
          help: "Exit 1 if the file is not formatted, without rewriting it",
        },
        { long: "stdin-name", value: "STDIN_NAME", help: STDIN_NAME },
      ],
    },
  ],
};

interface Common {
  file: string;
  format: Format;
  failOn: FailOn;
  max: number;
  stdinName: string | null;
  allowPath: boolean;
  include: string[];
  showSuppressed: boolean;
  summary: boolean;
  focus: string[];
}

interface RenderOpts {
  output: string;
  force: boolean;
  debug: boolean;
}

interface InspectArgs {
  file: string;
  ids: string[];
  format: Format;
  stdinName: string | null;
  allowPath: boolean;
  include: string[];
  ports: boolean;
  corridors: Rect | null;
}

function reportOpts(
  c: Common,
  metrics: report.Opts["metrics"],
  deferred: report.Deferrals | null,
  omitted: report.Opts["omitted"],
): report.Opts {
  return {
    max: c.max,
    showSuppressed: c.showSuppressed,
    summary: c.summary,
    metrics,
    deferred,
    omitted,
  };
}

export function iconPath(): string[] {
  const v = process.env["KASHIKA_ICON_PATH"];
  return v === undefined || v === "" ? [] : v.split(delimiter);
}

/// The message Rust's `io::Error` displays for a Node file system error.
export function ioError(e: unknown): string {
  const code = (e as { code?: string }).code;
  const known: Record<string, string> = {
    ENOENT: "No such file or directory (os error 2)",
    EACCES: "Permission denied (os error 13)",
    EPERM: "Operation not permitted (os error 1)",
    EISDIR: "Is a directory (os error 21)",
    ENOTDIR: "Not a directory (os error 20)",
    EEXIST: "File exists (os error 17)",
    ELOOP: "Too many levels of symbolic links (os error 62)",
    ENAMETOOLONG: "File name too long (os error 63)",
  };
  if (code !== undefined && known[code] !== undefined) return known[code];
  return e instanceof Error ? e.message : String(e);
}

/// The sandbox for `file`. Each `--include` must name an existing file; it is canonicalized
/// relative to the current directory.
export function options(
  file: string,
  allowPath: boolean,
  include: readonly string[],
): Result<Options, string> {
  const canonical: string[] = [];
  for (const p of include) {
    let c: string;
    try {
      c = realpathSync(p);
    } catch (e) {
      return err(`--include \`${p}\`: ${ioError(e)}`);
    }
    if (!statSync(c).isFile()) {
      return err(`--include \`${p}\` is not a file; include each file, or pass --allow-path`);
    }
    canonical.push(c);
  }
  return ok(
    new Options({
      path: file !== "-" ? file : null,
      allowPath,
      iconPath: iconPath(),
      include: canonical,
    }),
  );
}

function decodeUtf8(bytes: Uint8Array): string | null {
  try {
    return new TextDecoder("utf-8", { fatal: true, ignoreBOM: true }).decode(bytes);
  } catch {
    return null;
  }
}

const INVALID_UTF8 = "stream did not contain valid UTF-8";

export function readInput(file: string, stdinName: string | null): Result<[string, string], string> {
  if (file === "-") {
    let bytes: Uint8Array;
    try {
      bytes = readFileSync(0);
    } catch (e) {
      return err(`cannot read stdin: ${ioError(e)}`);
    }
    const s = decodeUtf8(bytes);
    if (s === null) return err(`cannot read stdin: ${INVALID_UTF8}`);
    return ok([stdinName ?? "<stdin>", s]);
  }
  let bytes: Uint8Array;
  try {
    bytes = readFileSync(file);
  } catch (e) {
    return err(`cannot read ${file}: ${ioError(e)}`);
  }
  const s = decodeUtf8(bytes);
  if (s === null) return err(`cannot read ${file}: ${INVALID_UTF8}`);
  return ok([file, s]);
}

export async function main(argv: readonly string[]): Promise<number> {
  const parsed = parseArgs(APP, argv);
  if (parsed.kind === "exit") {
    print(parsed.stdout);
    eprint(parsed.stderr);
    if (
      parsed.code !== 0 &&
      argv[0] === "route" &&
      argv.some((a) => a.endsWith("->") && a.startsWith("-"))
    ) {
      eprint(`help: ${routeCmd.NO_ARROW}\n`);
    }
    return parsed.code;
  }
  const m = parsed.matches;
  switch (m.command) {
    case "check":
      return run(common(m), null);
    case "fmt":
      return fmtCmd.run(m.positional("FILE")[0]!, m.flag("check"), m.value("stdin-name"));
    case "render":
      return run(common(m), {
        output: m.value("output")!,
        force: m.flag("force"),
        debug: m.flag("debug"),
      });
    case "inspect":
      return runInspect({
        file: m.positional("FILE")[0]!,
        ids: m.positional("IDS"),
        format: m.value("format") as Format,
        stdinName: m.value("stdin-name"),
        allowPath: m.flag("allow-path"),
        include: m.values("include"),
        ports: m.flag("ports"),
        corridors: (() => {
          const v = m.value("corridors");
          if (v === null) return null;
          const r = parseRect(v);
          return r.ok ? r.value : null;
        })(),
      });
    case "route": {
      const radius = nonNegative(m.value("radius")!);
      const max = atLeastOne(m.value("max")!);
      return routeCmd.run({
        file: m.positional("FILE")[0]!,
        rest: m.positional("REST"),
        line: m.value("line"),
        radius: radius.ok ? radius.value : 8.0,
        max: max.ok ? max.value : 3,
        format: m.value("format") as Format,
        allowPath: m.flag("allow-path"),
        include: m.values("include"),
        stdinName: m.value("stdin-name"),
      });
    }
    case "measure":
      return measure.run({
        text: m.positional("TEXT"),
        size: parseF64(m.value("size")!)!,
        weight: m.value("weight") as measure.Weight,
        format: m.value("format") as Format,
      });
    case "icons":
      return icons.run({
        dirs: m.positional("DIRS"),
        search: m.value("search"),
        format: m.value("format") as Format,
      });
    case "ref":
      return refCmd.run(m.positional("TOPIC")[0] ?? null, m.flag("json"));
    default:
      throw new Error(`unknown command ${m.command}`);
  }
}

function common(m: Matches): Common {
  return {
    file: m.positional("FILE")[0]!,
    format: m.value("format") as Format,
    failOn: m.value("fail-on") as FailOn,
    max: Number(m.value("max")!),
    stdinName: m.value("stdin-name"),
    allowPath: m.flag("allow-path"),
    include: m.values("include"),
    showSuppressed: m.flag("show-suppressed"),
    summary: m.flag("summary"),
    focus: m.values("focus"),
  };
}

async function run(c: Common, render: RenderOpts | null): Promise<number> {
  const fail = (msg: string) => toolError(c.format, report.SCHEMA, msg);
  const input = readInput(c.file, c.stdinName);
  if (!input.ok) return fail(input.error);
  const [name, src] = input.value;
  if (render !== null) {
    if (output.kind(render.output) === null) {
      return fail(`output must end in ${output.SUPPORTED}`);
    }
  }
  const opts = options(c.file, c.allowPath, c.include);
  if (!opts.ok) return fail(opts.error);
  const { file, scene, diags: compiled } = compileFile(src, opts.value);
  if (scene === null) {
    if (render !== null) {
      return fail(
        `${name} is a library (no \`diagram\` statement); render a diagram that imports it`,
      );
    }
    return reportLibrary(c, name, src, file, compiled);
  }
  let diags0 = compiled;
  if (c.focus.length > 0) {
    const known = knownIds(sceneIds(scene), c.focus, "--focus");
    if (!known.ok) return fail(known.error);
    diags0 = retainFocus(diags0, c.focus);
  }
  const focus = (d: Diagnostic) =>
    c.focus.length === 0 || d.elements.some((e) => c.focus.includes(e.id));
  const ctx = Ctx.new(scene);
  const found = checkBounded(ctx, Math.max(c.max, report.KEEP), focus);
  const metrics = found.metrics(scene);
  const all: [Diagnostic, Deferred[]][] = [
    ...diags0.map((d): [Diagnostic, Deferred[]] => [d, []]),
    ...found.diags.map((d, i): [Diagnostic, Deferred[]] => [d, found.deferred[i]!]),
  ];
  all.sort((a, b) => order(a[0], b[0]));
  const diags = all.map(([d]) => d);
  const deferred = all.map(([, x]) => x);
  const omitted = found.omitted;
  const { errors, warns } = report.counts(diags, omitted);
  let out: report.Output | null = null;
  if (render !== null) {
    const written = errors === 0 || render.force;
    if (written) {
      const marked = report.visible(diags, c.showSuppressed);
      const svg = renderSvgWith(scene, marked, { errorMarkers: errors > 0, debug: render.debug });
      const kind = output.kind(render.output)!;
      const bytes = await output.encode(kind, svg);
      if (!bytes.ok) {
        return fail(`cannot render ${render.output}: ${bytes.error}`);
      }
      const w = writeAtomic(render.output, bytes.value);
      if (!w.ok) {
        return fail(`cannot write ${render.output}: ${w.error}`);
      }
    }
    out = { path: render.output, written };
  }
  const ropts = reportOpts(c, metrics, [ctx, deferred], omitted);
  print(
    c.format === "text"
      ? report.text(name, src, diags, ropts, out)
      : report.json(name, src, diags, ropts, out),
  );
  return exitCode(c.failOn, errors, warns);
}

function retainFocus(diags: Diagnostic[], focus: readonly string[]): Diagnostic[] {
  return diags.filter(
    (d) => d.severity === "error" || d.elements.some((e) => focus.includes(e.id)),
  );
}

function reportLibrary(
  c: Common,
  name: string,
  src: string,
  file: File,
  diags0: Diagnostic[],
): number {
  let diags = diags0;
  if (c.focus.length > 0) {
    const defined = file.stmts.flatMap((s) => (s.kind === "Shape" ? [s.value.name.name] : []));
    const known = knownIds(defined, c.focus, "--focus");
    if (!known.ok) return toolError(c.format, report.SCHEMA, known.error);
    diags = retainFocus(diags, c.focus);
  }
  const opts = reportOpts(c, null, null, []);
  print(
    c.format === "text"
      ? report.text(name, src, diags, opts, null)
      : report.json(name, src, diags, opts, null),
  );
  const { errors, warns } = report.counts(diags, []);
  return exitCode(c.failOn, errors, warns);
}

function runInspect(a: InspectArgs): number {
  const fail = (msg: string) => toolError(a.format, inspect.SCHEMA, msg);
  const input = readInput(a.file, a.stdinName);
  if (!input.ok) return fail(input.error);
  const [name, src] = input.value;
  const opts = options(a.file, a.allowPath, a.include);
  if (!opts.ok) return fail(opts.error);
  const [scene, all] = compileWith(src, opts.value);
  const diags = all.filter((d) => !d.suppressed);
  const ctx = Ctx.new(scene);
  const lints = checkBounded(ctx, 0, (d) => !d.suppressed).omitted.reduce(
    (n, o) => n + o.count,
    0,
  );
  const known = knownIds(sceneIds(scene), a.ids, "inspect");
  if (!known.ok) return fail(known.error);
  const data = inspect.build(name, src, scene, diags, lints, {
    ids: a.ids,
    ports: a.ports,
    corridors: a.corridors,
  });
  if (a.format === "text") {
    print(term.text(inspect.text(data)));
  } else {
    print(`${toJsonString(inspect.toJson(data))}\n`);
  }
  return 0;
}

export function parseRect(s: string): Result<Rect, string> {
  const bad = () => err(`expected finite numbers x1,y1,x2,y2, got \`${s}\``);
  const v: number[] = [];
  for (const x of s.split(",")) {
    const n = parseF64(x.trim());
    if (n === null) return bad();
    v.push(n);
  }
  if (v.length !== 4 || !v.every((n) => Number.isFinite(n))) return bad();
  const [x1, y1, x2, y2] = v as [number, number, number, number];
  return ok(new Rect(Math.min(x1, x2), Math.min(y1, y2), Math.abs(x2 - x1), Math.abs(y2 - y1)));
}

export function nonNegative(s: string): Result<number, string> {
  const v = parseF64(s);
  if (v === null) return err(`\`${s}\` is not a number`);
  if (Number.isFinite(v) && v >= 0.0) return ok(v);
  return err(`\`${s}\` must be a finite number, 0 or more`);
}

export function atLeastOne(s: string): Result<number, string> {
  if (/^\+?[0-9]+$/.test(s)) {
    const n = Number(s);
    if (n >= 1 && Number.isSafeInteger(n)) return ok(n);
  }
  return err(`\`${s}\` must be a whole number, 1 or more`);
}

/// Every id in the scene, with `line@L<n>`/`text@L<n>` for unnamed lines and texts.
function sceneIds(scene: Scene): string[] {
  return [
    ...scene.shapes.map((s) => s.id),
    ...scene.junctions.map((j) => j.id),
    ...scene.lines.map((l) => l.id),
    ...scene.texts.map((t) => t.displayId()),
  ];
}

function knownIds(all: readonly string[], ids: readonly string[], what: string): Result<null, string> {
  for (const id of ids) {
    if (!all.includes(id)) {
      const s = didYouMean(id, all);
      const hint = s === null ? "" : `; did you mean \`${s}\`?`;
      return err(`unknown id \`${id}\` in ${what}${hint}`);
    }
  }
  return ok(null);
}

function resolveLink(path: string): string {
  try {
    return realpathSync(path);
  } catch {
    // fall through
  }
  try {
    return join(dirname(path), readlinkSync(path));
  } catch {
    return path;
  }
}

export function writeAtomic(path0: string, bytes: Uint8Array): Result<null, string> {
  const path = resolveLink(path0);
  const dir = dirname(path) === "" ? "." : dirname(path);
  const name = basename(path);
  const nonce = randomBytes(8).toString("hex");
  const tmp = join(dir, `.${name}.${nonce}.tmp`);
  let fd: number;
  try {
    fd = openSync(tmp, "wx");
  } catch (e) {
    return err(ioError(e));
  }
  try {
    let off = 0;
    while (off < bytes.length) off += writeSync(fd, bytes, off);
    try {
      const mode = statSync(path).mode;
      chmodSync(tmp, mode & 0o7777);
    } catch (e) {
      if ((e as { code?: string }).code !== "ENOENT") throw e;
    }
    fsyncSync(fd);
    closeSync(fd);
    fd = -1;
    renameSync(tmp, path);
  } catch (e) {
    if (fd >= 0) {
      try {
        closeSync(fd);
      } catch {
        // already closed
      }
    }
    try {
      unlinkSync(tmp);
    } catch {
      // nothing to clean up
    }
    return err(ioError(e));
  }
  try {
    const d = openSync(dir, "r");
    try {
      fsyncSync(d);
    } finally {
      closeSync(d);
    }
  } catch {
    // directory sync is best effort
  }
  return ok(null);
}

function exitCode(failOn: FailOn, errors: number, warns: number): number {
  if (failOn === "never") return 0;
  if (errors > 0) return 2;
  if (failOn === "warn" && warns > 0) return 1;
  return 0;
}

export function toolError(format: Format, schema: string, msg: string): number {
  if (format === "text") {
    eprint(`kashika: ${term.text(msg)}\n`);
  } else {
    print(`${toJsonString({ schema, tool: TOOL_VERSION, tool_error: msg })}\n`);
  }
  return 3;
}
