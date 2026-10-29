use kashika_diag::Diagnostic;
use kashika_geom::Rect;
use kashika_model::{Font, Scene, Shape, TextRole, TitlePos, compile};

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
fn padding_pair_insets_each_axis() {
    let s = ok("box a \"A\" at (0,0) size (40,40) padding=(10,3)\n");
    assert_eq!(shape(&s, "a").text_area, Rect::new(10.0, 3.0, 20.0, 34.0));
    let s = ok("group g \"G\" at (100,100) size (200,100) padding=(4,10) {\n}\n");
    let g = shape(&s, "g");
    assert_eq!(g.text_area, Rect::new(104.0, 110.0, 192.0, 80.0));
    assert_eq!(g.text[0].rect.x, 104.0);
    assert!(close(g.title_band, 27.5), "{}", g.title_band);
}

#[test]
fn padding_pair_sets_the_overflow_size() {
    let d = errors("box a \"Payments API\" at (0,0) size (50,10) padding=(4,2)\n");
    assert_eq!(
        d[0].help,
        vec!["make `a` at least size (100,22), or shorten the text".to_string()]
    );
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
fn text_takes_font_size_and_text_align_or_their_aliases() {
    let s = ok("text a \"a\" at (100,100) font-size=20 text-align=left\n\
         text b \"a\" at (100,200) size=20 align=left\n\
         box bx at (300,100) size (200,100) {\n  text \"c\" font-size=10 text-align=right\n  text \"c\" size=10 align=right\n}\n");
    let (a, b) = (&s.texts[0], &s.texts[1]);
    assert_eq!(
        (a.size, a.align, a.rect.x, a.rect.w),
        (b.size, b.align, b.rect.x, b.rect.w)
    );
    assert_eq!(a.size, 20.0);
    let x = shape(&s, "bx");
    assert_eq!((x.text[0].size, x.text[1].size), (10.0, 10.0));
    assert_eq!(x.text[0].rect.right(), x.text[1].rect.right());
    let d = errors("text \"a\" at (0,0) size=12 font-size=14 align=left text-align=right\n");
    assert_eq!(d.len(), 2, "{d:#?}");
    assert!(d.iter().all(|d| d.code == "attr"));
    assert_eq!(
        d[0].message,
        "duplicate attribute: `size` is another name for `font-size`"
    );
    assert_eq!(
        d[0].help,
        vec!["keep one of `size` and `font-size`".to_string()]
    );
    let d = errors("box a at (0,0) size (200,100) {\n  text \"x\" text-align=left align=left\n}\n");
    assert_eq!(
        d[0].message,
        "duplicate attribute: `align` is another name for `text-align`"
    );
}

#[test]
fn wrong_context_names_suggest_the_right_one() {
    let help = |src: &str| errors(src)[0].help.clone();
    assert_eq!(
        help("box a \"A\" at (0,0) size (90,40) align=left\n"),
        ["did you mean `text-align`?"]
    );
    assert_eq!(
        help("box a \"A\" at (0,0) size (90,40) size=12\n"),
        ["did you mean `font-size`?"]
    );
    assert_eq!(
        help("box a \"A\" at (0,0) size=(90,40)\n"),
        ["`size` is a clause, not an attribute: write `size (w,h)`, without `=`"]
    );
    assert_eq!(
        help("box a \"A\" at (0,0) size (90,40) valign=top\n"),
        ["did you mean `text-valign`?"]
    );
    assert_eq!(
        help("style s { text-size=12 }\n")[0],
        "did you mean `font-size`?"
    );
    assert_eq!(
        help("text \"t\" at (0,0) font-color=red\n"),
        ["did you mean `color`?"]
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
    let d = errors(&format!(
        "box a at (0,0) size (9,9) gap={}\n",
        "9".repeat(300)
    ));
    assert_eq!(d[0].code, "attr");
    assert_eq!(d[0].message, "`gap` must be at most 1000000");
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
fn group_title_bottom_reserves_a_band_at_the_bottom() {
    let s = ok(
        "group g \"VPC\" at (10,10) size (300,200) title=bottom icon=\"server\" text-align=right {\n}\n",
    );
    let g = shape(&s, "g");
    assert_eq!(g.title, TitlePos::Bottom);
    assert_eq!(g.title_band, 8.0 + 17.5);
    let t = g.text[0].rect;
    assert!(close(t.bottom(), 202.0) && close(t.right(), 302.0), "{t:?}");
    let icon = g.icon.as_ref().unwrap().rect;
    assert_eq!(icon.x, 18.0);
    assert!(close(icon.center().y, t.center().y));
    let s = ok("group g \"VPC\" at (0,0) size (300,200) title=top {\n}\n");
    assert_eq!(shape(&s, "g").title, TitlePos::Top);
}

#[test]
fn title_is_a_style_key_with_inline_precedence() {
    let s = ok("style boundary { title=bottom }\nstyle low { title=top }\n\
         group a \"A\" at (0,0) size (100,80) {\n}\n\
         group b \"B\" at (120,0) size (100,80) style=low {\n}\n\
         group c \"C\" at (240,0) size (100,80) style=low title=bottom {\n}\n");
    let titles: Vec<TitlePos> = ["a", "b", "c"].map(|id| shape(&s, id).title).into();
    assert_eq!(titles, [TitlePos::Bottom, TitlePos::Top, TitlePos::Bottom]);
    let d = errors("style low { title=top }\nbox b \"B\" at (0,0) size (80,40) style=low\n");
    assert_eq!(
        d[0].message,
        "this style sets `title`, but only groups have a title"
    );
}

#[test]
fn title_is_a_group_attribute() {
    let d = errors("box b \"B\" at (0,0) size (80,40) title=bottom\n");
    assert_eq!(d[0].code, "attr");
    assert_eq!(
        d[0].help,
        ["`title=` places a group's title; a box places its text with `text-valign`"]
    );
    let d = errors("group g \"G\" at (0,0) size (300,200) title=left {\n}\n");
    assert_eq!(d[0].message, "`title` expects one of: top, bottom");
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

/// The size `label-overflow` asks for when the box is drawn at `size (1,1)`.
fn overflow_size(label: &str, attrs: &str) -> (f64, f64) {
    let d = errors(&format!("box a {label} at (0,0) size (1,1) {attrs}\n"));
    let help = &d.iter().find(|d| d.code == "label-overflow").unwrap().help[0];
    let size = help
        .split_once("size (")
        .and_then(|(_, r)| r.split_once(')'))
        .unwrap()
        .0;
    let (w, h) = size.split_once(',').unwrap();
    (w.parse().unwrap(), h.parse().unwrap())
}

#[test]
fn omitted_size_is_the_size_label_overflow_reports() {
    for (label, attrs) in [
        ("\"Payments API\"", ""),
        ("\"Payments API\"", "sublabel=\"v2.4.1\" padding=(12,4)"),
        ("\"Decision\"", "shape=diamond"),
        ("\"Orders\"", "shape=cylinder"),
        ("\"Two\\nlines\\nand three\"", "shape=cylinder"),
        ("\"Worker pool\"", "shape=hexagon"),
        ("\"Internet\"", "shape=cloud"),
        ("\"Server\"", "icon=\"server\""),
        ("\"Server\"", "icon=\"server\" icon-pos=top"),
        ("\"**Bold** title\"", "font-weight=bold font-size=18"),
        (
            "",
            "{\n  text \"**Worker**\"\n  text \"retries charges\" size=12\n}",
        ),
    ] {
        let want = overflow_size(label, attrs);
        let s = ok(&format!("box a {label} at (0,0) {attrs}\n"));
        let r = shape(&s, "a").rect;
        assert_eq!((r.w, r.h), want, "{label} {attrs}");
    }
    let s = ok("box a at (0,0) {\n  text \"wide absolute item\" at (8,8)\n}\n");
    assert!(shape(&s, "a").rect.w > 100.0);
}

#[test]
fn auto_axes_fit_and_fixed_axes_stay() {
    let s = ok("box a \"Payments API\" at (0,0) size (auto, 60)\n\
         box b \"Q\" at (0,100) size (200, auto) shape=diamond\n\
         box c \"Payments API\" at (0,200) size (auto,auto)\n");
    let r = |id| shape(&s, id).rect;
    assert_eq!((r("a").w, r("a").h), (108.0, 60.0));
    assert_eq!((r("b").w, r("b").h), (200.0, 77.0));
    assert_eq!((r("c").w, r("c").h), (108.0, 34.0));
}

#[test]
fn fitted_sizes_round_up_to_the_grid() {
    let (s, d) = compile(
        "diagram \"t\" size (300,300) grid=10\n\
         box a \"Payments API\" at (0,0)\n\
         box b \"Payments API\" at (0,100) size (auto, 35)\n",
    );
    assert!(d.iter().all(|d| d.code == "off-grid"), "{d:#?}");
    let r = |id| shape(&s, id).rect;
    assert_eq!((r("a").w, r("a").h), (110.0, 40.0));
    assert_eq!((r("b").w, r("b").h), (110.0, 35.0));
}

#[test]
fn anchors_and_pins_use_the_fitted_size() {
    let s = ok(
        "box a \"Client\" at (100,100) pin=center\nbox b \"B\" at a.right+(20,0) size (40,40) pin=left\n",
    );
    let a = shape(&s, "a").rect;
    assert_eq!((a.center().x, a.center().y), (100.0, 100.0));
    assert_eq!(shape(&s, "b").rect.x, a.right() + 20.0);
}

#[test]
fn nothing_to_fit_is_a_size_error() {
    let d = errors("box a at (0,0)\nbox b at (0,0) size (auto, 40)\n");
    assert_eq!(d.len(), 2, "{d:#?}");
    assert!(d.iter().all(|d| d.code == "size"));
    assert_eq!(d[0].message, "`a` has no size to fit: it has no text");
    assert_eq!(d[0].help, vec!["give it a size: `size (w,h)`".to_string()]);
    let d = errors("box a \"A\" at (0,0) size (auto, 0)\n");
    assert_eq!(d[0].message, "`a` must have a positive size");
}
