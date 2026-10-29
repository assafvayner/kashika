use std::path::Path;

use kashika_model::{Options, compile, compile_with};
use kashika_render::{render_svg, svg_to_png};

fn svg(src: &str) -> String {
    let (scene, diags) = compile(src);
    assert!(diags.is_empty(), "{diags:#?}");
    render_svg(&scene)
}

fn svg_in(dir: &Path, src: &str) -> String {
    let (scene, diags) = compile_with(
        src,
        &Options {
            path: Some(dir.join("main.kash")),
            ..Options::default()
        },
    );
    assert!(diags.is_empty(), "{diags:#?}");
    render_svg(&scene)
}

fn png_header(w: u32, h: u32) -> Vec<u8> {
    let mut b = vec![
        0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 13, b'I', b'H', b'D', b'R',
    ];
    b.extend(w.to_be_bytes());
    b.extend(h.to_be_bytes());
    b.extend([8, 6, 0, 0, 0, 0, 0, 0, 0]);
    b
}

#[test]
fn diamond_draws_a_closed_path() {
    let s = svg("diagram \"d\" size (200,100)\nbox d \"D\" at (0,0) size (120,80) shape=diamond\n");
    assert!(
        s.contains(
            r##"<path d="M 60 0 L 120 40 L 60 80 L 0 40 L 60 0 Z" fill="#ffffff" stroke-linejoin="round" stroke="#334155" stroke-width="1.5"/>"##
        ),
        "{s}"
    );
    assert!(!s.contains("<rect x=\"0\" y=\"0\" width=\"120\""));
}

#[test]
fn cylinder_draws_arcs_as_cubics() {
    let s = svg(
        "diagram \"c\" size (200,100)\nbox c \"DB\" at (0,0) size (120,80) shape=cylinder style=datastore\n",
    );
    assert!(s.contains(r#"<path d="M 0 20 C "#), "{s}");
    assert!(s.contains(r##"fill="#e6f4ea""##));
}

#[test]
fn images_and_icons_are_embedded() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("logo.png"), png_header(200, 100)).unwrap();
    let s = svg_in(
        dir.path(),
        "diagram \"i\" size (300,200)\nimage logo \"logo.png\" at (20,20) size (80,80) fit=cover\n\
         box a \"A\" at (120,20) size (160,60) icon=\"server\"\n",
    );
    assert!(
        s.contains(
            r#"<image x="20" y="20" width="80" height="80" preserveAspectRatio="xMidYMid slice" href="data:image/png;base64,iVBORw0KGgo"#
        ),
        "{s}"
    );
    let rect = s.find(r#"<rect x="120" y="20""#).expect("box");
    let icon = s
        .find(r#"<image x="128" y="38" width="24" height="24" preserveAspectRatio="xMidYMid meet" href="data:image/svg+xml;base64,"#)
        .expect("icon");
    assert!(rect < icon, "icon draws over its box");
    assert!(!s.contains("<script"));
}

#[test]
fn svg_shapes_draw_only_their_image() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("t.svg"),
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="40" height="20"><path d="M 0 0 L 40 20"/></svg>"##,
    )
    .unwrap();
    let s = svg_in(
        dir.path(),
        "diagram \"s\" size (300,200)\nshape t from \"t.svg\" text-area=below\nbox a at (100,100) size (80,40) shape=t\n",
    );
    assert!(
        s.contains(
            r#"<image x="100" y="100" width="80" height="40" preserveAspectRatio="xMidYMid meet""#
        ),
        "{s}"
    );
    assert!(!s.contains(r#"<rect x="100" y="100""#));
}

#[test]
fn image_paint_order_follows_file_order_with_boxes() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("logo.png"), png_header(4, 4)).unwrap();
    let s = svg_in(
        dir.path(),
        "diagram \"o\" size (300,200)\nbox a at (0,0) size (50,50)\nimage i \"logo.png\" at (60,0) size (50,50)\n\
         box b at (120,0) size (50,50)\nline l a.bottom -> v 20\n",
    );
    let a = s.find(r#"<rect x="0" y="0" width="50""#).unwrap();
    let i = s.find("<image").unwrap();
    let b = s.find(r#"<rect x="120""#).unwrap();
    let l = s.find("<path d=\"M 25 50").unwrap();
    assert!(a < i && i < b && b < l);
}

#[test]
fn output_is_deterministic() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("logo.png"), png_header(4, 4)).unwrap();
    let src = "diagram \"o\" size (300,200)\nimage i \"logo.png\" at (60,0) size (50,50)\nbox c at (0,60) size (90,60) shape=cloud icon=\"lock\"\n";
    assert_eq!(svg_in(dir.path(), src), svg_in(dir.path(), src));
}

fn red_png() -> Vec<u8> {
    let mut pm = resvg::tiny_skia::Pixmap::new(8, 8).unwrap();
    pm.fill(resvg::tiny_skia::Color::from_rgba8(255, 0, 0, 255));
    pm.encode_png().unwrap()
}

#[test]
fn png_output_draws_embedded_images() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("red.png"), red_png()).unwrap();
    let s = svg_in(
        dir.path(),
        "diagram \"p\" size (100,100)\nimage r \"red.png\" at (10,10) size (40,40)\n",
    );
    let png = svg_to_png(&s).unwrap();
    let pm = resvg::tiny_skia::Pixmap::decode_png(&png).unwrap();
    let px = |x, y| pm.pixel(x, y).unwrap();
    assert_eq!((px(30, 30).red(), px(30, 30).green()), (255, 0));
    assert_eq!(px(80, 80).alpha(), 0, "outside the image stays transparent");
}

#[test]
fn repeated_images_embed_once_and_still_rasterize() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("red.png"), red_png()).unwrap();
    let s = svg_in(
        dir.path(),
        "diagram \"p\" size (100,100)\nimage a \"red.png\" at (10,10) size (30,30)\nimage b \"red.png\" at (60,60) size (30,30) fit=cover\n",
    );
    assert_eq!(s.matches("data:image/png;base64,").count(), 1, "{s}");
    let png = svg_to_png(&s).unwrap();
    let pm = resvg::tiny_skia::Pixmap::decode_png(&png).unwrap();
    let px = |x, y| pm.pixel(x, y).unwrap();
    assert_eq!(px(20, 20).red(), 255);
    assert_eq!(px(75, 75).red(), 255);
    assert_eq!(px(50, 50).alpha(), 0);
}
