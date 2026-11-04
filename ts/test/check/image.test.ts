import { rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { expect, it } from "vitest";

import { toJsonString } from "../../src/diag/json.js";
import { lintIn, tempdir } from "./common/mod.js";

function withDir(f: (dir: string) => void): void {
  const dir = tempdir();
  try {
    f(dir);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}

it("image_distort_flags_stretch_beyond_five_percent", () =>
  withDir((dir) => {
    const d = lintIn(
      dir,
      "image-distort",
      'image a "logo.png" at (0,0) size (80,80) fit=stretch\n' +
        'image b "logo.png" at (100,0) size (80,41) fit=stretch\n' +
        'image c "logo.png" at (200,0) size (80,80)\n',
    );
    expect(d.length).toBe(1);
    expect(d[0]!.span.line).toBe(2);
    expect(d[0]!.message).toBe("image 'a' is stretched 50% away from its 200x100 aspect ratio");
    expect(toJsonString(d[0]!.facts.get("natural")!)).toBe("[200.0,100.0]");
    expect(toJsonString(d[0]!.facts.get("distortion")!)).toBe("0.5");
    expect(d[0]!.help.some((h) => h.includes("size (80, 40)"))).toBe(true);
  }));

it("image_distort_is_quiet_for_kept_ratios_and_other_fits", () =>
  withDir((dir) => {
    expect(
      lintIn(
        dir,
        "image-distort",
        'image a "logo.png" at (0,0) size (80,40) fit=stretch\n' +
          'image b "logo.png" at (100,0) size (80,80) fit=cover\n' +
          'image c "logo.png" at (200,0) size (80,80) fit=contain\n',
      ),
    ).toEqual([]);
  }));

it("image_distort_can_be_allowed", () =>
  withDir((dir) => {
    const d = lintIn(
      dir,
      "image-distort",
      'image a "logo.png" at (0,0) size (80,80) fit=stretch allow=image-distort\n',
    );
    expect(d.length).toBe(1);
    expect(d[0]!.suppressed).toBe(true);
  }));

it("image_distort_flags_stretched_svg_shapes", () =>
  withDir((dir) => {
    const d = lintIn(
      dir,
      "image-distort",
      'shape s from "server" fit=stretch\nshape c from "server"\n' +
        "box a at (0,0) size (80,40) shape=s\n" +
        "box b at (100,0) size (60,60) shape=s\n" +
        "box k at (200,0) size (80,40) shape=c\n",
    );
    expect(d.length).toBe(1);
    expect(d[0]!.span.line).toBe(4);
    expect(d[0]!.message).toBe("box 'a' is stretched 100% away from its 24x24 aspect ratio");
    expect(
      d[0]!.help.some((h) => h.includes("fit=cover on shape `s`, or size (80, 80)")),
    ).toBe(true);
  }));

const SVG_HEAD = '<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20">';

it("svg_dropped_flags_lost_images_once_per_file", () =>
  withDir((dir) => {
    writeFileSync(
      join(dir, "pic.svg"),
      `${SVG_HEAD}<image href="photo.png" width="10" height="10"/><image href="https://example.com/x.png" width="10" height="10"/><rect width="5" height="5"/></svg>`,
    );
    const d = lintIn(
      dir,
      "svg-dropped",
      'image a "pic.svg" at (0,0) size (40,40)\nbox b "B" at (100,0) size (120,40) icon="pic.svg"\n',
    );
    expect(d.length).toBe(1);
    expect(d[0]!.span.line).toBe(2);
    expect(d[0]!.message).toBe("'pic.svg' drops 2 `<image>` elements when embedded");
    expect(toJsonString(d[0]!.facts.get("dropped")!)).toBe('{"image":2}');
    expect(d[0]!.help).toEqual(["embed it as a path, or use a separate `image`"]);
  }));

it("svg_dropped_is_quiet_for_data_images_and_text", () =>
  withDir((dir) => {
    writeFileSync(
      join(dir, "ok.svg"),
      `${SVG_HEAD}<image href="data:image/svg+xml;utf8,&lt;svg xmlns='http://www.w3.org/2000/svg' width='4' height='4'&gt;&lt;rect width='4' height='4'/&gt;&lt;/svg&gt;" width="10" height="10"/><text x="2" y="15" font-size="10">Hi</text></svg>`,
    );
    expect(
      lintIn(dir, "svg-dropped", 'image a "ok.svg" at (0,0) size (40,40)\n'),
    ).toEqual([]);
  }));
