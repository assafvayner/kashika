use std::fmt::Write;
use std::sync::OnceLock;

use skrifa::color::{Brush, ColorPainter, CompositeMode, Transform};
use skrifa::instance::{LocationRef, Size};
use skrifa::metrics::BoundingBox;
use skrifa::outline::{DrawSettings, OutlinePen};
use skrifa::{FontRef, GlyphId, MetadataProvider};

use crate::markup::{Font, Run};

/// Line height as a multiple of the font size.
pub const LINE_HEIGHT: f64 = 1.25;

static INTER_REGULAR: &[u8] = include_bytes!("../fonts/Inter-Regular.ttf");
static INTER_BOLD: &[u8] = include_bytes!("../fonts/Inter-Bold.ttf");
static JETBRAINS_MONO: &[u8] = include_bytes!("../fonts/JetBrainsMono-Regular.ttf");
static TWEMOJI: &[u8] = include_bytes!("../fonts/TwemojiMozilla.ttf");

/// The bytes of every bundled font file: Inter regular and bold, JetBrains Mono, then
/// Twemoji Mozilla.
pub fn font_files() -> [&'static [u8]; 4] {
    [INTER_REGULAR, INTER_BOLD, JETBRAINS_MONO, TWEMOJI]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FaceId {
    Regular,
    Bold,
    Mono,
    Emoji,
}

/// A bundled font, parsed once, with the metrics every measurement needs.
struct Face {
    font: FontRef<'static>,
    units_per_em: f64,
}

fn face(id: FaceId) -> &'static Face {
    static FACES: OnceLock<[Face; 4]> = OnceLock::new();
    let faces = FACES.get_or_init(|| {
        font_files().map(|data| {
            let font = FontRef::new(data).expect("bundled font parses");
            let units_per_em = f64::from(
                font.metrics(Size::unscaled(), LocationRef::default())
                    .units_per_em,
            );
            Face { font, units_per_em }
        })
    });
    &faces[id as usize]
}

fn emoji_shaper() -> &'static rustybuzz::Face<'static> {
    static FACE: OnceLock<rustybuzz::Face<'static>> = OnceLock::new();
    FACE.get_or_init(|| rustybuzz::Face::from_slice(TWEMOJI, 0).expect("bundled font parses"))
}

/// The faces a run falls back through, in order: its own, the other text face, then emoji.
fn chain(font: Font) -> [FaceId; 3] {
    match font {
        Font::Regular => [FaceId::Regular, FaceId::Mono, FaceId::Emoji],
        Font::Bold => [FaceId::Bold, FaceId::Mono, FaceId::Emoji],
        Font::Mono => [FaceId::Mono, FaceId::Regular, FaceId::Emoji],
    }
}

fn has(id: FaceId, c: char) -> bool {
    face(id).font.charmap().map(c).is_some()
}

const ZWJ: char = '\u{200D}';
const TEXT_STYLE: char = '\u{FE0E}';
const KEYCAP: char = '\u{20E3}';

fn is_regional_indicator(c: char) -> bool {
    ('\u{1F1E6}'..='\u{1F1FF}').contains(&c)
}

/// Characters that extend an emoji: presentation selectors, the keycap, skin tones and tags.
fn extends_emoji(c: char) -> bool {
    matches!(c, '\u{FE0E}' | '\u{FE0F}' | KEYCAP | '\u{1F3FB}'..='\u{1F3FF}' | '\u{E0020}'..='\u{E007F}')
}

/// The face for the cluster starting at `chars[i]`, and the index after it. A lone
/// default-ignorable character has no face of its own: it joins the segment before it.
/// A cluster is a single character, or an emoji with its extenders and ZWJ-joined emoji,
/// or a pair of regional indicators. Such a sequence takes the emoji face unless it asks
/// for text presentation; an ASCII digit, `#` or `*` does so only with a keycap.
fn cluster(chars: &[char], i: usize, chain: [FaceId; 3]) -> (Option<FaceId>, usize) {
    let c = chars[i];
    if is_default_ignorable(c) {
        return (None, i + 1);
    }
    let mut end = i + 1;
    if has(FaceId::Emoji, c) {
        if is_regional_indicator(c) && chars.get(end).copied().is_some_and(is_regional_indicator) {
            end += 1;
        }
        loop {
            match chars.get(end).copied() {
                Some(m) if extends_emoji(m) => end += 1,
                Some(ZWJ)
                    if chars.get(end + 1).is_some_and(|&n| {
                        !n.is_ascii() && !is_default_ignorable(n) && has(FaceId::Emoji, n)
                    }) =>
                {
                    end += 2;
                }
                _ => break,
            }
        }
    }
    let seq = &chars[i + 1..end];
    let emoji = if c.is_ascii() {
        seq.contains(&KEYCAP)
    } else {
        !seq.is_empty() && !seq.contains(&TEXT_STYLE)
    };
    if emoji {
        return (Some(FaceId::Emoji), end);
    }
    let id = chain.into_iter().find(|&f| has(f, c)).unwrap_or(chain[0]);
    (Some(id), i + 1)
}

/// Splits a run into maximal pieces that one face draws, chosen per cluster from the
/// run's fallback chain.
fn segments(run: &Run) -> Vec<(FaceId, &str)> {
    let chain = chain(run.font);
    let chars: Vec<char> = run.text.chars().collect();
    let offsets: Vec<usize> = run
        .text
        .char_indices()
        .map(|(b, _)| b)
        .chain([run.text.len()])
        .collect();
    let mut out: Vec<(FaceId, usize, usize)> = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let (id, end) = cluster(&chars, i, chain);
        let (from, to) = (offsets[i], offsets[end]);
        match (out.last_mut(), id) {
            (Some(last), None) => last.2 = to,
            (Some(last), Some(id)) if last.0 == id => last.2 = to,
            (_, id) => out.push((id.unwrap_or(chain[0]), from, to)),
        }
        i = end;
    }
    out.into_iter()
        .map(|(id, from, to)| (id, &run.text[from..to]))
        .collect()
}

/// A glyph to draw, in font units: its advance and its offset from the pen.
struct Glyph {
    id: GlyphId,
    advance: f64,
    dx: f64,
    dy: f64,
}

/// The glyphs of one segment. Text faces map each character through the cmap, with no
/// kerning or shaping, and skip default-ignorable characters. The emoji face is shaped
/// with its GSUB, which forms the ligatures of emoji sequences.
fn glyphs(id: FaceId, text: &str) -> Vec<Glyph> {
    if id == FaceId::Emoji {
        let mut buf = rustybuzz::UnicodeBuffer::new();
        buf.push_str(text);
        buf.set_direction(rustybuzz::Direction::LeftToRight);
        let out = rustybuzz::shape(emoji_shaper(), &[], buf);
        return out
            .glyph_infos()
            .iter()
            .zip(out.glyph_positions())
            .map(|(info, pos)| Glyph {
                id: GlyphId::new(info.glyph_id),
                advance: f64::from(pos.x_advance),
                dx: f64::from(pos.x_offset),
                dy: f64::from(pos.y_offset),
            })
            .collect();
    }
    let f = &face(id).font;
    text.chars()
        .filter(|&c| !is_default_ignorable(c))
        .map(|c| {
            let id = glyph(f, c);
            Glyph {
                id,
                advance: f64::from(advance(f, id)),
                dx: 0.0,
                dy: 0.0,
            }
        })
        .collect()
}

/// Unicode's `Default_Ignorable_Code_Point` ranges (DerivedCoreProperties.txt): joiners,
/// variation selectors, invisible format and bidi controls, the soft hyphen and fillers.
const DEFAULT_IGNORABLE: &[(char, char)] = &[
    ('\u{00AD}', '\u{00AD}'),
    ('\u{034F}', '\u{034F}'),
    ('\u{061C}', '\u{061C}'),
    ('\u{115F}', '\u{1160}'),
    ('\u{17B4}', '\u{17B5}'),
    ('\u{180B}', '\u{180F}'),
    ('\u{200B}', '\u{200F}'),
    ('\u{202A}', '\u{202E}'),
    ('\u{2060}', '\u{206F}'),
    ('\u{3164}', '\u{3164}'),
    ('\u{FE00}', '\u{FE0F}'),
    ('\u{FEFF}', '\u{FEFF}'),
    ('\u{FFA0}', '\u{FFA0}'),
    ('\u{FFF0}', '\u{FFF8}'),
    ('\u{1BCA0}', '\u{1BCA3}'),
    ('\u{1D173}', '\u{1D17A}'),
    ('\u{E0000}', '\u{E0FFF}'),
];

/// Whether `c` is a Unicode default-ignorable code point (ZWJ, ZWNJ, a variation selector,
/// a zero-width space, the soft hyphen, ...). These measure and draw as nothing.
pub fn is_default_ignorable(c: char) -> bool {
    DEFAULT_IGNORABLE
        .iter()
        .any(|&(lo, hi)| (lo..=hi).contains(&c))
}

/// Scripts and symbol sets with a sample character, in the order `covered_scripts` names
/// them.
const SCRIPTS: &[(&str, char)] = &[
    ("Latin", 'A'),
    ("Greek", 'Ω'),
    ("Cyrillic", 'Ж'),
    ("Armenian", 'Ա'),
    ("Georgian", 'ა'),
    ("Hebrew", 'א'),
    ("Arabic", 'ب'),
    ("Devanagari", 'क'),
    ("Thai", 'ก'),
    ("Han", '日'),
    ("Kana", 'あ'),
    ("Hangul", '한'),
    ("box drawing", '┌'),
    ("emoji", '🚀'),
];

/// The scripts and symbol sets whose sample character every font's fallback chain can
/// draw, such as `["Latin", "Greek", "emoji"]`.
pub fn covered_scripts() -> Vec<&'static str> {
    SCRIPTS
        .iter()
        .filter(|&&(_, c)| {
            [Font::Regular, Font::Bold, Font::Mono]
                .iter()
                .all(|&f| chain(f).into_iter().any(|id| has(id, c)))
        })
        .map(|&(name, _)| name)
        .collect()
}

fn glyph(f: &FontRef, c: char) -> GlyphId {
    f.charmap().map(c).unwrap_or(GlyphId::NOTDEF)
}

fn advance(f: &FontRef, g: GlyphId) -> f32 {
    f.glyph_metrics(Size::unscaled(), LocationRef::default())
        .advance_width(g)
        .unwrap_or(0.0)
}

/// The distinct characters, in order of first appearance, that no font in the run's
/// fallback chain has a glyph for. Drawing and measuring use the run's own font's
/// `.notdef` glyph for them. Default-ignorable characters are never missing.
pub fn missing_glyphs<'a>(runs: impl IntoIterator<Item = &'a Run>) -> Vec<char> {
    let mut out = Vec::new();
    for run in runs {
        for (id, text) in segments(run) {
            for c in text.chars() {
                if !has(id, c) && !is_default_ignorable(c) && !out.contains(&c) {
                    out.push(c);
                }
            }
        }
    }
    out
}

/// Whether the runs draw something and all of it is emoji, which keep their own colors.
pub fn only_emoji<'a>(runs: impl IntoIterator<Item = &'a Run>) -> bool {
    let mut emoji = false;
    for run in runs {
        for (id, text) in segments(run) {
            if id == FaceId::Emoji {
                emoji = true;
            } else if text
                .chars()
                .any(|c| !c.is_whitespace() && !is_default_ignorable(c))
            {
                return false;
            }
        }
    }
    emoji
}

/// Advance width of a run in pixels: the sum of glyph advances, without kerning. Only
/// emoji are shaped.
pub(crate) fn run_width(run: &Run, size: f64) -> f64 {
    segments(run)
        .into_iter()
        .map(|(id, text)| {
            let units: f64 = glyphs(id, text).iter().map(|g| g.advance).sum();
            units * size / face(id).units_per_em
        })
        .sum()
}

/// Distance from the top of a line box to its baseline. Inter's ascender-to-descender
/// range is centered in the line box, and every font shares that baseline.
pub(crate) fn baseline_offset(size: f64) -> f64 {
    let m = face(FaceId::Regular)
        .font
        .metrics(Size::unscaled(), LocationRef::default());
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

fn draw(out: &mut String, id: FaceId, g: GlyphId, x: f64, y: f64, scale: f64) {
    let mut b = PathOut {
        out,
        x,
        y,
        scale,
        start: (0.0, 0.0),
        last: (0.0, 0.0),
    };
    if let Some(outline) = face(id).font.outline_glyphs().get(g) {
        let _ = outline.draw(
            DrawSettings::unhinted(Size::unscaled(), LocationRef::default()),
            &mut b,
        );
    }
}

/// Calls `f` with each glyph of a line, its face, pen position and scale.
fn walk(
    runs: &[Run],
    size: f64,
    x: f64,
    baseline: f64,
    mut f: impl FnMut(FaceId, &Glyph, f64, f64, f64),
) {
    let mut pen = x;
    for run in runs {
        for (id, text) in segments(run) {
            let scale = size / face(id).units_per_em;
            for g in glyphs(id, text) {
                f(id, &g, pen + g.dx * scale, baseline - g.dy * scale, scale);
                pen += g.advance * scale;
            }
        }
    }
}

/// SVG path data for the text-colored glyphs of a line of runs whose pen starts at
/// `(x, baseline)`. Emoji are left out; `color_layers` draws them.
pub fn outline(runs: &[Run], size: f64, x: f64, baseline: f64) -> String {
    let mut out = String::new();
    walk(runs, size, x, baseline, |id, g, x, y, scale| {
        if id != FaceId::Emoji {
            draw(&mut out, id, g.id, x, y, scale);
        }
    });
    out
}

/// One filled path of a color glyph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layer {
    pub d: String,
    /// The layer's RGBA color, or `None` to use the text color.
    pub color: Option<[u8; 4]>,
}

/// The emoji of a line, drawn like `outline`, as their color layers in paint order.
pub fn color_layers(runs: &[Run], size: f64, x: f64, baseline: f64) -> Vec<Layer> {
    let mut out = Vec::new();
    walk(runs, size, x, baseline, |id, g, x, y, scale| {
        if id != FaceId::Emoji {
            return;
        }
        let font = &face(id).font;
        let fills = match font.color_glyphs().get(g.id) {
            Some(color) => {
                let mut p = Fills(Vec::new());
                let _ = color.paint(LocationRef::default(), &mut p);
                p.0
            }
            None => vec![(g.id, None)],
        };
        for (layer, color) in fills {
            let mut d = String::new();
            draw(&mut d, id, layer, x, y, scale);
            if !d.is_empty() {
                out.push(Layer { d, color });
            }
        }
    });
    out
}

/// Collects the solid glyph fills of a COLR glyph, which is all a COLRv0 glyph paints.
struct Fills(Vec<(GlyphId, Option<[u8; 4]>)>);

impl ColorPainter for Fills {
    fn push_transform(&mut self, _: Transform) {}
    fn pop_transform(&mut self) {}
    fn push_clip_glyph(&mut self, _: GlyphId) {}
    fn push_clip_box(&mut self, _: BoundingBox) {}
    fn pop_clip(&mut self) {}
    fn fill(&mut self, _: Brush<'_>) {}
    fn push_layer(&mut self, _: CompositeMode) {}

    fn fill_glyph(&mut self, glyph: GlyphId, _: Option<Transform>, brush: Brush<'_>) {
        let Brush::Solid {
            palette_index,
            alpha,
        } = brush
        else {
            return;
        };
        let color = if palette_index == 0xFFFF {
            None
        } else {
            let palettes = face(FaceId::Emoji).font.color_palettes();
            let Some(c) = palettes
                .get(0)
                .and_then(|p| p.colors().get(usize::from(palette_index)).copied())
            else {
                return;
            };
            let a = (f32::from(c.alpha) * alpha).round().clamp(0.0, 255.0) as u8;
            Some([c.red, c.green, c.blue, a])
        };
        self.0.push((glyph, color));
    }
}
