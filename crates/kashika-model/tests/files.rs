use std::path::Path;

use kashika_diag::Diagnostic;
use kashika_geom::{Pt, Rect};
use kashika_model::{compile_with, EdgeKind, Fit, Options, Scene, Shape, ShapeKind};

const HEAD: &str = "diagram \"t\" size (800,600)\n";

const SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="40" height="20" viewBox="0 0 40 20"><path d="M 10 0 L 30 0 L 40 20 L 0 20 Z" fill="#123456"/></svg>"##;

/// A PNG header with the given size; enough for `imagesize`.
fn png(w: u32, h: u32) -> Vec<u8> {
    let mut b = vec![
        0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 13, b'I', b'H', b'D', b'R',
    ];
    b.extend(w.to_be_bytes());
    b.extend(h.to_be_bytes());
    b.extend([8, 6, 0, 0, 0, 0, 0, 0, 0]);
    b
}

fn shape<'a>(s: &'a Scene, id: &str) -> &'a Shape {
    s.shapes.iter().find(|x| x.id == id).unwrap()
}

fn start(s: &Scene, id: &str) -> Pt {
    s.lines.iter().find(|l| l.id == id).unwrap().path[0].from()
}

fn close(a: Pt, b: Pt) -> bool {
    a.dist(b) <= 0.3
}

fn write(dir: &Path, name: &str, body: &[u8]) {
    let p = dir.join(name);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, body).unwrap();
}

fn opts(dir: &Path) -> Options {
    Options {
        path: Some(dir.join("main.kash")),
        ..Options::default()
    }
}

fn run_with(src: &str, o: &Options) -> (Scene, Vec<Diagnostic>) {
    compile_with(src, o)
}

/// Writes `src` as `main.kash` in `dir` (so the main file's path exists) and compiles it.
fn run(dir: &Path, src: &str) -> (Scene, Vec<Diagnostic>) {
    write(dir, "main.kash", src.as_bytes());
    run_with(src, &opts(dir))
}

#[test]
fn icons_dir_must_exist() {
    let dir = tempfile::tempdir().unwrap();
    let (_, d) = run(dir.path(), "diagram \"t\" size (800,600) icons=\"nope\"\n");
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(d[0].code, "file");
    assert!(d[0].message.contains("`nope`"), "{}", d[0].message);
}

#[test]
fn icons_dir_inside_the_project_is_accepted() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("assets/icons")).unwrap();
    let (_, d) = run(
        dir.path(),
        "diagram \"t\" size (800,600) icons=\"assets/icons\"\n",
    );
    assert!(d.is_empty(), "{d:#?}");
}

#[test]
fn icons_dir_outside_the_project_needs_allow_path() {
    let tmp = tempfile::tempdir().unwrap();
    let proj = tmp.path().join("proj");
    std::fs::create_dir_all(tmp.path().join("shared")).unwrap();
    std::fs::create_dir_all(&proj).unwrap();
    let src = "diagram \"t\" size (800,600) icons=\"../shared\"\n";
    let (_, d) = run(&proj, src);
    assert_eq!(d[0].code, "file");
    assert!(
        d[0].message.contains("outside the project root"),
        "{}",
        d[0].message
    );
    let (_, d) = run_with(
        src,
        &Options {
            allow_path: true,
            ..opts(&proj)
        },
    );
    assert!(d.is_empty(), "{d:#?}");
}

#[test]
fn icons_attr_must_be_a_string() {
    let dir = tempfile::tempdir().unwrap();
    let (_, d) = run(dir.path(), "diagram \"t\" size (800,600) icons=12\n");
    assert_eq!(d[0].code, "attr");
}

#[test]
fn svg_shapes_use_bbox_or_hull() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "t.svg", SVG.as_bytes());
    let (s, d) = run(
        dir.path(),
        &format!(
            "{HEAD}shape tb from \"t.svg\"\nshape th from \"t.svg\" outline=hull text-area=below\n\
             box a \"A\" at (100,100) size (80,40) shape=tb\nbox b \"B\" at (300,100) size (80,40) shape=th\n\
             line la a.top@0.1 -> v -20\nline lb b.top@0.1 -> v -20\n"
        ),
    );
    assert!(d.is_empty(), "{d:#?}");
    assert_eq!(start(&s, "la"), Pt::new(108.0, 100.0));
    assert!(
        close(start(&s, "lb"), Pt::new(308.0, 124.0)),
        "{:?}",
        start(&s, "lb")
    );
    let a = shape(&s, "a");
    assert_eq!(a.image.as_ref().unwrap().fit, Fit::Stretch);
    assert_eq!(
        a.image.as_ref().unwrap().rect,
        Rect::new(100.0, 100.0, 80.0, 40.0)
    );
    assert_eq!(a.text_area, Rect::new(108.0, 108.0, 64.0, 24.0));
    assert_eq!(
        shape(&s, "b").text_area,
        Rect::new(300.0, 148.0, 80.0, 17.5)
    );
    assert_eq!(s.assets.len(), 1, "both shapes share one asset");
    assert_eq!(s.assets[0].natural, (40.0, 20.0));
    assert_eq!(s.assets[0].path, "t.svg");
}

#[test]
fn svg_shape_file_errors() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "icons/s3.svg", SVG.as_bytes());
    write(dir.path(), "x.png", &png(4, 4));
    let (_, d) = run(
        dir.path(),
        &format!("{HEAD}shape a from \"icons/s4.svg\"\n"),
    );
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(d[0].code, "file");
    assert_eq!(d[0].span.line, 2);
    assert_eq!(d[0].help, vec!["did you mean `icons/s3.svg`?".to_string()]);
    let (_, d) = run(dir.path(), &format!("{HEAD}shape b from \"x.png\"\n"));
    assert_eq!(d[0].code, "file");
    assert!(d[0].message.contains("needs an SVG"), "{}", d[0].message);
    let (_, d) = run(
        dir.path(),
        &format!("{HEAD}shape c from \"icons/s3.svg\" outline=circle\n"),
    );
    assert_eq!(d[0].code, "attr");
}

#[test]
fn bundled_icons_work_as_shapes() {
    let dir = tempfile::tempdir().unwrap();
    let (s, d) = run(
        dir.path(),
        &format!("{HEAD}shape srv from \"server\"\nbox a at (0,0) size (48,48) shape=srv\n"),
    );
    assert!(d.is_empty(), "{d:#?}");
    assert_eq!(s.assets[0].path, "builtin:server");
}

const TRI: &str = "shape lb {\n  path \"M 0.5 0 L 1 1 L 0 1 Z\"\n}\n";

#[test]
fn imports_bring_in_shapes_and_styles() {
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        "lib/net.kash",
        format!("style edge {{ fill=#ff0000 }}\n{TRI}import \"inner.kash\" as in\n").as_bytes(),
    );
    write(
        dir.path(),
        "lib/inner.kash",
        b"shape dot {\n  path \"M 0 0 L 1 0 L 1 1 Z\"\n}\n",
    );
    write(
        dir.path(),
        "plain.kash",
        b"style svc2 { stroke=#00ff00 }\nstyle service { fill=#000000 }\n",
    );
    let (s, d) = run(
        dir.path(),
        &format!(
            "{HEAD}import \"lib/net.kash\" as net\nimport \"plain.kash\"\nimport \"plain.kash\"\n\
             box a at (0,0) size (10,10) shape=net.lb style=net.edge\n\
             box b at (20,0) size (10,10) shape=net.in.dot style=svc2\n\
             box c at (40,0) size (10,10) style=service\n"
        ),
    );
    assert!(d.is_empty(), "{d:#?}");
    assert_eq!(shape(&s, "a").shape, "net.lb");
    assert_eq!(shape(&s, "a").style.fill, "#ff0000");
    assert_eq!(shape(&s, "b").shape, "net.in.dot");
    assert_eq!(shape(&s, "b").style.stroke, "#00ff00");
    assert_eq!(
        shape(&s, "c").style.fill,
        "#000000",
        "imported style overrides the built-in"
    );
    assert_eq!(shape(&s, "c").style.stroke, "#1a73e8", "key by key");
}

#[test]
fn library_errors_point_at_the_import() {
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        "lib/bad.kash",
        b"box n at (0,0) size (1,1)\nstyle s { fil=#fff }\n",
    );
    let (s, d) = run(dir.path(), &format!("{HEAD}import \"lib/bad.kash\"\n"));
    let mut got: Vec<_> = d
        .iter()
        .map(|x| (x.code, x.span.line, x.message.clone()))
        .collect();
    got.sort();
    assert_eq!(got.len(), 2, "{d:#?}");
    assert_eq!(got[0].0, "attr");
    assert_eq!(got[0].1, 2);
    assert!(
        got[0].2.starts_with("in `lib/bad.kash` at 2:11: "),
        "{}",
        got[0].2
    );
    assert_eq!(got[1].0, "parse");
    assert!(
        got[1]
            .2
            .starts_with("in `lib/bad.kash` at 1:1: `box` is not allowed in a library; library files may contain only"),
        "{}",
        got[1].2
    );
    assert!(s.shapes.is_empty());
    assert!(s.literals.is_empty(), "{:?}", s.literals);
}

#[test]
fn import_cycles_are_errors() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "a.kash", b"import \"b.kash\"\n");
    write(dir.path(), "b.kash", b"import \"a.kash\"\n");
    let (_, d) = run(dir.path(), &format!("{HEAD}import \"a.kash\"\n"));
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(d[0].code, "file");
    assert_eq!((d[0].span.line, d[0].span.col), (2, 8));
    assert_eq!(
        d[0].message,
        "in `a.kash` at 1:8: in `b.kash` at 1:8: import cycle: a.kash -> b.kash -> a.kash"
    );
}

#[test]
fn importing_the_main_file_is_a_cycle() {
    let dir = tempfile::tempdir().unwrap();
    let (_, d) = run(dir.path(), &format!("{HEAD}import \"main.kash\"\n"));
    assert_eq!(d[0].code, "file");
    assert!(
        d[0].message
            .contains("import cycle: main.kash -> main.kash"),
        "{}",
        d[0].message
    );
}

#[test]
fn import_rules() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "plain.kash", b"style s { fill=#000000 }\n");
    write(dir.path(), "x.svg", SVG.as_bytes());
    for (body, code, needle) in [
        (
            "box a at (0,0) size (1,1)\nimport \"plain.kash\"\n",
            "parse",
            "must come right after `diagram`",
        ),
        ("import \"plan.kash\"\n", "file", "not found"),
        ("import \"x.svg\"\n", "file", "`.kash`"),
        ("import \"server\"\n", "file", "bundled icon"),
        (
            "import \"https://x.dev/a.kash\"\n",
            "file",
            "not a local file",
        ),
        (
            "group g at (0,0) size (9,9) {\n  import \"plain.kash\"\n}\n",
            "parse",
            "only allowed at the top level",
        ),
        (
            "import \"plain.kash\"\nstyle s { fill=#ffffff }\n",
            "duplicate-style",
            "defined twice",
        ),
    ] {
        let (_, d) = run(dir.path(), &format!("{HEAD}{body}"));
        assert_eq!(d.len(), 1, "{body}: {d:#?}");
        assert_eq!(d[0].code, code, "{body}");
        assert!(d[0].message.contains(needle), "{body}: {}", d[0].message);
    }
    let (_, d) = run(dir.path(), &format!("{HEAD}import \"plan.kash\"\n"));
    assert_eq!(d[0].help, vec!["did you mean `plain.kash`?".to_string()]);
}

#[test]
fn styles_from_two_imports_clash() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "one.kash", b"style s { fill=#000000 }\n");
    write(dir.path(), "two.kash", b"style s { fill=#ffffff }\n");
    let (_, d) = run(
        dir.path(),
        &format!("{HEAD}import \"one.kash\"\nimport \"two.kash\"\n"),
    );
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(d[0].code, "duplicate-style");
    assert_eq!(d[0].span.line, 3);
    assert_eq!(
        d[0].message,
        "in `two.kash` at 1:1: style `s` is defined twice"
    );
    let (_, d) = run(
        dir.path(),
        &format!("{HEAD}import \"one.kash\" as a\nimport \"two.kash\" as b\n"),
    );
    assert!(d.is_empty(), "{d:#?}");
}

#[test]
fn imported_shapes_clash_by_qualified_name() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "tri.kash", TRI.as_bytes());
    let (_, d) = run(dir.path(), &format!("{HEAD}import \"tri.kash\"\n{TRI}"));
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(d[0].code, "duplicate-style");
    let (_, d) = run(
        dir.path(),
        &format!("{HEAD}import \"tri.kash\" as t\n{TRI}box a at (0,0) size (9,9) shape=t.lb\n"),
    );
    assert!(d.is_empty(), "{d:#?}");
}

#[test]
fn imports_outside_the_root_need_allow_path() {
    let tmp = tempfile::tempdir().unwrap();
    let proj = tmp.path().join("proj");
    std::fs::create_dir_all(&proj).unwrap();
    write(tmp.path(), "shared.kash", TRI.as_bytes());
    let src = format!("{HEAD}import \"../shared.kash\"\nbox a at (0,0) size (9,9) shape=lb\n");
    let (_, d) = run(&proj, &src);
    assert!(d.iter().any(|x| x.code == "file"), "{d:#?}");
    let (_, d) = run_with(
        &src,
        &Options {
            allow_path: true,
            ..opts(&proj)
        },
    );
    assert!(d.is_empty(), "{d:#?}");
}

#[test]
fn deep_import_chains_stop() {
    let dir = tempfile::tempdir().unwrap();
    for i in 0..40 {
        write(
            dir.path(),
            &format!("l{i}.kash"),
            format!("import \"l{}.kash\"\n", i + 1).as_bytes(),
        );
    }
    write(dir.path(), "l40.kash", b"");
    let (_, d) = run(dir.path(), &format!("{HEAD}import \"l0.kash\"\n"));
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(d[0].code, "file");
    assert_eq!(
        d[0].message,
        "in `l0.kash` at 1:8: in `l1.kash` at 1:8: … 29 more imports …: in `l31.kash` at 1:8: imports are nested more than 32 deep"
    );
}

#[test]
fn library_files_check_on_their_own() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "x.svg", SVG.as_bytes());
    write(dir.path(), "inner.kash", b"style s { fil=#fff }\n");
    let src = "import \"inner.kash\"\nimport \"gone.kash\"\nshape t from \"y.svg\"\nshape u from \"x.svg\"\nstyle a { fill=#fff stroke=nope }\nstyle a { fill=#000 }\n";
    write(dir.path(), "main.kash", src.as_bytes());
    let d = kashika_model::compile_library_with(src, &opts(dir.path())).expect("a library");
    let got: Vec<_> = d
        .iter()
        .map(|x| (x.code, x.span.line, x.message.as_str()))
        .collect();
    assert_eq!(
        got,
        vec![
            (
                "attr",
                1,
                "in `inner.kash` at 1:11: unknown attribute `fil` on style"
            ),
            ("file", 2, "file `gone.kash` not found"),
            ("file", 3, "file `y.svg` not found"),
            (
                "attr",
                5,
                "`stroke` expects a color like `#1a73e8`, a CSS color name or `none`"
            ),
            ("duplicate-style", 6, "style `a` is defined twice"),
        ],
        "{d:#?}"
    );
    for src in [
        "",
        "box a at (0,0) size (1,1)\n",
        "style a { fill=#fff }\nbox a at (0,0) size (1,1)\n",
        "diagram \"t\" size (10,10)\nstyle a { fill=#fff }\n",
    ] {
        assert!(
            kashika_model::compile_library_with(src, &opts(dir.path())).is_none(),
            "{src:?}"
        );
    }
}

#[test]
fn svg_shapes_in_libraries_resolve_from_the_library() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "lib/icons/t.svg", SVG.as_bytes());
    write(
        dir.path(),
        "lib/aws.kash",
        b"shape t from \"icons/t.svg\"\n",
    );
    let (s, d) = run(
        dir.path(),
        &format!("{HEAD}import \"lib/aws.kash\" as aws\nbox a at (0,0) size (40,20) shape=aws.t\n"),
    );
    assert!(d.is_empty(), "{d:#?}");
    assert_eq!(s.assets.len(), 1);
    assert!(shape(&s, "a").image.is_some());
}

#[test]
fn images_resolve_with_their_natural_size() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "assets/logo.png", &png(200, 100));
    let (s, d) = run(
        dir.path(),
        &format!(
            "{HEAD}image logo \"assets/logo.png\" at (20,20) size (80,80) fit=cover\n\
             image mark \"assets/logo.png\" at logo.right+(20,0) size (40,40) pin=left\n\
             line l logo.right -> mark.left\n"
        ),
    );
    assert!(d.is_empty(), "{d:#?}");
    let logo = shape(&s, "logo");
    assert_eq!(logo.kind, ShapeKind::Image);
    assert_eq!(logo.shape, "image");
    assert_eq!(logo.rect, Rect::new(20.0, 20.0, 80.0, 80.0));
    let img = logo.image.as_ref().unwrap();
    assert_eq!((img.fit, img.rect), (Fit::Cover, logo.rect));
    assert_eq!(shape(&s, "mark").image.as_ref().unwrap().fit, Fit::Contain);
    assert_eq!(shape(&s, "mark").rect, Rect::new(120.0, 40.0, 40.0, 40.0));
    assert_eq!(s.assets.len(), 1);
    assert_eq!(
        (s.assets[0].mime, s.assets[0].natural),
        ("image/png", (200.0, 100.0))
    );
    assert_eq!(start(&s, "l"), Pt::new(100.0, 60.0));
}

#[test]
fn stretched_images_report_their_distortion() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "logo.png", &png(200, 100));
    let (s, _) = run(
        dir.path(),
        &format!(
            "{HEAD}image a \"logo.png\" at (0,0) size (80,80) fit=stretch\n\
             image b \"logo.png\" at (100,0) size (80,40) fit=stretch\n\
             image c \"logo.png\" at (200,0) size (80,80)\n"
        ),
    );
    assert_eq!(shape(&s, "a").image_distortion(&s.assets), Some(0.5));
    assert_eq!(shape(&s, "b").image_distortion(&s.assets), Some(0.0));
    assert_eq!(shape(&s, "c").image_distortion(&s.assets), None);
}

#[test]
fn image_errors() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "logo.png", &png(2, 2));
    for (body, code) in [
        ("image a \"lgo.png\" at (0,0) size (8,8)\n", "file"),
        ("image a \"logo.png\" at (0,0) size (0,8)\n", "size"),
        (
            "image a \"logo.png\" at (0,0) size (8,8) fit=fill\n",
            "attr",
        ),
        (
            "image a \"logo.png\" at (0,0) size (8,8) label=\"x\"\n",
            "attr",
        ),
        (
            "group g at (0,0) size (9,9) {\n  image a \"logo.png\" at (0,0) size (8,8)\n}\n",
            "parse",
        ),
    ] {
        let (_, d) = run(dir.path(), &format!("{HEAD}{body}"));
        assert_eq!(
            d.iter().map(|x| x.code).collect::<Vec<_>>(),
            vec![code],
            "{body}: {d:#?}"
        );
    }
    let (s, _) = run(
        dir.path(),
        &format!("{HEAD}image a \"lgo.png\" at (0,0) size (8,8)\n"),
    );
    assert!(
        shape(&s, "a").image.is_none(),
        "geometry survives a missing file"
    );
}

#[test]
fn icons_shrink_the_text_area() {
    let dir = tempfile::tempdir().unwrap();
    let (s, d) = run(
        dir.path(),
        &format!(
            "{HEAD}box a \"A\" at (0,0) size (200,60) icon=\"server\"\n\
             box b \"B\" at (0,100) size (100,100) icon=\"server\" icon-pos=top icon-size=32\n\
             group g \"G\" at (300,0) size (200,100) icon=\"server\" {{\n}}\n"
        ),
    );
    assert!(d.is_empty(), "{d:#?}");
    let a = shape(&s, "a");
    assert_eq!(
        a.icon.as_ref().unwrap().rect,
        Rect::new(8.0, 18.0, 24.0, 24.0)
    );
    assert_eq!(a.icon.as_ref().unwrap().fit, Fit::Contain);
    assert_eq!(a.text_area, Rect::new(40.0, 8.0, 152.0, 44.0));
    let b = shape(&s, "b");
    assert_eq!(
        b.icon.as_ref().unwrap().rect,
        Rect::new(34.0, 108.0, 32.0, 32.0)
    );
    assert_eq!(b.text_area, Rect::new(8.0, 148.0, 84.0, 44.0));
    let g = shape(&s, "g");
    assert_eq!(
        g.icon.as_ref().unwrap().rect,
        Rect::new(308.0, 8.0, 17.5, 17.5)
    );
    assert_eq!(g.text_area, Rect::new(333.5, 8.0, 158.5, 84.0));
    assert_eq!(s.assets.len(), 1, "one bundled icon, loaded once");
}

#[test]
fn icon_attribute_errors() {
    let dir = tempfile::tempdir().unwrap();
    for (body, code) in [
        ("box a at (0,0) size (9,9) icon-pos=top\n", "attr"),
        (
            "box a at (0,0) size (9,9) icon=\"server\" icon-size=0\n",
            "size",
        ),
        (
            "box a at (0,0) size (9,9) icon=\"server\" icon-pos=right\n",
            "attr",
        ),
        (
            "group g at (0,0) size (9,9) icon=\"server\" icon-pos=top {\n}\n",
            "attr",
        ),
        ("box a at (0,0) size (9,9) icon=\"servr\"\n", "file"),
    ] {
        let (_, d) = run(dir.path(), &format!("{HEAD}{body}"));
        assert_eq!(
            d.iter().map(|x| x.code).collect::<Vec<_>>(),
            vec![code],
            "{body}: {d:#?}"
        );
    }
}

#[test]
fn icons_are_found_on_the_search_path() {
    let tmp = tempfile::tempdir().unwrap();
    let proj = tmp.path().join("proj");
    write(&proj, "assets/icons/db.svg", SVG.as_bytes());
    write(tmp.path(), "vendor/s3.svg", SVG.as_bytes());
    let src = "diagram \"t\" size (800,600) icons=\"assets/icons\"\n\
               box a at (0,0) size (90,40) icon=\"db.svg\"\nbox b at (0,50) size (90,40) icon=\"s3.svg\"\n";
    let (_, d) = run(&proj, src);
    assert_eq!(
        d.iter().map(|x| x.code).collect::<Vec<_>>(),
        vec!["file"],
        "{d:#?}"
    );
    let (s, d) = run_with(
        src,
        &Options {
            icon_path: vec![tmp.path().join("vendor")],
            ..opts(&proj)
        },
    );
    assert!(d.is_empty(), "{d:#?}");
    assert_eq!(
        s.assets.iter().map(|a| a.path.as_str()).collect::<Vec<_>>(),
        vec!["db.svg", "s3.svg"]
    );
}

#[test]
fn image_geometry_is_recorded_like_a_box() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "logo.png", &png(10, 10));
    let (s, d) = run(
        dir.path(),
        &format!(
            "{HEAD}box a at (0,0) size (10,10)\n\
             image i \"logo.png\" at a.right+(5,0) size (30,40) pin=left\n"
        ),
    );
    assert!(d.is_empty(), "{d:#?}");
    let lits: Vec<_> = s
        .literals
        .iter()
        .filter(|l| l.owner == "i")
        .map(|l| (l.what, l.value))
        .collect();
    assert_eq!(
        lits,
        vec![("dx", 5.0), ("dy", 0.0), ("width", 30.0), ("height", 40.0)]
    );
    assert!(s
        .placement
        .iter()
        .any(|e| e.from == "i" && e.to == "a" && e.kind == EdgeKind::At));
}

#[test]
fn images_paint_with_boxes_in_file_order() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "logo.png", &png(10, 10));
    let (s, d) = run(
        dir.path(),
        &format!(
            "{HEAD}box a at (0,0) size (10,10)\n\
             image i \"logo.png\" at (20,0) size (10,10)\n\
             box b at (40,0) size (10,10)\n"
        ),
    );
    assert!(d.is_empty(), "{d:#?}");
    let order: Vec<_> = s.shapes.iter().map(|x| x.id.as_str()).collect();
    assert_eq!(order, ["a", "i", "b"]);
}

#[test]
fn duplicate_and_unknown_image_references() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "logo.png", &png(10, 10));
    let (_, d) = run(
        dir.path(),
        &format!(
            "{HEAD}box i at (0,0) size (10,10)\n\
             image i \"logo.png\" at (20,0) size (10,10)\n"
        ),
    );
    assert_eq!(
        d.iter().map(|x| x.code).collect::<Vec<_>>(),
        vec!["duplicate-id"]
    );
}

#[test]
fn icon_overflow_hint_accounts_for_the_icon() {
    let dir = tempfile::tempdir().unwrap();
    let src = |size: &str| {
        format!("{HEAD}box a \"A rather long label\" at (0,0) size ({size}) icon=\"server\"\n")
    };
    let (_, d) = run(dir.path(), &src("100,40"));
    assert_eq!(
        d.iter().map(|x| x.code).collect::<Vec<_>>(),
        vec!["label-overflow"]
    );
    let help = &d[0].help[0];
    let size = help
        .split("size (")
        .nth(1)
        .and_then(|s| s.split(')').next())
        .unwrap();
    let (_, fixed) = run(dir.path(), &src(size));
    assert!(fixed.is_empty(), "({size}): {fixed:#?}");
    let (_, d2) = run(
        dir.path(),
        &format!("{HEAD}box a \"A rather long label\" at (0,0) size (100,40)\n"),
    );
    let plain = d2[0].help[0]
        .split("size (")
        .nth(1)
        .unwrap()
        .split(',')
        .next()
        .unwrap()
        .to_string();
    let with: f64 = size.split(',').next().unwrap().parse().unwrap();
    assert_eq!(with, plain.parse::<f64>().unwrap() + 32.0);
}

#[test]
fn images_are_anchorable_and_pinnable() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "logo.png", &png(10, 10));
    let (s, d) = run(
        dir.path(),
        &format!(
            "{HEAD}image i \"logo.png\" at (100,100) size (40,20) pin=center\n\
             line l i.top -> v -5\n"
        ),
    );
    assert!(d.is_empty(), "{d:#?}");
    assert_eq!(shape(&s, "i").rect, Rect::new(80.0, 90.0, 40.0, 20.0));
    assert_eq!(start(&s, "l"), Pt::new(100.0, 90.0));
}
