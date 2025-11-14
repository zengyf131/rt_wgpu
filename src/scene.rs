use std::cell::RefCell;
use std::rc::Rc;

use cgmath::prelude::*;
use rand::{Rng, SeedableRng, rngs::StdRng};

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
        background: vec3(0.7, 0.8, 1.0),
    };

    let mut world = PrimitiveList::new();

    let material_ground = Rc::new(RefCell::new(Lambertian::from_color(vec3(0.8, 0.8, 0.0))));
    let material_center = Rc::new(RefCell::new(Lambertian::from_color(vec3(0.1, 0.2, 0.5))));
    let material_left = Rc::new(RefCell::new(Dielectric::new(1.50)));
    let material_bubble = Rc::new(RefCell::new(Dielectric::new(1.00 / 1.50)));
    let material_right = Rc::new(RefCell::new(Metal::new(vec3(0.8, 0.6, 0.2), 1.0)));

    world.add(Box::new(Sphere::sphere(
        vec3(0.0, -100.5, -1.0),
        100.0,
        material_ground,
    )));
    world.add(Box::new(Sphere::sphere(
        vec3(0.0, 0.0, -1.2),
        0.5,
        material_center,
    )));
    world.add(Box::new(Sphere::sphere(
        vec3(-1.0, 0.0, -1.0),
        0.5,
        material_left,
    )));
    world.add(Box::new(Sphere::sphere(
        vec3(-1.0, 0.0, -1.0),
        0.4,
        material_bubble,
    )));
    world.add(Box::new(Sphere::sphere(
        vec3(1.0, 0.0, -1.0),
        0.5,
        material_right,
    )));

    (camera, Box::new(world))
}

pub fn bouncing_spheres() -> (Camera, Box<dyn Primitive>) {
    let camera = Camera {
        image_width: 1920,
        image_height: 1080,
        samples_per_pixel: 500,
        max_depth: 50,
        samples_per_frame: 1,
        vfov: 20.0,
        lookfrom: vec3(13.0, 2.0, 3.0),
        lookat: vec3(0.0, 0.0, 0.0),
        vup: vec3(0.0, 1.0, 0.0),
        defocus_angle: 0.6,
        focus_dist: 10.0,
        background: vec3(0.7, 0.8, 1.0),
    };

    let mut world = PrimitiveList::new();

    let mut rng = StdRng::seed_from_u64(0);
    let checker = Rc::new(RefCell::new(CheckerTexture::from_colors(
        0.32,
        vec3(0.2, 0.3, 0.1),
        vec3(0.9, 0.9, 0.9),
    )));
    world.add(Box::new(Sphere::sphere(
        vec3(0.0, -1000.0, 0.0),
        1000.0,
        Rc::new(RefCell::new(Lambertian::new(checker))),
    )));

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
                    world.add(Box::new(Sphere::sphere_moving(
                        center,
                        center2,
                        0.2,
                        sphere_material,
                    )));
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
    world.add(Box::new(Sphere::sphere(
        vec3(0.0, 1.0, 0.0),
        1.0,
        material1,
    )));

    let material2 = Rc::new(RefCell::new(Lambertian::from_color(vec3(0.4, 0.2, 0.1))));
    world.add(Box::new(Sphere::sphere(
        vec3(-4.0, 1.0, 0.0),
        1.0,
        material2,
    )));

    let material3 = Rc::new(RefCell::new(Metal::new(vec3(0.7, 0.6, 0.5), 0.0)));
    world.add(Box::new(Sphere::sphere(
        vec3(4.0, 1.0, 0.0),
        1.0,
        material3,
    )));

    let world = BVHNode::from_prim_list(world);

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
        background: vec3(0.7, 0.8, 1.0),
    };

    let earth_texture = Rc::new(RefCell::new(ImageTexture::from_bytes(include_bytes!(
        "earthmap.jpg"
    ))));
    let earth_surface = Rc::new(RefCell::new(Lambertian::new(earth_texture)));
    let globe = Box::new(Sphere::sphere(vec3(0.0, 0.0, 0.0), 2.0, earth_surface));

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
        background: vec3(0.7, 0.8, 1.0),
    };

    let mut world = Box::new(PrimitiveList::new());

    let pertext = Rc::new(RefCell::new(NoiseTexture::new(4.0)));
    world.add(Box::new(Sphere::sphere(
        vec3(0.0, -1000.0, 0.0),
        1000.0,
        Rc::new(RefCell::new(Lambertian::new(pertext.clone()))),
    )));
    world.add(Box::new(Sphere::sphere(
        vec3(0.0, 2.0, 0.0),
        2.0,
        Rc::new(RefCell::new(Lambertian::new(pertext))),
    )));

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
        background: vec3(0.7, 0.8, 1.0),
    };

    let mut world = Box::new(PrimitiveList::new());

    // Materials
    let left_red = Rc::new(RefCell::new(Lambertian::from_color(vec3(1.0, 0.2, 0.2))));
    let back_green = Rc::new(RefCell::new(Lambertian::from_color(vec3(0.2, 1.0, 0.2))));
    let right_blue = Rc::new(RefCell::new(Lambertian::from_color(vec3(0.2, 0.2, 1.0))));
    let upper_orange = Rc::new(RefCell::new(Lambertian::from_color(vec3(1.0, 0.5, 0.0))));
    let lower_teal = Rc::new(RefCell::new(Lambertian::from_color(vec3(0.2, 0.8, 0.8))));

    // Quads
    world.add(Box::new(Quad::new(
        vec3(-3.0, -2.0, 5.0),
        vec3(0.0, 0.0, -4.0),
        vec3(0.0, 4.0, 0.0),
        left_red,
    )));
    world.add(Box::new(Quad::new(
        vec3(-2.0, -2.0, 0.0),
        vec3(4.0, 0.0, 0.0),
        vec3(0.0, 4.0, 0.0),
        back_green,
    )));
    world.add(Box::new(Quad::new(
        vec3(3.0, -2.0, 1.0),
        vec3(0.0, 0.0, 4.0),
        vec3(0.0, 4.0, 0.0),
        right_blue,
    )));
    world.add(Box::new(Quad::new(
        vec3(-2.0, 3.0, 1.0),
        vec3(4.0, 0.0, 0.0),
        vec3(0.0, 0.0, 4.0),
        upper_orange,
    )));
    world.add(Box::new(Quad::new(
        vec3(-2.0, -3.0, 5.0),
        vec3(4.0, 0.0, 0.0),
        vec3(0.0, 0.0, -4.0),
        lower_teal,
    )));

    (camera, world)
}

pub fn simple_light() -> (Camera, Box<dyn Primitive>) {
    let camera = Camera {
        image_width: 1920,
        image_height: 1080,
        samples_per_pixel: 100,
        max_depth: 50,
        samples_per_frame: 1,
        vfov: 20.0,
        lookfrom: vec3(26.0, 3.0, 6.0),
        lookat: vec3(0.0, 2.0, 0.0),
        vup: vec3(0.0, 1.0, 0.0),
        defocus_angle: 0.0,
        focus_dist: 10.0,
        background: vec3(0.0, 0.0, 0.0),
    };

    let mut world = Box::new(PrimitiveList::new());

    let pertext = Rc::new(RefCell::new(NoiseTexture::new(4.0)));
    world.add(Box::new(Sphere::sphere(
        vec3(0.0, -1000.0, 0.0),
        1000.0,
        Rc::new(RefCell::new(Lambertian::new(pertext.clone()))),
    )));
    world.add(Box::new(Sphere::sphere(
        vec3(0.0, 2.0, 0.0),
        2.0,
        Rc::new(RefCell::new(Lambertian::new(pertext))),
    )));

    let difflight = Rc::new(RefCell::new(DiffuseLight::from_color(vec3(4.0, 4.0, 4.0))));
    world.add(Box::new(Sphere::sphere(
        vec3(0.0, 7.0, 0.0),
        2.0,
        difflight.clone(),
    )));
    world.add(Box::new(Quad::new(
        vec3(3.0, 1.0, -2.0),
        vec3(2.0, 0.0, 0.0),
        vec3(0.0, 2.0, 0.0),
        difflight,
    )));

    (camera, world)
}

pub fn cornell_box() -> (Camera, Box<dyn Primitive>) {
    let camera = Camera {
        image_width: 1080,
        image_height: 1080,
        samples_per_pixel: 200,
        max_depth: 50,
        samples_per_frame: 1,
        vfov: 40.0,
        lookfrom: vec3(278.0, 278.0, -800.0),
        lookat: vec3(278.0, 278.0, 0.0),
        vup: vec3(0.0, 1.0, 0.0),
        defocus_angle: 0.0,
        focus_dist: 10.0,
        background: vec3(0.0, 0.0, 0.0),
    };

    let mut world = Box::new(PrimitiveList::new());

    let red = Rc::new(RefCell::new(Lambertian::from_color(vec3(0.65, 0.05, 0.05))));
    let white = Rc::new(RefCell::new(Lambertian::from_color(vec3(0.73, 0.73, 0.73))));
    let green = Rc::new(RefCell::new(Lambertian::from_color(vec3(0.12, 0.45, 0.15))));
    let light = Rc::new(RefCell::new(DiffuseLight::from_color(vec3(
        15.0, 15.0, 15.0,
    ))));

    world.add(Box::new(Quad::new(
        vec3(555.0, 0.0, 0.0),
        vec3(0.0, 555.0, 0.0),
        vec3(0.0, 0.0, 555.0),
        green,
    )));
    world.add(Box::new(Quad::new(
        vec3(0.0, 0.0, 0.0),
        vec3(0.0, 555.0, 0.0),
        vec3(0.0, 0.0, 555.0),
        red,
    )));
    world.add(Box::new(Quad::new(
        vec3(343.0, 554.0, 332.0),
        vec3(-130.0, 0.0, 0.0),
        vec3(0.0, 0.0, -105.0),
        light,
    )));
    world.add(Box::new(Quad::new(
        vec3(0.0, 0.0, 0.0),
        vec3(555.0, 0.0, 0.0),
        vec3(0.0, 0.0, 555.0),
        white.clone(),
    )));
    world.add(Box::new(Quad::new(
        vec3(555.0, 555.0, 555.0),
        vec3(-555.0, 0.0, 0.0),
        vec3(0.0, 0.0, -555.0),
        white.clone(),
    )));
    world.add(Box::new(Quad::new(
        vec3(0.0, 0.0, 555.0),
        vec3(555.0, 0.0, 0.0),
        vec3(0.0, 555.0, 0.0),
        white.clone(),
    )));

    let box1 = quad_box(
        vec3(0.0, 0.0, 0.0),
        vec3(165.0, 330.0, 165.0),
        white.clone(),
    );
    let box1 = Box::new(RotateY::new(box1, degrees(15.0)));
    let box1 = Box::new(Translate::new(box1, vec3(265.0, 0.0, 295.0)));
    world.add(box1);

    let box2 = quad_box(vec3(0.0, 0.0, 0.0), vec3(165.0, 165.0, 165.0), white);
    let box2 = Box::new(RotateY::new(box2, degrees(-18.0)));
    let box2 = Box::new(Translate::new(box2, vec3(130.0, 0.0, 65.0)));
    world.add(box2);

    (camera, world)
}

pub fn cornell_smoke() -> (Camera, Box<dyn Primitive>) {
    let camera = Camera {
        image_width: 600,
        image_height: 600,
        samples_per_pixel: 200,
        max_depth: 50,
        samples_per_frame: 1,
        vfov: 40.0,
        lookfrom: vec3(278.0, 278.0, -800.0),
        lookat: vec3(278.0, 278.0, 0.0),
        vup: vec3(0.0, 1.0, 0.0),
        defocus_angle: 0.0,
        focus_dist: 10.0,
        background: vec3(0.0, 0.0, 0.0),
    };

    let mut world = Box::new(PrimitiveList::new());

    let red = Rc::new(RefCell::new(Lambertian::from_color(vec3(0.65, 0.05, 0.05))));
    let white = Rc::new(RefCell::new(Lambertian::from_color(vec3(0.73, 0.73, 0.73))));
    let green = Rc::new(RefCell::new(Lambertian::from_color(vec3(0.12, 0.45, 0.15))));
    let light = Rc::new(RefCell::new(DiffuseLight::from_color(vec3(7.0, 7.0, 7.0))));

    world.add(Box::new(Quad::new(
        vec3(555.0, 0.0, 0.0),
        vec3(0.0, 555.0, 0.0),
        vec3(0.0, 0.0, 555.0),
        green,
    )));
    world.add(Box::new(Quad::new(
        vec3(0.0, 0.0, 0.0),
        vec3(0.0, 555.0, 0.0),
        vec3(0.0, 0.0, 555.0),
        red,
    )));
    world.add(Box::new(Quad::new(
        vec3(113.0, 554.0, 127.0),
        vec3(330.0, 0.0, 0.0),
        vec3(0.0, 0.0, 305.0),
        light,
    )));
    world.add(Box::new(Quad::new(
        vec3(0.0, 0.0, 0.0),
        vec3(555.0, 0.0, 0.0),
        vec3(0.0, 0.0, 555.0),
        white.clone(),
    )));
    world.add(Box::new(Quad::new(
        vec3(555.0, 555.0, 555.0),
        vec3(-555.0, 0.0, 0.0),
        vec3(0.0, 0.0, -555.0),
        white.clone(),
    )));
    world.add(Box::new(Quad::new(
        vec3(0.0, 0.0, 555.0),
        vec3(555.0, 0.0, 0.0),
        vec3(0.0, 555.0, 0.0),
        white.clone(),
    )));

    let box1 = quad_box(
        vec3(0.0, 0.0, 0.0),
        vec3(165.0, 330.0, 165.0),
        white.clone(),
    );
    let box1 = Box::new(RotateY::new(box1, degrees(15.0)));
    let box1 = Box::new(Translate::new(box1, vec3(265.0, 0.0, 295.0)));

    let box2 = quad_box(vec3(0.0, 0.0, 0.0), vec3(165.0, 165.0, 165.0), white);
    let box2 = Box::new(RotateY::new(box2, degrees(-18.0)));
    let box2 = Box::new(Translate::new(box2, vec3(130.0, 0.0, 65.0)));

    world.add(Box::new(ConstantMedium::from_color(
        box1,
        0.01,
        vec3(0.0, 0.0, 0.0),
    )));
    world.add(Box::new(ConstantMedium::from_color(
        box2,
        0.01,
        vec3(1.0, 1.0, 1.0),
    )));

    (camera, world)
}

pub fn final_scene() -> (Camera, Box<dyn Primitive>) {
    let camera = Camera {
        image_width: 1080,
        image_height: 1080,
        samples_per_pixel: 1000,
        max_depth: 40,
        samples_per_frame: 1,
        vfov: 40.0,
        lookfrom: vec3(478.0, 278.0, -600.0),
        lookat: vec3(278.0, 278.0, 0.0),
        vup: vec3(0.0, 1.0, 0.0),
        defocus_angle: 0.0,
        focus_dist: 10.0,
        background: vec3(0.0, 0.0, 0.0),
    };

    let mut rng = rand::rng();

    let mut boxes1 = Box::new(PrimitiveList::new());
    let ground = Rc::new(RefCell::new(Lambertian::from_color(vec3(0.48, 0.83, 0.53))));
    let boxes_per_side: usize = 20;
    for i in 0..boxes_per_side {
        for j in 0..boxes_per_side {
            let w = 100.0;
            let x0 = -1000.0 + i as f32 * w;
            let z0 = -1000.0 + j as f32 * w;
            let y0 = 0.0;
            let x1 = x0 + w;
            let y1 = rng.random_range(1.0..101.0);
            let z1 = z0 + w;
            boxes1.add(quad_box(vec3(x0, y0, z0), vec3(x1, y1, z1), ground.clone()));
        }
    }

    let mut world = Box::new(PrimitiveList::new());

    world.add(boxes1);

    let light = Rc::new(RefCell::new(DiffuseLight::from_color(vec3(7.0, 7.0, 7.0))));
    world.add(Box::new(Quad::new(
        vec3(123.0, 554.0, 147.0),
        vec3(300.0, 0.0, 0.0),
        vec3(0.0, 0.0, 265.0),
        light,
    )));

    let center1 = vec3(400.0, 400.0, 200.0);
    let center2 = center1 + vec3(30.0, 0.0, 0.0);
    let sphere_material = Rc::new(RefCell::new(Lambertian::from_color(vec3(0.7, 0.3, 0.1))));
    world.add(Box::new(Sphere::sphere_moving(
        center1,
        center2,
        50.0,
        sphere_material,
    )));

    world.add(Box::new(Sphere::sphere(
        vec3(260.0, 150.0, 45.0),
        50.0,
        Rc::new(RefCell::new(Dielectric::new(1.5))),
    )));
    world.add(Box::new(Sphere::sphere(
        vec3(0.0, 150.0, 145.0),
        50.0,
        Rc::new(RefCell::new(Metal::new(vec3(0.8, 0.8, 0.9), 1.0))),
    )));

    let boundary = Box::new(Sphere::sphere(
        vec3(360.0, 150.0, 145.0),
        70.0,
        Rc::new(RefCell::new(Dielectric::new(1.5))),
    ));
    world.add(boundary);
    let boundary = Box::new(Sphere::sphere(
        vec3(360.0, 150.0, 145.0),
        70.0,
        Rc::new(RefCell::new(Dielectric::new(1.5))),
    ));
    world.add(Box::new(ConstantMedium::from_color(
        boundary,
        0.2,
        vec3(0.2, 0.4, 0.9),
    )));
    let boundary = Box::new(Sphere::sphere(
        vec3(0.0, 0.0, 0.0),
        5000.0,
        Rc::new(RefCell::new(Dielectric::new(1.5))),
    ));
    world.add(Box::new(ConstantMedium::from_color(
        boundary,
        0.0001,
        vec3(1.0, 1.0, 1.0),
    )));

    let image_bytes = include_bytes!("earthmap.jpg");
    let emat = Rc::new(RefCell::new(Lambertian::new(Rc::new(RefCell::new(
        ImageTexture::from_bytes(image_bytes),
    )))));
    world.add(Box::new(Sphere::sphere(
        vec3(400.0, 200.0, 400.0),
        100.0,
        emat,
    )));
    let pertext = Rc::new(RefCell::new(NoiseTexture::new(0.2)));
    world.add(Box::new(Sphere::sphere(
        vec3(220.0, 280.0, 300.0),
        80.0,
        Rc::new(RefCell::new(Lambertian::new(pertext))),
    )));

    let mut boxes2 = PrimitiveList::new();
    let white = Rc::new(RefCell::new(Lambertian::from_color(vec3(0.73, 0.73, 0.73))));
    let ns: usize = 1000;
    for j in 0..ns {
        boxes2.add(Box::new(Sphere::sphere(
            vec3(
                rng.random_range(0.0..165.0),
                rng.random_range(0.0..165.0),
                rng.random_range(0.0..165.0),
            ),
            10.0,
            white.clone(),
        )));
    }

    world.add(Box::new(Translate::new(
        Box::new(RotateY::new(
            Box::new(BVHNode::from_prim_list(boxes2)),
            degrees(15.0),
        )),
        vec3(-100.0, 270.0, 395.0),
    )));

    (camera, world)
}

// Returns the 3D box (six sides) that contains the two opposite vertices a & b.
fn quad_box(a: Vec3, b: Vec3, mat: Rc<RefCell<dyn Material>>) -> Box<PrimitiveList> {
    let mut sides = Box::new(PrimitiveList::new());

    // Construct the two opposite vertices with the minimum and maximum coordinates.
    let min = vec3(a.x.min(b.x), a.y.min(b.y), a.z.min(b.z));
    let max = vec3(a.x.max(b.x), a.y.max(b.y), a.z.max(b.z));

    let dx = vec3(max.x - min.x, 0.0, 0.0);
    let dy = vec3(0.0, max.y - min.y, 0.0);
    let dz = vec3(0.0, 0.0, max.z - min.z);

    sides.add(Box::new(Quad::new(
        vec3(min.x, min.y, max.z),
        dx,
        dy,
        mat.clone(),
    ))); // front
    sides.add(Box::new(Quad::new(
        vec3(max.x, min.y, max.z),
        -dz,
        dy,
        mat.clone(),
    ))); // right
    sides.add(Box::new(Quad::new(
        vec3(max.x, min.y, min.z),
        -dx,
        dy,
        mat.clone(),
    ))); // back
    sides.add(Box::new(Quad::new(
        vec3(min.x, min.y, min.z),
        dz,
        dy,
        mat.clone(),
    ))); // left
    sides.add(Box::new(Quad::new(
        vec3(min.x, max.y, max.z),
        dx,
        -dz,
        mat.clone(),
    ))); // top
    sides.add(Box::new(Quad::new(vec3(min.x, min.y, min.z), dx, dz, mat))); // bottom

    sides
}
