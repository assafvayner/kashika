/// Escaping for text that reaches the terminal, so input can't smuggle escape sequences.

function unsafeChar(cp: number): boolean {
  return (
    cp <= 0x1f ||
    (cp >= 0x7f && cp <= 0x9f) ||
    cp === 0x200e ||
    cp === 0x200f ||
    (cp >= 0x202a && cp <= 0x202e) ||
    (cp >= 0x2066 && cp <= 0x2069)
  );
}

/// `s` with control characters, DEL and bidirectional overrides written as `\u{..}`. Tabs
/// are kept, and so are newlines when `newlines` is set.
function escapeWith(s: string, newlines: boolean): string {
  let out = "";
  for (const c of s) {
    const cp = c.codePointAt(0)!;
    const bad = unsafeChar(cp) && !(c === "\t" || (newlines && c === "\n"));
    out += bad ? `\\u{${cp.toString(16)}}` : c;
  }
  return out;
}

/// One line of terminal output: every control character but tab is escaped.
export function line(s: string): string {
  return escapeWith(s, false);
}

/// Multi-line terminal output: like `line`, but newlines are kept.
export function text(s: string): string {
  return escapeWith(s, true);
}
