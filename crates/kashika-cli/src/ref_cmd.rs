use std::process::ExitCode;

use kashika_diag::did_you_mean;
use kashika_model::StrokeStyle;

use crate::Format;

const RULES: &str = include_str!("../ref/rules.md");
const SYNTAX: &str = include_str!("../ref/syntax.md");
const ANCHORS: &str = include_str!("../ref/anchors.md");
const PATHS: &str = include_str!("../ref/paths.md");
const LINTS: &str = include_str!("../ref/lints.md");
const RECIPES: &str = include_str!("../ref/recipes.md");
const EXAMPLE: &str = include_str!("../ref/example.md");
const UPLOAD: &str = include_str!("../../../examples/upload/upload.kash");
const LIMITS: &str = include_str!("../ref/limits.md");
const SHAPES: &str = include_str!("../ref/shapes.md");
const STYLES: &str = include_str!("../ref/styles.md");
const METRICS: &str = include_str!("../ref/metrics.md");
const CLI: &str = include_str!("../ref/cli.md");

pub const SCHEMA: &str = "kashika.ref/1";

pub const TOPICS: &[&str] = &[
    "syntax", "anchors", "paths", "shapes", "styles", "metrics", "lints", "examples", "cli",
];

fn examples() -> String {
    format!("{RECIPES}\n{EXAMPLE}\n```kash\n{UPLOAD}```\n")
}

fn shapes() -> String {
    let mut t = String::from(
        "| shape | unit path (120x80 box) | outline | text area |\n|---|---|---|---|\n",
    );
    for s in kashika_model::SHAPES {
        if let Some((d, outline, a)) = s.source.map(|f| f(120.0, 80.0)) {
            let name = s.name;
            let n = |v: f64| format!("{}", (v * 1000.0).round() / 1000.0);
            let d: Vec<String> = d
                .split_whitespace()
                .map(|w| w.parse().map_or(w.to_string(), n))
                .collect();
            t.push_str(&format!(
                "| `{name}` | `{}` | {outline} | ({},{},{},{}) |\n",
                d.join(" "),
                n(a.x),
                n(a.y),
                n(a.w),
                n(a.h)
            ));
        }
    }
    SHAPES.replace("{builtins}", t.trim_end())
}

/// `` `a`, `b`, `c` `` for a list of names.
fn code_list<'a>(names: impl IntoIterator<Item = &'a str>) -> String {
    names
        .into_iter()
        .map(|n| format!("`{n}`"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn styles_table() -> String {
    let mut t = String::from("| built-in | fill | stroke |\n|---|---|---|");
    for s in kashika_model::STYLES {
        let fill = match s.fill {
            "none" => "none".to_string(),
            f => format!("`{f}`"),
        };
        let mut stroke = format!("`{}`", s.stroke);
        match s.stroke_style {
            Some(StrokeStyle::Dashed) => stroke.push_str(", dashed"),
            Some(StrokeStyle::Dotted) => stroke.push_str(", dotted"),
            Some(StrokeStyle::Solid) | None => {}
        }
        if let Some(c) = s.color {
            stroke.push_str(&format!(", text `{c}`"));
        }
        t.push_str(&format!("\n| `{}` | {fill} | {stroke} |", s.name));
    }
    t
}

/// Fills the `{shapes}`, `{icons}`, `{styles}` and `{markers}` placeholders from the
/// built-in registries, so the pages never drift from them.
fn fill(md: &str) -> String {
    md.replace("{shapes}", &code_list(kashika_model::shape_names()))
        .replace(
            "{icons}",
            &code_list(kashika_model::BUNDLED_ICONS.iter().map(|(n, _)| *n)),
        )
        .replace("{styles}", &styles_table())
        .replace(
            "{markers}",
            &code_list(kashika_model::Marker::NAMES.iter().copied()),
        )
}

fn metrics() -> String {
    const SAMPLE: &str = "The quick brown fox jumps over the lazy dog 0123456789";
    let n = SAMPLE.chars().count() as f64;
    let width = |size: f64, bold: bool| {
        kashika_text::measure(SAMPLE, size, bold).map_or(0.0, |b| b.width()) / n
    };
    let mut s = String::from(METRICS);
    s.push_str("\n| font-size | regular | bold | line height |\n|---|---|---|---|\n");
    for size in [10.0, 11.0, 12.0, 14.0, 16.0, 18.0, 20.0] {
        s.push_str(&format!(
            "| {size} | {:.2} | {:.2} | {:.2} |\n",
            width(size, false),
            width(size, true),
            size * kashika_text::LINE_HEIGHT
        ));
    }
    s
}

pub fn page(topic: Option<&str>) -> Result<String, String> {
    raw_page(topic).map(|md| fill(&md))
}

fn raw_page(topic: Option<&str>) -> Result<String, String> {
    let Some(topic) = topic else {
        let ex = examples();
        return Ok([RULES, SYNTAX, ANCHORS, PATHS, LINTS, ex.as_str(), LIMITS].join("\n"));
    };
    Ok(match topic {
        "syntax" => SYNTAX.into(),
        "anchors" => ANCHORS.into(),
        "paths" => PATHS.into(),
        "shapes" => shapes(),
        "styles" => STYLES.into(),
        "metrics" => metrics(),
        "lints" => LINTS.into(),
        "examples" => examples(),
        "cli" => CLI.into(),
        code => return lint(code).ok_or_else(|| unknown(code)),
    })
}

/// The cells of each row of the diagnostics table: code, severity, trigger, fix.
fn rows() -> impl Iterator<Item = Vec<&'static str>> {
    LINTS
        .lines()
        .filter(|l| l.starts_with("| `"))
        .map(|l| l.trim_matches('|').split('|').map(str::trim).collect())
}

fn lint(code: &str) -> Option<String> {
    let row = rows().find(|r| r[0].trim_matches('`') == code)?;
    Some(format!(
        "# {code} ({})\n\nTrigger: {}.\n\nFix: {}.\n",
        row[1], row[2], row[3]
    ))
}

fn unknown(topic: &str) -> String {
    let codes: Vec<&str> = rows().map(|r| r[0].trim_matches('`')).collect();
    let mut msg = format!("unknown ref topic `{topic}`");
    let all = TOPICS.iter().copied().chain(codes.iter().copied());
    let sep = match did_you_mean(topic, all) {
        Some(s) => {
            msg.push_str(&format!("; did you mean `{s}`?"));
            " "
        }
        None => "; ",
    };
    msg.push_str(&format!(
        "{sep}Topics: {}, or a diagnostic code",
        TOPICS.join(", ")
    ));
    msg
}

pub fn run(topic: Option<&str>, json: bool) -> ExitCode {
    let format = if json { Format::Json } else { Format::Text };
    match page(topic) {
        Err(msg) => crate::tool_error(format, SCHEMA, &msg),
        Ok(md) if json => {
            println!(
                "{}",
                serde_json::json!({
                    "schema": SCHEMA,
                    "tool": env!("CARGO_PKG_VERSION"),
                    "topic": topic,
                    "topics": TOPICS,
                    "markdown": md,
                })
            );
            ExitCode::SUCCESS
        }
        Ok(md) => {
            print!("{md}");
            ExitCode::SUCCESS
        }
    }
}
