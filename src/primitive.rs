use std::{rc::Rc, cell::RefCell};

use crate::material::Material;
use crate::structure::*;
use crate::utils::*;

pub trait Primitive {
    fn to_raw(&mut self, raw_vec: &mut RawVec) -> usize;
    fn aabb(&self) -> AABB;
}

pub struct Sphere {
    center: Vec3,
    center_dir: Vec3,
    radius: f32,
    mat: Rc<RefCell<dyn Material>>,
    aabb: AABB,
}
impl Sphere {
    pub fn sphere(center: Vec3, radius: f32, mat: Rc<RefCell<dyn Material>>) -> Self {
        let rvec = vec3(radius, radius, radius);
        Self {
            center,
            center_dir: vec3(0.0, 0.0, 0.0),
            radius,
            mat,
            aabb: AABB::from_points(center - rvec, center + rvec),
        }
    }

    pub fn sphere_moving(center1: Vec3, center2: Vec3, radius: f32, mat: Rc<RefCell<dyn Material>>) -> Self {
        let rvec = vec3(radius, radius, radius);
        let box1 = AABB::from_points(center1 - rvec, center1 + rvec);
        let box2 = AABB::from_points(center2 - rvec, center2 + rvec);
        Self {
            center: center1,
            center_dir: center2 - center1,
            radius,
            mat,
            aabb: AABB::merge_aabbs(box1, box2),
        }
    }
}
impl Primitive for Sphere {
    fn to_raw(&mut self, raw_vec: &mut RawVec) -> usize {
        if self.mat.borrow().mat_id() < 0 {
            let _ = self.mat.borrow_mut().to_raw(raw_vec);
        }
        let this_raw = PrimitiveRaw {
            data0: [self.center.x, self.center.y, self.center.z, self.radius],
            data1: [self.center_dir.x, self.center_dir.y, self.center_dir.z, 0.0],

            type_id: 1,
            mat_id: self.mat.borrow().mat_id(),
            left_child_id: -1,
            right_child_id: -1,
            next_elem_id: -1,
            aabb: self.aabb,
            _pad: [0; 1],
        };

        raw_vec.primitives.push(this_raw);
        return raw_vec.primitives.len() - 1;
    }

    fn aabb(&self) -> AABB {
        self.aabb
    }
}

pub struct PrimitiveList {
    prim_list: Vec<Box<dyn Primitive>>,
    aabb: AABB,
}
impl PrimitiveList {
    pub fn new() -> Self {
        Self {
            prim_list: Vec::new(),
            aabb: AABB::empty(),
        }
    }

    pub fn clear(&mut self) {
        self.prim_list.clear();
    }

    pub fn add(&mut self, prim: Box<dyn Primitive>) {
        self.aabb = AABB::merge_aabbs(self.aabb, prim.aabb());
        self.prim_list.push(prim);
    }
}
impl Primitive for PrimitiveList {
    fn to_raw(&mut self, raw_vec: &mut RawVec) -> usize {
        let mut first_pid: usize = 0;
        let mut prev_pid: usize = 0;
        for i in 0..self.prim_list.len() {
            let prim = &mut self.prim_list[i];
            let pid = prim.to_raw(raw_vec);
            if i > 0 {
                let prev_raw = &mut raw_vec.primitives[prev_pid];
                prev_raw.next_elem_id = pid as i32;
            } else {
                first_pid = pid;
            }
            prev_pid = pid;
        }

        return first_pid;
    }

    fn aabb(&self) -> AABB {
        self.aabb
    }
}

pub struct BVHNode {
    left: Option<Box<dyn Primitive>>,
    right: Option<Box<dyn Primitive>>,
    aabb: AABB,
}
impl BVHNode {
    pub fn from_prim_list(prim_list: PrimitiveList) -> Self {
        Self::from_vec(prim_list.prim_list)
    }

    pub fn from_vec(mut prim_vec: Vec<Box<dyn Primitive>>) -> Self {
        let mut aabb = AABB::empty();
        for prim in prim_vec.iter() {
            aabb = AABB::merge_aabbs(aabb, prim.aabb());
        }

        let vec_len = prim_vec.len();
        let left: Option<Box<dyn Primitive>>;
        let right: Option<Box<dyn Primitive>>;
        if vec_len == 1 {
            left = None;
            right = Some(prim_vec.pop().unwrap());
        } else if vec_len == 2 {
            right = Some(prim_vec.pop().unwrap());
            left = Some(prim_vec.pop().unwrap());
        } else {
            let axis: u32 = aabb.longest_axis();
            let box_compare = |a: &Box<dyn Primitive>, b: &Box<dyn Primitive>, axis_index: u32| {
                let a_axis_interval = a.aabb().axis_interval(axis_index);
                let b_axis_interval = b.aabb().axis_interval(axis_index);
                return a_axis_interval.min.total_cmp(&b_axis_interval.min);
            };
            let comparator = |a: &Box<dyn Primitive>, b: &Box<dyn Primitive>| { box_compare(a, b, axis) };
            prim_vec.sort_by(comparator);

            let mid = vec_len / 2;
            let right_vec = prim_vec.split_off(mid);
            left = Some(Box::new(Self::from_vec(prim_vec)));
            right = Some(Box::new(Self::from_vec(right_vec)));
        }

        Self {
            left,
            right,
            aabb,
        }
    }
}
impl Primitive for BVHNode {
    fn to_raw(&mut self, raw_vec: &mut RawVec) -> usize {
        let this_pid = raw_vec.primitives.len();
        let this_raw = PrimitiveRaw {
            data0: [0.0; 4],
            data1: [0.0; 4],

            type_id: 0,
            mat_id: -1,
            left_child_id: -1,
            right_child_id: -1,
            next_elem_id: -1,
            aabb: self.aabb,
            _pad: [0; 1],
        };

        raw_vec.primitives.push(this_raw);

        let left_id = if let Some(left_child) = &mut self.left {
            left_child.to_raw(raw_vec) as i32
        } else {
            -1
        };
        let right_id = if let Some(right_child) = &mut self.right {
            right_child.to_raw(raw_vec) as i32
        } else {
            -1
        };

        let this_raw = &mut raw_vec.primitives[this_pid];
        this_raw.left_child_id = left_id;
        this_raw.right_child_id = right_id;

        this_pid
    }

    fn aabb(&self) -> AABB {
        self.aabb
    }
}
