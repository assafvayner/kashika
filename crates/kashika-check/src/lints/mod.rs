mod axis;
mod block;
mod canvas;
mod collision;
mod contrast;
mod cross;
mod crowding;
mod diagonal;
mod endpoint;
mod glyph;
mod grid;
mod image;
mod jog;
mod near;
mod overlap;
mod shared;
mod spill;
mod textarea;
mod through;
mod transit;

use crate::ctx::Ctx;
use crate::sink::Sink;

pub type Lint = fn(&Ctx, &mut Sink);

/// Every warning lint.
pub const ALL: &[Lint] = &[
    block::block_reducible,
    canvas::out_of_canvas,
    collision::label_collision,
    contrast::low_contrast,
    cross::cross,
    crowding::port_crowding,
    diagonal::diagonal_tail,
    endpoint::endpoint_off_edge,
    glyph::missing_glyph,
    grid::off_grid,
    image::image_distort,
    jog::jog,
    image::svg_dropped,
    near::near,
    axis::near_axis,
    overlap::overlap,
    shared::shared_segment,
    spill::spill,
    textarea::text_outside_shape,
    through::through,
    transit::group_transit,
];
