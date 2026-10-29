use kashika_geom::fmt_num;
use serde_json::Value;

const FOLD_WIDTH: usize = 100;

/// The parser's AST as `{:#?}`, with spans written `line:col-end_line:end_col` and
/// short leaf-only groups folded onto one line.
pub fn ast(file: &kashika_syntax::ast::File) -> String {
    let mut lines: Vec<String> = format!("{file:#?}").lines().map(str::to_string).collect();
    loop {
        let folded = fold_once(&lines);
        if folded == lines {
            break;
        }
        lines = folded;
    }
    let mut out = lines.join("\n");
    out.push('\n');
    out
}

fn indent(s: &str) -> usize {
    s.len() - s.trim_start().len()
}

fn opener(s: &str) -> Option<char> {
    s.chars().last().filter(|c| matches!(c, '(' | '[' | '{'))
}

fn fold_once(lines: &[String]) -> Vec<String> {
    let mut out = Vec::with_capacity(lines.len());
    let mut i = 0;
    while i < lines.len() {
        if let Some(open) = opener(&lines[i]) {
            let ind = indent(&lines[i]);
            let close = lines[i + 1..]
                .iter()
                .position(|l| indent(l) == ind)
                .map(|p| i + 1 + p);
            if let Some(j) = close {
                let inner = &lines[i + 1..j];
                let leaf = inner
                    .iter()
                    .all(|l| indent(l) == ind + 4 && opener(l).is_none());
                if leaf {
                    let parts: Vec<&str> = inner
                        .iter()
                        .map(|l| l.trim().trim_end_matches(','))
                        .collect();
                    let body = parts.join(", ");
                    let closing = lines[j].trim();
                    let joined = if open == '{' {
                        format!("{} {body} {closing}", lines[i])
                    } else {
                        format!("{}{body}{closing}", lines[i])
                    };
                    let joined = compact_span(&joined);
                    if joined.len() <= FOLD_WIDTH {
                        out.push(joined);
                        i = j + 1;
                        continue;
                    }
                }
            }
        }
        out.push(lines[i].clone());
        i += 1;
    }
    out
}

/// `Span { line: 4, col: 1, end_line: 4, end_col: 33 }` becomes `4:1-4:33`.
fn compact_span(s: &str) -> String {
    let Some(start) = s.find("Span { line: ") else {
        return s.to_string();
    };
    let Some(len) = s[start..].find(" }") else {
        return s.to_string();
    };
    let inner = &s[start + "Span { ".len()..start + len];
    let nums: Vec<&str> = inner
        .split(", ")
        .filter_map(|kv| kv.split(": ").nth(1))
        .collect();
    if nums.len() != 4 {
        return s.to_string();
    }
    format!(
        "{}{}:{}-{}:{}{}",
        &s[..start],
        nums[0],
        nums[1],
        nums[2],
        nums[3],
        &s[start + len + 2..]
    )
}

/// One-line JSON with every float rounded by [`fmt_num`].
pub fn compact(v: &Value) -> String {
    match v {
        Value::Number(n) => n
            .as_i64()
            .map(|i| i.to_string())
            .or_else(|| n.as_u64().map(|u| u.to_string()))
            .unwrap_or_else(|| fmt_num(n.as_f64().unwrap_or(f64::NAN))),
        Value::Array(a) => format!("[{}]", a.iter().map(compact).collect::<Vec<_>>().join(",")),
        Value::Object(o) => format!(
            "{{{}}}",
            o.iter()
                .map(|(k, v)| format!("{}:{}", Value::String(k.clone()), compact(v)))
                .collect::<Vec<_>>()
                .join(",")
        ),
        other => other.to_string(),
    }
}

/// Indented JSON, floats rounded by [`fmt_num`]; arrays and objects of scalars stay on one line.
pub fn pretty(v: &Value) -> String {
    let mut out = String::new();
    write_pretty(v, 0, &mut out);
    out.push('\n');
    out
}

fn is_scalar(v: &Value) -> bool {
    !matches!(v, Value::Array(_) | Value::Object(_))
}

fn flat(v: &Value) -> bool {
    match v {
        Value::Array(a) => a.iter().all(is_scalar) || (a.len() <= 4 && a.iter().all(flat_point)),
        Value::Object(o) => o.values().all(|x| is_scalar(x) || flat_point(x)),
        _ => true,
    }
}

/// `{"x":1,"y":2}` points and `[x,y]` pairs read best inline.
fn flat_point(v: &Value) -> bool {
    match v {
        Value::Object(o) => o.len() <= 4 && o.values().all(is_scalar),
        Value::Array(a) => a.len() <= 4 && a.iter().all(is_scalar),
        _ => false,
    }
}

fn write_pretty(v: &Value, depth: usize, out: &mut String) {
    let pad = "  ".repeat(depth + 1);
    let end = "  ".repeat(depth);
    match v {
        Value::Array(a) if !a.is_empty() && !flat(v) => {
            out.push_str("[\n");
            for (i, x) in a.iter().enumerate() {
                out.push_str(&pad);
                write_pretty(x, depth + 1, out);
                out.push_str(if i + 1 < a.len() { ",\n" } else { "\n" });
            }
            out.push_str(&end);
            out.push(']');
        }
        Value::Object(o) if !o.is_empty() && !flat(v) => {
            out.push_str("{\n");
            for (i, (k, x)) in o.iter().enumerate() {
                out.push_str(&pad);
                out.push_str(&Value::String(k.clone()).to_string());
                out.push_str(": ");
                write_pretty(x, depth + 1, out);
                out.push_str(if i + 1 < o.len() { ",\n" } else { "\n" });
            }
            out.push_str(&end);
            out.push('}');
        }
        other => out.push_str(&compact(other)),
    }
}

/// The resolved scene as rounded, indented JSON.
pub fn scene(scene: &kashika_model::Scene) -> String {
    pretty(&serde_json::to_value(scene).expect("scene serializes"))
}

/// One block per diagnostic: `severity[code] L:C-L:C: message`, then help, suppression,
/// elements, facts and suggestions. `this is kashika <version>` is redacted.
pub fn diagnostics(diags: &[Value]) -> String {
    let mut out = String::new();
    for d in diags {
        let s = &d["span"];
        out.push_str(&format!(
            "{}[{}] {}:{}-{}:{}: {}\n",
            d["severity"].as_str().unwrap_or("?"),
            d["code"].as_str().unwrap_or("?"),
            s["line"],
            s["col"],
            s["end_line"],
            s["end_col"],
            d["message"].as_str().unwrap_or("")
        ));
        for h in d["help"].as_array().into_iter().flatten() {
            out.push_str(&format!("  = help: {}\n", h.as_str().unwrap_or("")));
        }
        if d["suppressed"] == Value::Bool(true) {
            match d["reason"].as_str() {
                Some(r) => out.push_str(&format!("  = suppressed: {r}\n")),
                None => out.push_str("  = suppressed\n"),
            }
        }
        for key in ["elements", "facts", "suggestions"] {
            let v = &d[key];
            let empty = v.is_null()
                || v.as_array().is_some_and(Vec::is_empty)
                || v.as_object().is_some_and(serde_json::Map::is_empty);
            if !empty {
                out.push_str(&format!("  {key}: {}\n", compact(v)));
            }
        }
    }
    if out.is_empty() {
        out.push_str("(no diagnostics)\n");
    }
    out.replace(
        &format!("this is kashika {}", env!("CARGO_PKG_VERSION")),
        "this is kashika <tool>",
    )
}
