use std::rc::Rc;

use cgmath::prelude::*;
use rand::{Rng, SeedableRng, rngs::StdRng};
use wgpu::util::DeviceExt;

use crate::camera::{Camera, CameraUniforms};
use crate::material::*;
use crate::primitive::*;
use crate::structure::*;
use crate::texture::*;
use crate::utils::*;

pub struct Scene {
    pub camera: Camera,
    pub world: Rc<dyn Primitive>,
    pub lights: Option<Rc<dyn Primitive>>,

    pub scene_uniforms: SceneUniforms,
    pub camera_uniforms: CameraUniforms,
    pub scene_uniforms_buffer: wgpu::Buffer,
    pub camera_uniforms_buffer: wgpu::Buffer,
    pub accum_pixel_buffer: wgpu::Buffer,
    pub scene_bind_group: wgpu::BindGroup,
}
impl Scene {
    pub fn new(
        device: &wgpu::Device,
        camera: Camera,
        world: Rc<dyn Primitive>,
        lights: Option<Rc<dyn Primitive>>,
    ) -> Self {
        let mut raw_vec = RawVec::new();
        let root_id = world.to_raw(&mut raw_vec) as u32;
        let light_id: i32;
        if let Some(lights) = &lights {
            light_id = lights.to_raw(&mut raw_vec) as i32;
        } else {
            light_id = -1;
        }
        if raw_vec.tex_data.is_empty() {
            raw_vec.tex_data.push(0.0);
        }
        // log!("{:?}, {:?}", materials_raw, primitives_raw);

        let scene_uniforms = SceneUniforms::from_scene(root_id, light_id);

        let camera_uniforms = camera.to_raw();
        let camera_uniforms_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Uniforms Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let accum_pixel_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Accum Pixel Buffer"),
            size: (camera.image_width * camera.image_height * 16) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let scene_uniforms_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Scene Uniforms Buffer"),
            contents: bytemuck::cast_slice(&[scene_uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let primitive_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Primitive Buffer"),
            contents: bytemuck::cast_slice(&raw_vec.primitives),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let material_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Material Buffer"),
            contents: bytemuck::cast_slice(&raw_vec.materials),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let texture_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Texture Buffer"),
            contents: bytemuck::cast_slice(&raw_vec.textures),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let tex_data_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Tex Data Buffer"),
            contents: bytemuck::cast_slice(&raw_vec.tex_data),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let scene_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &Self::layout(device),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: scene_uniforms_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: camera_uniforms_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: accum_pixel_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: primitive_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: material_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: texture_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: tex_data_buffer.as_entire_binding(),
                },
            ],
            label: Some("scene_bind_group"),
        });

        Self {
            camera,
            world,
            lights,
            scene_uniforms,
            camera_uniforms,
            scene_uniforms_buffer,
            camera_uniforms_buffer,
            accum_pixel_buffer,
            scene_bind_group,
        }
    }

    pub fn layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        let scene_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT | wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT | wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT | wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT | wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::FRAGMENT | wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 5,
                        visibility: wgpu::ShaderStages::FRAGMENT | wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 6,
                        visibility: wgpu::ShaderStages::FRAGMENT | wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
                label: Some("scene_bind_group_layout"),
            });

        scene_bind_group_layout
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum SceneEnum {
    ThreeSpheres,
    BouncingSpheres,
    Earth,
    PerlinSpheres,
    Quads,
    SimpleLight,
    CornellBox,
    CornellSmoke,
    FinalScene,
}

pub fn get_scene(device: &wgpu::Device, scene_enum: SceneEnum) -> Scene {
    match scene_enum {
        SceneEnum::ThreeSpheres => three_spheres(device),
        SceneEnum::BouncingSpheres => bouncing_spheres(device),
        SceneEnum::Earth => earth(device),
        SceneEnum::PerlinSpheres => perlin_spheres(device),
        SceneEnum::Quads => quads(device),
        SceneEnum::SimpleLight => simple_light(device),
        SceneEnum::CornellBox => cornell_box(device),
        SceneEnum::CornellSmoke => cornell_smoke(device),
        SceneEnum::FinalScene => final_scene(device),
    }
}

pub fn three_spheres(device: &wgpu::Device) -> Scene {
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

    let material_ground = Lambertian::from_color(vec3(0.8, 0.8, 0.0));
    let material_center = Lambertian::from_color(vec3(0.1, 0.2, 0.5));
    let material_left = Dielectric::new(1.50);
    let material_bubble = Dielectric::new(1.00 / 1.50);
    let material_right = Metal::new(vec3(0.8, 0.6, 0.2), 1.0);

    world.add(Sphere::sphere(
        vec3(0.0, -100.5, -1.0),
        100.0,
        material_ground,
    ));
    world.add(Sphere::sphere(vec3(0.0, 0.0, -1.2), 0.5, material_center));
    world.add(Sphere::sphere(vec3(-1.0, 0.0, -1.0), 0.5, material_left));
    world.add(Sphere::sphere(vec3(-1.0, 0.0, -1.0), 0.4, material_bubble));
    world.add(Sphere::sphere(vec3(1.0, 0.0, -1.0), 0.5, material_right));

    Scene::new(device, camera, Rc::new(world), None)
}

pub fn bouncing_spheres(device: &wgpu::Device) -> Scene {
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
    let checker = CheckerTexture::from_colors(0.32, vec3(0.2, 0.3, 0.1), vec3(0.9, 0.9, 0.9));
    world.add(Sphere::sphere(
        vec3(0.0, -1000.0, 0.0),
        1000.0,
        Lambertian::new(checker),
    ));

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
                    let sphere_material = Lambertian::from_color(albedo);
                    world.add(Sphere::sphere_moving(center, center2, 0.2, sphere_material));
                } else if choose_mat < 0.95 {
                    // metal
                    let albedo: Vector3<f32> = random_vec3(&mut rng);
                    let fuzz: f32 = rng.random_range(0.0..0.5);
                    let sphere_material = Metal::new(albedo, fuzz);
                    world.add(Sphere::sphere(center, 0.2, sphere_material));
                } else {
                    // glass
                    let sphere_material = Dielectric::new(1.5);
                    world.add(Sphere::sphere(center, 0.2, sphere_material));
                }
            }
        }
    }

    let material1 = Dielectric::new(1.5);
    world.add(Sphere::sphere(vec3(0.0, 1.0, 0.0), 1.0, material1));

    let material2 = Lambertian::from_color(vec3(0.4, 0.2, 0.1));
    world.add(Sphere::sphere(vec3(-4.0, 1.0, 0.0), 1.0, material2));

    let material3 = Metal::new(vec3(0.7, 0.6, 0.5), 0.0);
    world.add(Sphere::sphere(vec3(4.0, 1.0, 0.0), 1.0, material3));

    let world = BVHNode::from_prim_list(world);

    Scene::new(device, camera, world, None)
}

pub fn earth(device: &wgpu::Device) -> Scene {
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

    let earth_texture = ImageTexture::from_bytes(include_bytes!("earthmap.jpg"));
    let earth_surface = Lambertian::new(earth_texture);
    let world = Sphere::sphere(vec3(0.0, 0.0, 0.0), 2.0, earth_surface);

    Scene::new(device, camera, world, None)
}

pub fn perlin_spheres(device: &wgpu::Device) -> Scene {
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

    let mut world = PrimitiveList::new();

    let pertext = NoiseTexture::new(4.0);
    world.add(Sphere::sphere(
        vec3(0.0, -1000.0, 0.0),
        1000.0,
        Lambertian::new(pertext.clone()),
    ));
    world.add(Sphere::sphere(
        vec3(0.0, 2.0, 0.0),
        2.0,
        Lambertian::new(pertext),
    ));

    Scene::new(device, camera, Rc::new(world), None)
}

pub fn quads(device: &wgpu::Device) -> Scene {
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

    let mut world = PrimitiveList::new();

    // Materials
    let left_red = Lambertian::from_color(vec3(1.0, 0.2, 0.2));
    let back_green = Lambertian::from_color(vec3(0.2, 1.0, 0.2));
    let right_blue = Lambertian::from_color(vec3(0.2, 0.2, 1.0));
    let upper_orange = Lambertian::from_color(vec3(1.0, 0.5, 0.0));
    let lower_teal = Lambertian::from_color(vec3(0.2, 0.8, 0.8));

    // Quads
    world.add(Quad::new(
        vec3(-3.0, -2.0, 5.0),
        vec3(0.0, 0.0, -4.0),
        vec3(0.0, 4.0, 0.0),
        left_red,
    ));
    world.add(Quad::new(
        vec3(-2.0, -2.0, 0.0),
        vec3(4.0, 0.0, 0.0),
        vec3(0.0, 4.0, 0.0),
        back_green,
    ));
    world.add(Quad::new(
        vec3(3.0, -2.0, 1.0),
        vec3(0.0, 0.0, 4.0),
        vec3(0.0, 4.0, 0.0),
        right_blue,
    ));
    world.add(Quad::new(
        vec3(-2.0, 3.0, 1.0),
        vec3(4.0, 0.0, 0.0),
        vec3(0.0, 0.0, 4.0),
        upper_orange,
    ));
    world.add(Quad::new(
        vec3(-2.0, -3.0, 5.0),
        vec3(4.0, 0.0, 0.0),
        vec3(0.0, 0.0, -4.0),
        lower_teal,
    ));

    Scene::new(device, camera, Rc::new(world), None)
}

pub fn simple_light(device: &wgpu::Device) -> Scene {
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

    let mut world = PrimitiveList::new();

    let pertext = NoiseTexture::new(4.0);
    world.add(Sphere::sphere(
        vec3(0.0, -1000.0, 0.0),
        1000.0,
        Lambertian::new(pertext.clone()),
    ));
    world.add(Sphere::sphere(
        vec3(0.0, 2.0, 0.0),
        2.0,
        Lambertian::new(pertext),
    ));

    let difflight = DiffuseLight::from_color(vec3(4.0, 4.0, 4.0));
    world.add(Sphere::sphere(vec3(0.0, 7.0, 0.0), 2.0, difflight.clone()));
    world.add(Quad::new(
        vec3(3.0, 1.0, -2.0),
        vec3(2.0, 0.0, 0.0),
        vec3(0.0, 2.0, 0.0),
        difflight.clone(),
    ));

    let lights = Quad::new(
        vec3(3.0, 1.0, -2.0),
        vec3(2.0, 0.0, 0.0),
        vec3(0.0, 2.0, 0.0),
        difflight,
    );

    Scene::new(device, camera, Rc::new(world), Some(lights))
}

pub fn cornell_box(device: &wgpu::Device) -> Scene {
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

    let mut world = PrimitiveList::new();

    let red = Lambertian::from_color(vec3(0.65, 0.05, 0.05));
    let white = Lambertian::from_color(vec3(0.73, 0.73, 0.73));
    let green = Lambertian::from_color(vec3(0.12, 0.45, 0.15));
    let light = DiffuseLight::from_color(vec3(15.0, 15.0, 15.0));

    world.add(Quad::new(
        vec3(555.0, 0.0, 0.0),
        vec3(0.0, 555.0, 0.0),
        vec3(0.0, 0.0, 555.0),
        green,
    ));
    world.add(Quad::new(
        vec3(0.0, 0.0, 0.0),
        vec3(0.0, 555.0, 0.0),
        vec3(0.0, 0.0, 555.0),
        red,
    ));
    world.add(Quad::new(
        vec3(343.0, 554.0, 332.0),
        vec3(-130.0, 0.0, 0.0),
        vec3(0.0, 0.0, -105.0),
        light.clone(),
    ));
    world.add(Quad::new(
        vec3(0.0, 0.0, 0.0),
        vec3(555.0, 0.0, 0.0),
        vec3(0.0, 0.0, 555.0),
        white.clone(),
    ));
    world.add(Quad::new(
        vec3(555.0, 555.0, 555.0),
        vec3(-555.0, 0.0, 0.0),
        vec3(0.0, 0.0, -555.0),
        white.clone(),
    ));
    world.add(Quad::new(
        vec3(0.0, 0.0, 555.0),
        vec3(555.0, 0.0, 0.0),
        vec3(0.0, 555.0, 0.0),
        white.clone(),
    ));

    let box1 = quad_box(
        vec3(0.0, 0.0, 0.0),
        vec3(165.0, 330.0, 165.0),
        white.clone(),
    );
    let box1 = RotateY::new(box1, degrees(15.0));
    let box1 = Translate::new(box1, vec3(265.0, 0.0, 295.0));
    world.add(box1);

    let box2 = quad_box(vec3(0.0, 0.0, 0.0), vec3(165.0, 165.0, 165.0), white);
    let box2 = RotateY::new(box2, degrees(-18.0));
    let box2 = Translate::new(box2, vec3(130.0, 0.0, 65.0));
    world.add(box2);

    let lights = Quad::new(
        vec3(343.0, 554.0, 332.0),
        vec3(-130.0, 0.0, 0.0),
        vec3(0.0, 0.0, -105.0),
        light,
    );

    Scene::new(device, camera, Rc::new(world), Some(lights))
}

pub fn cornell_smoke(device: &wgpu::Device) -> Scene {
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

    let mut world = PrimitiveList::new();

    let red = Lambertian::from_color(vec3(0.65, 0.05, 0.05));
    let white = Lambertian::from_color(vec3(0.73, 0.73, 0.73));
    let green = Lambertian::from_color(vec3(0.12, 0.45, 0.15));
    let light = DiffuseLight::from_color(vec3(7.0, 7.0, 7.0));

    world.add(Quad::new(
        vec3(555.0, 0.0, 0.0),
        vec3(0.0, 555.0, 0.0),
        vec3(0.0, 0.0, 555.0),
        green,
    ));
    world.add(Quad::new(
        vec3(0.0, 0.0, 0.0),
        vec3(0.0, 555.0, 0.0),
        vec3(0.0, 0.0, 555.0),
        red,
    ));
    world.add(Quad::new(
        vec3(113.0, 554.0, 127.0),
        vec3(330.0, 0.0, 0.0),
        vec3(0.0, 0.0, 305.0),
        light.clone(),
    ));
    world.add(Quad::new(
        vec3(0.0, 0.0, 0.0),
        vec3(555.0, 0.0, 0.0),
        vec3(0.0, 0.0, 555.0),
        white.clone(),
    ));
    world.add(Quad::new(
        vec3(555.0, 555.0, 555.0),
        vec3(-555.0, 0.0, 0.0),
        vec3(0.0, 0.0, -555.0),
        white.clone(),
    ));
    world.add(Quad::new(
        vec3(0.0, 0.0, 555.0),
        vec3(555.0, 0.0, 0.0),
        vec3(0.0, 555.0, 0.0),
        white.clone(),
    ));

    let box1 = quad_box(
        vec3(0.0, 0.0, 0.0),
        vec3(165.0, 330.0, 165.0),
        white.clone(),
    );
    let box1 = RotateY::new(box1, degrees(15.0));
    let box1 = Translate::new(box1, vec3(265.0, 0.0, 295.0));

    let box2 = quad_box(vec3(0.0, 0.0, 0.0), vec3(165.0, 165.0, 165.0), white);
    let box2 = RotateY::new(box2, degrees(-18.0));
    let box2 = Translate::new(box2, vec3(130.0, 0.0, 65.0));

    world.add(ConstantMedium::from_color(box1, 0.01, vec3(0.0, 0.0, 0.0)));
    world.add(ConstantMedium::from_color(box2, 0.01, vec3(1.0, 1.0, 1.0)));

    let lights = Quad::new(
        vec3(113.0, 554.0, 127.0),
        vec3(330.0, 0.0, 0.0),
        vec3(0.0, 0.0, 305.0),
        light,
    );

    Scene::new(device, camera, Rc::new(world), Some(lights))
}

pub fn final_scene(device: &wgpu::Device) -> Scene {
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

    let mut rng = StdRng::seed_from_u64(0);

    let mut boxes1 = PrimitiveList::new();
    let ground = Lambertian::from_color(vec3(0.48, 0.83, 0.53));
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

    let mut world = PrimitiveList::new();

    world.add(Rc::new(boxes1));

    let light = DiffuseLight::from_color(vec3(7.0, 7.0, 7.0));
    world.add(Quad::new(
        vec3(123.0, 554.0, 147.0),
        vec3(300.0, 0.0, 0.0),
        vec3(0.0, 0.0, 265.0),
        light.clone(),
    ));

    let center1 = vec3(400.0, 400.0, 200.0);
    let center2 = center1 + vec3(30.0, 0.0, 0.0);
    let sphere_material = Lambertian::from_color(vec3(0.7, 0.3, 0.1));
    world.add(Sphere::sphere_moving(
        center1,
        center2,
        50.0,
        sphere_material,
    ));

    world.add(Sphere::sphere(
        vec3(260.0, 150.0, 45.0),
        50.0,
        Dielectric::new(1.5),
    ));
    world.add(Sphere::sphere(
        vec3(0.0, 150.0, 145.0),
        50.0,
        Metal::new(vec3(0.8, 0.8, 0.9), 1.0),
    ));

    let boundary = Sphere::sphere(vec3(360.0, 150.0, 145.0), 70.0, Dielectric::new(1.5));
    world.add(boundary);
    let boundary = Sphere::sphere(vec3(360.0, 150.0, 145.0), 70.0, Dielectric::new(1.5));
    world.add(ConstantMedium::from_color(
        boundary,
        0.2,
        vec3(0.2, 0.4, 0.9),
    ));
    let boundary = Sphere::sphere(vec3(0.0, 0.0, 0.0), 5000.0, Dielectric::new(1.5));
    world.add(ConstantMedium::from_color(
        boundary,
        0.0001,
        vec3(1.0, 1.0, 1.0),
    ));

    let image_bytes = include_bytes!("earthmap.jpg");
    let emat = Lambertian::new(ImageTexture::from_bytes(image_bytes));
    world.add(Sphere::sphere(vec3(400.0, 200.0, 400.0), 100.0, emat));
    let pertext = NoiseTexture::new(0.2);
    world.add(Sphere::sphere(
        vec3(220.0, 280.0, 300.0),
        80.0,
        Lambertian::new(pertext),
    ));

    let mut boxes2 = PrimitiveList::new();
    let white = Lambertian::from_color(vec3(0.73, 0.73, 0.73));
    let ns: usize = 1000;
    for _j in 0..ns {
        boxes2.add(Sphere::sphere(
            vec3(
                rng.random_range(0.0..165.0),
                rng.random_range(0.0..165.0),
                rng.random_range(0.0..165.0),
            ),
            10.0,
            white.clone(),
        ));
    }

    world.add(Translate::new(
        RotateY::new(BVHNode::from_prim_list(boxes2), degrees(15.0)),
        vec3(-100.0, 270.0, 395.0),
    ));

    let lights = Quad::new(
        vec3(123.0, 554.0, 147.0),
        vec3(300.0, 0.0, 0.0),
        vec3(0.0, 0.0, 265.0),
        light.clone(),
    );

    Scene::new(device, camera, Rc::new(world), Some(lights))
}

// Returns the 3D box (six sides) that contains the two opposite vertices a & b.
fn quad_box(a: Vec3, b: Vec3, mat: Rc<dyn Material>) -> Rc<PrimitiveList> {
    let mut sides = PrimitiveList::new();

    // Construct the two opposite vertices with the minimum and maximum coordinates.
    let min = vec3(a.x.min(b.x), a.y.min(b.y), a.z.min(b.z));
    let max = vec3(a.x.max(b.x), a.y.max(b.y), a.z.max(b.z));

    let dx = vec3(max.x - min.x, 0.0, 0.0);
    let dy = vec3(0.0, max.y - min.y, 0.0);
    let dz = vec3(0.0, 0.0, max.z - min.z);

    sides.add(Quad::new(vec3(min.x, min.y, max.z), dx, dy, mat.clone())); // front
    sides.add(Quad::new(vec3(max.x, min.y, max.z), -dz, dy, mat.clone())); // right
    sides.add(Quad::new(vec3(max.x, min.y, min.z), -dx, dy, mat.clone())); // back
    sides.add(Quad::new(vec3(min.x, min.y, min.z), dz, dy, mat.clone())); // left
    sides.add(Quad::new(vec3(min.x, max.y, max.z), dx, -dz, mat.clone())); // top
    sides.add(Quad::new(vec3(min.x, min.y, min.z), dx, dz, mat)); // bottom

    Rc::new(sides)
}
