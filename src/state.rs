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
use crate::utils::*;
use crate::wfpt::WavefrontPathTracing;

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
    pub async fn new(window: Arc<Window>) -> anyhow::Result<Self> {
        let size = PhysicalSize::<u32>::new(1920, 1080);
        let _ = window.request_inner_size(size);

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
        let current_pos = vec2(position.x as f32, position.y as f32);
        if self.render_data.mouse_pressed {
            if let Some(prev_pos) = self.render_data.mouse_prev_pos {
                let movement = current_pos - prev_pos;
                let mut moved = false;
                match self.render_data.mouse_key {
                    MouseButton::Middle => {
                        self.scene.camera.translate(movement);
                        moved = true;
                    }
                    MouseButton::Right => {
                        self.scene.camera.orbit(movement);
                        moved = true;
                    }
                    _ => {}
                }
                if moved {
                    self.scene.camera_uniforms = self.scene.camera.to_raw();
                    self.render_data.frame_id = 0;
                    self.render_data.timer.reset();
                    self.render_data.image_dirty = true;
                    self.render_data.mouse_prev_pos = Some(current_pos);
                    log!(
                        "Camera from {:?}, at {:?}",
                        self.scene.camera.lookfrom,
                        self.scene.camera.lookat
                    );
                }
            } else {
                self.render_data.mouse_prev_pos = Some(current_pos);
            }
        }
    }

    pub fn handle_mouse_input(&mut self, mouse_state: ElementState, button: MouseButton) {
        if self.render_data.render_status == RenderStatus::Render {
            if mouse_state == ElementState::Pressed {
                if !self.egui_renderer.context().is_pointer_over_area() {
                    self.render_data.mouse_pressed = true;
                    self.render_data.mouse_key = button;
                }
            } else {
                if self.render_data.mouse_pressed && self.render_data.mouse_key == button {
                    self.render_data.mouse_pressed = false;
                    self.render_data.mouse_prev_pos = None;
                }
            }
        }
    }

    pub fn handle_mouse_wheel(&mut self, delta: MouseScrollDelta, phase: TouchPhase) {
        if self.render_data.render_status == RenderStatus::Render {
            if let MouseScrollDelta::PixelDelta(p_delta) = delta {
                self.scene.camera.zoom(p_delta.y as f32);
                self.scene.camera_uniforms = self.scene.camera.to_raw();
                self.render_data.frame_id = 0;
                self.render_data.timer.reset();
                self.render_data.image_dirty = true;
            }
        }
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

            if self.render_data.scene_config.samples_per_pixel > 0 {
                scene.camera.samples_per_pixel = self.render_data.scene_config.samples_per_pixel;
                scene.camera_uniforms = scene.camera.to_raw();
                self.queue.write_buffer(
                    &scene.camera_uniforms_buffer,
                    0,
                    bytemuck::bytes_of(&scene.camera_uniforms),
                );
            }

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

        if self.render_data.download_image {
            self.render_data.download_image = false;
            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Authoring Download Encoder"),
                });
            self.renderer_texture.download(&mut encoder);
            self.queue.submit(std::iter::once(encoder.finish()));
        }
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        self.window.request_redraw();

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
            if self.render_data.image_dirty {
                encoder.clear_buffer(&self.scene.accum_pixel_buffer, 0, None);
                self.render_data.image_dirty = false;
            }

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
