// https://github.com/kaphula/winit-egui-wgpu-template/blob/master/src/egui_tools.rs

use std::sync::Arc;

use egui::{Context, Slider};
use egui_wgpu::wgpu::{CommandEncoder, Device, Queue, StoreOp, TextureFormat, TextureView};
use egui_wgpu::{Renderer, RendererOptions, ScreenDescriptor, wgpu};
use egui_winit::State;
use winit::event::WindowEvent;
use winit::window::Window;

use crate::camera::Camera;
use crate::scene::{Scene, SceneEnum};
use crate::structure::*;

pub struct EguiRenderer {
    state: State,
    renderer: Renderer,
    frame_started: bool,
}

impl EguiRenderer {
    pub fn context(&self) -> &Context {
        self.state.egui_ctx()
    }

    pub fn new(
        device: &Device,
        output_color_format: TextureFormat,
        window: Arc<Window>,
    ) -> EguiRenderer {
        let egui_context = Context::default();

        let egui_state = egui_winit::State::new(
            egui_context,
            egui::viewport::ViewportId::ROOT,
            window.as_ref(),
            Some(window.scale_factor() as f32),
            None,
            Some(2 * 1024), // default dimension is 2048
        );

        let egui_renderer_options = RendererOptions::default();

        let egui_renderer = Renderer::new(device, output_color_format, egui_renderer_options);

        EguiRenderer {
            state: egui_state,
            renderer: egui_renderer,
            frame_started: false,
        }
    }

    pub fn handle_input(&mut self, window: Arc<Window>, event: &WindowEvent) {
        let _ = self.state.on_window_event(window.as_ref(), event);
    }

    pub fn ppp(&mut self, v: f32) {
        self.context().set_pixels_per_point(v);
    }

    pub fn begin_frame(&mut self, window: &Window) {
        let raw_input = self.state.take_egui_input(window);
        self.state.egui_ctx().begin_pass(raw_input);
        self.frame_started = true;
    }

    pub fn end_frame_and_draw(
        &mut self,
        device: &Device,
        queue: &Queue,
        encoder: &mut CommandEncoder,
        window: &Window,
        window_surface_view: &TextureView,
        screen_descriptor: ScreenDescriptor,
    ) {
        if !self.frame_started {
            panic!("begin_frame must be called before end_frame_and_draw can be called!");
        }

        self.ppp(screen_descriptor.pixels_per_point);

        let full_output = self.state.egui_ctx().end_pass();

        self.state
            .handle_platform_output(window, full_output.platform_output);

        let tris = self
            .state
            .egui_ctx()
            .tessellate(full_output.shapes, self.state.egui_ctx().pixels_per_point());
        for (id, image_delta) in &full_output.textures_delta.set {
            self.renderer
                .update_texture(device, queue, *id, image_delta);
        }
        self.renderer
            .update_buffers(device, queue, encoder, &tris, &screen_descriptor);
        let rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: window_surface_view,
                resolve_target: None,
                ops: egui_wgpu::wgpu::Operations {
                    load: egui_wgpu::wgpu::LoadOp::Load,
                    store: StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            label: Some("egui main render pass"),
            occlusion_query_set: None,
        });

        self.renderer
            .render(&mut rpass.forget_lifetime(), &tris, &screen_descriptor);
        for x in &full_output.textures_delta.free {
            self.renderer.free_texture(x)
        }

        self.frame_started = false;
    }

    pub fn render(&self, rd: &mut RenderData, scene: &Scene) {
        let rc = &mut rd.scene_config;

        match rd.render_status {
            RenderStatus::Config => {
                egui::Window::new("Ray tracing").show(self.context(), |ui| {
                    egui::Grid::new("my_grid")
                        .num_columns(2)
                        .spacing([40.0, 4.0])
                        .striped(true)
                        .show(ui, |ui| {
                            ui.label("Scene");
                            ui.horizontal(|ui| {
                                ui.selectable_value(
                                    &mut rc.scene_enum,
                                    SceneEnum::ThreeSpheres,
                                    "ThreeSpheres",
                                );
                                ui.selectable_value(
                                    &mut rc.scene_enum,
                                    SceneEnum::BouncingSpheres,
                                    "BouncingSpheres",
                                );
                                ui.selectable_value(
                                    &mut rc.scene_enum,
                                    SceneEnum::SimpleLight,
                                    "SimpleLight",
                                );
                            });
                            ui.end_row();

                            ui.label("");
                            ui.horizontal(|ui| {
                                ui.selectable_value(
                                    &mut rc.scene_enum,
                                    SceneEnum::CornellBox,
                                    "CornellBox",
                                );
                                ui.selectable_value(
                                    &mut rc.scene_enum,
                                    SceneEnum::CornellSmoke,
                                    "CornellSmoke",
                                );
                                ui.selectable_value(
                                    &mut rc.scene_enum,
                                    SceneEnum::FinalScene,
                                    "FinalScene",
                                );
                            });
                            ui.end_row();

                            ui.label("Renderer");
                            ui.horizontal(|ui| {
                                ui.selectable_value(
                                    &mut rc.renderer_type,
                                    RendererType::PT,
                                    "PathTracing",
                                );
                                ui.selectable_value(
                                    &mut rc.renderer_type,
                                    RendererType::WFPT,
                                    "Wavefront",
                                );
                            });
                            ui.end_row();

                            ui.label("Sampling Strategy");
                            ui.horizontal(|ui| {
                                ui.selectable_value(
                                    &mut rc.sampling_type,
                                    SamplingStrategy::BSDF,
                                    "Material",
                                );
                                ui.selectable_value(
                                    &mut rc.sampling_type,
                                    SamplingStrategy::Light,
                                    "Light",
                                );
                                ui.selectable_value(
                                    &mut rc.sampling_type,
                                    SamplingStrategy::MIS,
                                    "Multiple Importance Sampling",
                                );
                            });
                            ui.end_row();

                            ui.label("Samples Per Pixel");
                            ui.add(Slider::new(&mut rc.samples_per_pixel, 0..=1000));
                            ui.end_row();

                            if ui.button("Render").clicked() {
                                rd.update_config = true;
                            }
                        });
                });
            }
            RenderStatus::Render => {
                egui::Window::new("Ray tracing").show(self.context(), |ui| {
                    egui::Grid::new("my_grid")
                        .num_columns(1)
                        .spacing([40.0, 4.0])
                        .striped(true)
                        .show(ui, |ui| {
                            let camera = &scene.camera;

                            let cur_samples = u32::min(
                                rd.frame_id * camera.samples_per_frame,
                                camera.samples_per_pixel,
                            );

                            if cur_samples < camera.samples_per_pixel {
                                rd.timer.start();
                                ui.label(format!(
                                    "Samples {}/{}",
                                    cur_samples, camera.samples_per_pixel
                                ));
                                ui.end_row();

                                ui.label(format!("Render time: {:.2}ms", rd.timer.elapsed()));
                                ui.end_row();
                                ui.label(format!(
                                    "Avg frame time: {:.2}ms",
                                    rd.timer.elapsed() / (rd.frame_id as f64 - 1.0)
                                ));
                                // log!("Samples {}/{}", cur_samples, rd.samples_per_pixel);
                                // log!("Render time: {}", rd.timer.elapsed() / rd.frame_id as f64);
                            } else {
                                rd.timer.pause();
                                ui.label(format!(
                                    "Samples {}/{}",
                                    cur_samples, camera.samples_per_pixel
                                ));
                                ui.end_row();

                                ui.label(format!("Render time: {:.2}ms", rd.timer.elapsed()));
                                ui.end_row();
                                ui.label(format!(
                                    "Avg frame time: {:.2}ms",
                                    rd.timer.elapsed()
                                        / (camera.samples_per_pixel as f64
                                            / camera.samples_per_frame as f64)
                                            .ceil()
                                ));
                            }

                            if ui.button("Config").clicked() {
                                rd.render_status = RenderStatus::Config;
                            }
                        });
                });
            }
        }
    }
}
