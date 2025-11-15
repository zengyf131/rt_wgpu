use crate::utils::*;

pub trait Renderer {
    fn render(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        queue: &wgpu::Queue,
        view: &wgpu::TextureView,
        rd: &mut RenderData,
    );
}

pub struct RenderData {
    pub frame_id: u32,
    pub timer: Timer,
}
impl RenderData {
    pub fn new() -> Self {
        Self {
            frame_id: 0,
            timer: Timer::new(),
        }
    }
}

pub struct RawVec {
    pub primitives: Vec<PrimitiveRaw>,
    pub materials: Vec<MaterialRaw>,
    pub textures: Vec<TextureRaw>,
    pub tex_data: Vec<f32>,
}
impl RawVec {
    pub fn new() -> Self {
        Self {
            primitives: Vec::new(),
            materials: Vec::new(),
            textures: Vec::new(),
            tex_data: Vec::new(),
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, Debug)]
pub struct PrimitiveRaw {
    pub type_id: u32, // 0: bvh, 1: sphere
    pub mat_id: i32,
    pub left_child_id: i32,
    pub right_child_id: i32,
    pub next_elem_id: i32,
    pub aabb: AABB,
    pub _pad: [i32; 1],

    pub data0: [f32; 4],
    pub data1: [f32; 4],
    pub data2: [f32; 4],
    pub data3: [f32; 4],
    pub data4: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, Debug)]
pub struct MaterialRaw {
    pub type_id: u32,
    pub tex_id: i32,
    pub _pad0: [u32; 2],

    pub data0: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, Debug)]
pub struct TextureRaw {
    pub type_id: u32,
    pub start: u32,
    pub end: u32,
    pub _pad0: [u32; 1],

    pub data0: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SceneUniforms {
    pub renderer_type: u32,
    pub root_id: u32,
    pub light_id: i32,
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
