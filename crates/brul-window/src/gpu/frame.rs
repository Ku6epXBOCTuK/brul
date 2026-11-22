use std::rc::Rc;

use brul_gpu::effect::{EffectFrame, Effects, effect_frame_bytes, effect_instance_bytes};
use brul_gui::{Atlas, Dirty, Texture, WidgetSnapshot};

use crate::gpu_frame::{CLIP, DECOR, GLYPH, IMAGE, SHAPE};
use crate::surface::FrameInfo;

use super::Gpu;
use super::effects::EffectRange;
use super::textures::Resources;

impl Gpu {
    pub(crate) fn clear_surface(&self, surface: &dyn crate::surface::Surface) -> bool {
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame)
            | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                self.surface.configure(&self.device, &self.config);
                surface.redraw();
                return false;
            }
            _ => return false,
        };

        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("brul-window"),
            });
        {
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("brul-clear"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(self.clear_color),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
        }

        self.queue.submit(Some(encoder.finish()));
        surface.pre_present_notify();
        self.queue.present(frame);
        true
    }

    pub(crate) fn render(
        &mut self,
        surface: &dyn crate::surface::Surface,
        snapshot: &WidgetSnapshot,
        atlas: &Atlas,
        dirty: &Dirty,
        info: FrameInfo,
        effects: &Effects,
    ) -> bool {
        let (width, height) = surface.size();
        let uploads = self.frame.update(snapshot, atlas, dirty);
        let clip_len = self.frame.buffer(CLIP).len();
        let shape_len = self.frame.buffer(SHAPE).len();
        let decor_len = self.frame.buffer(DECOR).len();
        let glyph_len = self.frame.buffer(GLYPH).len();
        let image_len = self.frame.buffer(IMAGE).len();
        let instance_len = shape_len + decor_len;
        let quad_len = glyph_len + image_len;
        let clip_grown = clip_len > self.clip_capacity;
        let instance_grown = instance_len > self.instance_capacity;
        let quad_grown = quad_len > self.quad_capacity;
        if clip_grown {
            self.clip_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("brul-clips"),
                size: clip_len as u64,
                usage: brul_gpu::instance_usage(),
                mapped_at_creation: false,
            });
            self.clip_capacity = clip_len;
            self.textures.invalidate();
        }
        if instance_grown {
            self.instance_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("brul-shape-instances"),
                size: instance_len as u64,
                usage: brul_gpu::instance_usage(),
                mapped_at_creation: false,
            });
            self.instance_capacity = instance_len;
        }
        if quad_grown {
            self.quad_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("brul-quad-instances"),
                size: quad_len as u64,
                usage: brul_gpu::instance_usage(),
                mapped_at_creation: false,
            });
            self.quad_capacity = quad_len;
            self.textures.invalidate();
        }
        if clip_grown {
            self.queue
                .write_buffer(&self.clip_buffer, 0, self.frame.buffer(CLIP));
        }
        if instance_grown {
            let mut bytes = self.frame.buffer(SHAPE).to_vec();
            bytes.extend_from_slice(self.frame.buffer(DECOR));
            self.queue.write_buffer(&self.instance_buffer, 0, &bytes);
        }
        if quad_grown {
            let mut bytes = self.frame.buffer(GLYPH).to_vec();
            bytes.extend_from_slice(self.frame.buffer(IMAGE));
            self.queue.write_buffer(&self.quad_buffer, 0, &bytes);
        }
        for upload in uploads {
            let (buffer, offset) = match upload.buffer {
                CLIP if !clip_grown => (&self.clip_buffer, upload.offset),
                SHAPE if !instance_grown => (&self.instance_buffer, upload.offset),
                DECOR if !instance_grown => (&self.instance_buffer, shape_len + upload.offset),
                GLYPH if !quad_grown => (&self.quad_buffer, upload.offset),
                IMAGE if !quad_grown => (&self.quad_buffer, glyph_len + upload.offset),
                _ => continue,
            };
            self.queue
                .write_buffer(buffer, offset as u64, &upload.bytes);
        }
        let viewport = [
            f32::to_ne_bytes(width as f32),
            f32::to_ne_bytes(height as f32),
        ]
        .concat();
        if self.viewport != viewport {
            self.queue.write_buffer(&self.viewport_buffer, 0, &viewport);
            self.viewport = viewport;
        }
        let mut effect_ranges = Vec::new();
        if self.effects_enabled {
            let frame = EffectFrame {
                time: info.time,
                delta: info.delta,
                frame: info.frame,
                pad: 0,
                viewport: [width as f32, height as f32, 0.0, 0.0],
            };
            let bytes = effect_frame_bytes(&frame).to_vec();
            if self.effect_uniform != bytes {
                self.queue
                    .write_buffer(&self.effect_uniform_buffer, 0, &bytes);
                self.effect_uniform = bytes;
            }
            let (instances, ranges) = super::effects::instances(snapshot, effects, info);
            effect_ranges = ranges;
            if !instances.is_empty() {
                let bytes = effect_instance_bytes(&instances);
                if bytes.len() > self.effect_capacity {
                    self.effect_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                        label: Some("brul-effect-instances"),
                        size: bytes.len() as u64,
                        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                        mapped_at_creation: false,
                    });
                    self.effect_capacity = bytes.len();
                }
                self.queue.write_buffer(&self.effect_buffer, 0, &bytes);
            }
            effect_ranges.retain(|range| {
                let Some(def) = effects.get(range.name) else {
                    return false;
                };
                if self.effect_pipelines.contains_key(range.name) {
                    return true;
                }
                match super::effects::pipeline(
                    &self.device,
                    &self.effect_empty_layout,
                    &self.effect_layout,
                    def,
                    self.config.format,
                ) {
                    Ok(pipeline) => {
                        self.effect_pipelines.insert(range.name, pipeline);
                        true
                    }
                    Err(error) => {
                        tracing::warn!(?error, effect = range.name, "effect shader rejected");
                        false
                    }
                }
            });
        }
        let shape_count = shape_len / brul_gpu::INSTANCE_SIZE as usize;
        let decor_count = decor_len / brul_gpu::INSTANCE_SIZE as usize;
        let glyph_count = glyph_len / brul_gpu::TEXTURE_INSTANCE_SIZE as usize;
        let image_ranges: Vec<_> = self
            .frame
            .images()
            .iter()
            .map(|(start, count, texture)| {
                (*start + glyph_count as u32, *count, Rc::clone(texture))
            })
            .collect();
        self.present_frame(
            surface,
            (width.max(1), height.max(1)),
            atlas,
            (shape_count, decor_count, glyph_count),
            image_ranges,
            effect_ranges,
        )
    }

    fn present_frame(
        &mut self,
        surface: &dyn crate::surface::Surface,
        (width, height): (u32, u32),
        atlas: &Atlas,
        (shape_count, decor_count, glyph_count): (usize, usize, usize),
        image_ranges: Vec<(u32, u32, Rc<Texture>)>,
        effect_ranges: Vec<EffectRange>,
    ) -> bool {
        self.textures.tick();
        let glyph_bind_group = (glyph_count > 0).then(|| {
            self.textures
                .atlas(
                    atlas,
                    &Resources {
                        device: &self.device,
                        queue: &self.queue,
                        viewport: &self.viewport_buffer,
                        clips: &self.clip_buffer,
                        instances: &self.quad_buffer,
                    },
                )
                .clone()
        });
        let mut image_bind_groups = Vec::with_capacity(image_ranges.len());
        for (_, _, texture) in &image_ranges {
            image_bind_groups.push(
                self.textures
                    .image(
                        texture,
                        &Resources {
                            device: &self.device,
                            queue: &self.queue,
                            viewport: &self.viewport_buffer,
                            clips: &self.clip_buffer,
                            instances: &self.quad_buffer,
                        },
                    )
                    .clone(),
            );
        }

        let shape_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("brul-shape-bind-group"),
            layout: &self.shape_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &self.viewport_buffer,
                        offset: 0,
                        size: None,
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &self.instance_buffer,
                        offset: 0,
                        size: None,
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &self.clip_buffer,
                        offset: 0,
                        size: None,
                    }),
                },
            ],
        });
        let surface_frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame)
            | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                self.surface.configure(&self.device, &self.config);
                surface.redraw();
                return false;
            }
            _ => return false,
        };
        let view = surface_frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("brul-present"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("brul-present"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(self.clear_color),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            if shape_count > 0 {
                pass.set_pipeline(&self.shape_pipeline);
                pass.set_bind_group(0, &shape_bind_group, &[]);
                pass.draw(0..6, 0..shape_count as u32);
            }
            if decor_count > 0 {
                pass.set_pipeline(&self.shape_pipeline);
                pass.set_bind_group(0, &shape_bind_group, &[]);
                pass.draw(0..6, shape_count as u32..(shape_count + decor_count) as u32);
            }
            if glyph_count > 0 || image_ranges.iter().any(|(_, count, _)| *count > 0) {
                pass.set_pipeline(&self.quad_pipeline);
                if let Some(group) = &glyph_bind_group {
                    pass.set_bind_group(0, group, &[]);
                    pass.draw(0..6, 0..glyph_count as u32);
                }
                for ((start, count, _), group) in image_ranges.iter().zip(&image_bind_groups) {
                    if *count == 0 {
                        continue;
                    }
                    pass.set_bind_group(0, group, &[]);
                    pass.draw(0..6, *start..*start + *count);
                }
            }
        }

        if !effect_ranges.is_empty() {
            encoder.copy_texture_to_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &surface_frame.texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::TexelCopyTextureInfo {
                    texture: &self.effect_backdrop,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
            );
            let read = self.effect_history_write;
            let write = 1 - read;
            let advancing: Vec<_> = effect_ranges
                .iter()
                .filter(|range| {
                    self.effect_pipelines
                        .get(range.name)
                        .and_then(|pipeline| pipeline.advance.as_ref())
                        .is_some()
                })
                .collect();
            if !advancing.is_empty() {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("brul-effect-advance"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &self.effect_history_views[write],
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
                for range in advancing {
                    let Some(pipeline) = self.effect_pipelines.get(range.name) else {
                        continue;
                    };
                    let Some(advance) = pipeline.advance.as_ref() else {
                        continue;
                    };
                    let group = super::effects::bind_group(
                        &self.device,
                        &self.effect_layout,
                        &self.effect_uniform_buffer,
                        &self.effect_buffer,
                        &self.effect_backdrop_view,
                        &self.effect_history_views[read],
                        &self.effect_sampler,
                    );
                    pass.set_pipeline(advance);
                    pass.set_bind_group(0, &self.effect_empty_group, &[]);
                    pass.set_bind_group(1, &group, &[]);
                    pass.draw(0..6, range.start..range.start + range.count);
                }
            }
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("brul-effect"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
                for range in &effect_ranges {
                    let Some(pipeline) = self.effect_pipelines.get(range.name) else {
                        continue;
                    };
                    let group = super::effects::bind_group(
                        &self.device,
                        &self.effect_layout,
                        &self.effect_uniform_buffer,
                        &self.effect_buffer,
                        &self.effect_backdrop_view,
                        &self.effect_history_views[write],
                        &self.effect_sampler,
                    );
                    pass.set_pipeline(&pipeline.main);
                    pass.set_bind_group(0, &self.effect_empty_group, &[]);
                    pass.set_bind_group(1, &group, &[]);
                    pass.draw(0..6, range.start..range.start + range.count);
                }
            }
            self.effect_history_write = write;
        }

        self.queue.submit(Some(encoder.finish()));
        if self.capture && self.captured.is_none() {
            self.captured = self.readback(&surface_frame.texture, width, height);
        }
        surface.pre_present_notify();
        self.queue.present(surface_frame);
        true
    }
}
