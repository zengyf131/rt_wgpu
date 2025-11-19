use std::{f32, sync::Arc};

use winit::{
    application::ApplicationHandler,
    dpi::{PhysicalPosition, PhysicalSize},
    event::*,
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

use crate::camera::Camera;
use crate::gui::EguiRenderer;
use crate::log;
use crate::print_texture::PrintTexture;
use crate::pt::PathTracing;
use crate::scene::*;
use crate::structure::*;
use crate::wfpt::WavefrontPathTracing;

// This will store the state of our game
pub struct State {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    is_surface_configured: bool,
    pub window: Arc<Window>,

    mouse_pos: (f64, f64),

    scene: Scene,
    egui_renderer: EguiRenderer,
    gui_enable: bool,
    render_data: RenderData,

    renderer_pt: PathTracing,
    renderer_wfpt: WavefrontPathTracing,
    renderer_texture: PrintTexture,
}

impl State {
    // We don't need this to be async right now,
    // but we will in the next tutorial
    pub async fn new(window: Arc<Window>) -> anyhow::Result<Self> {
        let size = PhysicalSize::<u32>::new(1920, 1080);
        let _ = window.request_inner_size(size);

        // The instance is a handle to our GPU
        // BackendBit::PRIMARY => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            #[cfg(not(target_arch = "wasm32"))]
            backends: wgpu::Backends::PRIMARY,
            #[cfg(target_arch = "wasm32")]
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                // WebGL doesn't support all of wgpu's features, so if
                // we're building for the web we'll have to disable some.
                required_limits: if cfg!(target_arch = "wasm32") {
                    wgpu::Limits {
                        max_storage_buffer_binding_size: 1 << 30,
                        max_buffer_size: 1 << 30,
                        ..Default::default()
                    }
                } else {
                    wgpu::Limits {
                        max_storage_buffer_binding_size: 1 << 30,
                        max_buffer_size: 1 << 30,
                        ..Default::default()
                    }
                },
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off,
            })
            .await?;

        let surface_caps = surface.get_capabilities(&adapter);
        // log!("{:?}", surface_caps);
        // Shader code in this tutorial assumes an sRGB surface texture. Using a different
        // one will result in all the colors coming out darker. If you want to support non
        // sRGB surfaces, you'll need to account for that when drawing to the frame.
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        let scene = get_scene(&device, SceneEnum::CornellBox);
        let scene_bind_group_layout = Scene::layout(&device);
        let egui_renderer = EguiRenderer::new(&device, config.format, window.clone());

        let renderer_pt = PathTracing::new(&device, &config, &scene_bind_group_layout);
        let renderer_wfpt = WavefrontPathTracing::new(&device, &config, &scene_bind_group_layout);
        let renderer_texture = PrintTexture::new(&device, &config, &scene_bind_group_layout);

        Ok(Self {
            surface,
            device,
            queue,
            config,
            is_surface_configured: false,
            window,

            mouse_pos: (0.0, 0.0),

            scene,
            egui_renderer,
            gui_enable: true,
            render_data: RenderData::new(),

            renderer_pt,
            renderer_wfpt,
            renderer_texture,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            log!("{}, {}", width, height);
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
            self.is_surface_configured = true;

            self.scene
                .scene_uniforms
                .update(&self.config, &self.render_data);
            self.queue.write_buffer(
                &self.scene.scene_uniforms_buffer,
                0,
                bytemuck::bytes_of(&self.scene.scene_uniforms),
            );
        }
    }

    pub fn handle_key(&mut self, event_loop: &ActiveEventLoop, key: KeyCode, pressed: bool) {
        if key == KeyCode::Escape && pressed {
            event_loop.exit();
        } else if key == KeyCode::KeyH && pressed {
            self.gui_enable = !self.gui_enable;
        }
    }

    pub fn handle_mouse_moved(&mut self, position: PhysicalPosition<f64>) {
        self.mouse_pos = (position.x, position.y);
    }

    pub fn handle_gui(&mut self, event: &WindowEvent) {
        self.egui_renderer.handle_input(self.window.clone(), event);
    }

    pub fn update(&mut self) {
        if self.render_data.update_config {
            let mut scene = get_scene(&self.device, self.render_data.scene_config.scene_enum);

            // let size =
            //     PhysicalSize::<u32>::new(scene.camera.image_width, scene.camera.image_height);
            // if self.window.inner_size().width != size.width
            //     || self.window.inner_size().height != size.height
            // {
            //     let _ = self.window.request_inner_size(size);
            //     self.is_surface_configured = false;
            // }

            scene.scene_uniforms.update(&self.config, &self.render_data);
            self.queue.write_buffer(
                &scene.scene_uniforms_buffer,
                0,
                bytemuck::bytes_of(&scene.scene_uniforms),
            );

            match self.render_data.scene_config.renderer_type {
                RendererType::PT => {}
                RendererType::WFPT => {
                    self.renderer_wfpt.configure(&self.device);
                }
            }
            self.renderer_texture
                .configure(&self.device, &self.config, &scene);

            self.render_data.reset();
            self.render_data.render_status = RenderStatus::Render;
            self.scene = scene;
        }
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        self.window.request_redraw();

        // We can't render unless the surface is configured
        if !self.is_surface_configured {
            return Ok(());
        }

        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        if self.render_data.render_status == RenderStatus::Render {
            self.scene.camera_uniforms.frame_id = self.render_data.frame_id;

            match self.render_data.scene_config.renderer_type {
                RendererType::PT => {
                    self.renderer_pt.render(
                        &mut encoder,
                        &self.queue,
                        self.renderer_texture.target_view(),
                        &self.scene,
                        &mut self.render_data,
                    );
                }
                RendererType::WFPT => {
                    self.renderer_wfpt.render(
                        &mut encoder,
                        &self.queue,
                        self.renderer_texture.target_view(),
                        &self.scene,
                        &mut self.render_data,
                    );
                }
            }

            self.renderer_texture.render(
                &mut encoder,
                &self.queue,
                &view,
                &self.scene,
                &mut self.render_data,
            );

            self.render_data.frame_id += 1;
        }

        if self.gui_enable {
            let screen_descriptor = egui_wgpu::ScreenDescriptor {
                size_in_pixels: [self.config.width, self.config.height],
                pixels_per_point: self.window.scale_factor() as f32,
            };

            self.egui_renderer.begin_frame(&self.window);

            self.egui_renderer
                .render(&mut self.render_data, &self.scene);

            self.egui_renderer.end_frame_and_draw(
                &self.device,
                &self.queue,
                &mut encoder,
                &self.window,
                &view,
                screen_descriptor,
            );
        }

        // submit will accept anything that implements IntoIter
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        // if self.render_data.frame_id == 1 {
        //     let mut encoder = self
        //         .device
        //         .create_command_encoder(&wgpu::CommandEncoderDescriptor {
        //             label: Some("Render Encoder"),
        //         });
        //     self.renderer.print(&mut encoder, &self.queue);
        //     self.queue.submit(std::iter::once(encoder.finish()));
        // }

        Ok(())
    }
}
