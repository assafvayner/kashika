import { existsSync, mkdirSync, readFileSync, readdirSync, rmSync, symlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { expect, test } from "vitest";

import { ROOT, fixture, kashika, tempdir } from "../support/cli.js";

const count = (s: string, sub: string) => s.split(sub).length - 1;
const isControl = (c: string) => {
  const cp = c.codePointAt(0)!;
  return cp <= 0x1f || (cp >= 0x7f && cp <= 0x9f);
};

test("check_clean_file_exits_zero", () => {
  const r = kashika(["check", fixture("basic.kash")]);
  expect(r.code).toBe(0);
  expect(r.stdout).toContain("0 errors, 0 warnings");
});

test("check_reports_typo_with_source_and_suggestion", () => {
  const r = kashika(["check", fixture("typo.kash")]);
  expect(r.code).toBe(2);
  expect(r.stdout).toContain(
    "typo.kash:4:10: error[unknown-anchor]: unknown anchor `rigth` on `api`",
  );
  expect(r.stdout).toContain("    4 | line api.rigth -> db.left end=arrow");
  expect(r.stdout).toContain("= help: did you mean `right`?");
  expect(r.stdout).toContain("1 error, 0 warnings");
});

test("json_envelope", () => {
  const r = kashika(["check", fixture("typo.kash"), "--format", "json"]);
  expect(r.code).toBe(2);
  const v = JSON.parse(r.stdout);
  expect(v.schema).toBe("kashika.check/1");
  expect(v.counts.error).toBe(1);
  expect(v.truncated).toBe(false);
  expect(v.output).toBeNull();
  expect(v.input.sha256.length).toBe(64);
  expect(v.diagnostics[0].code).toBe("unknown-anchor");
  expect(v.diagnostics[0].span.line).toBe(4);
});

test("render_writes_svg", () => {
  const dir = tempdir();
  const out = join(dir, "out.svg");
  const r = kashika(["render", fixture("basic.kash"), "-o", out]);
  expect(r.code).toBe(0);
  expect(r.stdout).toContain("output: written");
  expect(readFileSync(out, "utf8").startsWith("<svg")).toBe(true);
  expect(readdirSync(dir).length, "no temp files left behind").toBe(1);
});

test("render_does_not_write_on_error_unless_forced", () => {
  const dir = tempdir();
  const out = join(dir, "out.svg");
  const r = kashika(["render", fixture("typo.kash"), "-o", out]);
  expect(r.code).toBe(2);
  expect(r.stdout).toContain("output: not written");
  expect(existsSync(out)).toBe(false);
  expect(kashika(["render", fixture("typo.kash"), "-o", out, "--force"]).code).toBe(2);
  expect(existsSync(out)).toBe(true);
});

test("stdin_input_uses_stdin_name", () => {
  const src = readFileSync(fixture("typo.kash"), "utf8");
  const r = kashika(["check", "-", "--stdin-name", "snippet.kash"], { input: src });
  expect(r.code).toBe(2);
  expect(r.stdout).toContain("snippet.kash:4:");
});

test("fail_on_controls_exit_codes", () => {
  expect(kashika(["check", fixture("typo.kash"), "--fail-on", "never"]).code).toBe(0);
});

test("unknown_output_extension_is_a_tool_error", () => {
  const r = kashika(["render", fixture("basic.kash"), "-o", "x.pdf"]);
  expect(r.code).toBe(3);
  expect(r.stderr).toContain("output must end in .svg, .png, .jpg, .jpeg or .webp");
});

test("usage_errors_exit_three_and_help_exits_zero", () => {
  expect(kashika(["check"]).code).toBe(3);
  expect(kashika(["--help"]).code).toBe(0);
});

test("missing_file_is_a_tool_error", () => {
  const r = kashika(["check", "nope.kash"]);
  expect(r.code).toBe(3);
  expect(r.stderr).toContain("cannot read nope.kash");
});

test("bundled_examples_check_clean", () => {
  for (const name of ["upload", "shapes"]) {
    const r = kashika(["check", join(ROOT, `examples/${name}/${name}.kash`)]);
    expect(r.code).toBe(0);
    expect(r.stdout).toContain("0 errors, 0 warnings");
  }
});

test("failed_write_leaves_no_temp_file", () => {
  const dir = tempdir();
  const out = join(dir, "out.svg");
  mkdirSync(out);
  expect(kashika(["render", fixture("basic.kash"), "-o", out]).code).toBe(3);
  expect(readdirSync(dir), "no temp files left behind").toEqual(["out.svg"]);
});

test("huge_coordinates_fail_cleanly_even_when_forced", () => {
  const dir = tempdir();
  const out = join(dir, "out.svg");
  const big = "1".repeat(300);
  const src = `diagram "x" size (${big},100)\nbox a at (0,0) size (${big},10)\nline l (0,0) -> curve via (${big},0) (0,${big}) (10,10) end=arrow\n`;
  expect(kashika(["render", "-", "-o", out, "--force"], { input: src }).code).toBe(2);
  const svg = readFileSync(out, "utf8");
  expect(svg.includes("inf") || svg.includes("NaN"), svg).toBe(false);
});

test("measure_prints_width_and_height", () => {
  let r = kashika(["measure", "API"]);
  expect([r.code, r.stdout]).toEqual([0, "width=22.36 height=17.5 lines=1\n"]);
  r = kashika(["measure", "API", "--size", "28", "--weight", "bold"]);
  expect([r.code, r.stdout]).toEqual([0, "width=46.91 height=35 lines=1\n"]);
});

test("measure_applies_markup_and_newlines", () => {
  const r = kashika(["measure", "`API`\\nPostgres"]);
  expect([r.code, r.stdout]).toEqual([0, "width=58.71 height=35 lines=2\n"]);
});

test("measure_json", () => {
  const r = kashika(["measure", "SQL", "--size", "12", "--format", "json"]);
  expect(r.code).toBe(0);
  const v = JSON.parse(r.stdout);
  expect(v.schema).toBe("kashika.measure/1");
  expect(v.width).toBe(23.66);
  expect(v.height).toBe(15.0);
  expect(v.lines).toBe(1);
});

test("measure_reports_unclosed_markup_and_bad_sizes", () => {
  const r = kashika(["measure", "**API"]);
  expect([r.code, r.stdout]).toEqual([2, "error[parse]: unclosed `**` in text\n"]);
  expect(kashika(["measure", "API", "--size", "0"]).code).toBe(3);
});

test("check_reports_label_overflow", () => {
  const r = kashika(["check", fixture("overflow.kash")]);
  expect(r.code).toBe(2);
  expect(r.stdout).toContain(
    "overflow.kash:2:1: error[label-overflow]: its text needs 91.64x17.5px but the text area of `api` is 64x14px (too small: 27.64px narrower, 3.5px shorter)",
  );
  expect(r.stdout).toContain("= help: make `api` at least size (108,34), or shorten the text");
});

test("text_fixture_checks_clean_and_renders_deterministically", () => {
  expect(kashika(["check", fixture("text.kash")]).code).toBe(0);
  const dir = tempdir();
  const a = join(dir, "a.svg");
  const b = join(dir, "b.svg");
  for (const out of [a, b]) {
    expect(kashika(["render", fixture("text.kash"), "-o", out]).code).toBe(0);
  }
  const svg = readFileSync(a, "utf8");
  expect(svg).toBe(readFileSync(b, "utf8"));
  expect(svg).toContain('aria-label="Worker"');
  expect(svg).toContain('aria-label="• retries charge()"');
  expect(svg).not.toContain("<text");
});

test("warnings_suppression_and_metrics_in_text", () => {
  const r = kashika(["check", fixture("crossing.kash")]);
  expect(r.code).toBe(1);
  expect(r.stdout).toContain("crossing.kash:5:1: warn[cross]: v1 crosses ab at (200,150)");
  expect(r.stdout).toContain("    5 | line v1 (200,20) -> v 260");
  expect(r.stdout).toContain("    4 | line ab a.right -> b.left end=arrow");
  expect(r.stdout).toContain("= help: if intentional: allow=cross:ab on v1");
  expect(r.stdout).toContain("0 errors, 1 warning (1 suppressed) · crossings=2 wire=720px");
  expect(r.stdout).not.toContain("v2 crosses");
});

test("show_suppressed_lists_them_with_the_reason", () => {
  const r = kashika(["check", fixture("crossing.kash"), "--show-suppressed"]);
  expect(r.code).toBe(1);
  expect(r.stdout).toContain("warn[cross]: v2 crosses ab at (240,150) (suppressed)");
  expect(r.stdout).toContain("      = reason: bus");
});

test("json_has_counts_metrics_elements_and_facts", () => {
  const r = kashika(["check", fixture("crossing.kash"), "--format", "json"]);
  expect(r.code).toBe(1);
  const v = JSON.parse(r.stdout);
  expect(v.counts).toEqual({ error: 0, warn: 1, suppressed: 1 });
  expect(v.metrics).toEqual({ crossings: 2, wire_px: 720 });
  const diags = v.diagnostics;
  expect(diags.length).toBe(1);
  expect(diags[0].elements[0].id).toBe("v1");
  expect(diags[0].elements[0].segment).toBe(1);
  expect(diags[0].elements[1].id).toBe("ab");
  expect(diags[0].facts.at).toEqual([200.0, 150.0]);
  expect(diags[0].suggestions).toEqual([]);
});

test("focus_keeps_only_matching_warnings", () => {
  let r = kashika(["check", fixture("crossing.kash"), "--focus", "b"]);
  expect(r.code).toBe(0);
  expect(r.stdout).toContain("0 errors, 0 warnings");
  expect(kashika(["check", fixture("crossing.kash"), "--focus", "a,v1"]).code).toBe(1);
  r = kashika(["check", fixture("crossing.kash"), "--focus", "v3"]);
  expect(r.code).toBe(3);
  expect(r.stderr).toContain("unknown id `v3` in --focus; did you mean `v1`?");
});

test("warnings_exit_one_unless_fail_on_relaxes_it", () => {
  for (const [failOn, code] of [
    ["warn", 1],
    ["error", 0],
    ["never", 0],
  ] as const) {
    expect(kashika(["check", fixture("crossing.kash"), "--fail-on", failOn]).code).toBe(code);
  }
});

test("render_still_writes_with_warnings", () => {
  const dir = tempdir();
  const out = join(dir, "x.svg");
  const r = kashika(["render", fixture("crossing.kash"), "-o", out]);
  expect(r.code).toBe(1);
  expect(r.stdout).toContain("output: written");
  expect(existsSync(out)).toBe(true);
});

test("inspect_prints_geometry_in_text", () => {
  const r = kashika(["inspect", fixture("basic.kash")]);
  expect(r.code).toBe(0);
  expect(r.stdout).toContain('box api "API" (line 2)');
  expect(r.stdout).toContain(
    "  anchors top (100,40) bottom (100,100) left (40,70) right (160,70)",
  );
  expect(r.stdout).toContain("line api_db (line 4) length 160px");
  expect(r.stdout).toContain("  segment 1 to (4:");
  expect(r.stdout).toContain('  label@0.5 "SQL" at (240,70)');
  expect(r.stdout).toContain("placement\n  db -> api (at)");
  expect(r.stdout).toContain("0 errors, 0 warnings");
});

test("inspect_json_with_ids_ports_and_corridors", () => {
  const r = kashika([
    "inspect",
    fixture("basic.kash"),
    "db",
    "--ports",
    "--corridors",
    "0,0,480,240",
    "--format",
    "json",
  ]);
  expect(r.code).toBe(0);
  const v = JSON.parse(r.stdout);
  expect(v.schema).toBe("kashika.inspect/1");
  expect(v.shapes.length).toBe(1);
  expect(v.shapes[0].bbox).toEqual([
    [320.0, 40.0],
    [440.0, 100.0],
  ]);
  expect(v.shapes[0].anchors.left).toEqual([320.0, 70.0]);
  expect(v.shapes[0].local_anchors.left).toEqual([320.0, 70.0]);
  expect(v.ports).toEqual([
    { shape: "db", side: "left", t: 0.5, at: [320.0, 70.0], line: "api_db", end: "end" },
  ]);
  expect(v.corridors.horizontal).toEqual([
    [0.0, 34.0],
    [106.0, 240.0],
  ]);
  expect(v.corridors.vertical).toEqual([
    [0.0, 34.0],
    [446.0, 480.0],
  ]);
  expect(v.placement).toEqual([{ from: "db", to: "api", kind: "at" }]);
});

test("inspect_works_on_files_with_errors", () => {
  const r = kashika(["inspect", fixture("typo.kash")]);
  expect(r.code).toBe(0);
  expect(r.stdout).toContain('box api "API" (line 2)');
  expect(r.stdout).toContain("1 error, 0 warnings");
});

test("inspect_rejects_unknown_ids_and_bad_corridors", () => {
  expect(kashika(["inspect", fixture("basic.kash"), "nope"]).code).toBe(3);
  expect(kashika(["inspect", fixture("basic.kash"), "--corridors", "1,2,3"]).code).toBe(3);
});

test("seeded_mistakes_trigger_their_lints", () => {
  const r = kashika(["check", fixture("mistakes.kash"), "--format", "json", "--max", "200"]);
  const codes = new Set(JSON.parse(r.stdout).diagnostics.map((d: { code: string }) => d.code));
  for (const code of [
    "cross",
    "near",
    "overlap",
    "spill",
    "diagonal-tail",
    "port-crowding",
    "low-contrast",
    "out-of-canvas",
    "off-grid",
  ]) {
    expect(codes.has(code), `${code} missing from ${[...codes]}`).toBe(true);
  }
});

test("library_files_check_on_their_own", () => {
  const dir = tempdir();
  const lib = join(dir, "lib.kash");
  writeFileSync(lib, 'style s { fill=#fff }\nshape t from "t.svg"\n');
  let r = kashika(["check", lib]);
  expect(r.code).toBe(2);
  expect(r.stdout).toContain("lib.kash:2:14: error[file]: file `t.svg` not found");
  expect(r.stdout).not.toContain("missing `diagram`");
  expect(r.stdout).toContain("1 error, 0 warnings\n");
  writeFileSync(lib, "style s { fill=#fff }\n");
  expect(kashika(["check", lib]).code).toBe(0);
  r = kashika(["render", lib, "-o", join(dir, "out.svg")]);
  expect(r.code).toBe(3);
  expect(r.stderr).toContain(
    "is a library (no `diagram` statement); render a diagram that imports it",
  );
  expect(existsSync(join(dir, "out.svg"))).toBe(false);

  writeFileSync(lib, "style s { fill=#fff }\nbox a at (0,0) size (1,1)\n");
  r = kashika(["check", lib]);
  expect(r.code).toBe(2);
  expect(r.stdout).toContain("missing `diagram` statement");
});

test("repeated_errors_are_all_printed", () => {
  let src = 'diagram "t" size (100,100)\n';
  for (let i = 0; i < 10; i++) src += `line l${i}: (0,${i}) -> h 10\n`;
  src += "box a at (50,50) size (10,10)\nbox a at (70,50) size (10,10)\n";
  const r = kashika(["check", "-", "--stdin-name", "r.kash", "--max", "3"], { input: src });
  expect(r.code).toBe(2);
  const out = r.stdout;
  expect(count(out, "a line id takes no `:`"), out).toBe(10);
  expect(out).toContain("error[duplicate-id]");
  expect(out).not.toContain("more like this");
  expect(out).toContain("11 errors, 0 warnings");
});

/// `n` boxes off a 100x100 canvas, one distinct `out-of-canvas` warning each, then a parse error.
function warningsThenError(n: number): string {
  let src = 'diagram "t" size (100,100)\n';
  for (let i = 0; i < n; i++) src += `box b${i} at (200,${i * 20}) size (10,10)\n`;
  src += "line l: (0,0) -> h 10\n";
  return src;
}

test("max_never_hides_errors_in_text", () => {
  const r = kashika(["check", "-", "--stdin-name", "w.kash", "--max", "5"], {
    input: warningsThenError(8),
  });
  expect(r.code).toBe(2);
  const out = r.stdout;
  expect(count(out, ": warn[out-of-canvas]"), out).toBe(4);
  expect(out).toContain("w.kash:10:7: error[parse]");
  expect(out).toContain("... and 4 more\n1 error, 8 warnings");
});

test("max_never_hides_errors_in_json", () => {
  let r = kashika(["check", "-", "--format", "json", "--max", "3"], {
    input: warningsThenError(8),
  });
  expect(r.code).toBe(2);
  let v = JSON.parse(r.stdout);
  const diags = v.diagnostics;
  expect(diags.length).toBe(3);
  expect(v.truncated).toBe(true);
  expect(v.counts.error).toBe(1);
  expect(diags[0].severity).toBe("error");
  expect(diags[1].code).toBe("out-of-canvas");

  r = kashika(["check", "-", "--format", "json", "--max", "0"], {
    input: warningsThenError(2),
  });
  v = JSON.parse(r.stdout);
  expect(v.diagnostics.map((d: { code: string }) => d.code)).toEqual(["parse"]);
  expect(v.truncated).toBe(true);
});

test("errors_print_even_when_they_alone_exceed_max", () => {
  let src = 'diagram "t" size (100,100)\n';
  for (let i = 0; i < 4; i++) {
    src += `box a${i} at (0,0) size (10,10)\nbox a${i} at (20,0) size (10,10)\n`;
  }
  const r = kashika(["check", "-", "--max", "1"], { input: src });
  expect(r.code).toBe(2);
  expect(count(r.stdout, "error[duplicate-id]"), r.stdout).toBe(4);
  expect(r.stdout).toContain("... and 6 more\n4 errors, 6 warnings");
});

test("text_output_escapes_control_characters", () => {
  const src = 'diagram "t" size (100,100)\ntext t "\u{1b}]52;c;aGk=\u{7}\u{9b}x" at (90,50)\n';
  let r = kashika(["check", "-", "--stdin-name", "a\u{1b}[2J.kash"], { input: src });
  const text = r.stdout;
  expect([...text].some((c) => isControl(c) && c !== "\n"), JSON.stringify(text)).toBe(false);
  expect(text).toContain("a\\u{1b}[2J.kash:2:1: warn[");
  expect(text).toContain('text t "\\u{1b}]52;c;aGk=\\u{7}\\u{9b}x"');

  r = kashika(["check", "-", "--format", "json"], { input: src });
  const json = r.stdout;
  expect([...json].some((c) => isControl(c) && c !== "\n"), JSON.stringify(json)).toBe(false);
});

test("tool_errors_escape_control_characters", () => {
  const r = kashika(["check", "missing\u{1b}]0;x\u{7}.kash"]);
  expect(r.code).toBe(3);
  expect(r.stderr).toContain("missing\\u{1b}]0;x\\u{7}.kash");
});

/// `n` horizontal lines crossing `n` vertical ones.
function crossingGrid(n: number): string {
  let src = 'diagram "grid" size (1000,1000)\n';
  for (let i = 0; i < n; i++) {
    const at = 50 + 20 * i;
    src += `line h${i} (20,${at}) -> h 900\n`;
    src += `line v${i} (${at},20) -> v 900\n`;
  }
  return src;
}

test("counts_stay_exact_past_the_findings_kept_per_code", () => {
  let r = kashika(["check", "-", "--format", "json", "--max", "5"], { input: crossingGrid(40) });
  const v = JSON.parse(r.stdout);
  expect(v.counts).toEqual({ error: 0, warn: 1600, suppressed: 0 });
  expect(v.metrics.crossings).toBe(1600);
  expect(v.by_code.cross).toEqual({ active: 1600, suppressed: 0 });
  expect(v.truncated).toBe(true);
  expect(v.diagnostics.length).toBe(5);
  r = kashika(["check", "-", "--max", "5"], { input: crossingGrid(40) });
  expect(r.code).toBe(1);
  expect(r.stdout).toContain("... and 1595 more\n");
  expect(r.stdout).toContain("0 errors, 1600 warnings");
});

function runOk(args: string[], stdin: string): [number, string, string] {
  const r = kashika(args, { input: stdin });
  return [r.code, r.stdout, r.stderr];
}

test("text_lists_errors_before_warnings", () => {
  const [code, out] = runOk(["check", "-", "--stdin-name", "w.kash"], warningsThenError(2));
  expect(code).toBe(2);
  const err = out.indexOf("error[parse]");
  const warn = out.indexOf("warn[out-of-canvas]");
  expect(err >= 0 && warn >= 0 && err < warn, out).toBe(true);
});

test("summary_counts_per_code", () => {
  const src = warningsThenError(3);
  let [, out] = runOk(["check", "-", "--summary"], src);
  expect(out).toContain("by code:\n  out-of-canvas: 3\n  parse: 1\n");
  [, out] = runOk(["check", "-", "--format", "json"], src);
  const v = JSON.parse(out);
  expect(v.by_code["out-of-canvas"].active).toBe(3);
  expect(v.by_code.parse.suppressed).toBe(0);
});

test("tool_error_schema_matches_the_command", () => {
  const cases: [string[], string][] = [
    [["check", "/nonexistent.kash", "--format", "json"], "kashika.check/1"],
    [["route", "/nonexistent.kash", "a", "b", "--format", "json"], "kashika.route/1"],
    [["inspect", "/nonexistent.kash", "--format", "json"], "kashika.inspect/1"],
    [["ref", "nosuch", "--json"], "kashika.ref/1"],
    [["measure", "x", "--size", "0", "--format", "json"], "kashika.measure/1"],
    [["icons", "/nonexistent-dir", "--format", "json"], "kashika.icons/1"],
    [["render", "/nonexistent.kash", "-o", "o.svg", "--format", "json"], "kashika.check/1"],
  ];
  for (const [args, schema] of cases) {
    const [code, out] = runOk(args, "");
    expect(code, args.join(" ")).toBe(3);
    const v = JSON.parse(out);
    expect(v.schema, args.join(" ")).toBe(schema);
    expect(typeof v.tool_error, args.join(" ")).toBe("string");
  }
});

test("inspect_rejects_check_flags", () => {
  for (const flag of [
    ["--fail-on", "never"],
    ["--max", "3"],
    ["--focus", "a"],
  ]) {
    expect(runOk(["inspect", "-", ...flag], "")[0], flag.join(" ")).toBe(3);
  }
});

test("inspect_corridors_without_ids_prints_only_corridors", () => {
  let [code, out] = runOk(
    ["inspect", "-", "--corridors", "0,0,800,600"],
    "box a at (10,10) size (100,60)\n",
  );
  expect(code).toBe(0);
  expect(out.startsWith("corridors "), out).toBe(true);
  expect(out).not.toContain("box a");
  [, out] = runOk(
    ["inspect", "-", "a", "--corridors", "0,0,800,600"],
    "box a at (10,10) size (100,60)\n",
  );
  expect(out).toContain("box a");
});

test("numeric_arguments_are_validated", () => {
  const src = "box a at (10,10) size (100,60)\n";
  for (const args of [
    ["inspect", "-", "--corridors", "0,0,nan,nan"],
    ["inspect", "-", "--corridors", "0,0,1e999,5"],
    ["route", "-", "a.right", "a.left", "--radius", "nan"],
    ["route", "-", "a.right", "a.left", "--radius", "-3"],
    ["route", "-", "a.right", "a.left", "--max", "0"],
  ]) {
    const [code, , err] = runOk(args, src);
    expect(code, args.join(" ")).toBe(3);
    expect(err).not.toContain("-- -3");
  }
});

test("fmt_reads_stdin_and_writes_stdout", () => {
  let [code, out] = runOk(["fmt", "-"], "box a at (10, 10)   size (100, 60)\n");
  expect(code).toBe(0);
  expect(out).toBe("box a at (10, 10) size (100, 60)\n");
  [code, out] = runOk(["fmt", "-", "--check"], "box a at (10, 10)   size (100, 60)\n");
  expect(code).toBe(1);
  expect(out).toContain("not formatted");
  [code, out] = runOk(["fmt", "-", "--check"], "box a at (10, 10) size (100, 60)\n");
  expect([code, out]).toEqual([0, ""]);
  expect(runOk(["fmt", "-"], "box a at (\n")[0]).toBe(2);
});

test("measure_takes_several_strings", () => {
  let [code, out] = runOk(["measure", "a", "bb"], "");
  expect(code).toBe(0);
  expect(out.trimEnd().split("\n").length, out).toBe(2);
  [, out] = runOk(["measure", "a", "bb", "--format", "json"], "");
  let v = JSON.parse(out);
  expect(v.results.length).toBe(2);
  expect(v.results[1].text).toBe("bb");
  [, out] = runOk(["measure", "a", "--format", "json"], "");
  v = JSON.parse(out);
  expect(typeof v.width === "number" && v.results === undefined).toBe(true);
});

test("icons_search_without_hits_and_bad_dir", () => {
  const [code, out, err] = runOk(["icons", "--search", "zzzzqq"], "");
  expect([code, out]).toEqual([1, ""]);
  expect(err).toContain("no icons or shapes match `zzzzqq`");
  expect(runOk(["icons", "/nonexistent-dir"], "")[0]).toBe(3);
});

test("icons_does_not_follow_symlinked_directories", () => {
  const dir = join(tmpdir(), `kashika-icons-loop-${process.pid}`);
  mkdirSync(join(dir, "sub"), { recursive: true });
  symlinkSync(dir, join(dir, "sub/loop"));
  const [code] = runOk(["icons", dir], "");
  rmSync(dir, { recursive: true, force: true });
  expect(code).toBe(0);
});

test("ref_unknown_topic_separates_the_topic_list", () => {
  const [code, , err] = runOk(["ref", "zzzzzz"], "");
  expect(code).toBe(3);
  expect(err).toContain("`zzzzzz`; pass a diagnostic code or one of these topics:\n  rules ");
});

test("library_validates_and_applies_focus", () => {
  const lib = 'shape pill {\n  path "M0,0 L1,0 L1,1 Z"\n}\n';
  const [code, , err] = runOk(["check", "-", "--focus", "nosuch"], lib);
  expect(code, err).toBe(3);
  expect(err).toContain("unknown id `nosuch`");
});

test("route_hints_that_points_take_no_arrow", () => {
  const r = kashika(["route", fixture("route.kash"), "a.right", "->", "b.left"]);
  expect(r.code).toBe(3);
  expect(r.stderr).toContain(
    "help: `route` takes the two points as separate arguments, with no `->`",
  );
});

test("version_prints_the_tool_version", () => {
  const r = kashika(["--version"]);
  expect([r.code, r.stdout]).toEqual([0, "kashika 0.1.0-dev0\n"]);
});
