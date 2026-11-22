use std::collections::HashMap;
use std::rc::Rc;

use brul_gui::{Atlas, Revision, Texture, TextureFrame};

pub(super) struct Textures {
    atlas: Option<AtlasTexture>,
    images: HashMap<usize, ImageTexture>,
    frame_index: u64,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
}

struct AtlasTexture {
    texture: wgpu::Texture,
    width: u32,
    height: u32,
    revision: u64,
    bind_group: Option<wgpu::BindGroup>,
}

struct ImageTexture {
    source: Rc<Texture>,
    texture: wgpu::Texture,
    bind_group: Option<wgpu::BindGroup>,
    revision: Option<Revision>,
    last_frame: u64,
}

pub(super) struct Resources<'a> {
    pub(super) device: &'a wgpu::Device,
    pub(super) queue: &'a wgpu::Queue,
    pub(super) viewport: &'a wgpu::Buffer,
    pub(super) clips: &'a wgpu::Buffer,
    pub(super) instances: &'a wgpu::Buffer,
}

impl Textures {
    pub(super) fn new(layout: wgpu::BindGroupLayout, sampler: wgpu::Sampler) -> Self {
        Self {
            atlas: None,
            images: HashMap::new(),
            frame_index: 0,
            layout,
            sampler,
        }
    }

    pub(super) fn tick(&mut self) {
        self.frame_index += 1;
        self.images
            .retain(|_, cache| cache.last_frame + 120 >= self.frame_index);
        if self.images.len() > 256 {
            let mut oldest: Vec<_> = self
                .images
                .iter()
                .map(|(&key, cache)| (cache.last_frame, key))
                .collect();
            oldest.sort_unstable();
            for (_, key) in oldest.into_iter().take(self.images.len() - 256) {
                self.images.remove(&key);
            }
        }
    }

    pub(super) fn invalidate(&mut self) {
        if let Some(cache) = &mut self.atlas {
            cache.bind_group = None;
        }
        for cache in self.images.values_mut() {
            cache.bind_group = None;
        }
    }

    pub(super) fn atlas(&mut self, atlas: &Atlas, res: &Resources<'_>) -> &wgpu::BindGroup {
        if self
            .atlas
            .as_ref()
            .is_none_or(|cache| cache.width != atlas.width() || cache.height != atlas.height())
        {
            self.atlas = Some(AtlasTexture {
                texture: create_texture(res.device, atlas.width(), atlas.height()),
                width: atlas.width(),
                height: atlas.height(),
                revision: u64::MAX,
                bind_group: None,
            });
        }
        let cache = self.atlas.as_mut().expect("atlas texture");
        if cache.revision != atlas.revision() {
            upload_texture(
                res.queue,
                &cache.texture,
                cache.width,
                cache.height,
                atlas.pixels(),
            );
            cache.revision = atlas.revision();
        }
        if cache.bind_group.is_none() {
            cache.bind_group = Some(quad_bind_group(
                res.device,
                &self.layout,
                res.viewport,
                res.clips,
                res.instances,
                &self.sampler,
                &cache.texture,
            ));
        }
        cache.bind_group.as_ref().expect("atlas bind group")
    }

    pub(super) fn image(&mut self, source: &Rc<Texture>, res: &Resources<'_>) -> &wgpu::BindGroup {
        let key = Rc::as_ptr(source) as usize;
        let frame = self.frame_index;
        let revision = source.revision();
        let cache = self.images.entry(key).or_insert_with(|| {
            let texture = create_texture(res.device, source.width(), source.height());
            ImageTexture {
                source: Rc::clone(source),
                texture,
                bind_group: None,
                revision: None,
                last_frame: frame,
            }
        });
        if cache.revision != Some(revision) {
            let source = Rc::clone(&cache.source);
            let pixels = upload_pixels(&source, source.width(), source.height());
            upload_texture(
                res.queue,
                &cache.texture,
                source.width(),
                source.height(),
                &pixels,
            );
            cache.revision = Some(revision);
        }
        cache.last_frame = frame;
        if cache.bind_group.is_none() {
            cache.bind_group = Some(quad_bind_group(
                res.device,
                &self.layout,
                res.viewport,
                res.clips,
                res.instances,
                &self.sampler,
                &cache.texture,
            ));
        }
        cache.bind_group.as_ref().expect("image bind group")
    }
}

fn upload_pixels(source: &Texture, width: u32, height: u32) -> Vec<u8> {
    match source.frame() {
        TextureFrame::Rgba(pixels) => pixels.to_vec(),
        TextureFrame::Yuv420 { y, u, v, strides } => {
            let [y_stride, u_stride, v_stride] = strides;
            let width = width as usize;
            let height = height as usize;
            let mut packed = vec![0u8; width * height * 4];
            for row in 0..height {
                for column in 0..width {
                    let destination = (row * width + column) * 4;
                    let chroma = (row / 2) * u_stride + column / 2;
                    packed[destination] = y[row * y_stride + column];
                    packed[destination + 1] = u[chroma];
                    packed[destination + 2] = v[(row / 2) * v_stride + column / 2];
                    packed[destination + 3] = 255;
                }
            }
            packed
        }
    }
}

fn create_texture(device: &wgpu::Device, width: u32, height: u32) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("brul-quad-texture"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    })
}

fn upload_texture(
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    width: u32,
    height: u32,
    pixels: &[u8],
) {
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        pixels,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(width * 4),
            rows_per_image: Some(height),
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
}

fn quad_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    viewport: &wgpu::Buffer,
    clips: &wgpu::Buffer,
    instances: &wgpu::Buffer,
    sampler: &wgpu::Sampler,
    texture: &wgpu::Texture,
) -> wgpu::BindGroup {
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("brul-quad-bind-group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: viewport,
                    offset: 0,
                    size: None,
                }),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: clips,
                    offset: 0,
                    size: None,
                }),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: instances,
                    offset: 0,
                    size: None,
                }),
            },
            wgpu::BindGroupEntry {
                binding: 5,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 6,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use brul_gui::ExternalTexture;

    struct Yuv;

    impl ExternalTexture for Yuv {
        fn revision(&self) -> Revision {
            Revision::START
        }

        fn frame(&self) -> TextureFrame<'_> {
            TextureFrame::Yuv420 {
                y: &[81, 82, 83, 84, 85, 86, 87, 88],
                u: &[128, 129],
                v: &[64, 65],
                strides: [4, 2, 2],
            }
        }
    }

    #[test]
    fn rgba_pixels_are_copied_for_upload() {
        let texture = Texture::from_rgba(1, 1, &[1, 2, 3, 4]).expect("texture");
        assert_eq!(upload_pixels(&texture, 1, 1), vec![1, 2, 3, 4]);
    }

    #[test]
    fn yuv_planes_are_packed_into_rgba_for_upload() {
        let texture = Texture::external(4, 2, Rc::new(Yuv)).expect("external texture");
        assert_eq!(
            upload_pixels(&texture, 4, 2),
            vec![
                81, 128, 64, 255, 82, 128, 64, 255, 83, 129, 65, 255, 84, 129, 65, 255, 85, 128,
                64, 255, 86, 128, 64, 255, 87, 129, 65, 255, 88, 129, 65, 255,
            ]
        );
    }
}
