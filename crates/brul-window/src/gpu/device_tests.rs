use super::*;
use std::future::ready;
use std::task::Poll;
use wgpu::{CompositeAlphaMode, TextureFormat, TextureUsages};

fn capabilities(alpha_modes: Vec<CompositeAlphaMode>) -> wgpu::SurfaceCapabilities {
    wgpu::SurfaceCapabilities {
        formats: vec![TextureFormat::Rgba8UnormSrgb],
        format_capabilities: Vec::new(),
        present_modes: Vec::new(),
        alpha_modes,
        usages: TextureUsages::RENDER_ATTACHMENT,
    }
}

#[test]
fn a_transparent_window_clears_to_transparent_black() {
    assert_eq!(
        clear_color(true),
        wgpu::Color {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 0.0,
        }
    );
}

#[test]
fn an_opaque_window_keeps_the_regular_clear_color() {
    assert_eq!(clear_color(false), crate::window::CLEAR);
}

#[test]
fn an_srgb_surface_format_becomes_its_unorm_pair() {
    assert_eq!(
        unorm_format(wgpu::TextureFormat::Bgra8UnormSrgb),
        wgpu::TextureFormat::Bgra8Unorm
    );
    assert_eq!(
        unorm_format(wgpu::TextureFormat::Rgba8UnormSrgb),
        wgpu::TextureFormat::Rgba8Unorm
    );
    assert_eq!(
        unorm_format(wgpu::TextureFormat::Rgba16Float),
        wgpu::TextureFormat::Rgba16Float
    );
}

#[test]
fn a_size_change_needs_a_size_that_differs() {
    assert_eq!(size_change((8, 6), 8, 6), None);
    assert_eq!(size_change((8, 6), 9, 6), Some((9, 6)));
    assert_eq!(size_change((8, 6), 8, 7), Some((8, 7)));
}

#[test]
fn an_empty_size_is_not_a_size_change() {
    assert_eq!(size_change((8, 6), 0, 6), None);
    assert_eq!(size_change((8, 6), 8, 0), None);
}

#[test]
fn the_first_preferred_alpha_mode_wins() {
    let modes = vec![
        CompositeAlphaMode::Inherit,
        CompositeAlphaMode::PostMultiplied,
        CompositeAlphaMode::PreMultiplied,
    ];
    assert_eq!(
        alpha_mode(&capabilities(modes)),
        CompositeAlphaMode::PreMultiplied
    );
    assert_eq!(
        alpha_mode(&capabilities(vec![
            CompositeAlphaMode::Inherit,
            CompositeAlphaMode::PostMultiplied,
        ])),
        CompositeAlphaMode::PostMultiplied
    );
}

#[test]
fn an_unexpected_set_of_alpha_modes_falls_back_to_the_first() {
    assert_eq!(
        alpha_mode(&capabilities(vec![CompositeAlphaMode::Opaque])),
        CompositeAlphaMode::Opaque
    );
    assert_eq!(
        alpha_mode(&capabilities(Vec::new())),
        CompositeAlphaMode::Auto
    );
}

struct PendingOnce(bool);

impl Future for PendingOnce {
    type Output = u8;

    fn poll(mut self: std::pin::Pin<&mut Self>, context: &mut Context<'_>) -> Poll<u8> {
        if self.0 {
            return Poll::Ready(7);
        }
        self.0 = true;
        context.waker().wake_by_ref();
        Poll::Pending
    }
}

#[test]
fn block_on_waits_for_a_future_that_is_ready_at_once() {
    assert_eq!(block_on(ready(7)), 7);
}

#[test]
fn block_on_polls_again_after_pending() {
    assert_eq!(block_on(PendingOnce(false)), 7);
}
