use kashika_diag::Diagnostic;
use kashika_geom::fmt_num;

use crate::ctx::Ctx;
use crate::sink::Sink;

/// `off-grid`: an authored literal is not a multiple of `grid`. Resolved anchors are never checked.
pub fn off_grid(ctx: &Ctx, out: &mut Sink) {
    let Some(g) = ctx.scene.grid.filter(|g| *g > 0.0) else {
        return;
    };
    ctx.scene
        .literals
        .iter()
        .filter(|l| {
            let r = l.value / g;
            (r - r.round()).abs() * g > 1e-6
        })
        .map(|l| {
            let lo = (l.value / g).floor() * g;
            let mut d = Diagnostic::warn(
                "off-grid",
                l.span,
                format!(
                    "{}={} on '{}' is not a multiple of grid {}",
                    l.what,
                    fmt_num(l.value),
                    l.owner,
                    fmt_num(g)
                ),
            )
            .with_fact("value", l.value)
            .with_fact("grid", g)
            .with_help(format!("use {} or {}", fmt_num(lo), fmt_num(lo + g)));
            if let Some(e) = ctx.element(&l.owner) {
                d = d.with_element(e);
            }
            d
        })
        .for_each(|d| out.push(d));
}
