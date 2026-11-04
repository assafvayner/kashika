import { describe, expect, test } from "vitest";
import { shapeEmoji } from "../../src/text/font.js";

/// `rustybuzz::shape` (0.20.1) of each string on Twemoji Mozilla, left to right with no
/// features: `[glyph id, x advance, x offset, y offset]` per glyph.
const CASES: [string, [number, number, number, number][]][] = [
  ["🚀", [[964, 512, 0, 0]]],
  ["🇺🇸", [[2947, 512, 0, 0]]],
  [
    "🇺🇸🇺",
    [
      [2947, 512, 0, 0],
      [226, 512, 0, 0],
    ],
  ],
  ["👍🏽", [[5137, 512, 0, 0]]],
  ["👩\u{200D}💻", [[6675, 512, 0, 0]]],
  ["👨\u{200D}👩\u{200D}👧\u{200D}👦", [[6000, 512, 0, 0]]],
  ["1\u{FE0F}\u{20E3}", [[13695, 512, 0, 0]]],
  [
    "1\u{20E3}",
    [
      [6, 512, 0, 0],
      [20, 512, 0, 0],
    ],
  ],
  ["☺\u{FE0E}", [[78, 512, 0, 0]]],
  ["❤\u{FE0F}", [[167, 512, 0, 0]]],
  ["❤", [[167, 512, 0, 0]]],
  ["🏴\u{E0067}\u{E0062}\u{E0065}\u{E006E}\u{E0067}\u{E007F}", [[4652, 512, 0, 0]]],
  ["🚀\u{FE0F}", [[964, 512, 0, 0]]],
  ["🚀\u{200B}", [[964, 512, 0, 0]]],
  ["👩\u{200D}\u{200D}💻", [[6675, 512, 0, 0]]],
  ["👩\u{FE0F}\u{200D}💻", [[6675, 512, 0, 0]]],
  [
    "👩\u{200C}\u{200D}💻",
    [
      [598, 512, 0, 0],
      [680, 512, 0, 0],
    ],
  ],
  [
    "👩\u{E0020}\u{200D}💻",
    [
      [598, 512, 0, 0],
      [680, 512, 0, 0],
    ],
  ],
  ["👩\u{034F}\u{200D}💻", [[6675, 512, 0, 0]]],
  [
    "🚀\u{180F}",
    [
      [964, 512, 0, 0],
      [0, 0, -512, 0],
    ],
  ],
  [
    "🚀\u{3164}\u{180F}",
    [
      [964, 512, 0, 0],
      [0, 512, 0, 0],
      [0, 512, 0, 0],
    ],
  ],
  [
    "👩\u{200D}💻\u{17B4}",
    [
      [598, 512, 0, 0],
      [680, 512, 0, 0],
    ],
  ],
];

describe("shape", () => {
  test("emoji_shaping_matches_rustybuzz", () => {
    for (const [s, want] of CASES) {
      expect(
        shapeEmoji(s).map((g) => [g.id, g.advance, g.dx, g.dy]),
        [...s].map((c) => c.codePointAt(0)!.toString(16)).join(" "),
      ).toEqual(want);
    }
  });
});
