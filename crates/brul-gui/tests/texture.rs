use brul_gui::{Color, Texture, TextureError};

const PNG: &[u8] = include_bytes!("fixtures/tint.png");

#[test]
fn decodes_png_to_rgba() {
    let texture = Texture::from_png(PNG).expect("decode png");

    assert_eq!(texture.width(), 2);
    assert_eq!(texture.height(), 2);
    assert_eq!(texture.pixels().len(), 16);
    assert_eq!(texture.color_at(0, 0), Some(Color::rgba(255, 0, 0, 255)));
    assert_eq!(texture.color_at(1, 0), Some(Color::rgba(0, 255, 0, 255)));
    assert_eq!(texture.color_at(0, 1), Some(Color::rgba(0, 0, 255, 128)));
    assert_eq!(texture.color_at(1, 1), Some(Color::rgba(255, 255, 255, 0)));
}

#[test]
fn tint_multiplies_rgba_without_touching_raw_pixels() {
    let texture = Texture::from_rgba(1, 1, &[100, 50, 25, 200])
        .expect("texture")
        .with_tint(Color::rgba(255, 128, 255, 128));

    assert_eq!(texture.tint(), Some(Color::rgba(255, 128, 255, 128)));
    assert_eq!(texture.color_at(0, 0), Some(Color::rgba(100, 25, 25, 100)));
    assert_eq!(texture.pixels(), &[100, 50, 25, 200]);
}

#[test]
fn tint_can_be_cleared() {
    let texture = Texture::from_rgba(1, 1, &[100, 50, 25, 200])
        .expect("texture")
        .with_tint(Color::WHITE)
        .without_tint();

    assert_eq!(texture.tint(), None);
    assert_eq!(texture.color_at(0, 0), Some(Color::rgba(100, 50, 25, 200)));
}

#[test]
fn invalid_png_is_rejected() {
    assert!(matches!(
        Texture::from_png(b"not a png"),
        Err(TextureError::Decode(_))
    ));
}

#[test]
fn invalid_rgba_is_rejected() {
    assert!(matches!(
        Texture::from_rgba(2, 2, &[0; 3]),
        Err(TextureError::InvalidRgba)
    ));
    assert!(matches!(
        Texture::from_rgba(0, 1, &[]),
        Err(TextureError::InvalidRgba)
    ));
}

#[test]
fn out_of_bounds_pixel_is_none() {
    let texture = Texture::from_rgba(1, 1, &[1, 2, 3, 4]).expect("texture");

    assert_eq!(texture.color_at(1, 0), None);
    assert_eq!(texture.color_at(0, 1), None);
}
