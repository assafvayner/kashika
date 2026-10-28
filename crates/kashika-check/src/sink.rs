use std::collections::BTreeMap;

use kashika_diag::{Diagnostic, order};
use kashika_model::Scene;
use serde::Serialize;

use crate::suppress::Suppressor;

/// What `check_bounded` reports.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Bounded {
    pub diags: Vec<Diagnostic>,
    /// Findings past the limit, per code and suppression, in code order.
    pub omitted: Vec<Omitted>,
    /// Every finding per code, including those filtered out or omitted.
    found: BTreeMap<&'static str, usize>,
}

impl Bounded {
    /// How many findings with `code` the lints made, including those filtered out or omitted.
    pub fn found(&self, code: &str) -> usize {
        self.found.get(code).copied().unwrap_or(0)
    }
}

/// How many findings with one code and suppression were counted but not kept.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Omitted {
    pub code: &'static str,
    pub suppressed: bool,
    pub count: usize,
}

#[derive(Default)]
struct Bucket {
    /// Kept findings with the order they were pushed in.
    kept: Vec<(usize, Diagnostic)>,
    omitted: usize,
}

impl Bucket {
    /// Keeps the first `keep` findings in `kashika_diag::sort` order.
    fn trim(&mut self, keep: usize) {
        if self.kept.len() <= keep {
            return;
        }
        self.kept
            .sort_by(|a, b| order(&a.1, &b.1).then(a.0.cmp(&b.0)));
        self.omitted += self.kept.len() - keep;
        self.kept.truncate(keep);
    }
}

/// Where lints put their findings: marks suppression, drops what `filter` rejects and keeps at
/// most `keep` findings per code and suppression, holding at most twice that at any time.
pub struct Sink<'a> {
    suppressor: Suppressor<'a>,
    filter: Option<&'a dyn Fn(&Diagnostic) -> bool>,
    keep: usize,
    pushed: usize,
    buckets: BTreeMap<(&'static str, bool), Bucket>,
    found: BTreeMap<&'static str, usize>,
}

impl<'a> Sink<'a> {
    pub fn new(
        scene: &'a Scene,
        keep: usize,
        filter: Option<&'a dyn Fn(&Diagnostic) -> bool>,
    ) -> Self {
        Self {
            suppressor: Suppressor::new(scene),
            filter,
            keep,
            pushed: 0,
            buckets: BTreeMap::new(),
            found: BTreeMap::new(),
        }
    }

    pub fn push(&mut self, mut d: Diagnostic) {
        self.suppressor.apply(&mut d);
        *self.found.entry(d.code).or_default() += 1;
        if self.filter.is_some_and(|f| !f(&d)) {
            return;
        }
        let b = self.buckets.entry((d.code, d.suppressed)).or_default();
        if self.keep == 0 {
            b.omitted += 1;
            return;
        }
        b.kept.push((self.pushed, d));
        self.pushed += 1;
        if b.kept.len() >= self.keep.saturating_mul(2) {
            b.trim(self.keep);
        }
    }

    pub fn finish(self) -> Bounded {
        let mut kept = Vec::new();
        let mut omitted = Vec::new();
        for ((code, suppressed), mut b) in self.buckets {
            b.trim(self.keep);
            kept.extend(b.kept);
            if b.omitted > 0 {
                omitted.push(Omitted {
                    code,
                    suppressed,
                    count: b.omitted,
                });
            }
        }
        kept.sort_unstable_by_key(|k| k.0);
        Bounded {
            diags: kept.into_iter().map(|k| k.1).collect(),
            omitted,
            found: self.found,
        }
    }
}
