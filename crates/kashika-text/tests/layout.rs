use kashika_geom::Rect;
use kashika_text::{HAlign, VAlign, measure, stack, stack_size};

#[test]
fn place_at_centers_vertically() {
    let b = measure("API", 14.0, false).unwrap();
    let (lines, r) = b.place_at(100.0, 50.0, HAlign::Center);
    assert!((r.x - (100.0 - b.width / 2.0)).abs() < 1e-9);
    assert_eq!(r.y, 41.25);
    assert_eq!(r.h, 17.5);
    assert!(
        (lines[0].baseline - (41.25 + 0.98876953125 * 14.0)).abs() < 1e-9,
        "{}",
        lines[0].baseline
    );
}

#[test]
fn place_at_left_and_right() {
    let b = measure("API", 14.0, false).unwrap();
    assert_eq!(b.place_at(10.0, 0.0, HAlign::Left).1.x, 10.0);
    let r = b.place_at(10.0, 0.0, HAlign::Right).1;
    assert!((r.right() - 10.0).abs() < 1e-9);
}

#[test]
fn multi_line_blocks_align_each_line() {
    let b = measure("Postgres\nAPI", 14.0, false).unwrap();
    let (lines, r) = b.place(0.0, 100.0, 0.0, HAlign::Right);
    assert!((lines[0].x + lines[0].width - 100.0).abs() < 1e-9);
    assert!((lines[1].x + lines[1].width - 100.0).abs() < 1e-9);
    assert_eq!(lines[1].baseline - lines[0].baseline, 17.5);
    assert!((r.w - b.width).abs() < 1e-9);
    assert_eq!(r.h, 35.0);
}

#[test]
fn stack_aligns_items_in_the_area() {
    let a = measure("Payments API", 14.0, false).unwrap();
    let s = measure("v2.4.1", 11.2, false).unwrap();
    let area = Rect::new(8.0, 8.0, 184.0, 104.0);
    let out = stack(
        &[(&a, HAlign::Left), (&s, HAlign::Right)],
        area,
        VAlign::Middle,
        2.0,
    );
    let h = 17.5 + 14.0 + 2.0;
    assert_eq!(stack_size(&[&a, &s], 2.0).1, h);
    assert_eq!(out[0].1.y, 8.0 + (104.0 - h) / 2.0);
    assert_eq!(out[0].1.x, 8.0);
    assert_eq!(out[1].1.y, out[0].1.y + 17.5 + 2.0);
    assert!((out[1].1.right() - 192.0).abs() < 1e-9);
    let top = stack(&[(&a, HAlign::Center)], area, VAlign::Top, 2.0);
    assert_eq!(top[0].1.y, 8.0);
    let bottom = stack(&[(&a, HAlign::Center)], area, VAlign::Bottom, 2.0);
    assert_eq!(bottom[0].1.bottom(), 112.0);
}
