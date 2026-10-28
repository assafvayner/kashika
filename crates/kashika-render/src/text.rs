use kashika_text::PlacedLine;

use crate::svg::esc;

/// Draws laid-out lines as glyph outlines, one `<path>` per line. `aria-label` keeps
/// the plain text searchable.
pub(crate) fn lines(out: &mut String, lines: &[PlacedLine], size: f64, color: &str) {
    for l in lines {
        let d = kashika_text::outline(&l.runs, size, l.x, l.baseline);
        if d.is_empty() {
            continue;
        }
        let plain: String = l.runs.iter().map(|r| r.text.as_str()).collect();
        out.push_str(&format!(
            "<path aria-label=\"{}\" d=\"{d}\" fill=\"{color}\"/>\n",
            esc(&plain)
        ));
    }
}
