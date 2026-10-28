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

const LABELED: &str = "diagram \"Label\" size (300,100)\n\
line l: (20,50) -> (280,50) label=\"passive open\" stroke=#dc2626 stroke-width=3\n";

#[test]
fn line_label_masks_the_line_between_words() {
    let (scene, diags) = compile(LABELED);
    assert!(diags.is_empty(), "{diags:#?}");
    let rect = scene.lines[0].labels[0].rect;
    let svg = render_svg(&scene);
    let r = |v: f64| (v * 100.0).round() / 100.0;
    let mask = format!(
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"#ffffff\"/>",
        r(rect.x),
        r(rect.y),
        r(rect.w),
        r(rect.h)
    );
    assert!(svg.contains(&mask), "no label mask in:\n{svg}");

    let pix = resvg::tiny_skia::Pixmap::decode_png(&svg_to_png(&svg).unwrap()).unwrap();
    let is_line = |x: u32, y: u32| {
        let p = pix.pixel(x, y).unwrap();
        p.red() > 150 && p.green() < 100
    };
    assert!(is_line(25, 50), "line itself is not drawn");
    let (x0, x1) = (
        rect.x.ceil() as u32 + 1,
        (rect.x + rect.w).floor() as u32 - 1,
    );
    for x in x0..=x1 {
        assert!(!is_line(x, 50), "line shows through the label at x={x}");
    }
}

const TRANSPARENT: &str = "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"4\" height=\"2\">\
<rect width=\"2\" height=\"2\" fill=\"#ff0000\"/></svg>";

#[test]
fn webp_keeps_alpha() {
    let webp = kashika_render::svg_to_webp(TRANSPARENT).unwrap();
    let mut dec = image_webp::WebPDecoder::new(std::io::Cursor::new(webp)).unwrap();
    assert!(dec.has_alpha());
    let mut buf = vec![0; dec.output_buffer_size().unwrap()];
    dec.read_image(&mut buf).unwrap();
    assert_eq!(&buf[..4], [255, 0, 0, 255]);
    assert_eq!(&buf[8..12], [0, 0, 0, 0]);
}

#[test]
fn jpeg_composites_transparency_onto_white() {
    let empty = "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"4\" height=\"2\"/>";
    let white = "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"4\" height=\"2\">\
<rect width=\"4\" height=\"2\" fill=\"#ffffff\"/></svg>";
    let a = kashika_render::svg_to_jpeg(empty).unwrap();
    assert_eq!(&a[..3], b"\xFF\xD8\xFF");
    assert_eq!(a, kashika_render::svg_to_jpeg(white).unwrap());
}
