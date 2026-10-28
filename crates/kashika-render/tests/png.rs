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
