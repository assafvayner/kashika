/// A small command-line parser with clap's conventions and error messages: subcommands with
/// aliases, `--flag`, `--key value`, `--key=value`, short options, repeatable and
/// comma-delimited options, positional arguments, `--help` and `--version`.

export interface Opt {
  /// The long name without `--`.
  long: string;
  short?: string;
  /// The value name (`FORMAT`); absent for a flag.
  value?: string;
  help?: string;
  multiple?: boolean;
  delimiter?: string;
  allowHyphen?: boolean;
  possible?: readonly string[];
  default?: string;
  required?: boolean;
  /// Validates a value: the error message, or `null` when it is fine.
  check?: (s: string) => string | null;
}

export interface Pos {
  /// The value name (`FILE`).
  name: string;
  help?: string;
  required: boolean;
  /// How many values it takes: 1 for a single one.
  max: number;
  /// At least one value when `required` and `max > 1`.
  multiple?: boolean;
}

export interface Command {
  name: string;
  aliases: readonly string[];
  about: string;
  args: readonly Pos[];
  opts: readonly Opt[];
}

export interface App {
  name: string;
  version: string;
  about: string;
  afterHelp: string;
  commands: readonly Command[];
}

export class Matches {
  constructor(
    readonly command: string,
    private vals: Map<string, string[]>,
    private flags: Set<string>,
    private pos: Map<string, string[]>,
  ) {}

  flag(long: string): boolean {
    return this.flags.has(long);
  }

  /// The last value of an option, or its default.
  value(long: string): string | null {
    const v = this.vals.get(long);
    return v === undefined || v.length === 0 ? null : v[v.length - 1]!;
  }

  values(long: string): string[] {
    return this.vals.get(long) ?? [];
  }

  positional(name: string): string[] {
    return this.pos.get(name) ?? [];
  }
}

export type Parsed =
  | { kind: "matches"; matches: Matches }
  /// Help, version or a usage error: what to print and the exit code.
  | { kind: "exit"; stdout: string; stderr: string; code: number };

const TRY_HELP = "\nFor more information, try '--help'.\n";

function optName(o: Opt): string {
  return o.value === undefined ? `--${o.long}` : `--${o.long} <${o.value}>`;
}

function posName(p: Pos): string {
  if (p.max === 1) return p.required ? `<${p.name}>` : `[${p.name}]`;
  return p.required ? `<${p.name}>...` : `[${p.name}]...`;
}

function usage(app: App, c: Command, onlyRequired: boolean): string {
  let u = `${app.name} ${c.name}`;
  if (!onlyRequired && c.opts.some((o) => !o.required)) u += " [OPTIONS]";
  for (const o of c.opts) {
    if (o.required) u += ` ${optName(o)}`;
  }
  for (const p of c.args) {
    if (!onlyRequired || p.required) u += ` ${posName(p)}`;
  }
  return u;
}

function usageError(msg: string, usageLine: string | null): Parsed {
  const u = usageLine === null ? "" : `\nUsage: ${usageLine}\n`;
  return { kind: "exit", stdout: "", stderr: `error: ${msg}\n${u}${TRY_HELP}`, code: 3 };
}

/// Jaro similarity, which clap's suggestions use.
function jaro(a: string, b: string): number {
  const x = Array.from(a);
  const y = Array.from(b);
  if (x.length === 0 && y.length === 0) return 1;
  if (x.length === 0 || y.length === 0) return 0;
  const range = Math.max(Math.floor(Math.max(x.length, y.length) / 2) - 1, 0);
  const xm = new Array<boolean>(x.length).fill(false);
  const ym = new Array<boolean>(y.length).fill(false);
  let matches = 0;
  for (let i = 0; i < x.length; i++) {
    const lo = Math.max(0, i - range);
    const hi = Math.min(y.length - 1, i + range);
    for (let j = lo; j <= hi; j++) {
      if (!ym[j] && x[i] === y[j]) {
        xm[i] = true;
        ym[j] = true;
        matches++;
        break;
      }
    }
  }
  if (matches === 0) return 0;
  let t = 0;
  let j = 0;
  for (let i = 0; i < x.length; i++) {
    if (!xm[i]) continue;
    while (!ym[j]) j++;
    if (x[i] !== y[j]) t++;
    j++;
  }
  return (matches / x.length + matches / y.length + (matches - t / 2) / matches) / 3;
}

function unexpected(app: App, c: Command, label: string, long: string | null, u: string): Parsed {
  let best: [number, Opt] | null = null;
  if (long !== null) {
    for (const o of c.opts) {
      const conf = jaro(long, o.long);
      if (conf > 0.7 && (best === null || conf > best[0])) best = [conf, o];
    }
  }
  if (best !== null) {
    const o = best[1];
    const req = c.args.filter((p) => p.required).map((p) => ` ${posName(p)}`);
    return usageError(
      `unexpected argument '${label}' found\n\n  tip: a similar argument exists: '--${o.long}'`,
      `${app.name} ${c.name} ${optName(o)}${req.join("")}`,
    );
  }
  if (c.args.length > 0) {
    return usageError(
      `unexpected argument '${label}' found\n\n  tip: to pass '${label}' as a value, use '-- ${label}'`,
      u,
    );
  }
  return usageError(`unexpected argument '${label}' found`, u);
}

function table(rows: [string, string][]): string {
  const width = Math.max(...rows.map(([l]) => l.length));
  return rows
    .map(([l, r]) => (r === "" ? `  ${l}` : `  ${l.padEnd(width)}  ${r}`))
    .join("\n");
}

function optHelp(o: Opt): string {
  const parts: string[] = [];
  if (o.help !== undefined) parts.push(o.help);
  if (o.default !== undefined) parts.push(`[default: ${o.default}]`);
  if (o.possible !== undefined) parts.push(`[possible values: ${o.possible.join(", ")}]`);
  return parts.join(" ");
}

export function commandHelp(app: App, c: Command): string {
  let out = `${c.about}\n\nUsage: ${usage(app, c, false)}\n`;
  const rows: [string, string][] = [];
  const optLabel = (o: Opt) => (o.short !== undefined ? `-${o.short}, ` : "    ") + optName(o);
  for (const p of c.args) rows.push([posName(p), p.help ?? ""]);
  for (const o of c.opts) rows.push([optLabel(o), optHelp(o)]);
  rows.push(["-h, --help", "Print help"]);
  if (c.args.length > 0) {
    out += `\nArguments:\n${table(rows.slice(0, c.args.length))}\n`;
  }
  out += `\nOptions:\n${table(rows.slice(c.args.length))}\n`;
  return out;
}

export function appHelp(app: App): string {
  const rows: [string, string][] = app.commands.map((c) => [
    c.name,
    c.aliases.length === 0
      ? c.about
      : `${c.about} [${c.aliases.length === 1 ? "alias" : "aliases"}: ${c.aliases.join(", ")}]`,
  ]);
  rows.push(["help", "Print this message or the help of the given subcommand(s)"]);
  return (
    `${app.about}\n\nUsage: ${app.name} <COMMAND>\n\nCommands:\n${table(rows)}\n\n` +
    `Options:\n${table([
      ["-h, --help", "Print help"],
      ["-V, --version", "Print version"],
    ])}\n\n${app.afterHelp}\n`
  );
}

function find(app: App, name: string): Command | undefined {
  return app.commands.find((c) => c.name === name || c.aliases.includes(name));
}

export function parse(app: App, argv: readonly string[]): Parsed {
  const first = argv[0];
  if (first === undefined) {
    return { kind: "exit", stdout: "", stderr: appHelp(app), code: 3 };
  }
  if (first === "-h" || first === "--help") {
    return { kind: "exit", stdout: appHelp(app), stderr: "", code: 0 };
  }
  if (first === "-V" || first === "--version") {
    return { kind: "exit", stdout: `${app.name} ${app.version}\n`, stderr: "", code: 0 };
  }
  if (first === "help") {
    const sub = argv[1];
    if (sub === undefined) return { kind: "exit", stdout: appHelp(app), stderr: "", code: 0 };
    const c = find(app, sub);
    if (c === undefined) {
      return usageError(`unrecognized subcommand '${sub}'`, `${app.name} <COMMAND>`);
    }
    return { kind: "exit", stdout: commandHelp(app, c), stderr: "", code: 0 };
  }
  if (first.startsWith("-")) {
    return usageError(`unexpected argument '${first}' found`, `${app.name} <COMMAND>`);
  }
  const c = find(app, first);
  if (c === undefined) {
    return usageError(`unrecognized subcommand '${first}'`, `${app.name} <COMMAND>`);
  }
  return parseCommand(app, c, argv.slice(1));
}

function parseCommand(app: App, c: Command, argv: readonly string[]): Parsed {
  const values = new Map<string, string[]>();
  const flags = new Set<string>();
  const seen = new Set<string>();
  const free: string[] = [];
  const u = usage(app, c, false);
  const take = (o: Opt, v: string): Parsed | null => {
    if (seen.has(o.long) && !o.multiple) {
      return usageError(`the argument '${optName(o)}' cannot be used multiple times`, u);
    }
    seen.add(o.long);
    const parts = o.delimiter === undefined ? [v] : v.split(o.delimiter);
    for (const part of parts) {
      if (o.possible !== undefined && !o.possible.includes(part)) {
        return usageError(
          `invalid value '${part}' for '${optName(o)}'\n  [possible values: ${o.possible.join(", ")}]`,
          null,
        );
      }
      const bad = o.check?.(part) ?? null;
      if (bad !== null) {
        return usageError(`invalid value '${part}' for '${optName(o)}': ${bad}`, null);
      }
      const list = values.get(o.long) ?? [];
      list.push(part);
      values.set(o.long, list);
    }
    return null;
  };
  let i = 0;
  let rest = false;
  while (i < argv.length) {
    const a = argv[i++]!;
    if (rest) {
      free.push(a);
      continue;
    }
    if (a === "--") {
      rest = true;
      continue;
    }
    let o: Opt | undefined;
    let inline: string | null = null;
    let label = a;
    if (a.startsWith("--")) {
      const eq = a.indexOf("=");
      const name = eq < 0 ? a.slice(2) : a.slice(2, eq);
      inline = eq < 0 ? null : a.slice(eq + 1);
      label = eq < 0 ? a : a.slice(0, eq);
      if (name === "help") {
        return { kind: "exit", stdout: commandHelp(app, c), stderr: "", code: 0 };
      }
      o = c.opts.find((x) => x.long === name);
    } else if (a.startsWith("-") && a.length > 1) {
      const s = Array.from(a)[1]!;
      if (s === "h") {
        return { kind: "exit", stdout: commandHelp(app, c), stderr: "", code: 0 };
      }
      o = c.opts.find((x) => x.short === s);
      label = `-${s}`;
      const after = a.slice(1 + s.length);
      if (after !== "") inline = after.startsWith("=") ? after.slice(1) : after;
    } else {
      free.push(a);
      continue;
    }
    if (o === undefined) {
      return unexpected(app, c, label, a.startsWith("--") ? label.slice(2) : null, u);
    }
    if (o.value === undefined) {
      if (inline !== null) {
        return usageError(
          `unexpected value '${inline}' for '${optName(o)}' found; no more were expected`,
          u,
        );
      }
      if (flags.has(o.long)) {
        return usageError(`the argument '${optName(o)}' cannot be used multiple times`, u);
      }
      flags.add(o.long);
      continue;
    }
    let v = inline;
    if (v === null) {
      const next = argv[i];
      const isFlag = next !== undefined && next.startsWith("-") && next !== "-";
      if (next === undefined || (isFlag && !o.allowHyphen)) {
        const possible =
          o.possible === undefined ? "" : `\n  [possible values: ${o.possible.join(", ")}]`;
        return usageError(
          `a value is required for '${optName(o)}' but none was supplied${possible}`,
          null,
        );
      }
      v = next;
      i++;
    }
    const e = take(o, v);
    if (e !== null) return e;
  }
  const pos = new Map<string, string[]>();
  let k = 0;
  for (const p of c.args) {
    const vs = free.slice(k, k + p.max);
    k += vs.length;
    pos.set(p.name, vs);
  }
  if (k < free.length) {
    const last = c.args[c.args.length - 1];
    if (last !== undefined && last.max > 1) {
      return usageError(
        `unexpected value '${free[k]}' for '${posName(last)}' found; no more were expected`,
        u,
      );
    }
    return usageError(`unexpected argument '${free[k]}' found`, u);
  }
  const missing = [
    ...c.opts.filter((o) => o.required && !values.has(o.long)).map(optName),
    ...c.args.filter((p) => p.required && pos.get(p.name)!.length === 0).map(posName),
  ];
  if (missing.length > 0) {
    return usageError(
      `the following required arguments were not provided:\n${missing.map((m) => `  ${m}`).join("\n")}`,
      usage(app, c, true),
    );
  }
  for (const o of c.opts) {
    if (!values.has(o.long) && o.default !== undefined) values.set(o.long, [o.default]);
  }
  return { kind: "matches", matches: new Matches(c.name, values, flags, pos) };
}
