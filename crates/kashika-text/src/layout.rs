use kashika_geom::Rect;
use serde::Serialize;

use crate::font::{LINE_HEIGHT, baseline_offset, run_width};
use crate::markup::{MarkupError, Run, parse_markup};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HAlign {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VAlign {
    Top,
    Middle,
    Bottom,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeasuredLine {
    pub runs: Vec<Run>,
    pub width: f64,
}

/// One measured paragraph: a string split on `\n`, with markup applied, at one size.
#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub lines: Vec<MeasuredLine>,
    pub size: f64,
    pub width: f64,
    pub height: f64,
}

/// A line positioned on the canvas. The pen starts at `(x, baseline)`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PlacedLine {
    pub runs: Vec<Run>,
    pub x: f64,
    pub baseline: f64,
    pub width: f64,
}

pub fn measure(text: &str, size: f64, bold: bool) -> Result<Block, MarkupError> {
    let lines: Vec<MeasuredLine> = parse_markup(text, bold)?
        .into_iter()
        .map(|runs| {
            let width = runs.iter().map(|r| run_width(r, size)).sum();
            MeasuredLine { runs, width }
        })
        .collect();
    let width = lines.iter().map(|l| l.width).fold(0.0, f64::max);
    let height = lines.len() as f64 * size * LINE_HEIGHT;
    Ok(Block {
        lines,
        size,
        width,
        height,
    })
}

impl Block {
    /// Places the lines top-down from `top`, each aligned within `[left, left + width]`.
    /// Returns the lines and their bounding box.
    pub fn place(&self, left: f64, width: f64, top: f64, align: HAlign) -> (Vec<PlacedLine>, Rect) {
        let lh = self.size * LINE_HEIGHT;
        let base = baseline_offset(self.size);
        let lines: Vec<PlacedLine> = self
            .lines
            .iter()
            .enumerate()
            .map(|(i, l)| PlacedLine {
                runs: l.runs.clone(),
                x: match align {
                    HAlign::Left => left,
                    HAlign::Center => left + (width - l.width) / 2.0,
                    HAlign::Right => left + width - l.width,
                },
                baseline: top + lh * i as f64 + base,
                width: l.width,
            })
            .collect();
        let x0 = lines.iter().map(|l| l.x).fold(f64::INFINITY, f64::min);
        let x1 = lines
            .iter()
            .map(|l| l.x + l.width)
            .fold(f64::NEG_INFINITY, f64::max);
        (lines, Rect::new(x0, top, x1 - x0, self.height))
    }

    /// Places the block with `at` as its horizontal alignment point and vertical center.
    pub fn place_at(&self, x: f64, y: f64, align: HAlign) -> (Vec<PlacedLine>, Rect) {
        let left = match align {
            HAlign::Left => x,
            HAlign::Center => x - self.width / 2.0,
            HAlign::Right => x - self.width,
        };
        self.place(left, self.width, y - self.height / 2.0, align)
    }
}

/// Width and height of blocks stacked top to bottom with `gap` between them.
pub fn stack_size(blocks: &[&Block], gap: f64) -> (f64, f64) {
    let w = blocks.iter().map(|b| b.width).fold(0.0, f64::max);
    let h: f64 =
        blocks.iter().map(|b| b.height).sum::<f64>() + gap * blocks.len().saturating_sub(1) as f64;
    (w, h)
}

/// Stacks blocks inside `area`. Each block is aligned horizontally by its own `HAlign`;
/// the stack as a whole is aligned vertically by `valign`.
pub fn stack(
    items: &[(&Block, HAlign)],
    area: Rect,
    valign: VAlign,
    gap: f64,
) -> Vec<(Vec<PlacedLine>, Rect)> {
    let blocks: Vec<&Block> = items.iter().map(|(b, _)| *b).collect();
    let (_, h) = stack_size(&blocks, gap);
    let mut top = match valign {
        VAlign::Top => area.y,
        VAlign::Middle => area.y + (area.h - h) / 2.0,
        VAlign::Bottom => area.bottom() - h,
    };
    let mut out = Vec::new();
    for (b, align) in items {
        out.push(b.place(area.x, area.w, top, *align));
        top += b.height + gap;
    }
    out
}
