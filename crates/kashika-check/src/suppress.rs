use kashika_diag::Diagnostic;
use kashika_model::{Allow, Scene};

/// Marks findings silenced by `allow=` on the diagram, or on a line, box or group that the
/// finding names. `allow=code:other` also requires `other` among the finding's elements.
pub fn suppress(scene: &Scene, diags: &mut [Diagnostic]) {
    let owners: Vec<(&str, &[Allow], &Option<String>)> = scene
        .lines
        .iter()
        .map(|l| (l.id.as_str(), l.allow.as_slice(), &l.reason))
        .chain(
            scene
                .shapes
                .iter()
                .map(|s| (s.id.as_str(), s.allow.as_slice(), &s.reason)),
        )
        .collect();
    for d in diags.iter_mut() {
        if scene.allow.iter().any(|a| a.code == d.code) {
            d.suppressed = true;
            continue;
        }
        let ids: Vec<&str> = d.elements.iter().map(|e| e.id.as_str()).collect();
        let by = owners.iter().find(|(id, allow, _)| {
            ids.contains(id)
                && allow.iter().any(|a| {
                    a.code == d.code
                        && a.other
                            .as_deref()
                            .is_none_or(|o| o != *id && ids.contains(&o))
                })
        });
        if let Some((_, _, reason)) = by {
            d.suppressed = true;
            d.reason = Option::clone(reason);
        }
    }
}
