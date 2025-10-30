use winit::{
    event::*,
    keyboard::{KeyCode, PhysicalKey},
};
use cgmath::{Vector3, prelude::*, vec3};

pub struct Camera {
    pub image_width: u32,
    pub image_height: u32,
    pub samples_per_pixel: u32,
    pub max_depth: u32,
    pub samples_per_frame: u32,
    pub vfov: f32,
    pub lookfrom: Vector3<f32>,
    pub lookat: Vector3<f32>,
    pub vup: Vector3<f32>,
    pub defocus_angle: f32,
    pub focus_dist: f32,
}
impl Camera {
    pub fn to_raw(&self) -> CameraUniforms {
        let aspect_ratio = self.image_width as f32 / self.image_height as f32;
        let center = self.lookfrom;
        let theta = self.vfov.to_radians();
        let h = f32::tan(theta / 2.0);
        let viewport_height: f32 = 2.0 * h * self.focus_dist;
        let viewport_width: f32 = viewport_height * aspect_ratio;

        let w = (self.lookfrom - self.lookat).normalize();
        let u = self.vup.cross(w).normalize();
        let v = w.cross(u);
        let viewport_u = viewport_width * u;
        let viewport_v = viewport_height * -v;
        let pixel_delta_u = viewport_u / self.image_width as f32;
        let pixel_delta_v = viewport_v / self.image_height as f32;
        let viewport_upper_left = center - (self.focus_dist * w) - viewport_u / 2.0 - viewport_v / 2.0;
        let pixel00_loc = viewport_upper_left + 0.5 * (pixel_delta_u + pixel_delta_v);

        let defocus_radius = self.focus_dist * f32::tan((self.defocus_angle / 2.0).to_radians());
        let defocus_disk_u = u * defocus_radius;
        let defocus_disk_v = v * defocus_radius;

        CameraUniforms {
            image_wh: [self.image_width, self.image_height],
            samples_per_pixel: self.samples_per_pixel,
            max_depth: self.max_depth,
            frame_id: 0,
            samples_per_frame: self.samples_per_frame,
            defocus_angle: self.defocus_angle,
            _padding: [0.0],
            center: [center.x, center.y, center.z, 0.0],
            pixel_delta_u: [pixel_delta_u.x, pixel_delta_u.y, pixel_delta_u.z, 0.0],
            pixel_delta_v: [pixel_delta_v.x, pixel_delta_v.y, pixel_delta_v.z, 0.0],
            pixel00_loc: [pixel00_loc.x, pixel00_loc.y, pixel00_loc.z, 0.0],
            defocus_disk_u: [defocus_disk_u.x, defocus_disk_u.y, defocus_disk_u.z, 0.0],
            defocus_disk_v: [defocus_disk_v.x, defocus_disk_v.y, defocus_disk_v.z, 0.0],
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniforms {
    pub image_wh: [u32; 2],
    pub samples_per_pixel: u32,
    pub max_depth: u32,
    pub frame_id: u32,
    pub samples_per_frame: u32,
    pub defocus_angle: f32,
    pub _padding: [f32; 1],

    pub center: [f32; 4],
    pub pixel_delta_u: [f32; 4],
    pub pixel_delta_v: [f32; 4],
    pub pixel00_loc: [f32; 4],
    pub defocus_disk_u: [f32; 4],
    pub defocus_disk_v: [f32; 4],
}