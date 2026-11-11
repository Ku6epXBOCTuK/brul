use brul_gui::{Color, Gradient, Rect};

const RED: Color = Color::rgb(255, 0, 0);
const BLUE: Color = Color::rgb(0, 0, 255);

fn linear(angle: f32) -> Gradient {
    Gradient::Linear {
        angle,
        from: RED,
        to: BLUE,
    }
}

#[test]
fn zero_degrees_runs_from_bottom_to_top() {
    let rect = Rect::new(0, 0, 4, 4);
    let gradient = linear(0.0);

    assert!(gradient.at(&rect, 0, 3).r > gradient.at(&rect, 0, 0).r);
    assert!(gradient.at(&rect, 0, 0).b > gradient.at(&rect, 0, 3).b);
}

#[test]
fn ninety_degrees_runs_left_to_right() {
    let rect = Rect::new(0, 0, 4, 4);
    let gradient = linear(90.0);

    assert!(gradient.at(&rect, 0, 0).r > gradient.at(&rect, 3, 0).r);
    assert!(gradient.at(&rect, 3, 0).b > gradient.at(&rect, 0, 0).b);
}

#[test]
fn a_diagonal_keeps_the_anti_diagonal_colors_equal() {
    let rect = Rect::new(0, 0, 4, 4);
    let gradient = linear(135.0);

    assert_eq!(gradient.at(&rect, 0, 3), gradient.at(&rect, 3, 0));
}
