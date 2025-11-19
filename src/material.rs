use std::{cell::RefCell, rc::Rc};

use crate::structure::*;
use crate::texture::{SolidColor, Texture};
use crate::utils::*;

pub trait Material {
    fn to_raw(&self, raw_vec: &mut RawVec) -> usize;
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, Debug)]
pub struct MaterialRaw {
    pub type_id: u32,
    pub tex_id: i32,
    pub _pad0: [u32; 2],

    pub data0: [f32; 4],
}

pub struct Lambertian {
    tex: Rc<dyn Texture>,
}
impl Lambertian {
    pub fn from_color(albedo: Vec3) -> Rc<Self> {
        Rc::new(Self {
            tex: SolidColor::new(albedo),
        })
    }

    pub fn new(tex: Rc<dyn Texture>) -> Rc<Self> {
        Rc::new(Self { tex })
    }
}
impl Material for Lambertian {
    fn to_raw(&self, raw_vec: &mut RawVec) -> usize {
        let tex_id = raw_vec.register_texture(&self.tex);

        let this_raw = MaterialRaw {
            type_id: 0,
            tex_id: tex_id as i32,
            _pad0: [0; 2],
            data0: [0.0; 4],
        };

        raw_vec.materials.push(this_raw);
        return raw_vec.materials.len() - 1;
    }
}

pub struct Metal {
    albedo: Vec3,
    fuzz: f32,
}
impl Metal {
    pub fn new(albedo: Vec3, fuzz: f32) -> Rc<Self> {
        Rc::new(Self {
            albedo,
            fuzz: f32::min(fuzz, 1.0),
        })
    }
}
impl Material for Metal {
    fn to_raw(&self, raw_vec: &mut RawVec) -> usize {
        let this_raw = MaterialRaw {
            type_id: 1,
            tex_id: -1,
            _pad0: [0; 2],
            data0: [self.albedo.x, self.albedo.y, self.albedo.z, self.fuzz],
        };

        raw_vec.materials.push(this_raw);
        return raw_vec.materials.len() - 1;
    }
}

pub struct Dielectric {
    mat_id: i32,
    refraction_index: f32,
}
impl Dielectric {
    pub fn new(refraction_index: f32) -> Rc<Self> {
        Rc::new(Self {
            mat_id: -1,
            refraction_index,
        })
    }
}
impl Material for Dielectric {
    fn to_raw(&self, raw_vec: &mut RawVec) -> usize {
        let this_raw = MaterialRaw {
            type_id: 2,
            tex_id: -1,
            _pad0: [0; 2],
            data0: [self.refraction_index, 0.0, 0.0, 0.0],
        };

        raw_vec.materials.push(this_raw);
        return raw_vec.materials.len() - 1;
    }
}

pub struct DiffuseLight {
    tex: Rc<dyn Texture>,
}
impl DiffuseLight {
    pub fn new(tex: Rc<dyn Texture>) -> Rc<Self> {
        Rc::new(Self { tex })
    }

    pub fn from_color(emit: Vec3) -> Rc<Self> {
        Rc::new(Self {
            tex: SolidColor::new(emit),
        })
    }
}
impl Material for DiffuseLight {
    fn to_raw(&self, raw_vec: &mut RawVec) -> usize {
        let tex_id = raw_vec.register_texture(&self.tex);

        let this_raw = MaterialRaw {
            type_id: 3,
            tex_id: tex_id as i32,
            _pad0: [0; 2],
            data0: [0.0; 4],
        };

        raw_vec.materials.push(this_raw);
        return raw_vec.materials.len() - 1;
    }
}

pub struct Isotropic {
    tex: Rc<dyn Texture>,
}
impl Isotropic {
    pub fn new(tex: Rc<dyn Texture>) -> Rc<Self> {
        Rc::new(Self { tex })
    }

    pub fn from_color(albedo: Vec3) -> Rc<Self> {
        Rc::new(Self {
            tex: SolidColor::new(albedo),
        })
    }
}
impl Material for Isotropic {
    fn to_raw(&self, raw_vec: &mut RawVec) -> usize {
        let tex_id = raw_vec.register_texture(&self.tex);

        let this_raw = MaterialRaw {
            type_id: 4,
            tex_id: tex_id as i32,
            _pad0: [0; 2],
            data0: [0.0; 4],
        };

        raw_vec.materials.push(this_raw);
        return raw_vec.materials.len() - 1;
    }
}
