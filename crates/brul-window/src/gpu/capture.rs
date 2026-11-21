use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use super::Gpu;

impl Gpu {
    pub(crate) fn capture(&mut self) -> Option<Vec<u8>> {
        self.captured.take()
    }

    pub(super) fn readback(
        &self,
        texture: &wgpu::Texture,
        width: u32,
        height: u32,
    ) -> Option<Vec<u8>> {
        let stride = (width * 4).div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
            * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("brul-frame-readback"),
            size: u64::from(stride) * u64::from(height),
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("brul-frame-readback"),
            });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(stride),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit(Some(encoder.finish()));
        let done = Arc::new(AtomicBool::new(false));
        buffer.map_async(wgpu::MapMode::Read, 0..buffer.size(), {
            let done = Arc::clone(&done);
            move |_| done.store(true, Ordering::SeqCst)
        });
        while !done.load(Ordering::SeqCst) {
            let _ = self.device.poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: None,
            });
        }
        let view = buffer.get_mapped_range(..).ok()?;
        let mut pixels = Vec::with_capacity(width as usize * height as usize * 4);
        for row in view.chunks(stride as usize).take(height as usize) {
            pixels.extend_from_slice(&row[..width as usize * 4]);
        }
        drop(view);
        buffer.unmap();
        Some(pixels)
    }
}
