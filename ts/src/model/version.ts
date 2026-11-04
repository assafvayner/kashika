import { type Result, err, ok } from "../diag/rust.js";

export const TOOL_VERSION = "0.1.0-dev0";

const FORM_HELP = 'write kashika="0.1.0"';

export interface Problem {
  message: string;
  help: string;
}

type Version = [bigint, bigint, bigint];

const U64_MAX = 18446744073709551615n;

function parse(s: string): Version | null {
  const part = (p: string | undefined): bigint | null => {
    if (p === undefined) return null;
    const good = p !== "" && /^[0-9]+$/.test(p) && (p === "0" || !p.startsWith("0"));
    if (!good) return null;
    const v = BigInt(p);
    return v <= U64_MAX ? v : null;
  };
  const it = s.split(".");
  const major = part(it[0]);
  if (major === null) return null;
  const minor = part(it[1]);
  if (minor === null) return null;
  const patch = part(it[2]);
  if (patch === null) return null;
  if (it.length > 3) {
    return null;
  }
  return [major, minor, patch];
}

function release(tool: string): string {
  return tool.split(/[-+]/)[0]!;
}

function le(a: Version, b: Version): boolean {
  for (let i = 0; i < 3; i++) {
    if (a[i] !== b[i]) return a[i]! < b[i]!;
  }
  return true;
}

export function check(file: string, tool: string): Result<null, Problem> {
  const want = parse(file);
  if (want === null) {
    return err({
      message: `\`${file}\` is not a valid kashika version`,
      help: FORM_HELP,
    });
  }
  const have = parse(release(tool)) ?? [0n, 0n, 0n];
  if (want[0] === have[0] && le(want, have)) {
    return ok(null);
  }
  const [maj, min, pat] = want;
  const help =
    want[0] === have[0]
      ? "upgrade kashika, or lower `kashika=` if the file uses nothing newer"
      : `use a kashika ${maj}.x release, or set \`kashika=\` to a ${have[0]}.x version`;
  return err({
    message: `this file needs kashika ${maj}.${min}.${pat} or newer (same major version); this is kashika ${tool}`,
    help,
  });
}

export function formHelp(): string {
  return FORM_HELP;
}
