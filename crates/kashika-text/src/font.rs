use std::fmt::Write;
use std::sync::OnceLock;

use skrifa::instance::{LocationRef, Size};
use skrifa::outline::{DrawSettings, OutlinePen};
use skrifa::{FontRef, GlyphId, MetadataProvider};

use crate::markup::{Font, Run};

/// Line height as a multiple of the font size.
pub const LINE_HEIGHT: f64 = 1.25;

static INTER_REGULAR: &[u8] = include_bytes!("../fonts/Inter-Regular.ttf");
static INTER_BOLD: &[u8] = include_bytes!("../fonts/Inter-Bold.ttf");
static JETBRAINS_MONO: &[u8] = include_bytes!("../fonts/JetBrainsMono-Regular.ttf");

fn face(font: Font) -> &'static FontRef<'static> {
    static FACES: OnceLock<[FontRef<'static>; 3]> = OnceLock::new();
    let faces = FACES.get_or_init(|| {
        [INTER_REGULAR, INTER_BOLD, JETBRAINS_MONO]
            .map(|data| FontRef::new(data).expect("bundled font parses"))
    });
    match font {
        Font::Regular => &faces[0],
        Font::Bold => &faces[1],
        Font::Mono => &faces[2],
    }
}

fn glyph(f: &FontRef, c: char) -> GlyphId {
    f.charmap().map(c).unwrap_or(GlyphId::NOTDEF)
}

fn advance(f: &FontRef, g: GlyphId) -> f32 {
    f.glyph_metrics(Size::unscaled(), LocationRef::default())
        .advance_width(g)
        .unwrap_or(0.0)
}

fn units_per_em(f: &FontRef) -> f64 {
    f64::from(
        f.metrics(Size::unscaled(), LocationRef::default())
            .units_per_em,
    )
}

/// Advance width of a run in pixels. No kerning or shaping: the sum of glyph advances.
pub(crate) fn run_width(run: &Run, size: f64) -> f64 {
    let f = face(run.font);
    let units: f64 = run
        .text
        .chars()
        .map(|c| f64::from(advance(f, glyph(f, c))))
        .sum();
    units * size / units_per_em(f)
}

/// Distance from the top of a line box to its baseline. Inter's ascender-to-descender
/// range is centered in the line box, and every font shares that baseline.
pub(crate) fn baseline_offset(size: f64) -> f64 {
    let f = face(Font::Regular);
    let m = f.metrics(Size::unscaled(), LocationRef::default());
    let em = f64::from(m.units_per_em);
    let asc = f64::from(m.ascent) / em;
    let desc = f64::from(m.descent) / em;
    ((LINE_HEIGHT - (asc - desc)) / 2.0 + asc) * size
}

fn num(v: f64) -> String {
    let r = (v * 100.0).round() / 100.0;
    if r == 0.0 {
        "0".to_string()
    } else {
        format!("{r}")
    }
}

struct PathOut<'a> {
    out: &'a mut String,
    x: f64,
    y: f64,
    scale: f64,
    start: (f32, f32),
    last: (f32, f32),
}

impl PathOut<'_> {
    fn pt(&self, x: f32, y: f32) -> String {
        format!(
            "{} {}",
            num(self.x + f64::from(x) * self.scale),
            num(self.y - f64::from(y) * self.scale)
        )
    }
}

impl OutlinePen for PathOut<'_> {
    fn move_to(&mut self, x: f32, y: f32) {
        self.start = (x, y);
        self.last = (x, y);
        let p = self.pt(x, y);
        let _ = write!(self.out, "M{p}");
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.last = (x, y);
        let p = self.pt(x, y);
        let _ = write!(self.out, "L{p}");
    }
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.last = (x, y);
        let (a, b) = (self.pt(x1, y1), self.pt(x, y));
        let _ = write!(self.out, "Q{a} {b}");
    }
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.last = (x, y);
        let (a, b, c) = (self.pt(x1, y1), self.pt(x2, y2), self.pt(x, y));
        let _ = write!(self.out, "C{a} {b} {c}");
    }
    fn close(&mut self) {
        if self.last != self.start {
            let p = self.pt(self.start.0, self.start.1);
            let _ = write!(self.out, "L{p}");
        }
        self.out.push('Z');
    }
}

/// SVG path data for a line of runs whose pen starts at `(x, baseline)`.
pub fn outline(runs: &[Run], size: f64, x: f64, baseline: f64) -> String {
    let mut out = String::new();
    let mut pen = x;
    for run in runs {
        let f = face(run.font);
        let scale = size / units_per_em(f);
        let glyphs = f.outline_glyphs();
        for c in run.text.chars() {
            let g = glyph(f, c);
            let mut b = PathOut {
                out: &mut out,
                x: pen,
                y: baseline,
                scale,
                start: (0.0, 0.0),
                last: (0.0, 0.0),
            };
            if let Some(outline) = glyphs.get(g) {
                let _ = outline.draw(
                    DrawSettings::unhinted(Size::unscaled(), LocationRef::default()),
                    &mut b,
                );
            }
            pen += f64::from(advance(f, g)) * scale;
        }
    }
    out
}
