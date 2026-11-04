import { describe, expect, test } from "vitest";
import { type Diagnostic } from "../../src/diag/diagnostic.js";
import { type Scene, compile } from "../../src/model/index.js";

const HEAD = 'diagram "t" size (800,600)\n';

const CSS: [string, string][] = [
  ["aliceblue", "#f0f8ff"],
  ["antiquewhite", "#faebd7"],
  ["aqua", "#00ffff"],
  ["aquamarine", "#7fffd4"],
  ["azure", "#f0ffff"],
  ["beige", "#f5f5dc"],
  ["bisque", "#ffe4c4"],
  ["black", "#000000"],
  ["blanchedalmond", "#ffebcd"],
  ["blue", "#0000ff"],
  ["blueviolet", "#8a2be2"],
  ["brown", "#a52a2a"],
  ["burlywood", "#deb887"],
  ["cadetblue", "#5f9ea0"],
  ["chartreuse", "#7fff00"],
  ["chocolate", "#d2691e"],
  ["coral", "#ff7f50"],
  ["cornflowerblue", "#6495ed"],
  ["cornsilk", "#fff8dc"],
  ["crimson", "#dc143c"],
  ["cyan", "#00ffff"],
  ["darkblue", "#00008b"],
  ["darkcyan", "#008b8b"],
  ["darkgoldenrod", "#b8860b"],
  ["darkgray", "#a9a9a9"],
  ["darkgreen", "#006400"],
  ["darkgrey", "#a9a9a9"],
  ["darkkhaki", "#bdb76b"],
  ["darkmagenta", "#8b008b"],
  ["darkolivegreen", "#556b2f"],
  ["darkorange", "#ff8c00"],
  ["darkorchid", "#9932cc"],
  ["darkred", "#8b0000"],
  ["darksalmon", "#e9967a"],
  ["darkseagreen", "#8fbc8f"],
  ["darkslateblue", "#483d8b"],
  ["darkslategray", "#2f4f4f"],
  ["darkslategrey", "#2f4f4f"],
  ["darkturquoise", "#00ced1"],
  ["darkviolet", "#9400d3"],
  ["deeppink", "#ff1493"],
  ["deepskyblue", "#00bfff"],
  ["dimgray", "#696969"],
  ["dimgrey", "#696969"],
  ["dodgerblue", "#1e90ff"],
  ["firebrick", "#b22222"],
  ["floralwhite", "#fffaf0"],
  ["forestgreen", "#228b22"],
  ["fuchsia", "#ff00ff"],
  ["gainsboro", "#dcdcdc"],
  ["ghostwhite", "#f8f8ff"],
  ["gold", "#ffd700"],
  ["goldenrod", "#daa520"],
  ["gray", "#808080"],
  ["green", "#008000"],
  ["greenyellow", "#adff2f"],
  ["grey", "#808080"],
  ["honeydew", "#f0fff0"],
  ["hotpink", "#ff69b4"],
  ["indianred", "#cd5c5c"],
  ["indigo", "#4b0082"],
  ["ivory", "#fffff0"],
  ["khaki", "#f0e68c"],
  ["lavender", "#e6e6fa"],
  ["lavenderblush", "#fff0f5"],
  ["lawngreen", "#7cfc00"],
  ["lemonchiffon", "#fffacd"],
  ["lightblue", "#add8e6"],
  ["lightcoral", "#f08080"],
  ["lightcyan", "#e0ffff"],
  ["lightgoldenrodyellow", "#fafad2"],
  ["lightgray", "#d3d3d3"],
  ["lightgreen", "#90ee90"],
  ["lightgrey", "#d3d3d3"],
  ["lightpink", "#ffb6c1"],
  ["lightsalmon", "#ffa07a"],
  ["lightseagreen", "#20b2aa"],
  ["lightskyblue", "#87cefa"],
  ["lightslategray", "#778899"],
  ["lightslategrey", "#778899"],
  ["lightsteelblue", "#b0c4de"],
  ["lightyellow", "#ffffe0"],
  ["lime", "#00ff00"],
  ["limegreen", "#32cd32"],
  ["linen", "#faf0e6"],
  ["magenta", "#ff00ff"],
  ["maroon", "#800000"],
  ["mediumaquamarine", "#66cdaa"],
  ["mediumblue", "#0000cd"],
  ["mediumorchid", "#ba55d3"],
  ["mediumpurple", "#9370db"],
  ["mediumseagreen", "#3cb371"],
  ["mediumslateblue", "#7b68ee"],
  ["mediumspringgreen", "#00fa9a"],
  ["mediumturquoise", "#48d1cc"],
  ["mediumvioletred", "#c71585"],
  ["midnightblue", "#191970"],
  ["mintcream", "#f5fffa"],
  ["mistyrose", "#ffe4e1"],
  ["moccasin", "#ffe4b5"],
  ["navajowhite", "#ffdead"],
  ["navy", "#000080"],
  ["oldlace", "#fdf5e6"],
  ["olive", "#808000"],
  ["olivedrab", "#6b8e23"],
  ["orange", "#ffa500"],
  ["orangered", "#ff4500"],
  ["orchid", "#da70d6"],
  ["palegoldenrod", "#eee8aa"],
  ["palegreen", "#98fb98"],
  ["paleturquoise", "#afeeee"],
  ["palevioletred", "#db7093"],
  ["papayawhip", "#ffefd5"],
  ["peachpuff", "#ffdab9"],
  ["peru", "#cd853f"],
  ["pink", "#ffc0cb"],
  ["plum", "#dda0dd"],
  ["powderblue", "#b0e0e6"],
  ["purple", "#800080"],
  ["rebeccapurple", "#663399"],
  ["red", "#ff0000"],
  ["rosybrown", "#bc8f8f"],
  ["royalblue", "#4169e1"],
  ["saddlebrown", "#8b4513"],
  ["salmon", "#fa8072"],
  ["sandybrown", "#f4a460"],
  ["seagreen", "#2e8b57"],
  ["seashell", "#fff5ee"],
  ["sienna", "#a0522d"],
  ["silver", "#c0c0c0"],
  ["skyblue", "#87ceeb"],
  ["slateblue", "#6a5acd"],
  ["slategray", "#708090"],
  ["slategrey", "#708090"],
  ["snow", "#fffafa"],
  ["springgreen", "#00ff7f"],
  ["steelblue", "#4682b4"],
  ["tan", "#d2b48c"],
  ["teal", "#008080"],
  ["thistle", "#d8bfd8"],
  ["tomato", "#ff6347"],
  ["turquoise", "#40e0d0"],
  ["violet", "#ee82ee"],
  ["wheat", "#f5deb3"],
  ["white", "#ffffff"],
  ["whitesmoke", "#f5f5f5"],
  ["yellow", "#ffff00"],
  ["yellowgreen", "#9acd32"],
];

function ok(src: string): Scene {
  const [scene, diags] = compile(src);
  expect(diags, JSON.stringify(diags.map((d) => [d.code, d.message]))).toEqual([]);
  return scene;
}

function errors(src: string): Diagnostic[] {
  return compile(src)[1];
}

describe("colors", () => {
  test("every_css_name_maps_to_its_hex", () => {
    expect(CSS.length).toBe(148);
    for (const [name, hex] of CSS) {
      const s = ok(`${HEAD}box a at (0,0) size (10,10) fill=${name}\n`);
      expect(s.shapes[0]!.style.fill, name).toBe(hex);
    }
  });

  test("names_are_case_insensitive", () => {
    const s = ok(`${HEAD}box a at (0,0) size (10,10) fill=SteelBlue stroke=REBECCAPURPLE\n`);
    expect(s.shapes[0]!.style.fill).toBe("#4682b4");
    expect(s.shapes[0]!.style.stroke).toBe("#663399");
  });

  test("names_work_in_styles_attributes_text_and_background", () => {
    const s = ok(
      'diagram "t" size (800,600) background=ivory\nstyle k { fill=tomato color=white }\nbox a at (0,0) size (10,10) style=k\nbox b at (20,0) size (10,10) stroke=navy\ntext "x" at (50,50) color=darkgreen\n',
    );
    expect(s.background).toBe("#fffff0");
    expect(s.shapes[0]!.style.fill).toBe("#ff6347");
    expect(s.shapes[0]!.style.color).toBe("#ffffff");
    expect(s.shapes[1]!.style.stroke).toBe("#000080");
    expect(s.texts[0]!.color).toBe("#006400");
  });

  test("none_still_works_and_transparent_is_not_a_name", () => {
    const s = ok(`${HEAD}box a at (0,0) size (10,10) fill=none\n`);
    expect(s.shapes[0]!.style.fill).toBe("none");
    const d = errors(`${HEAD}box a at (0,0) size (10,10) fill=transparent\n`);
    expect(d.length, JSON.stringify(d)).toBe(1);
    expect(d[0]!.code).toBe("attr");
  });

  test("unknown_name_suggests_the_closest", () => {
    let d = errors(`${HEAD}box a at (0,0) size (10,10) fill=navyblue\n`);
    expect(d.length, JSON.stringify(d)).toBe(1);
    expect(d[0]!.code).toBe("attr");
    expect(
      d[0]!.help.some((h) => h === "did you mean `navy`?"),
      JSON.stringify(d),
    ).toBe(true);
    d = errors('diagram "t" size (10,10) background=whit\n');
    expect(d[0]!.code).toBe("attr");
    expect(
      d[0]!.help.some((h) => h === "did you mean `white`?"),
      JSON.stringify(d),
    ).toBe(true);
  });
});
