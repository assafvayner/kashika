import { describe, expect, test } from "vitest";
import { type Problem, check } from "../../src/model/version.js";

function ok(f: string, t: string): boolean {
  return check(f, t).ok;
}

function problem(f: string, t: string): Problem {
  const r = check(f, t);
  if (r.ok) throw new Error(f);
  return r.error;
}

describe("version", () => {
  test("accepts_equal_and_older", () => {
    expect(ok("0.1.0", "0.1.0")).toBe(true);
    expect(ok("0.0.5", "0.1.0")).toBe(true);
    expect(ok("1.2.0", "1.4.3")).toBe(true);
    expect(ok("1.4.0", "1.4.3")).toBe(true);
  });

  test("rejects_newer_and_other_major", () => {
    expect(ok("0.2.0", "0.1.0")).toBe(false);
    expect(ok("0.1.1", "0.1.0")).toBe(false);
    expect(ok("1.0.0", "0.1.0")).toBe(false);
    expect(ok("1.5.0", "1.4.3")).toBe(false);
    expect(ok("1.0.0", "2.0.0")).toBe(false);
    expect(ok("0.9.0", "1.0.0")).toBe(false);
  });

  test("minor_not_enforced_across_older_minor_in_0x", () => {
    expect(ok("0.1.0", "0.7.2")).toBe(true);
  });

  test("rejects_malformed", () => {
    for (const s of [
      "1",
      "0.1",
      "1.4",
      "",
      "0.1.0-beta",
      "0.1.0+b",
      "1.2.3.4",
      "a.b",
      "0.1.",
      ".1",
      "01.0",
      "v0.1",
      "0.-1",
    ]) {
      const p = problem(s, "0.1.0");
      expect(p.help, s).toContain('kashika="0.1.0"');
    }
  });

  test("prerelease_tool_compares_by_release", () => {
    expect(ok("0.1.0", "0.1.0-dev0")).toBe(true);
    expect(ok("0.0.5", "0.1.0-dev0")).toBe(true);
    expect(ok("0.1.0", "0.1.0+build.5")).toBe(true);
    expect(ok("0.1.1", "0.1.0-dev0")).toBe(false);
    expect(ok("0.2.0", "0.1.0-dev0")).toBe(false);
    expect(ok("1.0.0", "0.1.0-dev0")).toBe(false);
  });

  test("prerelease_file_is_malformed", () => {
    const p = problem("0.1.0-dev0", "0.1.0-dev0");
    expect(p.help).toContain('kashika="0.1.0"');
  });

  test("message_prints_full_tool_version", () => {
    const p = problem("0.2.0", "0.1.0-dev0");
    expect(p.message.endsWith("this is kashika 0.1.0-dev0")).toBe(true);
  });

  test("message_text", () => {
    const p = problem("0.4.0", "0.1.0");
    expect(p.message).toBe(
      "this file needs kashika 0.4.0 or newer (same major version); this is kashika 0.1.0",
    );
  });
});
