use kashika_diag::{Diagnostic, LINT_CODES, Span, did_you_mean};
use kashika_syntax::ast::{self};

use crate::attrs::{self, LIMIT};
use crate::scene::*;

use super::Resolver;

impl<'a> Resolver<'a> {
    pub(super) fn allow_list(&mut self, a: &ast::Attr, check_ids: bool) -> Vec<Allow> {
        let ast::Value::Word(w) = &a.value else {
            self.diags.push(attrs::type_err(
                a,
                "lint codes like `cross` or `cross:other_line`",
            ));
            return Vec::new();
        };
        let mut out = Vec::new();
        for item in w.split(',') {
            let (code, other) = match item.split_once(':') {
                Some((c, o)) => (c, Some(o.to_string())),
                None => (item, None),
            };
            if !LINT_CODES.contains(&code) {
                let mut d = Diagnostic::error("attr", a.span, format!("unknown lint `{code}`"));
                if let Some(s) = did_you_mean(code, LINT_CODES.iter().copied()) {
                    d = d.with_help(format!("did you mean `{s}`?"));
                }
                self.diags.push(d);
                continue;
            }
            if let Some(o) = &other {
                if !check_ids {
                    self.diags.push(Diagnostic::error(
                        "attr",
                        a.span,
                        "diagram-level `allow` cannot name another line",
                    ));
                    continue;
                }
                if !self.ids.contains_key(o) {
                    if self.skipped_ids.contains(o) {
                        continue;
                    }
                    let d = self.unknown_id(o, a.span);
                    self.diags.push(d);
                    continue;
                }
            }
            out.push(Allow {
                code: code.to_string(),
                other,
            });
        }
        out
    }

    /// Checks that every value is a finite number within `LIMIT` of zero.
    pub(super) fn finite(&mut self, vals: &[f64], span: Span) -> bool {
        if vals.iter().any(|v| !v.is_finite()) {
            self.diags.push(Diagnostic::error(
                "range",
                span,
                "value is not a finite number",
            ));
            false
        } else if vals.iter().any(|v| v.abs() > LIMIT) {
            self.diags.push(
                Diagnostic::error(
                    "range",
                    span,
                    format!("value is out of range (more than {LIMIT} from the origin)"),
                )
                .with_help(format!("keep coordinates within -{LIMIT}..{LIMIT}")),
            );
            false
        } else {
            true
        }
    }

    pub(super) fn size_ok(&mut self, (w, h): (f64, f64), span: Span, what: &str) -> bool {
        if !self.finite_only(&[w, h], span) {
            return false;
        }
        let msg = if w <= 0.0 || h <= 0.0 {
            format!("{what} must have a positive size")
        } else if w > LIMIT || h > LIMIT {
            format!("{what} is larger than {LIMIT}")
        } else {
            return true;
        };
        self.diags.push(Diagnostic::error("size", span, msg));
        false
    }

    /// Like `finite`, but only rejects non-finite values.
    pub(super) fn finite_only(&mut self, vals: &[f64], span: Span) -> bool {
        vals.iter().all(|v| v.is_finite()) || self.finite(vals, span)
    }

    pub(super) fn unknown_id(&self, name: &str, span: Span) -> Diagnostic {
        let mut d = Diagnostic::error("unknown-id", span, format!("unknown id `{name}`"));
        if let Some(s) = did_you_mean(name, self.ids.keys().map(String::as_str)) {
            d = d.with_help(format!("did you mean `{s}`?"));
        }
        d
    }
}
