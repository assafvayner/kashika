use std::collections::HashMap;

use kashika_diag::{Diagnostic, Element, code};
use kashika_model::{Allow, Scene};

use crate::element::{line_el, shape_el};

/// Marks findings silenced by `allow=` on the diagram, or on a line, box or group that the
/// finding names. `allow=code:other` also requires `other` among the finding's elements. The
/// reason is that of the first such line, box or group (lines first, in file order) that
/// gives one, also when the diagram silences the code too.
pub fn suppress(scene: &Scene, diags: &mut [Diagnostic]) {
    let mut s = Suppressor::new(scene);
    for d in diags {
        s.apply(d);
    }
}

/// Whether the `allow=` entry `code` (with `other` for `code:other`) on the element `owner`
/// suppresses `d`: the codes match, `d` names `owner`, and also `other` when given.
pub fn suppresses(code: &str, other: Option<&str>, owner: &str, d: &Diagnostic) -> bool {
    code == d.code && names(d, owner) && other.is_none_or(|o| o != owner && names(d, o))
}

fn names(d: &Diagnostic, id: &str) -> bool {
    d.elements.iter().any(|e| e.id == id)
}

/// A line, box or group with an `allow=`.
pub(crate) struct Owner<'a> {
    pub id: &'a str,
    pub allow: &'a [Allow],
    pub reason: Option<&'a str>,
    pub el: Element,
}

pub(crate) struct Suppressor<'a> {
    diagram: &'a [Allow],
    /// Lines, then shapes, with an `allow=`.
    owners: Vec<Owner<'a>>,
    /// Index into `owners` of the first owner with each id.
    by_id: HashMap<&'a str, usize>,
    /// Per owner, whether each `allow=` entry has suppressed a finding.
    used: Vec<Vec<bool>>,
}

impl<'a> Suppressor<'a> {
    pub fn new(scene: &'a Scene) -> Self {
        let owners: Vec<_> = scene
            .lines
            .iter()
            .filter(|l| !l.allow.is_empty())
            .map(|l| Owner {
                id: &l.id,
                allow: &l.allow,
                reason: l.reason.as_deref(),
                el: line_el(l, None),
            })
            .chain(
                scene
                    .shapes
                    .iter()
                    .filter(|s| !s.allow.is_empty())
                    .map(|s| Owner {
                        id: &s.id,
                        allow: &s.allow,
                        reason: s.reason.as_deref(),
                        el: shape_el(s),
                    }),
            )
            .collect();
        let mut by_id = HashMap::new();
        for (i, o) in owners.iter().enumerate() {
            by_id.entry(o.id).or_insert(i);
        }
        let used = owners.iter().map(|o| vec![false; o.allow.len()]).collect();
        Self {
            diagram: &scene.allow,
            owners,
            by_id,
            used,
        }
    }

    pub fn apply(&mut self, d: &mut Diagnostic) {
        let by_diagram = self.diagram.iter().any(|a| a.code == d.code);
        if self.owners.is_empty() {
            d.suppressed |= by_diagram;
            return;
        }
        let mut by: Vec<usize> = Vec::new();
        for e in &d.elements {
            let Some(&i) = self.by_id.get(e.id.as_str()) else {
                continue;
            };
            let o = &self.owners[i];
            for (j, a) in o.allow.iter().enumerate() {
                if suppresses(&a.code, a.other.as_deref(), o.id, d) {
                    self.used[i][j] = true;
                    by.push(i);
                }
            }
        }
        by.sort_unstable();
        d.suppressed |= by_diagram || !by.is_empty();
        if let Some(r) = by.iter().find_map(|&i| self.owners[i].reason) {
            d.reason = Some(r.to_string());
        }
    }

    /// The `allow=` entries that have suppressed nothing so far, with their owner and whether
    /// none of that owner's entries has. `allow=unused-allow` is never listed.
    pub fn unused(&self) -> impl Iterator<Item = (&Owner<'a>, &'a Allow, bool)> + '_ {
        self.owners
            .iter()
            .enumerate()
            .filter(|(i, o)| self.by_id.get(o.id) == Some(i))
            .flat_map(move |(i, o)| {
                let used = &self.used[i];
                let none = !used.contains(&true);
                o.allow
                    .iter()
                    .zip(used)
                    .filter(|(a, u)| !**u && a.code != code::UNUSED_ALLOW)
                    .map(move |(a, _)| (o, a, none))
            })
    }
}
