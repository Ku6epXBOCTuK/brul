use brul_gui::{Color, Paint, Rect};

const RED: Color = Color::rgb(255, 0, 0);
const BLUE: Color = Color::rgb(0, 0, 255);

#[test]
fn square_fill_covers_inside_only() {
    let rect = Rect::new(10, 20, 4, 3).with_fill(Paint::Color(RED));

    assert_eq!(rect.sample(10, 20).fill_coverage, 1.0);
    assert_eq!(rect.sample(13, 22).fill_coverage, 1.0);
    assert_eq!(rect.sample(9, 20).fill_coverage, 0.0);
    assert_eq!(rect.sample(14, 20).fill_coverage, 0.0);
    assert_eq!(rect.sample(10, 23).fill_coverage, 0.0);
}

#[test]
fn rounded_corners_have_fractional_coverage() {
    let rect = Rect::new(0, 0, 16, 16)
        .with_radius(8)
        .with_fill(Paint::Color(RED));

    assert_eq!(rect.sample(8, 8).fill_coverage, 1.0);
    assert_eq!(rect.sample(0, 0).fill_coverage, 0.0);
    let corner = rect.sample(2, 2).fill_coverage;
    assert!(corner > 0.0 && corner < 1.0);
}

#[test]
fn radius_is_clamped_to_half_size() {
    let clamped = Rect::new(0, 0, 10, 10)
        .with_radius(5)
        .with_fill(Paint::Color(RED));
    let oversized = Rect::new(0, 0, 10, 10)
        .with_radius(100)
        .with_fill(Paint::Color(RED));

    assert_eq!(oversized.sample(0, 0), clamped.sample(0, 0));
    assert_eq!(oversized.sample(5, 5), clamped.sample(5, 5));
}

#[test]
fn border_sits_inside_the_edges() {
    let rect = Rect::new(0, 0, 10, 10)
        .with_fill(Paint::Color(RED))
        .with_border(2, BLUE);

    assert_eq!(rect.sample(0, 5).border_coverage, 1.0);
    assert_eq!(rect.sample(1, 5).border_coverage, 1.0);
    assert_eq!(rect.sample(2, 5).fill_coverage, 1.0);
    assert_eq!(rect.sample(5, 5).fill_coverage, 1.0);
}

#[test]
fn wide_border_leaves_no_fill() {
    let rect = Rect::new(0, 0, 6, 6)
        .with_fill(Paint::Color(RED))
        .with_border(4, BLUE);

    assert_eq!(rect.sample(0, 0).border_coverage, 1.0);
    assert_eq!(rect.sample(3, 3).border_coverage, 1.0);
    assert_eq!(rect.sample(3, 3).fill_coverage, 0.0);
}

#[test]
fn zero_size_is_empty() {
    assert_eq!(Rect::new(0, 0, 0, 5).sample(0, 0).fill_coverage, 0.0);
    assert_eq!(Rect::new(0, 0, 5, 0).sample(0, 0).fill_coverage, 0.0);
}
