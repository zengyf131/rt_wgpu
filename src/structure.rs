use std::{collections::HashMap, rc::Rc};
use winit::event::MouseButton;

use crate::material::{Material, MaterialRaw};
use crate::primitive::{Primitive, PrimitiveRaw};
use crate::scene::{Scene, SceneEnum};
use crate::texture::{Texture, TextureRaw};
use crate::utils::*;

pub trait Renderer {
    fn render(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        queue: &wgpu::Queue,
        view: &wgpu::TextureView,
        scene: &Scene,
        rd: &mut RenderData,
    );

    fn print(&self, encoder: &mut wgpu::CommandEncoder);
}

pub struct RenderData {
    pub frame_id: u32,
    pub timer: Timer,

    pub update_config: bool,
    pub render_status: RenderStatus,
    pub scene_config: SceneConfig,

    pub mouse_pressed: bool,
    pub mouse_key: MouseButton,
    pub mouse_prev_pos: Option<Vec2>,
    pub image_dirty: bool,

    pub download_image: bool,
}
impl RenderData {
    pub fn new() -> Self {
        Self {
            frame_id: 0,
            timer: Timer::new(),

            update_config: false,
            render_status: RenderStatus::Config,
            scene_config: SceneConfig::new(),

            mouse_pressed: false,
            mouse_key: MouseButton::Left,
            mouse_prev_pos: None,
            image_dirty: false,

            download_image: false,
        }
    }

    pub fn reset(&mut self) {
        self.frame_id = 0;
        self.timer.reset();
        self.update_config = false;
        self.mouse_pressed = false;
        self.mouse_prev_pos = None;
    }
}

#[derive(PartialEq)]
pub enum RenderStatus {
    Config,
    Render,
}

pub struct SceneConfig {
    pub scene_enum: SceneEnum,
    pub renderer_type: RendererType,
    pub sampling_type: SamplingStrategy,
    pub samples_per_pixel: u32,
}
impl SceneConfig {
    fn new() -> Self {
        Self {
            scene_enum: SceneEnum::CornellBox,
            renderer_type: RendererType::PT,
            sampling_type: SamplingStrategy::BSDF,
            samples_per_pixel: 0,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum RendererType {
    PT,
    WFPT,
}

#[derive(Clone, Copy, PartialEq)]
pub enum SamplingStrategy {
    BSDF,
    Light,
    MIS,
}

pub struct RawVec {
    pub primitives: Vec<PrimitiveRaw>,
    pub materials: Vec<MaterialRaw>,
    pub textures: Vec<TextureRaw>,
    pub tex_data: Vec<f32>,

    pub materials_hash: HashMap<*const (), usize>,
    pub textures_hash: HashMap<*const (), usize>,
}
impl RawVec {
    pub fn new() -> Self {
        Self {
            primitives: Vec::new(),
            materials: Vec::new(),
            textures: Vec::new(),
            tex_data: Vec::new(),

            materials_hash: HashMap::new(),
            textures_hash: HashMap::new(),
        }
    }

    pub fn register_material(&mut self, mat: &Rc<dyn Material>) -> usize {
        let mat_id: usize;
        let mat_key = Rc::as_ptr(mat).cast::<()>();
        if let Some(&id) = self.materials_hash.get(&mat_key) {
            mat_id = id;
        } else {
            mat_id = mat.to_raw(self);
            self.materials_hash.insert(mat_key, mat_id);
        }

        mat_id
    }

    pub fn register_texture(&mut self, tex: &Rc<dyn Texture>) -> usize {
        let tex_id: usize;
        let tex_key = Rc::as_ptr(tex).cast::<()>();
        if let Some(&id) = self.textures_hash.get(&tex_key) {
            tex_id = id;
        } else {
            tex_id = tex.to_raw(self);
            self.textures_hash.insert(tex_key, tex_id);
        }

        tex_id
    }
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SceneUniforms {
    pub surface_wh: [u32; 2],
    pub root_id: u32,
    pub light_id: i32,
    pub renderer_type: u32,
    pub sampling_type: u32,
    pub _pad0: [u32; 2],
}
impl SceneUniforms {
    pub fn from_scene(root_id: u32, light_id: i32) -> Self {
        Self {
            surface_wh: [0, 0],
            root_id,
            light_id,
            renderer_type: 0,
            sampling_type: 0,
            _pad0: [0; 2],
        }
    }

    pub fn update(&mut self, config: &wgpu::SurfaceConfiguration, rd: &RenderData) {
        self.surface_wh = [config.width, config.height];
        self.renderer_type = rd.scene_config.renderer_type as u32;
        self.sampling_type = rd.scene_config.sampling_type as u32;
    }
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, Debug)]
pub struct Interval {
    pub min: f32,
    pub max: f32,
}
impl Interval {
    fn expand(&self, delta: f32) -> Self {
        let padding = delta / 2.0;

        Self {
            min: self.min - padding,
            max: self.max - padding,
        }
    }

    fn merge_intervals(a: Self, b: Self) -> Self {
        let min = f32::min(a.min, b.min);
        let max = f32::max(a.max, b.max);
        Self { min, max }
    }

    fn empty() -> Self {
        Self {
            min: f32::MAX,
            max: f32::MIN,
        }
    }

    fn size(self) -> f32 {
        self.max - self.min
    }
}
impl std::ops::Add<f32> for Interval {
    type Output = Self;

    fn add(self, rhs: f32) -> Self::Output {
        Self {
            min: self.min + rhs,
            max: self.max + rhs,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, Debug)]
pub struct AABB {
    pub x: Interval,
    pub y: Interval,
    pub z: Interval,
}
impl AABB {
    pub fn from_points(a: Vec3, b: Vec3) -> Self {
        let x = if a.x <= b.x {
            Interval { min: a.x, max: b.x }
        } else {
            Interval { min: b.x, max: a.x }
        };
        let y = if a.y <= b.y {
            Interval { min: a.y, max: b.y }
        } else {
            Interval { min: b.y, max: a.y }
        };
        let z = if a.z <= b.z {
            Interval { min: a.z, max: b.z }
        } else {
            Interval { min: b.z, max: a.z }
        };

        let mut aabb = Self { x, y, z };
        aabb.pad_to_minimums();

        aabb
    }

    pub fn merge_aabbs(a: Self, b: Self) -> Self {
        let x = Interval::merge_intervals(a.x, b.x);
        let y = Interval::merge_intervals(a.y, b.y);
        let z = Interval::merge_intervals(a.z, b.z);

        Self { x, y, z }
    }

    pub fn empty() -> Self {
        Self {
            x: Interval::empty(),
            y: Interval::empty(),
            z: Interval::empty(),
        }
    }

    pub fn axis_interval(self, axis: u32) -> Interval {
        if axis == 1 {
            return self.y;
        } else if axis == 2 {
            return self.z;
        }
        return self.x;
    }

    pub fn longest_axis(self) -> u32 {
        if self.x.size() > self.y.size() {
            if self.x.size() > self.z.size() {
                return 0;
            } else {
                return 2;
            }
        } else {
            if self.y.size() > self.z.size() {
                return 1;
            } else {
                return 2;
            }
        }
    }

    fn pad_to_minimums(&mut self) {
        let delta: f32 = 0.0001;
        if self.x.size() < delta {
            self.x = self.x.expand(delta);
        }
        if self.y.size() < delta {
            self.y = self.y.expand(delta);
        }
        if self.z.size() < delta {
            self.z = self.z.expand(delta);
        }
    }
}
impl std::ops::Add<Vec3> for AABB {
    type Output = Self;

    fn add(self, rhs: Vec3) -> Self::Output {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            z: self.z + rhs.z,
        }
    }
}
