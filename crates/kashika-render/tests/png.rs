use kashika_model::compile;
use kashika_render::{render_svg, svg_to_png};

const BASIC: &str = "diagram \"Basic\" size (480,240)\n\
box api \"API\" at (40,40) size (120,60) style=service\n\
box db \"DB\" at api.right+(160,0) size (120,60) pin=left\n\
line api_db: api.right -> db.left end=arrow\n";

#[test]
fn png_has_signature_and_canvas_size() {
    let (scene, diags) = compile(BASIC);
    assert!(diags.is_empty(), "{diags:#?}");
    let png = svg_to_png(&render_svg(&scene)).unwrap();
    assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
    let w = u32::from_be_bytes(png[16..20].try_into().unwrap());
    let h = u32::from_be_bytes(png[20..24].try_into().unwrap());
    assert_eq!((w, h), (480, 240));
}

#[test]
fn invalid_svg_is_an_error() {
    assert!(svg_to_png("<not svg").is_err());
}

#[test]
fn png_is_deterministic_and_draws_glyph_paths_without_fonts() {
    let (scene, _) = compile(BASIC);
    let svg = render_svg(&scene);
    let a = svg_to_png(&svg).unwrap();
    assert_eq!(a, svg_to_png(&svg).unwrap());
    let pix = resvg::tiny_skia::Pixmap::decode_png(&a).unwrap();
    let dark = (60..140).any(|x| (50..90).any(|y| pix.pixel(x, y).unwrap().red() < 128));
    assert!(dark, "the API label was not rasterized");
}

#[test]
fn png_rasterizes_embedded_images() {
    use resvg::tiny_skia::{Color, Pixmap};
    let dir = tempfile::tempdir().unwrap();
    let mut red = Pixmap::new(10, 10).unwrap();
    red.fill(Color::from_rgba8(255, 0, 0, 255));
    std::fs::write(dir.path().join("red.png"), red.encode_png().unwrap()).unwrap();
    let opts = kashika_model::Options {
        path: Some(dir.path().join("i.kash")),
        ..Default::default()
    };
    let src = "diagram \"i\" size (100,100)\nimage logo \"red.png\" at (20,20) size (40,40)\n";
    let (scene, diags) = kashika_model::compile_with(src, &opts);
    assert!(diags.is_empty(), "{diags:#?}");
    let png = svg_to_png(&render_svg(&scene)).unwrap();
    let pix = Pixmap::decode_png(&png).unwrap();
    let p = pix.pixel(40, 40).unwrap();
    assert!(p.red() > 200 && p.green() < 50, "image not drawn: {p:?}");
    let q = pix.pixel(80, 80).unwrap();
    assert!(q.green() > 200, "background is not red: {q:?}");
}
