// String escapes for plain-text fields (titles, reasons). The lexer keeps `\\`, `\*` and
// `` \` `` in string values.

/// Replaces the escapes `\\`, `\*` and `` \` `` with the literal characters.
export function unescape(s: string): string {
  let out = "";
  const it = s[Symbol.iterator]();
  for (let n = it.next(); !n.done; n = it.next()) {
    const c = n.value;
    if (c === "\\") {
      const next = it.next();
      if (!next.done) {
        out += next.value;
      }
      continue;
    }
    out += c;
  }
  return out;
}
