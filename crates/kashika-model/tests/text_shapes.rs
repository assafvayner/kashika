use kashika_diag::Diagnostic;
use kashika_geom::Rect;
use kashika_model::{Font, Scene, Shape, TextRole, compile};

const HEAD: &str = "diagram \"t\" size (800,600)\n";

fn ok(body: &str) -> Scene {
    let (scene, diags) = compile(&format!("{HEAD}{body}"));
    assert!(diags.is_empty(), "{diags:#?}");
    scene
}

fn errors(body: &str) -> Vec<Diagnostic> {
    compile(&format!("{HEAD}{body}")).1
}

fn shape<'s>(s: &'s Scene, id: &str) -> &'s Shape {
    s.shapes.iter().find(|x| x.id == id).unwrap()
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-3
}

#[test]
fn label_is_centered_in_the_text_area() {
    let s = ok("box api \"API\" at (40,40) size (120,60)\n");
    let b = shape(&s, "api");
    assert_eq!(b.text_area, Rect::new(48.0, 48.0, 104.0, 44.0));
    assert_eq!(b.text.len(), 1);
    let t = &b.text[0];
    assert_eq!(t.role, TextRole::Label);
    assert_eq!((t.size, t.color.as_str(), t.bold), (14.0, "#0f172a", false));
    assert!(close(t.rect.w, 22.3604), "{:?}", t.rect);
    assert!(close(t.rect.x, 100.0 - 22.3604 / 2.0));
    assert_eq!((t.rect.y, t.rect.h), (61.25, 17.5));
    assert_eq!(t.lines[0].runs[0].font, Font::Regular);
    assert_eq!(b.title_band, 0.0);
}

#[test]
fn sublabel_is_a_smaller_muted_line_below() {
    let s =
        ok("box api \"Payments API\" at (0,0) size (200,120) sublabel=\"v2.4.1\" style=service\n");
    let b = shape(&s, "api");
    let (l, sub) = (&b.text[0], &b.text[1]);
    assert_eq!(sub.role, TextRole::Sublabel);
    assert!(close(sub.size, 11.2));
    assert_eq!(sub.color, "#64748b");
    let h = 17.5 + 2.0 + 14.0;
    assert!(close(l.rect.y, 8.0 + (104.0 - h) / 2.0));
    assert!(close(sub.rect.y, l.rect.bottom() + 2.0));
}

#[test]
fn text_align_valign_and_gap() {
    let s = ok(
        "box a \"A\" at (0,0) size (100,100) sublabel=\"b\" text-align=left text-valign=top gap=6\n",
    );
    let b = shape(&s, "a");
    assert_eq!((b.text[0].rect.x, b.text[0].rect.y), (8.0, 8.0));
    assert_eq!(b.text[1].rect.y, 8.0 + 17.5 + 6.0);
    let s = ok("box a \"A\" at (0,0) size (100,100) text-align=right text-valign=bottom\n");
    let r = shape(&s, "a").text[0].rect;
    assert!(close(r.right(), 92.0) && close(r.bottom(), 92.0), "{r:?}");
}

#[test]
fn bold_style_and_markup_pick_fonts() {
    let s = ok(
        "box a \"**A** `b` c\" at (0,0) size (200,60)\nbox z \"Z\" at (0,100) size (60,60) font-weight=bold\n",
    );
    let runs = &shape(&s, "a").text[0].lines[0].runs;
    let fonts: Vec<Font> = runs.iter().map(|r| r.font).collect();
    assert_eq!(
        fonts,
        vec![Font::Bold, Font::Regular, Font::Mono, Font::Regular]
    );
    let z = &shape(&s, "z").text[0];
    assert!(z.bold);
    assert_eq!(z.lines[0].runs[0].font, Font::Bold);
}

#[test]
fn unclosed_markup_in_labels_is_a_parse_error() {
    let d = errors("box a \"**A\" at (0,0) size (200,60)\n");
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(d[0].code, "parse");
    assert_eq!(d[0].message, "unclosed `**` in text");
    let d = errors("box a at (0,0) size (200,60) sublabel=\"`v\"\n");
    assert_eq!(d[0].code, "parse");
    assert_eq!(d[0].span.col, 30);
}

#[test]
fn label_overflow_reports_the_required_size() {
    let d = errors("box a \"Payments API\" at (0,0) size (80,30)\n");
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(d[0].code, "label-overflow");
    assert_eq!(
        d[0].message,
        "its text needs 91.64x17.5px but the text area of `a` is 64x14px (too small: 27.64px narrower, 3.5px shorter)"
    );
    assert_eq!(
        d[0].help,
        vec!["make `a` at least size (108,34), or shorten the text".to_string()]
    );
    assert!(errors("box a \"Payments API\" at (0,0) size (108,34)\n").is_empty());
}

#[test]
fn padding_changes_the_text_area() {
    let s = ok("box a \"A\" at (0,0) size (40,40) padding=2\n");
    assert_eq!(shape(&s, "a").text_area, Rect::new(2.0, 2.0, 36.0, 36.0));
}

#[test]
fn stacked_block_items() {
    let s = ok(
        "box api at (100,80) size (200,120) text-align=left {\n  text \"**Payments API**\"\n  text \"v2.4.1\" size=11 muted\n  text \"• validates cards\" size=12 align=right\n}\n",
    );
    let b = shape(&s, "api");
    let roles: Vec<TextRole> = b.text.iter().map(|t| t.role).collect();
    assert_eq!(roles, vec![TextRole::Stacked; 3]);
    assert_eq!(b.text[0].rect.x, 108.0);
    assert_eq!(b.text[1].color, "#64748b");
    assert_eq!(b.text[1].size, 11.0);
    assert!(close(b.text[2].rect.right(), 292.0));
    let h = 17.5 + 13.75 + 15.0 + 2.0 * 2.0;
    assert!(close(b.text[0].rect.y, 88.0 + (104.0 - h) / 2.0));
    assert!(close(b.text[1].rect.y, b.text[0].rect.bottom() + 2.0));
}

#[test]
fn absolute_block_items_use_the_box_frame() {
    let s = ok(
        "box a at (100,50) size (200,100) {\n  text \"x\" at (20,30) align=left\n  text \"y\" at a.center\n}\n",
    );
    let b = shape(&s, "a");
    assert_eq!(b.text[0].role, TextRole::Absolute);
    assert_eq!(b.text[0].rect.x, 120.0);
    assert_eq!(b.text[0].rect.y, 80.0 - 17.5 / 2.0);
    let c = b.text[1].rect.center();
    assert!(close(c.x, 200.0) && close(c.y, 100.0), "{c:?}");
}

#[test]
fn absolute_items_may_join_a_label() {
    let s =
        ok("box a \"A\" at (0,0) size (200,100) sublabel=\"s\" {\n  text \"x\" at (20,80)\n}\n");
    let roles: Vec<TextRole> = shape(&s, "a").text.iter().map(|t| t.role).collect();
    assert_eq!(
        roles,
        vec![TextRole::Label, TextRole::Sublabel, TextRole::Absolute]
    );
}

#[test]
fn label_with_stacked_items_is_a_text_conflict() {
    let d = errors("box a \"A\" at (0,0) size (200,100) {\n  text \"x\"\n}\n");
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(d[0].code, "text-conflict");
    assert_eq!(d[0].span.line, 3);
    let d = errors("box a at (0,0) size (200,100) sublabel=\"s\" {\n  text \"x\"\n}\n");
    assert_eq!(d[0].code, "text-conflict");
}

#[test]
fn absolute_item_larger_than_the_text_area_overflows() {
    let d = errors("box a at (0,0) size (40,40) {\n  text \"Payments API\" at (20,20)\n}\n");
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(d[0].code, "label-overflow");
    assert_eq!(d[0].span.line, 3);
    assert!(
        d[0].message.starts_with("a block item needs 91.64x17.5px"),
        "{}",
        d[0].message
    );
}

#[test]
fn block_item_attributes_are_checked() {
    let d = errors("box a at (0,0) size (200,100) {\n  text \"x\" weight=bold\n}\n");
    assert_eq!(d[0].code, "attr");
    assert_eq!(
        d[0].message,
        "unknown attribute `weight` on text block item"
    );
    assert_eq!(
        errors("box a at (0,0) size (200,100) {\n  text \"x\" size=0\n}\n")[0].code,
        "size"
    );
    assert_eq!(
        errors("box a at (0,0) size (200,100) {\n  text \"x\" at nope.left\n}\n")[0].code,
        "unknown-id"
    );
}

#[test]
fn text_attribute_values_are_checked() {
    assert_eq!(
        errors("box a at (0,0) size (9,9) text-align=middle\n")[0].code,
        "attr"
    );
    assert_eq!(
        errors("box a at (0,0) size (9,9) text-valign=center\n")[0].code,
        "attr"
    );
    assert_eq!(errors("box a at (0,0) size (9,9) gap=-1\n")[0].code, "attr");
    assert_eq!(
        errors("box a at (0,0) size (9,9) sublabel=3\n")[0].code,
        "attr"
    );
}

#[test]
fn group_title_sits_top_left_and_reserves_a_band() {
    let s = ok("group g \"VPC\" at (10,10) size (300,200) sublabel=\"10.0.0.0/16\" {\n}\n");
    let g = shape(&s, "g");
    assert_eq!((g.text[0].rect.x, g.text[0].rect.y), (18.0, 18.0));
    assert_eq!(g.title_band, 8.0 + 17.5 + 2.0 + 14.0);
    let s = ok("group g at (0,0) size (300,200) {\n}\n");
    assert_eq!(shape(&s, "g").title_band, 0.0);
    let s = ok("group g \"VPC\" at (0,0) size (300,200) text-align=center {\n}\n");
    assert!(close(shape(&s, "g").text[0].rect.center().x, 150.0));
}

#[test]
fn group_rejects_text_valign() {
    let d = errors("group g \"G\" at (0,0) size (300,200) text-valign=top {\n}\n");
    assert_eq!(d[0].code, "attr");
}

#[test]
fn group_label_overflow() {
    let d = errors("group g \"A very long group title\" at (0,0) size (60,200) {\n}\n");
    assert_eq!(d[0].code, "label-overflow");
}

#[test]
fn label_escapes_are_literal() {
    let s = ok("box a \"\\*\\*a\\*\\* \\` b\\\\c\" at (0,0) size (200,60) sublabel=\"`v` \\`\"\n");
    let t = &shape(&s, "a").text;
    assert_eq!(t[0].lines[0].runs.len(), 1, "{:?}", t[0].lines[0].runs);
    assert_eq!(t[0].lines[0].runs[0].text, "**a** ` b\\c");
    let fonts: Vec<Font> = t[1].lines[0].runs.iter().map(|r| r.font).collect();
    assert_eq!(fonts, vec![Font::Mono, Font::Regular]);
    assert_eq!(t[1].lines[0].runs[1].text, " `");
}

#[test]
fn group_label_markup_is_checked() {
    let d = errors("group g \"`VPC\" at (0,0) size (300,200) {\n}\n");
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(d[0].code, "parse");
}

#[test]
fn label_overflow_names_the_short_dimension() {
    let d = errors("box a \"Payments API\" at (0,0) size (200,30)\n");
    assert_eq!(d[0].code, "label-overflow");
    assert_eq!(
        d[0].message,
        "its text needs 91.64x17.5px but the text area of `a` is 184x14px (too short by 3.5px)"
    );
    let d = errors("box a \"Payments API\" at (0,0) size (80,60)\n");
    assert_eq!(
        d[0].message,
        "its text needs 91.64x17.5px but the text area of `a` is 64x44px (too narrow by 27.64px)"
    );
}
