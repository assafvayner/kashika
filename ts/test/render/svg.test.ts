import { readFileSync } from "node:fs";
import { join } from "node:path";
import { expect, test } from "vitest";

import { compile } from "../../src/model/index.js";
import { renderSvg } from "../../src/render/index.js";

const BASIC =
  'diagram "Basic" size (480,240) kashika="0.1.0"\n' +
  'box api "API" at (40,40) size (120,60) style=service\n' +
  'box db "Postgres" at api.right+(160,0) size (120,60) pin=left style=datastore\n' +
  'line api_db api.right -> db.left end=arrow label="SQL"\n';

function svg(src: string): string {
  const [scene, diags] = compile(src);
  expect(diags).toEqual([]);
  return renderSvg(scene);
}

function count(s: string, needle: string): number {
  return s.split(needle).length - 1;
}

function find(s: string, needle: string): number {
  const i = s.indexOf(needle);
  if (i < 0) throw new Error(`${needle} not found in ${s}`);
  return i;
}

/// The body of an insta snapshot, after its `---` header.
function snapshot(name: string): string {
  const text = readFileSync(
    join(import.meta.dirname, "../../../crates/kashika-render/tests/snapshots", `${name}.snap`),
    "utf8",
  );
  const head = text.indexOf("---\n", 4);
  return text.slice(head + 4);
}

test("renders_boxes_lines_markers_and_labels", () => {
  const s = svg(BASIC);
  expect(
    s.startsWith(
      '<svg xmlns="http://www.w3.org/2000/svg" width="480" height="240" viewBox="0 0 480 240">',
    ),
  ).toBe(true);
  expect(s).toContain(
    '<rect x="40" y="40" width="120" height="60" rx="0" fill="#e8f0fe" stroke="#1a73e8" stroke-width="1.5"/>',
  );
  expect(s).toContain('d="M 160 70 L 310 70"');
  expect(s).toContain('<polygon points="320,70 310,74 310,66"');
  expect(s).toContain('<path aria-label="SQL" d="M');
  expect(s).toContain('<path aria-label="Postgres" d="M');
  expect(s).not.toContain("<text");
});

test("groups_are_dashed_and_drawn_first", () => {
  const s = svg(
    'diagram "g" size (200,200)\ngroup g "VPC" at (10,10) size (180,180) {\n  box a at (10,30) size (50,30)\n}\n',
  );
  const group = find(s, 'stroke-dasharray="6 4"');
  const child = find(s, '<rect x="20" y="40"');
  expect(group).toBeLessThan(child);
  expect(s).toContain('aria-label="VPC"');
});

test("emoji_draw_as_colored_layers_in_a_labeled_group", () => {
  const src = 'diagram "e" size (200,100)\nbox a "Go 🚀" at (0,0) size (90,40) color=#123456\n';
  const s = svg(src);
  const g = find(s, '<g aria-label="Go 🚀">');
  const end = g + find(s.slice(g), "</g>");
  const group = s.slice(g, end);
  expect(group).toContain('<path d="M');
  expect(group).toContain('fill="#123456"/>');
  expect(count(group, "<path d=")).toBeGreaterThan(2);
  expect(s).not.toContain('<path aria-label="Go');
  expect(s).toBe(svg(src));
});

test("escapes_text", () => {
  const s = svg('diagram "e" size (100,100)\nbox a "a < b & c" at (0,0) size (90,40)\n');
  expect(s).toContain('aria-label="a &lt; b &amp; c"');
});

test("multi_line_labels_stack", () => {
  const s = svg('diagram "m" size (100,100)\nbox a "one\\ntwo" at (0,0) size (100,100)\n');
  const one = find(s, 'aria-label="one"');
  const two = find(s, 'aria-label="two"');
  expect(one).toBeLessThan(two);
});

test("snapshot_basic", () => {
  expect(svg(BASIC).trimEnd()).toBe(snapshot("svg__basic").trimEnd());
});

test("paint_order_is_groups_boxes_lines_then_overlays_in_file_order", () => {
  const s = svg(
    'diagram "p" size (300,300)\n' +
      'text "T" at (5,5)\n' +
      'line l (0,150) -> h 300 label="L"\n' +
      "box b at (100,100) size (50,50)\n" +
      "group outer at (0,0) size (300,300) {\n  group inner at (20,20) size (100,100) {\n  }\n}\n",
  );
  const outer = find(s, '<rect x="0" y="0" width="300"');
  const inner = find(s, '<rect x="20" y="20"');
  const bx = find(s, '<rect x="100" y="100"');
  const line = find(s, 'd="M 0 150 L 300 150"');
  const label = find(s, 'aria-label="L"');
  const text = find(s, 'aria-label="T"');
  expect(outer < inner && inner < bx && bx < line && line < text && text < label).toBe(true);
});

test("junction_dot_is_six_pixels", () => {
  const s = svg('diagram "j" size (100,100)\njunction a at (10,20) dot\njunction b at (30,40)\n');
  expect(s).toContain('<circle cx="10" cy="20" r="3"');
  expect(s).not.toContain('cx="30"');
});

function pathD(s: string): string {
  const start = s.indexOf('<path d="') + 9;
  return s.slice(start, start + s.slice(start).indexOf('"'));
}

test("markers_trim_a_final_curve", () => {
  const s = svg(
    'diagram "c" size (200,200)\nline l (0,100) -> curve left 20 (100,100) end=arrow start=diamond\n',
  );
  const d = pathD(s);
  expect(d.startsWith("M ") && !d.startsWith("M 0 100")).toBe(true);
  expect(d.endsWith(" 100 100")).toBe(false);
});

test("markers_at_both_ends_of_a_short_line_share_it", () => {
  const s = svg('diagram "c" size (200,200)\nline l (0,50) -> h 15 start=arrow end=arrow\n');
  expect(pathD(s)).toBe("M 7.5 50 L 7.5 50");
});

test("crows_foot_ends_keep_the_full_stroke", () => {
  const s = svg('diagram "c" size (200,100)\nline l (20,50) -> h 160 start=one end=many\n');
  expect(pathD(s)).toBe("M 20 50 L 180 50");
  expect(s).toContain(
    '<path d="M 28 44 L 28 56" fill="none" stroke="#334155" stroke-width="1.5" stroke-linecap="round"/>',
  );
  expect(s).toContain("M 168 50 L 180 56 M 168 50 L 180 50 M 168 50 L 180 44");
  expect(s).not.toContain("<mask ");
});

test("zero_ends_mask_the_line_inside_their_open_circle", () => {
  const s = svg('diagram "c" size (200,100)\nline l (20,50) -> h 160 end=zero-many\n');
  expect(s).toContain('<circle cx="160" cy="50" r="3.25" fill="#000"/></mask>');
  expect(s).toContain(
    '<circle cx="160" cy="50" r="4" fill="none" stroke="#334155" stroke-width="1.5"/>',
  );
  expect(s).not.toContain('<rect width="100%"');
});

test("line_labels_get_a_mask_and_texts_do_not", () => {
  const s = svg(
    'diagram "h" size (200,100)\nline l (0,50) -> h 200 label="L"\ntext "T" at (100,90)\n',
  );
  const mask = find(s, 'fill="#000"/></mask>');
  const id = maskIds(s)[0]!;
  const group = find(s, `<g mask="url(#${id})">`);
  const path = find(s, '<path d="M 0 50');
  const close = find(s, "</g>");
  expect(mask < group && group < path && path < close).toBe(true);
  expect(close).toBeLessThan(find(s, 'aria-label="L"'));
  expect(s).not.toContain('stroke-width="4"');
  const text = s.slice(find(s, 'aria-label="T"'));
  expect(text.split("\n")[0]!.endsWith('fill="#0f172a"/>')).toBe(true);
  expect(count(s, "<mask ")).toBe(1);
  expect(count(s, "<rect ")).toBe(2);
});

test("markup_switches_fonts_in_one_path", () => {
  const plain = svg('diagram "m" size (200,100)\ntext "ab" at (100,50)\n');
  const bold = svg('diagram "m" size (200,100)\ntext "**ab**" at (100,50)\n');
  expect(bold).toContain('aria-label="ab"');
  expect(plain).not.toBe(bold);
});

test("spaces_only_lines_draw_nothing", () => {
  const s = svg('diagram "s" size (100,100)\ntext "  " at (50,50)\n');
  expect(s).not.toContain("aria-label");
});

test("nested_groups_paint_before_later_sibling_groups", () => {
  const s = svg(
    'diagram "g" size (400,200)\ngroup a "A" at (10,10) size (180,180) {\n  group a2 "A2" at (10,30) size (100,100) {\n  }\n}\ngroup b "B" at (210,10) size (180,180) {\n}\n',
  );
  const pos = (label: string) => find(s, `aria-label="${label}"`);
  expect(pos("A") < pos("A2") && pos("A2") < pos("B")).toBe(true);
});

test("line_labels_and_texts_paint_in_file_order", () => {
  const s = svg('diagram "t" size (200,100)\ntext "T" at (10,10)\nline l (0,50) -> h 100 label="L"\n');
  expect(find(s, 'aria-label="T"')).toBeLessThan(find(s, 'aria-label="L"'));
});

test("canvas_is_transparent_unless_background_is_set", () => {
  const canvas = '<rect width="100%" height="100%"';
  let s = svg('diagram "t" size (100,100)\nbox a at (0,0) size (50,50)\n');
  expect(s).not.toContain(canvas);
  s = svg('diagram "t" size (100,100) background=none\nbox a at (0,0) size (50,50)\n');
  expect(s).not.toContain(canvas);
  s = svg('diagram "t" size (100,100) background=#0f172a\nbox a at (0,0) size (50,50)\n');
  const bg = find(s, '<rect width="100%" height="100%" fill="#0f172a"/>');
  const firstBox = find(s, '<rect x="0" y="0"');
  expect(bg).toBeLessThan(firstBox);
});

test("line_labels_mask_their_own_line_without_painting", () => {
  const src = 'diagram "t" size (300,100)\nline l (20,50) -> (280,50) label="open"\n';
  const s = svg(src);
  expect(s).not.toContain("#ffffff");
  expect(s).toContain("<mask ");
  expect(s).toContain('mask="url(#');
  expect(s).toBe(svg(src));
});

test("named_colors_render_as_hex", () => {
  const s = svg(
    'diagram "t" size (100,100) background=ivory\nbox a "x" at (10,10) size (40,40) fill=Tomato color=white\n',
  );
  for (const hex of ["#fffff0", "#ff6347", "#ffffff"]) {
    expect(s).toContain(hex);
  }
  for (const name of ["ivory", "tomato", "Tomato", "white"]) {
    expect(s).not.toContain(name);
  }
});

function maskIds(s: string): string[] {
  const out: string[] = [];
  const m = '<mask id="';
  let i = s.indexOf(m);
  while (i >= 0) {
    const rest = s.slice(i + m.length);
    out.push(rest.slice(0, rest.indexOf('"')));
    i = s.indexOf(m, i + m.length);
  }
  return out;
}

test("mask_ids_are_unique_per_diagram_and_stable", () => {
  const a =
    'diagram "a" size (200,100)\nline l (0,50) -> h 200 label="L"\nline m (0,80) -> h 200 label="M"\n';
  const b = 'diagram "b" size (200,100)\nline l (0,50) -> h 200 label="L"\n';
  const [ia, ib] = [maskIds(svg(a)), maskIds(svg(b))];
  expect(ia.length).toBe(2);
  for (const id of [...ia, ...ib]) {
    const m = /^kashika-(.*?)-mask-/.exec(id);
    expect(m, id).not.toBeNull();
    expect(/^[0-9a-f]{8}$/i.test(m![1]!), id).toBe(true);
  }
  expect(ia[0]).not.toBe(ia[1]);
  expect(ib.every((id) => !ia.includes(id))).toBe(true);
  expect(ia).toEqual(maskIds(svg(a)));
});

test("drops_characters_invalid_in_xml", () => {
  const [scene, diags] = compile('diagram "e" size (100,100)\nbox a "ab" at (0,0) size (90,40)\n');
  expect(diags).toEqual([]);
  scene.shapes[0]!.text[0]!.lines[0]!.runs[0]!.text = "a\u0001\u000b￾￿b\tc";
  const s = renderSvg(scene);
  expect(s).toContain('aria-label="ab\tc"');
});

test("repeated_icons_are_embedded_once", () => {
  const s = svg(
    'diagram "r" size (400,100)\nbox a at (0,0) size (60,60) icon="server"\nbox b at (100,0) size (60,60) icon="server"\nbox c at (200,0) size (60,60) icon="lock"\n',
  );
  expect(count(s, "data:image/svg+xml;base64,")).toBe(2);
  expect(count(s, '<use href="#kashika-')).toBe(3);
});
