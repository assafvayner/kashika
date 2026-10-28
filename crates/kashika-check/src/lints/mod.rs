mod canvas;
mod collision;
mod contrast;
mod crowding;
mod grid;

use kashika_diag::Diagnostic;

use crate::ctx::Ctx;

mod cross;
mod diagonal;
mod endpoint;
mod near;
mod overlap;
mod shared;
mod spill;
mod through;
mod transit;

pub type Lint = fn(&Ctx) -> Vec<Diagnostic>;

/// Every warning lint. Each lint task adds its `mod` line above and an entry here.
pub const ALL: &[Lint] = &[
    canvas::out_of_canvas,
    collision::label_collision,
    contrast::low_contrast,
    cross::cross,
    crowding::port_crowding,
    diagonal::diagonal_tail,
    endpoint::endpoint_off_edge,
    grid::off_grid,
    near::near,
    overlap::overlap,
    shared::shared_segment,
    spill::spill,
    through::through,
    transit::group_transit,
];
