use wgpu::util::DeviceExt;

use crate::camera::{Camera, CameraUniforms};
use crate::log;
use crate::primitive::Primitive;
use crate::scene::Scene;
use crate::structure::*;
use crate::utils::*;

pub struct WavefrontPathTracing {
    render_pipeline: wgpu::RenderPipeline,
    init_pipeline: wgpu::ComputePipeline,
    logic_pipeline: wgpu::ComputePipeline,
    new_path_pipeline: wgpu::ComputePipeline,
    material_pipeline: wgpu::ComputePipeline,
    ray_cast_pipeline: wgpu::ComputePipeline,
    ray_cast_light_pipeline: wgpu::ComputePipeline,

    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,

    scene_uniforms: SceneUniforms,
    camera_uniforms: CameraUniforms,

    scene_uniforms_buffer: wgpu::Buffer,
    camera_uniforms_buffer: wgpu::Buffer,
    accum_pixel_buffer: wgpu::Buffer,
    primitive_buffer: wgpu::Buffer,
    material_buffer: wgpu::Buffer,
    texture_buffer: wgpu::Buffer,
    tex_data_buffer: wgpu::Buffer,
    scene_bind_group: wgpu::BindGroup,

    ray_pool_buffer: wgpu::Buffer,
    queue_buffer: wgpu::Buffer,
    dispatch_args_buffers: [wgpu::Buffer; 2],
    compute_bind_groups: [wgpu::BindGroup; 2],

    staging_buffer: wgpu::Buffer,
}
impl WavefrontPathTracing {
    const RAYPOOL_BUFFER_SIZE: wgpu::BufferAddress =
        (32 + 32 + 4 + 4 + 4 + 8 + 16 + 4 + 16 + 4 + 48 + 4 + 4 + 48 + 4 + 32 + 32) * (1 << 20)
            + 16;
    const QUEUE_BUFFER_SIZE: wgpu::BufferAddress = (1 << 20) * 4 * 4 + 4 * 4;

    pub fn new(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        scene: &mut Scene,
    ) -> Self {
        let scene_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT | wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT | wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT | wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT | wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::FRAGMENT | wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 5,
                        visibility: wgpu::ShaderStages::FRAGMENT | wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 6,
                        visibility: wgpu::ShaderStages::FRAGMENT | wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
                label: Some("scene_bind_group_layout"),
            });

        let compute_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
                label: Some("compute_bind_group_layout"),
            });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("raytracing.wgsl").into()),
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&scene_bind_group_layout, &compute_bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList, // 1.
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw, // 2.
                cull_mode: Some(wgpu::Face::Back),
                // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                polygon_mode: wgpu::PolygonMode::Fill,
                // Requires Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        let compute_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("compute_pipeline_layout"),
                bind_group_layouts: &[&scene_bind_group_layout, &compute_bind_group_layout],
                push_constant_ranges: &[],
            });
        let init_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("init_pipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &shader,
            entry_point: Some("wavefront_init"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });
        let logic_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("logic_pipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &shader,
            entry_point: Some("wavefront_logic"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });
        let new_path_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("new_path_pipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &shader,
            entry_point: Some("wavefront_new_path"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });
        let material_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("material_pipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &shader,
            entry_point: Some("wavefront_material"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });
        let ray_cast_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("ray_cast_pipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &shader,
            entry_point: Some("wavefront_ray_cast"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });
        let ray_cast_light_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("ray_cast_light_pipeline"),
                layout: Some(&compute_pipeline_layout),
                module: &shader,
                entry_point: Some("wavefront_ray_cast_light"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
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

        let mut raw_vec = RawVec::new();
        let root_id = scene.world.to_raw(&mut raw_vec) as u32;
        let light_id: i32;
        if let Some(lights) = scene.lights.as_mut() {
            light_id = lights.to_raw(&mut raw_vec) as i32;
        } else {
            light_id = -1;
        }
        if raw_vec.tex_data.is_empty() {
            raw_vec.tex_data.push(0.0);
        }
        // log!("{:?}, {:?}", materials_raw, primitives_raw);

        let scene_uniforms = SceneUniforms {
            renderer_type: 1,
            root_id,
            light_id,
        };

        let camera_uniforms = scene.camera.to_raw();
        let camera_uniforms_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Uniforms Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let accum_pixel_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Accum Pixel Buffer"),
            size: (config.width * config.height * 16) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let scene_uniforms_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Scene Uniforms Buffer"),
            contents: bytemuck::cast_slice(&[scene_uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let primitive_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Primitive Buffer"),
            contents: bytemuck::cast_slice(&raw_vec.primitives),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let material_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Material Buffer"),
            contents: bytemuck::cast_slice(&raw_vec.materials),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let texture_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Texture Buffer"),
            contents: bytemuck::cast_slice(&raw_vec.textures),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let tex_data_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Tex Data Buffer"),
            contents: bytemuck::cast_slice(&raw_vec.tex_data),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let scene_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &scene_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: scene_uniforms_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: camera_uniforms_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: accum_pixel_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: primitive_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: material_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: texture_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: tex_data_buffer.as_entire_binding(),
                },
            ],
            label: Some("scene_bind_group"),
        });

        let ray_pool_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ray_pool_buffer"),
            size: Self::RAYPOOL_BUFFER_SIZE,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let queue_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("queue_buffer"),
            size: Self::QUEUE_BUFFER_SIZE,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let dispatch_args_buffer_0 = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("dispatch_args_buffer_0"),
            size: 4 * 4 * 2,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::INDIRECT,
            mapped_at_creation: false,
        });
        let dispatch_args_buffer_1 = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("dispatch_args_buffer_1"),
            size: 4 * 4 * 2,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::INDIRECT,
            mapped_at_creation: false,
        });
        let dispatch_args_buffers = [dispatch_args_buffer_0, dispatch_args_buffer_1];

        let compute_bind_group_0 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &compute_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: ray_pool_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: queue_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: dispatch_args_buffers[0].as_entire_binding(),
                },
            ],
            label: Some("compute_bind_group_0"),
        });
        let compute_bind_group_1 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &compute_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: ray_pool_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: queue_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: dispatch_args_buffers[1].as_entire_binding(),
                },
            ],
            label: Some("compute_bind_group_1"),
        });
        let compute_bind_groups = [compute_bind_group_0, compute_bind_group_1];

        let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("staging_buffer"),
            size: 4 * 4 * 2,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        Self {
            render_pipeline,
            init_pipeline,
            logic_pipeline,
            new_path_pipeline,
            material_pipeline,
            ray_cast_pipeline,
            ray_cast_light_pipeline,

            vertex_buffer,
            index_buffer,
            num_indices,

            scene_uniforms,
            camera_uniforms,

            scene_uniforms_buffer,
            camera_uniforms_buffer,
            accum_pixel_buffer,
            primitive_buffer,
            material_buffer,
            texture_buffer,
            tex_data_buffer,
            scene_bind_group,

            queue_buffer,
            ray_pool_buffer,
            dispatch_args_buffers,
            compute_bind_groups,

            staging_buffer,
        }
    }
}
impl Renderer for WavefrontPathTracing {
    fn render(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        queue: &wgpu::Queue,
        view: &wgpu::TextureView,
        rd: &mut RenderData,
    ) {
        self.camera_uniforms.frame_id = rd.frame_id;

        queue.write_buffer(
            &self.camera_uniforms_buffer,
            0,
            bytemuck::bytes_of(&self.camera_uniforms),
        );

        if self.camera_uniforms.frame_id * self.camera_uniforms.samples_per_frame
            < self.camera_uniforms.samples_per_pixel
        {
            let total_ray_count = self.camera_uniforms.image_wh[0]
                * self.camera_uniforms.image_wh[1]
                * self.camera_uniforms.samples_per_frame;
            let init_ray_count = total_ray_count.min(1 << 20);
            queue.write_buffer(
                &self.ray_pool_buffer,
                Self::RAYPOOL_BUFFER_SIZE - 16,
                bytemuck::cast_slice(&[init_ray_count]),
            );
            queue.write_buffer(
                &self.queue_buffer,
                Self::QUEUE_BUFFER_SIZE - 16,
                bytemuck::cast_slice(&[0_u32, 0, 1 << 20, 0]),
            );

            let wg = init_ray_count.div_ceil(256);
            {
                let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("compute_pass"),
                    timestamp_writes: None,
                });

                compute_pass.set_pipeline(&self.init_pipeline);
                compute_pass.set_bind_group(0, &self.scene_bind_group, &[]);
                compute_pass.set_bind_group(1, &self.compute_bind_groups[0], &[]);
                compute_pass.dispatch_workgroups(wg, 1, 1);
            }
            {
                let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("compute_pass"),
                    timestamp_writes: None,
                });
                compute_pass.set_pipeline(&self.ray_cast_pipeline);
                compute_pass.set_bind_group(0, &self.scene_bind_group, &[]);
                compute_pass.set_bind_group(1, &self.compute_bind_groups[1], &[]);
                compute_pass.dispatch_workgroups(wg, 1, 1);
            }

            let max_iter = total_ray_count.div_ceil(1 << 20) * self.camera_uniforms.max_depth;
            for _i in 0..max_iter {
                encoder.clear_buffer(&self.queue_buffer, Self::QUEUE_BUFFER_SIZE - 16, Some(16));
                {
                    encoder.clear_buffer(&self.dispatch_args_buffers[0], 0, Some(32));
                    let mut compute_pass =
                        encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                            label: Some("compute_pass"),
                            timestamp_writes: None,
                        });

                    compute_pass.set_pipeline(&self.logic_pipeline);
                    compute_pass.set_bind_group(0, &self.scene_bind_group, &[]);
                    compute_pass.set_bind_group(1, &self.compute_bind_groups[0], &[]);
                    compute_pass.dispatch_workgroups(wg, 1, 1);
                }

                {
                    encoder.clear_buffer(&self.dispatch_args_buffers[1], 0, Some(32));
                    let mut compute_pass =
                        encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                            label: Some("compute_pass"),
                            timestamp_writes: None,
                        });

                    compute_pass.set_bind_group(0, &self.scene_bind_group, &[]);
                    compute_pass.set_bind_group(1, &self.compute_bind_groups[1], &[]);

                    compute_pass.set_pipeline(&self.new_path_pipeline);
                    compute_pass.dispatch_workgroups_indirect(&self.dispatch_args_buffers[0], 0);

                    compute_pass.set_pipeline(&self.material_pipeline);
                    compute_pass.dispatch_workgroups_indirect(&self.dispatch_args_buffers[0], 16);
                }

                {
                    let mut compute_pass =
                        encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                            label: Some("compute_pass"),
                            timestamp_writes: None,
                        });

                    compute_pass.set_bind_group(0, &self.scene_bind_group, &[]);
                    compute_pass.set_bind_group(1, &self.compute_bind_groups[0], &[]);

                    compute_pass.set_pipeline(&self.ray_cast_pipeline);
                    compute_pass.dispatch_workgroups_indirect(&self.dispatch_args_buffers[1], 0);

                    compute_pass.set_pipeline(&self.ray_cast_light_pipeline);
                    compute_pass.dispatch_workgroups_indirect(&self.dispatch_args_buffers[1], 16);
                }
            }
        }

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
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

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.scene_bind_group, &[]);
            render_pass.set_bind_group(1, &self.compute_bind_groups[0], &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
        }

        rd.frame_id += 1;
    }

    fn print(&self, encoder: &mut wgpu::CommandEncoder) {
        encoder.copy_buffer_to_buffer(
            &self.dispatch_args_buffers[0],
            0,
            &self.staging_buffer,
            0,
            32,
        );

        let staging_buffer = self.staging_buffer.clone();
        encoder.map_buffer_on_submit(&self.staging_buffer, wgpu::MapMode::Read, .., move |res| {
            if res.is_ok() {
                let bytes = staging_buffer.get_mapped_range(..).to_vec();
                staging_buffer.unmap();
                let buf: Vec<u32> = Vec::from(bytemuck::cast_slice(bytes.as_slice()));
                log!("buf: {:?}", buf);
            }
        });
    }
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
