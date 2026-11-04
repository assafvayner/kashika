import { expect, test } from "vitest";

import { line, text } from "../../src/cli/term.js";

test("escapes_controls_and_keeps_tabs", () => {
  expect(line("a\u{1b}]52;c;x\u{7}b\tc")).toBe("a\\u{1b}]52;c;x\\u{7}b\tc");
  expect(line("\u{7f}\u{9b}\n\r")).toBe("\\u{7f}\\u{9b}\\u{a}\\u{d}");
  expect(line("\u{202e}abc")).toBe("\\u{202e}abc");
  expect(text("a\nb\u{1b}")).toBe("a\nb\\u{1b}");
  expect(line("plain — ünïcode")).toBe("plain — ünïcode");
});
