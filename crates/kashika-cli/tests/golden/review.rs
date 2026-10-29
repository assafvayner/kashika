use std::path::{Path, PathBuf};
use std::sync::Mutex;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;

/// A render case whose `out.svg` was blessed with new bytes or does not match.
pub struct Entry {
    pub name: String,
    pub description: String,
    /// The committed `out.svg`, if there was one.
    pub expected: Option<Vec<u8>>,
    pub actual: Vec<u8>,
    pub blessed: bool,
}

static ENTRIES: Mutex<Vec<Entry>> = Mutex::new(Vec::new());

pub fn record(entry: Entry) {
    ENTRIES
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .push(entry);
}

pub fn path() -> PathBuf {
    Path::new(env!("CARGO_TARGET_TMPDIR")).join("golden-review.html")
}

/// Writes the review page for the recorded entries, or removes a stale one when there are
/// none. Returns the page's path when it was written.
pub fn write() -> Option<PathBuf> {
    let mut entries = std::mem::take(&mut *ENTRIES.lock().unwrap_or_else(|e| e.into_inner()));
    let path = path();
    if entries.is_empty() {
        let _ = std::fs::remove_file(&path);
        return None;
    }
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    std::fs::write(&path, page(&entries)).ok()?;
    Some(path)
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn img(svg: &[u8]) -> String {
    format!(
        "<img src=\"data:image/svg+xml;base64,{}\">",
        STANDARD.encode(svg)
    )
}

fn page(entries: &[Entry]) -> String {
    let mut out = String::from(
        "<!doctype html>\n<html><head><meta charset=\"utf-8\"><title>Golden review</title>\n<style>\
body{font:14px system-ui,sans-serif;margin:16px;background:#fff;color:#111}\
section{margin:0 0 32px}h2{font-size:16px;margin:0}p{margin:4px 0 8px;color:#555}\
.pair{display:flex;gap:16px;flex-wrap:wrap}.side{flex:1;min-width:280px}\
.side h3{font-size:13px;margin:0 0 4px;color:#555}\
img{max-width:100%;border:1px solid #ccc;background:repeating-conic-gradient(#eee 0 25%,#fff 0 50%) 0 0/16px 16px}\
</style></head><body>\n<h1>Golden render review</h1>\n",
    );
    for e in entries {
        let status = if e.blessed { "blessed" } else { "mismatch" };
        let expected = e
            .expected
            .as_deref()
            .map_or_else(|| "<p>(none)</p>".to_string(), img);
        out.push_str(&format!(
            "<section><h2>{} ({status})</h2><p>{}</p><div class=\"pair\">\
<div class=\"side\"><h3>expected</h3>{expected}</div>\
<div class=\"side\"><h3>actual</h3>{}</div></div></section>\n",
            escape(&e.name),
            escape(&e.description),
            img(&e.actual)
        ));
    }
    out.push_str("</body></html>\n");
    out
}
