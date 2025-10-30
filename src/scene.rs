use std::cell::RefCell;
use std::rc::Rc;

use rand::{Rng, SeedableRng, rngs::StdRng};
use cgmath::prelude::*;

use crate::camera::Camera;
use crate::material::*;
use crate::primitive::*;
use crate::texture::*;
use crate::utils::*;

pub fn get_world_0() -> (Camera, Box<dyn Primitive>) {

    let camera = Camera {
        image_width: 1920,
        image_height: 1080,
        samples_per_pixel: 500,
        max_depth: 50,
        samples_per_frame: 1,
        vfov: 20.0,
        lookfrom: vec3(-2.0, 2.0, 1.0),
        lookat: vec3(0.0, 0.0, -1.0),
        vup: vec3(0.0, 1.0, 0.0),
        defocus_angle: 0.0,
        focus_dist: 3.4,
    };

    let mut world = PrimitiveList::new();

    let material_ground = Rc::new(RefCell::new(Lambertian::from_color(vec3(0.8, 0.8, 0.0))));
    let material_center = Rc::new(RefCell::new(Lambertian::from_color(vec3(0.1, 0.2, 0.5))));
    let material_left = Rc::new(RefCell::new(Dielectric::new(1.50)));
    let material_bubble = Rc::new(RefCell::new(Dielectric::new(1.00 / 1.50)));
    let material_right = Rc::new(RefCell::new(Metal::new(vec3(0.8, 0.6, 0.2), 1.0)));

    world.add(Box::new(Sphere::sphere(vec3( 0.0, -100.5, -1.0), 100.0, material_ground)));
    world.add(Box::new(Sphere::sphere(vec3( 0.0,    0.0, -1.2),   0.5, material_center)));
    world.add(Box::new(Sphere::sphere(vec3(-1.0,    0.0, -1.0),   0.5, material_left)));
    world.add(Box::new(Sphere::sphere(vec3(-1.0,    0.0, -1.0),   0.4, material_bubble)));
    world.add(Box::new(Sphere::sphere(vec3( 1.0,    0.0, -1.0),   0.5, material_right)));

    (camera, Box::new(world))
}

pub fn bouncing_spheres() -> (Camera, Box<dyn Primitive>) {

    let camera = Camera {
        image_width: 1920,
        image_height: 1080,
        samples_per_pixel: 50,
        max_depth: 50,
        samples_per_frame: 1,
        vfov: 20.0,
        lookfrom: vec3(13.0, 2.0, 3.0),
        lookat: vec3(0.0, 0.0, 0.0),
        vup: vec3(0.0, 1.0, 0.0),
        defocus_angle: 0.6,
        focus_dist: 10.0,
    };

    let mut world = PrimitiveList::new();
    
    let mut rng = StdRng::seed_from_u64(0);
    let checker = Rc::new(RefCell::new(CheckerTexture::from_colors(0.32, vec3(0.2, 0.3, 0.1), vec3(0.9, 0.9, 0.9))));
    world.add(Box::new(Sphere::sphere(vec3(0.0, -1000.0, 0.0), 1000.0, Rc::new(RefCell::new(Lambertian::new(checker))))));

    let random_vec3 = |rng: &mut StdRng| {
        return Vector3::<f32>::new(
            rng.random_range(0.0..1.0),
            rng.random_range(0.0..1.0),
            rng.random_range(0.0..1.0),
        );
    };

    for a in -11..11 {
        for b in -11..11 {
            let choose_mat: f32 = rng.random_range(0.0..1.0);
            let center = vec3(
                a as f32 + 0.9 * rng.random_range(0.0..1.0),
                0.2,
                b as f32 + 0.9 * rng.random_range(0.0..1.0),
            );

            if (center - vec3(4.0, 0.2, 0.0)).magnitude() > 0.9 {
                if choose_mat < 0.8 {
                    // diffuse
                    let albedo: Vector3<f32> =
                        random_vec3(&mut rng).mul_element_wise(random_vec3(&mut rng));
                    let center2 = center + vec3(0.0, rng.random_range(0.0..0.5), 0.0);
                    let sphere_material = Rc::new(RefCell::new(Lambertian::from_color(albedo)));
                    world.add(Box::new(Sphere::sphere_moving(center, center2, 0.2, sphere_material)));
                } else if choose_mat < 0.95 {
                    // metal
                    let albedo: Vector3<f32> = random_vec3(&mut rng);
                    let fuzz: f32 = rng.random_range(0.0..0.5);
                    let sphere_material = Rc::new(RefCell::new(Metal::new(albedo, fuzz)));
                    world.add(Box::new(Sphere::sphere(center, 0.2, sphere_material)));
                } else {
                    // glass
                    let sphere_material = Rc::new(RefCell::new(Dielectric::new(1.5)));
                    world.add(Box::new(Sphere::sphere(center, 0.2, sphere_material)));
                }
            }
        }
    }

    let material1 = Rc::new(RefCell::new(Dielectric::new(1.5)));
    world.add(Box::new(Sphere::sphere(vec3(0.0, 1.0, 0.0), 1.0, material1)));

    let material2 = Rc::new(RefCell::new(Lambertian::from_color(vec3(0.4, 0.2, 0.1))));
    world.add(Box::new(Sphere::sphere(vec3(-4.0, 1.0, 0.0), 1.0, material2)));

    let material3 = Rc::new(RefCell::new(Metal::new(vec3(0.7, 0.6, 0.5), 0.0)));
    world.add(Box::new(Sphere::sphere(vec3(4.0, 1.0, 0.0), 1.0, material3)));

    // let bvh_world = BVHNode::from_prim_list(world);

    (camera, Box::new(world))
}

pub fn earth() -> (Camera, Box<dyn Primitive>) {

    let camera = Camera {
        image_width: 1920,
        image_height: 1080,
        samples_per_pixel: 100,
        max_depth: 50,
        samples_per_frame: 1,
        vfov: 20.0,
        lookfrom: vec3(0.0, 0.0, 12.0),
        lookat: vec3(0.0, 0.0, 0.0),
        vup: vec3(0.0, 1.0, 0.0),
        defocus_angle: 0.0,
        focus_dist: 10.0,
    };

    let earth_texture = Rc::new(RefCell::new(ImageTexture::from_bytes(include_bytes!("earthmap.jpg"))));
    let earth_surface = Rc::new(RefCell::new(Lambertian::new(earth_texture)));
    let globe = Box::new(Sphere::sphere(vec3(0.0,0.0,0.0), 2.0, earth_surface));

    (camera, globe)
}

pub fn perlin_spheres() -> (Camera, Box<dyn Primitive>) {

    let camera = Camera {
        image_width: 1920,
        image_height: 1080,
        samples_per_pixel: 100,
        max_depth: 50,
        samples_per_frame: 1,
        vfov: 20.0,
        lookfrom: vec3(13.0, 2.0, 3.0),
        lookat: vec3(0.0, 0.0, 0.0),
        vup: vec3(0.0, 1.0, 0.0),
        defocus_angle: 0.0,
        focus_dist: 10.0,
    };

    let mut world = Box::new(PrimitiveList::new());

    let pertext = Rc::new(RefCell::new(NoiseTexture::new(4.0)));
    world.add(Box::new(Sphere::sphere(vec3(0.0,-1000.0,0.0), 1000.0, Rc::new(RefCell::new(Lambertian::new(pertext.clone()))))));
    world.add(Box::new(Sphere::sphere(vec3(0.0,2.0,0.0), 2.0, Rc::new(RefCell::new(Lambertian::new(pertext))))));

    (camera, world)
}

pub fn quads() -> (Camera, Box<dyn Primitive>) {

    let camera = Camera {
        image_width: 1080,
        image_height: 1080,
        samples_per_pixel: 100,
        max_depth: 50,
        samples_per_frame: 1,
        vfov: 80.0,
        lookfrom: vec3(0.0, 0.0, 9.0),
        lookat: vec3(0.0, 0.0, 0.0),
        vup: vec3(0.0, 1.0, 0.0),
        defocus_angle: 0.0,
        focus_dist: 10.0,
    };

    let mut world = Box::new(PrimitiveList::new());

    // Materials
    let left_red     = Rc::new(RefCell::new(Lambertian::from_color(vec3(1.0, 0.2, 0.2))));
    let back_green   = Rc::new(RefCell::new(Lambertian::from_color(vec3(0.2, 1.0, 0.2))));
    let right_blue   = Rc::new(RefCell::new(Lambertian::from_color(vec3(0.2, 0.2, 1.0))));
    let upper_orange = Rc::new(RefCell::new(Lambertian::from_color(vec3(1.0, 0.5, 0.0))));
    let lower_teal   = Rc::new(RefCell::new(Lambertian::from_color(vec3(0.2, 0.8, 0.8))));

    // Quads
    world.add(Box::new(Quad::new(vec3(-3.0,-2.0, 5.0), vec3(0.0, 0.0,-4.0), vec3(0.0, 4.0, 0.0), left_red)));
    world.add(Box::new(Quad::new(vec3(-2.0,-2.0, 0.0), vec3(4.0, 0.0, 0.0), vec3(0.0, 4.0, 0.0), back_green)));
    world.add(Box::new(Quad::new(vec3( 3.0,-2.0, 1.0), vec3(0.0, 0.0, 4.0), vec3(0.0, 4.0, 0.0), right_blue)));
    world.add(Box::new(Quad::new(vec3(-2.0, 3.0, 1.0), vec3(4.0, 0.0, 0.0), vec3(0.0, 0.0, 4.0), upper_orange)));
    world.add(Box::new(Quad::new(vec3(-2.0,-3.0, 5.0), vec3(4.0, 0.0, 0.0), vec3(0.0, 0.0,-4.0), lower_teal)));

    (camera, world)
}