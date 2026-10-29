use kashika_text::PlacedLine;

use crate::svg::esc;

/// Draws laid-out lines as glyph outlines, one `<path>` per line. `aria-label` keeps
/// the plain text searchable. A line with emoji is a `<g>` carrying the label, holding
/// the text path and then one path per emoji color layer.
pub(crate) fn lines(out: &mut String, lines: &[PlacedLine], size: f64, color: &str) {
    for l in lines {
        let d = kashika_text::outline(&l.runs, size, l.x, l.baseline);
        let layers = kashika_text::color_layers(&l.runs, size, l.x, l.baseline);
        let plain: String = l.runs.iter().map(|r| r.text.as_str()).collect();
        if layers.is_empty() {
            if !d.is_empty() {
                out.push_str(&format!(
                    "<path aria-label=\"{}\" d=\"{d}\" fill=\"{color}\"/>\n",
                    esc(&plain)
                ));
            }
            continue;
        }
        out.push_str(&format!("<g aria-label=\"{}\">\n", esc(&plain)));
        if !d.is_empty() {
            out.push_str(&format!("<path d=\"{d}\" fill=\"{color}\"/>\n"));
        }
        for layer in layers {
            let fill = match layer.color {
                None => format!("fill=\"{color}\""),
                Some([r, g, b, 255]) => format!("fill=\"#{r:02x}{g:02x}{b:02x}\""),
                Some([r, g, b, a]) => format!(
                    "fill=\"#{r:02x}{g:02x}{b:02x}\" fill-opacity=\"{}\"",
                    (f64::from(a) / 255.0 * 1000.0).round() / 1000.0
                ),
            };
            out.push_str(&format!("<path d=\"{}\" {fill}/>\n", layer.d));
        }
        out.push_str("</g>\n");
    }
}
