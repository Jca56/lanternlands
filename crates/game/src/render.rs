//! The world on the GPU: ground and sprites drawn into a picture of
//! their own, then that picture put on the window.
//!
//! The picture holds the art's own sRGB numbers and blends them as they
//! are, the way Lantern Studio composites, so soft edges and shadows
//! come out as they were drawn. Only the last step, onto the window,
//! converts.

use lntrn_core::bytes::{self, Pod};
use lntrn_core::impl_pod;
use lntrn_render::{Gpu, wgpu};
use sim::Terrain;

use crate::atlas::Sheet;
use crate::palette::{VOID, terrain_style};
use crate::scene::{Scene, SpriteInstance, TerrainInstance};

#[derive(Clone, Copy)]
#[repr(C)]
struct Globals {
    scale: [f32; 2],
    offset: [f32; 2],
    tile_px: f32,
    _pad: [f32; 3],
    /// Fill then rim for each terrain, the rim's width in its alpha.
    styles: [[f32; 4]; Terrain::COUNT * 2],
}
impl_pod!(unsafe Globals);

/// A buffer of instances that grows to hold what it is given.
struct Instances {
    label: &'static str,
    buffer: wgpu::Buffer,
    /// Bytes the buffer can hold.
    capacity: u64,
    count: u32,
}

impl Instances {
    fn new(gpu: &Gpu, label: &'static str) -> Instances {
        let capacity = 16 * 1024;
        Instances { label, buffer: Self::make(gpu, label, capacity), capacity, count: 0 }
    }

    fn make(gpu: &Gpu, label: &'static str, capacity: u64) -> wgpu::Buffer {
        gpu.device.create_buffer(&wgpu::BufferDescriptor { label: Some(label), size: capacity, usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false })
    }

    fn upload<T: Pod>(&mut self, gpu: &Gpu, items: &[T]) {
        let data = bytes::slice_as_bytes(items);
        if data.len() as u64 > self.capacity {
            self.capacity = (data.len() as u64).next_power_of_two();
            self.buffer = Self::make(gpu, self.label, self.capacity);
        }
        if !data.is_empty() {
            gpu.queue.write_buffer(&self.buffer, 0, data);
        }
        self.count = items.len() as u32;
    }
}

pub struct WorldRenderer {
    terrain: wgpu::RenderPipeline,
    sprites: wgpu::RenderPipeline,
    composite: wgpu::RenderPipeline,
    globals: wgpu::Buffer,
    globals_bind: wgpu::BindGroup,
    sheet_bind: wgpu::BindGroup,
    composite_layout: wgpu::BindGroupLayout,
    composite_sampler: wgpu::Sampler,
    terrain_instances: Instances,
    sprite_instances: Instances,
}

impl WorldRenderer {
    /// What the world is drawn into before it goes on the window.
    pub const TARGET_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

    /// `window_format` is what [`WorldRenderer::composite`] draws onto.
    pub fn new(gpu: &Gpu, window_format: wgpu::TextureFormat, sheet: &Sheet) -> WorldRenderer {
        let device = &gpu.device;
        let globals = device.create_buffer(&wgpu::BufferDescriptor { label: Some("world globals"), size: size_of::<Globals>() as u64, usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        let globals_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("world globals layout"),
            entries: &[wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::VERTEX_FRAGMENT, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None }],
        });
        let globals_bind = device.create_bind_group(&wgpu::BindGroupDescriptor { label: Some("world globals"), layout: &globals_layout, entries: &[wgpu::BindGroupEntry { binding: 0, resource: globals.as_entire_binding() }] });

        let picture_layout = |label| {
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some(label),
                entries: &[
                    wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
                    wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
                ],
            })
        };
        let sheet_layout = picture_layout("sprite sheet layout");
        let composite_layout = picture_layout("world picture layout");

        let sheet_view = upload_sheet(gpu, sheet).create_view(&wgpu::TextureViewDescriptor::default());
        // Smooth between texels and between the sheet's smaller copies.
        let sheet_sampler = device.create_sampler(&wgpu::SamplerDescriptor { label: Some("sprite sheet sampler"), mag_filter: wgpu::FilterMode::Linear, min_filter: wgpu::FilterMode::Linear, mipmap_filter: wgpu::MipmapFilterMode::Linear, ..Default::default() });
        let sheet_bind = picture_bind(gpu, &sheet_layout, &sheet_view, &sheet_sampler);
        // The world picture is the size of what it is put on: no filter.
        let composite_sampler = device.create_sampler(&wgpu::SamplerDescriptor { label: Some("world picture sampler"), ..Default::default() });

        let world = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("world shader"), source: wgpu::ShaderSource::Wgsl(include_str!("world.wgsl").into()) });
        let to_window = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("composite shader"), source: wgpu::ShaderSource::Wgsl(include_str!("composite.wgsl").into()) });

        let over = Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING);
        let terrain = pipeline(gpu, "terrain", &[Some(&globals_layout)], &world, ("vs_terrain", "fs_terrain"), Some((size_of::<TerrainInstance>(), &wgpu::vertex_attr_array![0 => Sint32x2, 1 => Uint32x2])), Self::TARGET_FORMAT, over);
        let sprites = pipeline(gpu, "sprites", &[Some(&globals_layout), Some(&sheet_layout)], &world, ("vs_sprite", "fs_sprite"), Some((size_of::<SpriteInstance>(), &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32x4, 3 => Float32x4])), Self::TARGET_FORMAT, over);
        let fragment = if window_format.is_srgb() { "fs_to_srgb" } else { "fs_as_is" };
        let composite = pipeline(gpu, "world to window", &[Some(&composite_layout)], &to_window, ("vs_composite", fragment), None, window_format, None);

        WorldRenderer { terrain, sprites, composite, globals, globals_bind, sheet_bind, composite_layout, composite_sampler, terrain_instances: Instances::new(gpu, "terrain instances"), sprite_instances: Instances::new(gpu, "sprite instances") }
    }

    /// Send a scene to the GPU, for a picture of `size` pixels.
    pub fn prepare(&mut self, gpu: &Gpu, scene: &Scene, size: [u32; 2]) {
        let mut styles = [[0.0; 4]; Terrain::COUNT * 2];
        for terrain in Terrain::ALL {
            let (style, at) = (terrain_style(terrain), terrain.rank() as usize * 2);
            styles[at] = [style.fill[0], style.fill[1], style.fill[2], 1.0];
            styles[at + 1] = [style.rim[0], style.rim[1], style.rim[2], style.rim_width];
        }
        // Tiles to clip space: the picture is two units across, and y
        // runs down the screen but up in clip space.
        let scale = [2.0 * scene.tile_px / size[0].max(1) as f32, -2.0 * scene.tile_px / size[1].max(1) as f32];
        let globals = Globals { scale, offset: scene.offset, tile_px: scene.tile_px, _pad: [0.0; 3], styles };
        gpu.queue.write_buffer(&self.globals, 0, bytes::bytes_of(&globals));
        self.terrain_instances.upload(gpu, &scene.terrain);
        self.sprite_instances.upload(gpu, &scene.sprites);
    }

    /// Draw the prepared scene over all of `target` (a
    /// [`WorldRenderer::TARGET_FORMAT`] picture).
    pub fn draw(&self, encoder: &mut wgpu::CommandEncoder, target: &wgpu::TextureView) {
        let [r, g, b] = VOID.map(f64::from);
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("world"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment { view: target, depth_slice: None, resolve_target: None, ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color { r, g, b, a: 1.0 }), store: wgpu::StoreOp::Store } })],
            depth_stencil_attachment: None,
            ..Default::default()
        });
        pass.set_bind_group(0, &self.globals_bind, &[]);
        if self.terrain_instances.count > 0 {
            pass.set_pipeline(&self.terrain);
            pass.set_vertex_buffer(0, self.terrain_instances.buffer.slice(..));
            pass.draw(0..4, 0..self.terrain_instances.count);
        }
        if self.sprite_instances.count > 0 {
            pass.set_pipeline(&self.sprites);
            pass.set_bind_group(1, &self.sheet_bind, &[]);
            pass.set_vertex_buffer(0, self.sprite_instances.buffer.slice(..));
            pass.draw(0..4, 0..self.sprite_instances.count);
        }
    }

    /// Put the world picture `world` on `window`, filling `rect`
    /// (`[x, y, width, height]` in pixels, inside the window, and the
    /// picture's own size).
    pub fn composite(&self, gpu: &Gpu, encoder: &mut wgpu::CommandEncoder, world: &wgpu::TextureView, window: &wgpu::TextureView, rect: [u32; 4]) {
        let bind = picture_bind(gpu, &self.composite_layout, world, &self.composite_sampler);
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("world to window"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment { view: window, depth_slice: None, resolve_target: None, ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store } })],
            depth_stencil_attachment: None,
            ..Default::default()
        });
        let [x, y, w, h] = rect;
        pass.set_viewport(x as f32, y as f32, w as f32, h as f32, 0.0, 1.0);
        pass.set_scissor_rect(x, y, w, h);
        pass.set_pipeline(&self.composite);
        pass.set_bind_group(0, &bind, &[]);
        pass.draw(0..3, 0..1);
    }
}

fn picture_bind(gpu: &Gpu, layout: &wgpu::BindGroupLayout, view: &wgpu::TextureView, sampler: &wgpu::Sampler) -> wgpu::BindGroup {
    gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("picture"),
        layout,
        entries: &[wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(view) }, wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(sampler) }],
    })
}

/// One pipeline drawing quads as four-vertex strips, one instance each
/// (`instances`: the instance's size and attributes), or a bare triangle.
#[allow(clippy::too_many_arguments)]
fn pipeline(
    gpu: &Gpu,
    label: &str,
    layouts: &[Option<&wgpu::BindGroupLayout>],
    module: &wgpu::ShaderModule,
    (vertex, fragment): (&str, &str),
    instances: Option<(usize, &[wgpu::VertexAttribute])>,
    format: wgpu::TextureFormat,
    blend: Option<wgpu::BlendState>,
) -> wgpu::RenderPipeline {
    let layout = gpu.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some(label), bind_group_layouts: layouts, immediate_size: 0 });
    let buffers: Vec<Option<wgpu::VertexBufferLayout>> = instances.iter().map(|&(size, attributes)| Some(wgpu::VertexBufferLayout { array_stride: size as u64, step_mode: wgpu::VertexStepMode::Instance, attributes })).collect();
    gpu.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(&layout),
        vertex: wgpu::VertexState { module, entry_point: Some(vertex), buffers: &buffers, compilation_options: Default::default() },
        fragment: Some(wgpu::FragmentState { module, entry_point: Some(fragment), targets: &[Some(wgpu::ColorTargetState { format, blend, write_mask: wgpu::ColorWrites::ALL })], compilation_options: Default::default() }),
        primitive: wgpu::PrimitiveState { topology: if instances.is_some() { wgpu::PrimitiveTopology::TriangleStrip } else { wgpu::PrimitiveTopology::TriangleList }, ..Default::default() },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    })
}

/// The sheet as a texture, every level of it.
fn upload_sheet(gpu: &Gpu, sheet: &Sheet) -> wgpu::Texture {
    let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("sprite sheet"),
        size: wgpu::Extent3d { width: sheet.width, height: sheet.height, depth_or_array_layers: 1 },
        mip_level_count: sheet.levels.len() as u32,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        // Not an sRGB format: the world pass wants the numbers as stored.
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let (mut width, mut height) = (sheet.width, sheet.height);
    for (level, pixels) in sheet.levels.iter().enumerate() {
        gpu.queue.write_texture(
            wgpu::TexelCopyTextureInfo { texture: &texture, mip_level: level as u32, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            pixels,
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(width * 4), rows_per_image: Some(height) },
            wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        );
        (width, height) = (width / 2, height / 2);
    }
    texture
}

#[cfg(test)]
#[path = "render_tests.rs"]
mod tests;
