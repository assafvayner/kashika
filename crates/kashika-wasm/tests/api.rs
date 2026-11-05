use serde_json::Value;

const SRC: &str = "diagram \"t\" size (300,200)\nbox a \"A\" at (20,20) size (80,40)\nbox b \"B\" at (200,20) size (80,40)\nline ab a.right -> b.left\n";

fn v(json: String) -> Value {
    serde_json::from_str(&json).unwrap()
}

#[test]
fn check_json() {
    let r = v(kashika_wasm::check(SRC));
    assert_eq!(r["errors"], 0);
    assert!(r["text"].is_string());
    assert!(r["diagnostics"].is_array());
}

#[test]
fn inspect_json_and_error() {
    let r = v(kashika_wasm::inspect(SRC, vec![], false));
    assert!(r["text"].as_str().unwrap().contains("a "));
    assert!(r["report"]["shapes"].is_array());
    let e = v(kashika_wasm::inspect(SRC, vec!["zz".into()], false));
    assert!(e["error"].as_str().unwrap().starts_with("unknown id"));
}

#[test]
fn route_json_and_errors() {
    let r = v(kashika_wasm::route(
        SRC,
        Some("a.right".into()),
        Some("b.left".into()),
        "corner",
        None,
        8.0,
        3,
    ));
    assert!(r["text"].as_str().unwrap().starts_with("#1"));
    assert!(r["suggestion"]["candidates"].is_array());
    let e = v(kashika_wasm::route(
        SRC,
        Some("a.right".into()),
        Some("b.left".into()),
        "zig",
        None,
        8.0,
        3,
    ));
    assert_eq!(e["kind"], "usage");
    assert!(e["error"].is_string());
}

#[test]
fn measure_ref_icons_format_elements() {
    assert_eq!(v(kashika_wasm::measure("ab", 14.0, false))["lines"], 1);
    assert!(v(kashika_wasm::measure("**x", 14.0, false))["error"].is_string());
    assert!(
        v(kashika_wasm::ref_page(None))["page"]
            .as_str()
            .unwrap()
            .contains("## Topics")
    );
    assert!(v(kashika_wasm::ref_page(Some("nope".into())))["error"].is_string());
    assert!(
        v(kashika_wasm::icons(Some("database".into())))["icons"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["name"] == "cylinder")
    );
    assert_eq!(
        v(kashika_wasm::format("diagram \"t\"  size (1,1)\n"))["source"],
        "diagram \"t\" size (1, 1)\n"
    );
    assert!(v(kashika_wasm::format("box a (\n"))["error"].is_string());
    assert_eq!(v(kashika_wasm::elements(SRC))["elements"][0]["id"], "a");
}
