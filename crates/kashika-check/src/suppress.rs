use std::collections::HashMap;

use kashika_diag::Diagnostic;
use kashika_model::{Allow, Scene};

/// Marks findings silenced by `allow=` on the diagram, or on a line, box or group that the
/// finding names. `allow=code:other` also requires `other` among the finding's elements. The
/// reason is that of the first such line, box or group (lines first, in file order) that
/// gives one, also when the diagram silences the code too.
pub fn suppress(scene: &Scene, diags: &mut [Diagnostic]) {
    let s = Suppressor::new(scene);
    for d in diags {
        s.apply(d);
    }
}

pub(crate) struct Suppressor<'a> {
    diagram: &'a [Allow],
    /// Lines, then shapes, with an `allow=`: (id, allow, reason).
    owners: Vec<(&'a str, &'a [Allow], Option<&'a str>)>,
    /// Index into `owners` of the first owner with each id.
    by_id: HashMap<&'a str, usize>,
}

impl<'a> Suppressor<'a> {
    pub fn new(scene: &'a Scene) -> Self {
        let owners: Vec<_> = scene
            .lines
            .iter()
            .map(|l| (l.id.as_str(), l.allow.as_slice(), l.reason.as_deref()))
            .chain(
                scene
                    .shapes
                    .iter()
                    .map(|s| (s.id.as_str(), s.allow.as_slice(), s.reason.as_deref())),
            )
            .filter(|(_, allow, _)| !allow.is_empty())
            .collect();
        let mut by_id = HashMap::new();
        for (i, (id, _, _)) in owners.iter().enumerate() {
            by_id.entry(*id).or_insert(i);
        }
        Self {
            diagram: &scene.allow,
            owners,
            by_id,
        }
    }

    pub fn apply(&self, d: &mut Diagnostic) {
        let by_diagram = self.diagram.iter().any(|a| a.code == d.code);
        if self.owners.is_empty() {
            d.suppressed |= by_diagram;
            return;
        }
        let ids: Vec<&str> = d.elements.iter().map(|e| e.id.as_str()).collect();
        let mut by: Vec<usize> = ids
            .iter()
            .filter_map(|id| self.by_id.get(id).copied())
            .filter(|&i| {
                let (id, allow, _) = self.owners[i];
                allow.iter().any(|a| {
                    a.code == d.code
                        && a.other
                            .as_deref()
                            .is_none_or(|o| o != id && ids.contains(&o))
                })
            })
            .collect();
        by.sort_unstable();
        d.suppressed |= by_diagram || !by.is_empty();
        if let Some(r) = by.iter().find_map(|&i| self.owners[i].2) {
            d.reason = Some(r.to_string());
        }
    }
}
