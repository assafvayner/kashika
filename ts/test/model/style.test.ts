import { describe, expect, test } from "vitest";
import { StyleProps, builtin } from "../../src/model/style.js";

describe("style", () => {
  test("overlay_is_key_by_key", () => {
    const base = builtin("service")!;
    const user = StyleProps.default();
    user.fill = "#ffffff";
    const s = base.overlay(user).resolve();
    expect(s.fill).toBe("#ffffff");
    expect(s.stroke).toBe("#1a73e8");
    expect(s.strokeWidth).toBe(1.5);
  });
});
