use std::io::Cursor;
use wgpu::util::DeviceExt;

use crate::camera::{Camera, CameraUniforms};
use crate::log;
use crate::primitive::Primitive;
use crate::scene::Scene;
use crate::structure::*;
use crate::utils::*;

pub struct PrintTexture {
    print_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,

    print_bind_group_layout: wgpu::BindGroupLayout,
    render_texture: Option<wgpu::Texture>,
    render_view: Option<wgpu::TextureView>,
    render_sampler: Option<wgpu::Sampler>,
    print_bind_group: Option<wgpu::BindGroup>,

    staging_buffer: Option<wgpu::Buffer>,
    image_wh: Option<Vector2<u32>>,
}
impl PrintTexture {
    pub fn new(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        scene_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("raytracing.wgsl").into()),
        });

        let print_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("print_bind_group_layout"),
            });
        let print_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Print Pipeline Layout"),
                bind_group_layouts: &[scene_bind_group_layout, &print_bind_group_layout],
                push_constant_ranges: &[],
            });

        let print_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Print Pipeline"),
            layout: Some(&print_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_print"),
                buffers: &[Vertex::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_print"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });
        let num_indices = INDICES.len() as u32;

        Self {
            print_pipeline,
            vertex_buffer,
            index_buffer,
            num_indices,

            print_bind_group_layout,
            render_texture: None,
            render_view: None,
            render_sampler: None,
            print_bind_group: None,

            staging_buffer: None,
            image_wh: None,
        }
    }

    pub fn configure(
        &mut self,
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        scene: &Scene,
    ) {
        let render_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("render_texture"),
            size: wgpu::Extent3d {
                width: scene.camera.image_width,
                height: scene.camera.image_height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: config.format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let render_view = render_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let render_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let print_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.print_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&render_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&render_sampler),
                },
            ],
            label: Some("print_bind_group"),
        });

        let padded_bytes_per_row = ((scene.camera.image_width * 4 + 255) / 256) * 256;
        let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Staging Buffer"),
            size: (padded_bytes_per_row * scene.camera.image_height) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        self.render_texture = Some(render_texture);
        self.render_view = Some(render_view);
        self.render_sampler = Some(render_sampler);
        self.print_bind_group = Some(print_bind_group);
        self.staging_buffer = Some(staging_buffer);
        self.image_wh = Some(vec2(scene.camera.image_width, scene.camera.image_height));
    }

    pub fn target_view(&self) -> &wgpu::TextureView {
        self.render_view.as_ref().unwrap()
    }

    pub fn download(&self, encoder: &mut wgpu::CommandEncoder) {
        let filename = String::from("render.png");
        let tex_wh = self.image_wh.unwrap();
        let unpadded_bytes_per_row = tex_wh.x * 4;
        let padded_bytes_per_row = ((unpadded_bytes_per_row + 255) / 256) * 256;

        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: self.render_texture.as_ref().unwrap(),
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: self.staging_buffer.as_ref().unwrap(),
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row),
                    rows_per_image: Some(tex_wh.y),
                },
            },
            wgpu::Extent3d {
                width: tex_wh.x,
                height: tex_wh.y,
                depth_or_array_layers: 1,
            },
        );

        let staging_buffer = self.staging_buffer.as_ref().unwrap().clone();
        encoder.map_buffer_on_submit(
            self.staging_buffer.as_ref().unwrap(),
            wgpu::MapMode::Read,
            ..,
            move |res| {
                if res.is_ok() {
                    let bytes = staging_buffer.get_mapped_range(..).to_vec();
                    staging_buffer.unmap();

                    let mut image_data = vec![0u8; (tex_wh.x * tex_wh.y * 4) as usize];
                    // Remove padding
                    for y in 0..tex_wh.y {
                        let src_offset = (y * padded_bytes_per_row) as usize;
                        let dst_offset = (y * unpadded_bytes_per_row) as usize;

                        image_data[dst_offset..dst_offset + unpadded_bytes_per_row as usize]
                            .copy_from_slice(
                                &bytes[src_offset..src_offset + unpadded_bytes_per_row as usize],
                            );
                    }
                    // BGRA to RGBA
                    for p in 0..(tex_wh.x * tex_wh.y) as usize {
                        let tmp = image_data[4 * p];
                        image_data[4 * p] = image_data[4 * p + 2];
                        image_data[4 * p + 2] = tmp;
                    }
                    let img = image::ImageBuffer::from_raw(tex_wh.x, tex_wh.y, image_data).unwrap();
                    download_image(&img, filename);
                }
            },
        );
    }
}
impl Renderer for PrintTexture {
    fn render(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        queue: &wgpu::Queue,
        view: &wgpu::TextureView,
        scene: &Scene,
        rd: &mut RenderData,
    ) {
        queue.write_buffer(
            &scene.camera_uniforms_buffer,
            0,
            bytemuck::bytes_of(&scene.camera_uniforms),
        );

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Print Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_pipeline(&self.print_pipeline);
            render_pass.set_bind_group(0, &scene.scene_bind_group, &[]);
            render_pass.set_bind_group(1, &self.print_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
        }
    }

    fn print(&self, encoder: &mut wgpu::CommandEncoder) {}
}

pub fn download_image(img: &image::ImageBuffer<image::Rgba<u8>, Vec<u8>>, filename: String) {
    let mut png_data: Vec<u8> = Vec::new();
    let mut cursor = Cursor::new(&mut png_data);
    img.write_to(&mut cursor, image::ImageFormat::Png)
        .expect("Error encoding image to png.");

    let task = rfd::AsyncFileDialog::new()
        .add_filter("image", &["png"])
        .set_file_name(filename.clone())
        .save_file();
    execute_future(async move {
        let file = task.await;
        if let Some(file) = file {
            file.write(png_data.as_slice())
                .await
                .expect(format!("Error saving {}", filename).as_str());
        }
    });
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
}
impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 1] = wgpu::vertex_attr_array![0 => Float32x3];

    fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;

        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

const VERTICES: &[Vertex] = &[
    Vertex {
        position: [-1.0, -1.0, 0.0],
    },
    Vertex {
        position: [1.0, -1.0, 0.0],
    },
    Vertex {
        position: [-1.0, 1.0, 0.0],
    },
    Vertex {
        position: [1.0, 1.0, 0.0],
    },
];

const INDICES: &[u16] = &[0, 1, 2, 3, 2, 1];

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct PixelData {
    rgb: [f32; 3],
}
