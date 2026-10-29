use kashika_diag::{Diagnostic, code};

use crate::sink::Sink;

/// `unused-allow`: an `allow=` entry on a line, box or group that suppressed no finding. Runs
/// after every other lint, so each entry has met every finding.
pub fn unused_allow(out: &mut Sink) {
    let found: Vec<Diagnostic> = out
        .suppressor()
        .unused()
        .map(|(o, a, none)| {
            let entry = match &a.other {
                Some(other) => format!("{}:{other}", a.code),
                None => a.code.clone(),
            };
            let what = match o.el.kind {
                "line" => o.id.to_string(),
                kind => format!("{kind} '{}'", o.id),
            };
            let within = o.el.span.line <= a.span.line && a.span.end_line <= o.el.span.end_line;
            let help = if none && o.reason.is_some() {
                "remove it, and `reason=` with it, which then explains nothing"
            } else {
                "remove it"
            };
            Diagnostic::warn(
                code::UNUSED_ALLOW,
                if within { a.span } else { o.el.span },
                format!("`allow={entry}` on {what} suppresses nothing"),
            )
            .with_element(o.el.clone())
            .with_help(help)
        })
        .collect();
    for d in found {
        out.push(d);
    }
}
