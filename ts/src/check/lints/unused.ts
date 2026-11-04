import { code } from "../../diag/codes.js";
import { Diagnostic } from "../../diag/diagnostic.js";
import { type Sink } from "../sink.js";

/// `unused-allow`: an `allow=` entry on a line, box or group that suppressed no finding. Runs
/// after every other lint, so each entry has met every finding.
export function unusedAllow(out: Sink): void {
  const found: Diagnostic[] = out
    .suppressor()
    .unused()
    .map(([o, a, none]) => {
      const entry = a.other !== null ? `${a.code}:${a.other}` : a.code;
      const what = o.el.kind === "line" ? o.id : `${o.el.kind} '${o.id}'`;
      const within = o.el.span.line <= a.span.line && a.span.endLine <= o.el.span.endLine;
      const help =
        none && o.reason !== null
          ? "remove it, and `reason=` with it, which then explains nothing"
          : "remove it";
      return Diagnostic.warn(
        code.UNUSED_ALLOW,
        within ? a.span : o.el.span,
        `\`allow=${entry}\` on ${what} suppresses nothing`,
      )
        .withElement(o.el)
        .withHelp(help);
    });
  for (const d of found) {
    out.push(d);
  }
}
