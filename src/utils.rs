use wasm_bindgen::prelude::*;

pub use cgmath::{
    Angle, EuclideanSpace, InnerSpace, Matrix, MetricSpace, One, Rotation, Rotation2, Rotation3,
    SquareMatrix, Transform, Transform2, Transform3, VectorSpace, Zero,
};
pub use cgmath::{
    Deg, Matrix2, Matrix3, Matrix4, Point2, Point3, Quaternion, Rad, Vector2, Vector3, Vector4,
    dot, frustum, ortho, perspective, vec2, vec3, vec4,
};

pub type Vec3 = Vector3<f32>;
pub type Vec2 = Vector2<f32>;
pub type Mat3 = Matrix3<f32>;
pub type Mat4 = Matrix4<f32>;

pub type Degrees = Deg<f32>;
pub type Radians = Rad<f32>;

#[macro_export]
macro_rules! log {
    ( $( $t:tt )* ) => {
        web_sys::console::log_1(&format!( $( $t )* ).into());
    }
}

#[wasm_bindgen(module = "/src/helper.js")]
extern "C" {
    pub fn get_time_milliseconds() -> f64;
}

pub const fn degrees<T>(v: T) -> Deg<T> {
    cgmath::Deg(v)
}

pub const fn radians<T>(v: T) -> Rad<T> {
    cgmath::Rad(v)
}

pub struct Timer {
    start_time: Option<f64>,
    accumulated: f64,
    paused: bool,
}
impl Timer {
    pub fn new() -> Self {
        Self {
            start_time: None,
            accumulated: 0.0,
            paused: true,
        }
    }

    pub fn start(&mut self) {
        if self.paused {
            self.start_time = Some(get_time_milliseconds());
            self.paused = false;
        }
    }

    pub fn pause(&mut self) {
        if !self.paused {
            if let Some(start) = self.start_time {
                self.accumulated += get_time_milliseconds() - start;
            }
            self.start_time = None;
            self.paused = true;
        }
    }

    pub fn reset(&mut self) {
        self.start_time = None;
        self.accumulated = 0.0;
        self.paused = true;
    }

    pub fn elapsed(&self) -> f64 {
        if self.paused {
            self.accumulated
        } else {
            if let Some(start) = self.start_time {
                self.accumulated + (get_time_milliseconds() - start)
            } else {
                self.accumulated
            }
        }
    }

    pub fn is_paused(&self) -> bool {
        self.paused
    }
}
