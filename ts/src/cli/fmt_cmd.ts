import { format } from "../syntax/index.js";
import { print, readInput, toolError, writeAtomic } from "./main.js";
import * as report from "./report.js";
import * as term from "./term.js";

export function run(file0: string, check: boolean, stdinName: string | null): number {
  const stdin = file0 === "-";
  const input = readInput(file0, stdinName);
  if (!input.ok) {
    return toolError("text", report.SCHEMA, input.error);
  }
  const [file, src] = input.value;
  const out = format(src);
  if (!out.ok) {
    const opts: report.Opts = {
      max: 50,
      showSuppressed: false,
      summary: false,
      metrics: null,
      deferred: null,
      omitted: [],
    };
    print(report.text(file, src, out.error, opts, null));
    return 2;
  }
  if (stdin && !check) {
    print(out.value);
    return 0;
  }
  if (out.value === src) {
    return 0;
  }
  if (check) {
    const name = term.line(file);
    if (stdin) {
      print(`${name}: not formatted\n`);
    } else {
      print(`${name}: not formatted; run \`kashika fmt ${name}\`\n`);
    }
    return 1;
  }
  const w = writeAtomic(file, new TextEncoder().encode(out.value));
  if (!w.ok) {
    return toolError("text", report.SCHEMA, `cannot write ${file}: ${w.error}`);
  }
  return 0;
}
