import { expect, it } from "vitest";

import { lint } from "./common/mod.js";

it("literal_endpoint_near_an_outline", () => {
  const d = lint(
    "endpoint-off-edge",
    "box a at (100,100) size (100,60)\nline l (0,115) -> (97,115)\n",
  );
  expect(d.length).toBe(1);
  expect(d[0]!.message).toBe("l ends at (97,115), 3px from the outline of 'a'");
  expect(d[0]!.help).toEqual(["snap it to the outline with an anchor: `a.left@0.25`"]);
  expect(
    lint("endpoint-off-edge", "box a at (100,100) size (100,60)\nline l (0,115) -> (100,115)\n"),
  ).toEqual([]);
  expect(
    lint(
      "endpoint-off-edge",
      "box a at (100,100) size (100,60)\nline l (0,115) -> a.left@0.25+(-3,0)\n",
    ),
  ).toEqual([]);
  expect(
    lint("endpoint-off-edge", "box a at (100,100) size (100,60)\nline l (0,115) -> (80,115)\n"),
  ).toEqual([]);
});
