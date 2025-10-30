use rand::{Rng, SeedableRng, rngs::StdRng};
use cgmath::prelude::*;

use crate::camera::Camera;
use crate::material::*;
use crate::primitive::*;
use crate::texture::*;
use crate::utils::*;

pub fn get_world_0(image_width: u32) -> (Camera, Box<dyn Primitive>) {

    let camera = Camera {
        image_width,
        aspect_ratio: 16.0 / 9.0,
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

    let material_ground = Box::new(Lambertian::from_color(vec3(0.8, 0.8, 0.0)));
    let material_center = Box::new(Lambertian::from_color(vec3(0.1, 0.2, 0.5)));
    let material_left = Box::new(Dielectric::new(1.50));
    let material_bubble = Box::new(Dielectric::new(1.00 / 1.50));
    let material_right = Box::new(Metal::new(vec3(0.8, 0.6, 0.2), 1.0));

    world.add(Box::new(Sphere::sphere(vec3( 0.0, -100.5, -1.0), 100.0, material_ground)));
    world.add(Box::new(Sphere::sphere(vec3( 0.0,    0.0, -1.2),   0.5, material_center)));
    world.add(Box::new(Sphere::sphere(vec3(-1.0,    0.0, -1.0),   0.5, material_left)));
    world.add(Box::new(Sphere::sphere(vec3(-1.0,    0.0, -1.0),   0.4, material_bubble)));
    world.add(Box::new(Sphere::sphere(vec3( 1.0,    0.0, -1.0),   0.5, material_right)));

    (camera, Box::new(world))
}

pub fn bouncing_spheres(image_width: u32) -> (Camera, Box<dyn Primitive>) {

    let camera = Camera {
        image_width,
        aspect_ratio: 16.0 / 9.0,
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
    let checker = Box::new(CheckerTexture::from_colors(0.32, vec3(0.2, 0.3, 0.1), vec3(0.9, 0.9, 0.9)));
    world.add(Box::new(Sphere::sphere(vec3(0.0, -1000.0, 0.0), 1000.0, Box::new(Lambertian::new(checker)))));

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
                    let sphere_material = Box::new(Lambertian::from_color(albedo));
                    world.add(Box::new(Sphere::sphere_moving(center, center2, 0.2, sphere_material)));
                } else if choose_mat < 0.95 {
                    // metal
                    let albedo: Vector3<f32> = random_vec3(&mut rng);
                    let fuzz: f32 = rng.random_range(0.0..0.5);
                    let sphere_material = Box::new(Metal::new(albedo, fuzz));
                    world.add(Box::new(Sphere::sphere(center, 0.2, sphere_material)));
                } else {
                    // glass
                    let sphere_material = Box::new(Dielectric::new(1.5));
                    world.add(Box::new(Sphere::sphere(center, 0.2, sphere_material)));
                }
            }
        }
    }

    let material1 = Box::new(Dielectric::new(1.5));
    world.add(Box::new(Sphere::sphere(vec3(0.0, 1.0, 0.0), 1.0, material1)));

    let material2 = Box::new(Lambertian::from_color(vec3(0.4, 0.2, 0.1)));
    world.add(Box::new(Sphere::sphere(vec3(-4.0, 1.0, 0.0), 1.0, material2)));

    let material3 = Box::new(Metal::new(vec3(0.7, 0.6, 0.5), 0.0));
    world.add(Box::new(Sphere::sphere(vec3(4.0, 1.0, 0.0), 1.0, material3)));

    // let bvh_world = BVHNode::from_prim_list(world);

    (camera, Box::new(world))
}

pub fn earth(image_width: u32) -> (Camera, Box<dyn Primitive>) {

    let camera = Camera {
        image_width,
        aspect_ratio: 16.0 / 9.0,
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

    let earth_texture = Box::new(ImageTexture::from_bytes(include_bytes!("earthmap.jpg")));
    let earth_surface = Box::new(Lambertian::new(earth_texture));
    let globe = Box::new(Sphere::sphere(vec3(0.0,0.0,0.0), 2.0, earth_surface));

    (camera, globe)
}