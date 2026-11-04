import { expect, it } from "vitest";

import { check } from "../../src/check/index.js";
import { lint, sceneSrc } from "./common/mod.js";

it("off_grid_checks_authored_literals_only", () => {
  const s = sceneSrc(
    'diagram "t" size (800,600) grid=10\nbox a at (40,45) size (120,60)\nbox b at a.right+(33,0) size (100,60) pin=left\n',
  );
  const msgs = check(s)
    .filter((d) => d.code === "off-grid")
    .map((d) => d.message);
  expect(msgs).toEqual([
    "y=45 on 'a' is not a multiple of grid 10",
    "dx=33 on 'b' is not a multiple of grid 10",
  ]);
  expect(lint("off-grid", "box a at (40,45) size (120,60)\n")).toEqual([]);
});
