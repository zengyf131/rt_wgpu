use std::{rc::Rc, cell::RefCell};

use crate::texture::{Texture, SolidColor};
use crate::structure::*;
use crate::utils::*;

pub trait Material {
    fn to_raw(&mut self, raw_vec: &mut RawVec) -> usize;
    fn mat_id(&self) -> i32;
}

pub struct Lambertian {
    mat_id: i32,
    tex: Rc<RefCell<dyn Texture>>,
}
impl Lambertian {
    pub fn from_color(albedo: Vec3) -> Self {
        Self {
            mat_id: -1,
            tex: Rc::new(RefCell::new(SolidColor::new(albedo))),
        }
    }

    pub fn new(tex: Rc<RefCell<dyn Texture>>) -> Self {
        Self {
            mat_id: -1,
            tex,
        }
    }
}
impl Material for Lambertian {
    fn to_raw(&mut self, raw_vec: &mut RawVec) -> usize {
        if self.tex.borrow().tex_id() < 0 {
            self.tex.borrow_mut().to_raw(raw_vec);
        }

        let this_raw = MaterialRaw {
            type_id: 0,
            tex_id: self.tex.borrow().tex_id(),
            _pad0: [0; 2],
            data0: [0.0; 4],
        };

        self.mat_id = raw_vec.materials.len() as i32;
        raw_vec.materials.push(this_raw);
        return raw_vec.materials.len() - 1;
    }

    fn mat_id(&self) -> i32 {
        self.mat_id
    }
}

pub struct Metal {
    mat_id: i32,
    albedo: Vec3,
    fuzz: f32,
}
impl Metal {
    pub fn new(albedo: Vec3, fuzz: f32) -> Self {
        Self {
            mat_id: -1,
            albedo,
            fuzz: f32::min(fuzz, 1.0),
        }
    }
}
impl Material for Metal {
    fn to_raw(&mut self, raw_vec: &mut RawVec) -> usize {
        let this_raw = MaterialRaw {
            type_id: 1,
            tex_id: -1,
            _pad0: [0; 2],
            data0: [self.albedo.x, self.albedo.y, self.albedo.z, self.fuzz],
        };

        self.mat_id = raw_vec.materials.len() as i32;
        raw_vec.materials.push(this_raw);
        return raw_vec.materials.len() - 1;
    }

    fn mat_id(&self) -> i32 {
        self.mat_id
    }
}

pub struct Dielectric {
    mat_id: i32,
    refraction_index: f32,
}
impl Dielectric {
    pub fn new(refraction_index: f32) -> Self {
        Self {
            mat_id: -1,
            refraction_index,
        }
    }
}
impl Material for Dielectric {
    fn to_raw(&mut self, raw_vec: &mut RawVec) -> usize {
        let this_raw = MaterialRaw {
            type_id: 2,
            tex_id: -1,
            _pad0: [0; 2],
            data0: [self.refraction_index, 0.0, 0.0, 0.0],
        };

        self.mat_id = raw_vec.materials.len() as i32;
        raw_vec.materials.push(this_raw);
        return raw_vec.materials.len() - 1;
    }

    fn mat_id(&self) -> i32 {
        self.mat_id
    }
}

pub struct DiffuseLight {
    mat_id: i32,
    tex: Rc<RefCell<dyn Texture>>,
}
impl DiffuseLight {
    pub fn new(tex: Rc<RefCell<dyn Texture>>) -> Self {
        Self {
            mat_id: -1,
            tex,
        }
    }

    pub fn from_color(emit: Vec3) -> Self {
        Self {
            mat_id: -1,
            tex: Rc::new(RefCell::new(SolidColor::new(emit))),
        }
    }
}
impl Material for DiffuseLight {
    fn to_raw(&mut self, raw_vec: &mut RawVec) -> usize {
        if self.tex.borrow().tex_id() < 0 {
            self.tex.borrow_mut().to_raw(raw_vec);
        }

        let this_raw = MaterialRaw {
            type_id: 3,
            tex_id: self.tex.borrow().tex_id(),
            _pad0: [0; 2],
            data0: [0.0; 4],
        };

        self.mat_id = raw_vec.materials.len() as i32;
        raw_vec.materials.push(this_raw);
        return raw_vec.materials.len() - 1;
    }

    fn mat_id(&self) -> i32 {
        self.mat_id
    }
}