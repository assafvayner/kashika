use kashika_model::{BUNDLED_ICONS, decode_bundled_icon};
use serde_json::{Map, Value, json};

fn main() {
    let mut out = Map::new();
    for (name, _) in BUNDLED_ICONS {
        let a = decode_bundled_icon(name, "#010203").unwrap_or_else(|e| panic!("{name}: {e}"));
        let b = decode_bundled_icon(name, "#040506").unwrap_or_else(|e| panic!("{name}: {e}"));
        let sa = String::from_utf8(a.bytes).expect("utf8 svg");
        let sb = String::from_utf8(b.bytes).expect("utf8 svg");
        assert_eq!(
            sa.replace("#010203", "#040506"),
            sb,
            "{name}: color substitution is not byte-identical"
        );
        let hull: Vec<Value> = a.unit_hull.iter().map(|p| json!([p.x, p.y])).collect();
        let dropped: Vec<Value> = a.dropped.iter().map(|(n, c)| json!([n, c])).collect();
        out.insert(
            name.to_string(),
            json!({
                "natural": [a.natural.0, a.natural.1],
                "unit_hull": hull,
                "current_color": a.current_color.is_some(),
                "dropped": dropped,
                "svg": sa,
            }),
        );
    }
    println!("{}", serde_json::to_string(&Value::Object(out)).unwrap());
}
