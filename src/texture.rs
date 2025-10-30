use image::GenericImageView;

use crate::structure::*;
use crate::utils::*;

pub trait Texture {
    fn to_raw(&mut self, raw_vec: &mut RawVec) -> usize;
    fn tex_id(&self) -> i32;
}

pub struct SolidColor {
    tex_id: i32,
    albedo: Vec3,
}
impl SolidColor {
    pub fn new(albedo: Vec3) -> Self {
        Self {
            tex_id: -1,
            albedo,
        }
    }
}
impl Texture for SolidColor {
    fn to_raw(&mut self, raw_vec: &mut RawVec) -> usize {
        let data_start = raw_vec.tex_data.len() as u32;
        let this_raw = TextureRaw {
            type_id: 0,
            start: data_start,
            end: data_start,
            _pad0: [0],
            
            data0: [
                self.albedo.x,
                self.albedo.y,
                self.albedo.z,
                0.0,
            ],
        };

        self.tex_id = raw_vec.textures.len() as i32;
        raw_vec.textures.push(this_raw);
        return self.tex_id as usize;
    }

    fn tex_id(&self) -> i32 {
        self.tex_id
    }
}

pub struct CheckerTexture {
    tex_id: i32,
    inv_scale: f32,
    even: Box<dyn Texture>,
    odd: Box<dyn Texture>,
}
impl CheckerTexture {
    pub fn from_colors(scale: f32, c1: Vec3, c2: Vec3) -> Self {
        Self {
            tex_id: -1,
            inv_scale: 1.0 / scale,
            even: Box::new(SolidColor::new(c1)),
            odd: Box::new(SolidColor::new(c2)),
        }
    }
}
impl Texture for CheckerTexture {
    fn to_raw(&mut self, raw_vec: &mut RawVec) -> usize {
        if self.even.tex_id() < 0 {
            self.even.to_raw(raw_vec);
        }
        if self.odd.tex_id() < 0 {
            self.odd.to_raw(raw_vec);
        }

        let data_start = raw_vec.tex_data.len() as u32;
        let this_raw = TextureRaw {
            type_id: 1,
            start: data_start,
            end: data_start,
            _pad0: [0],

            data0: [
                self.even.tex_id() as f32,
                self.odd.tex_id() as f32,
                self.inv_scale,
                0.0,
            ],
        };

        self.tex_id = raw_vec.textures.len() as i32;
        raw_vec.textures.push(this_raw);
        return self.tex_id as usize;
    }

    fn tex_id(&self) -> i32 {
        self.tex_id
    }
}

pub struct ImageTexture {
    tex_id: i32,
    image: image::DynamicImage,
}
impl ImageTexture {
    pub fn from_bytes(bytes: &[u8]) -> Self {
        Self {
            tex_id: -1,
            image: image::load_from_memory(bytes).expect("Error loading image"),
        }
    }
}
impl Texture for ImageTexture {
    fn to_raw(&mut self, raw_vec: &mut RawVec) -> usize {
        let img_w = self.image.dimensions().0;
        let img_h = self.image.dimensions().1;
        let data_start = raw_vec.tex_data.len() as u32;
        let data_end = data_start + img_w * img_h;
        raw_vec.tex_data.extend_from_slice(self.image.to_rgba32f().into_raw().as_slice());

        let this_raw = TextureRaw {
            type_id: 2,
            start: data_start,
            end: data_end,
            _pad0: [0],

            data0: [
                img_w as f32,
                img_h as f32,
                0.0,
                0.0,
            ],
        };

        self.tex_id = raw_vec.textures.len() as i32;
        raw_vec.textures.push(this_raw);
        return self.tex_id as usize;
    }

    fn tex_id(&self) -> i32 {
        self.tex_id
    }
}