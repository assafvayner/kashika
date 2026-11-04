import { displayF64, round } from "../diag/rust.js";
import { type PlacedLine, colorLayers, outline } from "../text/index.js";
import { esc } from "./svg.js";

function hex2(v: number): string {
  return v.toString(16).padStart(2, "0");
}

/// Draws laid-out lines as glyph outlines, one `<path>` per line. `aria-label` keeps
/// the plain text searchable. A line with emoji is a `<g>` carrying the label, holding
/// the text path and then one path per emoji color layer.
export function lines(placed: readonly PlacedLine[], size: number, color: string): string {
  let out = "";
  for (const l of placed) {
    const d = outline(l.runs, size, l.x, l.baseline);
    const layers = colorLayers(l.runs, size, l.x, l.baseline);
    const plain = l.runs.map((r) => r.text).join("");
    if (layers.length === 0) {
      if (d !== "") {
        out += `<path aria-label="${esc(plain)}" d="${d}" fill="${color}"/>\n`;
      }
      continue;
    }
    out += `<g aria-label="${esc(plain)}">\n`;
    if (d !== "") {
      out += `<path d="${d}" fill="${color}"/>\n`;
    }
    for (const layer of layers) {
      let fill: string;
      if (layer.color === null) {
        fill = `fill="${color}"`;
      } else {
        const [r, g, b, a] = layer.color;
        fill =
          a === 255
            ? `fill="#${hex2(r)}${hex2(g)}${hex2(b)}"`
            : `fill="#${hex2(r)}${hex2(g)}${hex2(b)}" fill-opacity="${displayF64(round((a / 255.0) * 1000.0) / 1000.0)}"`;
      }
      out += `<path d="${layer.d}" ${fill}/>\n`;
    }
    out += "</g>\n";
  }
  return out;
}
