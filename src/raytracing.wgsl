// Vertex shader
struct VertexInput {
    @location(0) position: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
}

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4<f32>(model.position, 1.0);
    return out;
}

// Fragment shader
@group(0) @binding(0)
var<uniform> scene_metadata: SceneMetadata;

@group(0) @binding(1)
var<uniform> camera: Camera;

@group(0) @binding(2)
var<storage, read_write> accum_image: array<PixelData>;

@group(0) @binding(3)
var<storage, read> primitive_list: array<Primitive>;

@group(0) @binding(4)
var<storage, read> material_list: array<Material>;

@group(0) @binding(5)
var<storage, read> texture_list: array<Texture>;

@group(0) @binding(6)
var<storage, read> tex_data: array<f32>;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let pixel_loc_u32 = vec2(u32(in.clip_position.x), u32(in.clip_position.y));
    var rng_state = random_init(pixel_loc_u32, camera.image_wh, camera.frame_id);
    return camera_render_pixel(in.clip_position, &rng_state);
}

// Ray tracing part

struct SceneMetadata {
    renderer_type: u32,
    root_id: u32,
    use_bvh: u32,
    // light_id: u32,
}

// Camera
struct Camera {
    image_wh: vec2<u32>,
    samples_per_pixel: u32,
    max_depth: u32,
    frame_id: u32,
    samples_per_frame: u32,
    defocus_angle: f32,

    center: vec3<f32>,
    pixel_delta_u: vec3<f32>,
    pixel_delta_v: vec3<f32>,
    pixel00_loc: vec3<f32>,
    defocus_disk_u: vec3<f32>,
    defocus_disk_v: vec3<f32>,
    background: vec3<f32>,
}

fn camera_render_pixel(pixel_loc: vec4<f32>, rng_state: ptr<function, u32>) -> vec4<f32> {
    let pixel_loc_u32 = vec2<u32>(pixel_loc.xy);
    let pixel_index = pixel_loc_u32.x + pixel_loc_u32.y * camera.image_wh.x;
    let prev_num_samples = min(camera.frame_id * camera.samples_per_frame, camera.samples_per_pixel);
    if prev_num_samples == camera.samples_per_pixel {
        let pixel_color = accum_image[pixel_index].rgb / f32(camera.samples_per_pixel);
        let gamma_color = linear_to_srgb(pixel_color);
        return vec4<f32>(gamma_color, 1.0);
    }

    switch scene_metadata.renderer_type {
        case 0u: { // pt
            for (var sample = 0u; sample < camera.samples_per_frame; sample += 1u) {
                let r = camera_get_ray(pixel_loc_u32.xy, rng_state);
                let pixel_color = pt_ray_color(r, rng_state);
                accum_image[pixel_index].rgb += pixel_color;
            }
        }
        case 1u: { // wfpt
            // Empty
        }
        default: {}
    }

    let now_num_samples = min((camera.frame_id + 1) * camera.samples_per_frame, camera.samples_per_pixel);
    let pixel_color = accum_image[pixel_index].rgb / f32(now_num_samples);
    let gamma_color = linear_to_srgb(pixel_color);

    return vec4<f32>(gamma_color, 1.0);
}

fn camera_get_ray(pixel_loc: vec2<u32>, rng_state: ptr<function, u32>) -> Ray {
    let pixel_loc_f32 = vec2<f32>(pixel_loc);
    let offset = sample_square(rng_state);
    let pixel_sample = camera.pixel00_loc + ((pixel_loc_f32.x + offset.x) * camera.pixel_delta_u) + ((pixel_loc_f32.y + offset.y) * camera.pixel_delta_v);
    var ray_origin = camera.center;
    if camera.defocus_angle > 0.0 {
        ray_origin = camera_defocus_disk_sample(rng_state);
    } 
    let ray_direction = pixel_sample - ray_origin;
    let ray_time = random_f32(rng_state);

    let r = Ray(ray_origin, ray_direction, ray_time);

    return r;
}

fn camera_defocus_disk_sample(rng_state: ptr<function, u32>) -> vec3<f32> {
    let p = random_vec3_in_unit_disk(rng_state);
    return camera.center + (p.x * camera.defocus_disk_u) + (p.y * camera.defocus_disk_v);
}

struct PixelData {
    rgb: vec3<f32>,
}

// Integrator
// Path Tracing
fn pt_ray_color(primary_ray: Ray, rng_state: ptr<function, u32>) -> vec3<f32> {
    var ray = primary_ray;
    var depth = 0u;
    var ray_color = vec3(0.0);
    var attenuation = vec3(1.0);

    while depth < camera.max_depth {
        var rec = hit_record_new();
        var hit = false;
        if scene_metadata.use_bvh == 1u {
            hit = primitive_hit(scene_metadata.root_id, ray, Interval(0.001, INF), &rec, rng_state);
        } else {
            hit = primitive_hit_list(ray, Interval(0.001, INF), &rec, rng_state);
        }
        if hit {
            var srec = scatter_record_new();
            let b_emit = material_emitted(rec.mat_id, rec, rec.uv, rec.p);
            if material_scatter(rec.mat_id, ray, rec, &srec, rng_state) {
                // Light importance sampling
                // scattered = Ray(rec.p, primitive_random(scene_metadata.light_id, rec.p, rng_state), r.tm);
                // pdf_value = primitive_pdf_value(scene_metadata.light_id, rec.p, scattered.dir, rng_state);

                let scattering_pdf = material_scatter_pdf(rec.mat_id, ray, rec, srec.scatter_direction);
                let pdf_value = scattering_pdf;

                ray = Ray(rec.p, srec.scatter_direction, ray.tm);
                ray_color += attenuation * b_emit;
                attenuation *= srec.attenuation * scattering_pdf / pdf_value;
                depth += 1u;
            } else {
                ray_color += attenuation * b_emit;
                break;
            }
        } else {
            ray_color += attenuation * camera.background;
            break;
        }
    }

    return ray_color;
}

// Wavefront Path Tracing
struct WavefrontRayPool {
    ray: array<Ray, 1048576>, // 2^20
    ray_id: array<u32, 1048576>,
    initialized: array<u32, 1048576>,
    terminated: array<u32, 1048576>,
    pixel: array<vec2<u32>, 1048576>,
    ray_color: array<vec3<f32>, 1048576>,
    depth: array<u32, 1048576>,
    attenuation: array<vec3<f32>, 1048576>,
    rng_state: array<u32, 1048576>,
    rec: array<HitRecord, 1048576>,
    hit: array<u32, 1048576>,
    srec: array<ScatterRecord, 1048576>,
    ray_count: atomic<u32>,
}

struct DispatchArgs {
    x: atomic<u32>,
    y: u32,
    z: u32,
    _pad: u32,
}

struct WavefrontQueues {
    new_path: array<u32, 1048576>, // 2^20
    material: array<u32, 1048576>,
    ray_cast: array<u32, 1048576>,
}

@group(1) @binding(0)
var<storage, read_write> wf_ray_pool: WavefrontRayPool;
@group(1) @binding(1)
var<storage, read_write> wf_queues: WavefrontQueues;
@group(1) @binding(2)
var<storage, read_write> dispatch_args: array<DispatchArgs>;

@compute @workgroup_size(256)
fn wavefront_logic(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x;
    if wf_ray_pool.initialized[i] == 0u {
        return;
    }
    if wf_ray_pool.terminated[i] == 1u || wf_ray_pool.depth[i] > camera.max_depth || wf_ray_pool.hit[i] == 0u {
        wf_ray_pool.initialized[i] = 0u;
        if wf_ray_pool.hit[i] == 0u {
            wf_ray_pool.ray_color[i] += wf_ray_pool.attenuation[i] * camera.background;
        }

        // Accum image
        let pixel_index = wf_ray_pool.pixel[i].x + wf_ray_pool.pixel[i].y * camera.image_wh.x;
        accum_image[pixel_index].rgb += wf_ray_pool.ray_color[i];

        // Request new primary ray
        let ray_count = atomicLoad(&wf_ray_pool.ray_count);
        if ray_count < camera.image_wh.x * camera.image_wh.y * camera.samples_per_frame {
            let new_path_index = atomicAdd(&dispatch_args[0].x, 1u);
            wf_queues.new_path[new_path_index] = gid.x;
            atomicAdd(&wf_ray_pool.ray_count, 1u);
        }
    } else {
        // Requst material evaluation
        let material_index = atomicAdd(&dispatch_args[1].x, 1u);
        wf_queues.material[material_index] = gid.x;

        // Update attenuation
        let pdf_value = wf_ray_pool.srec[i].sample_pdf;
        wf_ray_pool.attenuation[i] *= wf_ray_pool.srec[i].attenuation * wf_ray_pool.srec[i].scatter_pdf / pdf_value;
        wf_ray_pool.depth[i] += 1u;
    }
}

@compute @workgroup_size(256)
fn wavefront_new_path(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = wf_queues.new_path[gid.x];

    let ray_id = wf_ray_pool.ray_id[i];
    let pixel_index = ray_id / camera.samples_per_frame;
    let pixel = vec2(
        pixel_index % camera.image_wh.x,
        pixel_index / camera.image_wh.x,
    );
    let frame_sample_id = ray_id % camera.samples_per_frame;
    let sample_id = camera.frame_id * camera.samples_per_frame + frame_sample_id;
    var rng_state = random_init(pixel, camera.image_wh, sample_id);
    let ray = camera_get_ray(pixel, &rng_state);

    wf_ray_pool.ray[i] = ray;
    wf_ray_pool.ray_id[i] = ray_id;
    wf_ray_pool.initialized[i] = 1u;
    wf_ray_pool.terminated[i] = 0u;
    wf_ray_pool.pixel[i] = pixel;
    wf_ray_pool.ray_color[i] = vec3(0.0);
    wf_ray_pool.depth[i] = 0u;
    wf_ray_pool.attenuation[i] = vec3(1.0);
    wf_ray_pool.rng_state[i] = rng_state;
    wf_ray_pool.rec[i] = hit_record_new();
    wf_ray_pool.hit[i] = 0u;
    wf_ray_pool.srec[i] = scatter_record_new();

    // Request ray cast
    let ray_cast_index = atomicAdd(&dispatch_args[0].x, 1u);
    wf_queues.ray_cast[ray_cast_index] = i;
}

// Same as new_path but doesn't need queue (initial rays)
@compute @workgroup_size(256)
fn wavefront_init(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x;

    let ray_id = wf_ray_pool.ray_id[i];
    let pixel_index = ray_id / camera.samples_per_frame;
    let pixel = vec2(
        pixel_index % camera.image_wh.x,
        pixel_index / camera.image_wh.x,
    );
    let frame_sample_id = ray_id % camera.samples_per_frame;
    let sample_id = camera.frame_id * camera.samples_per_frame + frame_sample_id;
    var rng_state = random_init(pixel, camera.image_wh, sample_id);
    let ray = camera_get_ray(pixel, &rng_state);

    wf_ray_pool.ray[i] = ray;
    wf_ray_pool.ray_id[i] = ray_id;
    wf_ray_pool.terminated[i] = 0u;
    wf_ray_pool.pixel[i] = pixel;
    wf_ray_pool.ray_color[i] = vec3(0.0);
    wf_ray_pool.depth[i] = 0u;
    wf_ray_pool.attenuation[i] = vec3(1.0);
    wf_ray_pool.rng_state[i] = rng_state;
    wf_ray_pool.rec[i] = hit_record_new();
    wf_ray_pool.hit[i] = 0u;
    wf_ray_pool.srec[i] = scatter_record_new();

    // Request ray cast
    let ray_cast_index = atomicAdd(&dispatch_args[0].x, 1u);
    wf_queues.ray_cast[ray_cast_index] = i;
}

@compute @workgroup_size(256)
fn wavefront_material(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = wf_queues.material[gid.x];

    let emission = material_emitted(wf_ray_pool.rec[i].mat_id, wf_ray_pool.rec[i], wf_ray_pool.rec[i].uv, wf_ray_pool.rec[i].p);

    var srec = wf_ray_pool.srec[i];
    var rng_state = wf_ray_pool.rng_state[i];
    let bounce = material_scatter(wf_ray_pool.rec[i].mat_id, wf_ray_pool.ray[i], wf_ray_pool.rec[i], &srec, &rng_state);

    wf_ray_pool.ray[i] = Ray(wf_ray_pool.rec[i].p, srec.scatter_direction, wf_ray_pool.ray[i].tm);
    wf_ray_pool.srec[i] = srec;
    wf_ray_pool.rng_state[i] = rng_state;
    wf_ray_pool.terminated[i] = u32(!bounce);
    wf_ray_pool.ray_color[i] += wf_ray_pool.attenuation[i] * emission;

    // Request ray cast
    if bounce {
        let ray_cast_index = atomicAdd(&dispatch_args[0].x, 1u);
        wf_queues.ray_cast[ray_cast_index] = i;
    }
}

@compute @workgroup_size(256)
fn wavefront_ray_cast(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = wf_queues.ray_cast[gid.x];

    var rec = hit_record_new();
    var rng_state = wf_ray_pool.rng_state[i];
    var hit = false;
    if scene_metadata.use_bvh == 1u {
        hit = primitive_hit(scene_metadata.root_id, wf_ray_pool.ray[i], Interval(0.001, INF), &rec, &rng_state);
    } else {
        hit = primitive_hit_list(wf_ray_pool.ray[i], Interval(0.001, INF), &rec, &rng_state);
    }

    wf_ray_pool.hit[i] = u32(hit);
    wf_ray_pool.rec[i] = rec;
    wf_ray_pool.rng_state[i] = rng_state;
}


// Ray
struct Ray {
    orig: vec3<f32>,
    dir: vec3<f32>,
    tm: f32,
}

fn ray_new() -> Ray {
    return Ray(
        vec3(0.0),
        vec3(0.0),
        0.0,
    );
}

fn ray_at(ray: Ray, t: f32) -> vec3<f32> {
    return ray.orig + t * ray.dir;
}

// Ray intersection
struct HitRecord {
    p: vec3<f32>,
    mat_id: u32,
    normal: vec3<f32>,
    t: f32,
    uv: vec2<f32>,
    front_face: u32,
}

fn hit_record_new() -> HitRecord {
    return HitRecord(
        vec3(0.0, 0.0, 0.0),
        0,
        vec3(0.0, 0.0, 0.0),
        0.0,
        vec2(0.0, 0.0),
        0u,
    );
}

fn hit_record_set_face_normal(hit_record: ptr<function, HitRecord>, r: Ray, outward_normal: vec3<f32>) {
    (*hit_record).front_face = u32(dot(r.dir, outward_normal) < 0.0);
    if (*hit_record).front_face == 1u {
        (*hit_record).normal = outward_normal;
    } else {
        (*hit_record).normal = -outward_normal;
    }
}

// Interval
struct Interval {
    min: f32,
    max: f32,
}

fn interval_contains(it: Interval, x: f32) -> bool {
    return it.min <= x && x <= it.max;
}

fn interval_surrounds(it: Interval, x: f32) -> bool {
    return it.min < x && x < it.max;
}

fn interval_clamp(it: Interval, x: f32) -> f32 {
    if x < it.min { return it.min; }
    if x > it.max { return it.max; }
    return x;
}

fn interval_expand(it: Interval, delta: f32) -> Interval {
    let padding = delta / 2.0;
    return Interval(it.min - padding, it.max + padding);
}

// AABB
struct AABB {
    x: Interval,
    y: Interval,
    z: Interval,
}

fn aabb_axis(aabb: AABB, n: u32) -> Interval {
    if n == 1u { return aabb.y; }
    if n == 2u { return aabb.z; }
    return aabb.x;
}

fn aabb_hit(aabb: AABB, r: Ray, _ray_t: Interval) -> bool {
    let ray_orig = r.orig;
    let ray_dir = r.dir;
    var ray_t = _ray_t;

    for (var axis = 0u; axis < 3u; axis += 1u) {
        let ax = aabb_axis(aabb, axis);
        let adinv = 1.0 / ray_dir[axis];

        let t0 = (ax.min - ray_orig[axis]) * adinv;
        let t1 = (ax.max - ray_orig[axis]) * adinv;

        if t0 < t1 {
            if t0 > ray_t.min { ray_t.min = t0; }
            if t1 < ray_t.max { ray_t.max = t1; }
        } else {
            if t1 > ray_t.min { ray_t.min = t1; }
            if t0 < ray_t.max { ray_t.max = t0; }
        }

        if ray_t.max <= ray_t.min {
            return false;
        }
    }
    return true;
}

// Hittables
struct Primitive {
    type_id: u32,
    mat_id: i32,
    left_id: i32,
    right_id: i32,
    next_elem_id: i32,
    aabb: AABB,

    data0: vec4<f32>,
    data1: vec4<f32>,
    data2: vec4<f32>,
    data3: vec4<f32>,
    data4: vec4<f32>,
}

struct HitStackEntry {
    pid: u32,
    stage: u32,

    // Current status
    // cur_r: Ray,
    // cur_ray_t: Interval,
    // cur_rec: HitRecord,
    cur_hit: bool,

    // Local variable
    // rec1_t: f32,
}

fn primitive_hit(pid: u32, _r: Ray, _ray_t: Interval, _rec: ptr<function, HitRecord>, rng_state: ptr<function, u32>) -> bool {
    var stack: array<HitStackEntry, 8>;
    var stack_top = 1u;
    var r = _r;
    var ray_t = _ray_t;
    var hit = false;
    var rec = (*_rec);

    // For constant medium: does not support recursion where the boundary primitive of a cm is another cm
    var prev_ray_t = Interval(0.0, 0.0);
    var prev_rec = hit_record_new();
    var prev_rec1_t = 0.0;

    stack[0] = HitStackEntry(pid, 0u, hit);

    while (stack_top > 0u) {
        stack_top -= 1u;
        var this_entry = stack[stack_top];
        let p = primitive_list[this_entry.pid];

        if this_entry.stage == 0u && p.next_elem_id >= 0 {
            stack[stack_top] = HitStackEntry(u32(p.next_elem_id), 0u, hit);
            stack_top += 1u;
        }

        switch p.type_id {
            case 0u: { // bvh node or prim list
                if !aabb_hit(p.aabb, r, ray_t) {
                    continue;
                }

                if p.right_id >= 0 {
                    stack[stack_top] = HitStackEntry(u32(p.right_id), 0u, hit);
                    stack_top += 1u;
                }
                if p.left_id >= 0 {
                    stack[stack_top] = HitStackEntry(u32(p.left_id), 0u, hit);
                    stack_top += 1u;
                }
            }
            case 1u: { // sphere
                let center = p.data0.xyz;
                let center_dir = p.data1.xyz;
                let radius = p.data0.w;

                let current_center = center + r.tm * center_dir;
                let oc = current_center - r.orig;
                let a = dot(r.dir, r.dir);
                let h = dot(r.dir, oc);
                let c = dot(oc, oc) - radius * radius;
                let discriminant = h*h - a*c;

                if discriminant < 0.0 {
                    continue;
                }

                let sqrtd = sqrt(discriminant);

                var root = (h - sqrtd) / a;
                if !interval_surrounds(ray_t, root) {
                    root = (h + sqrtd) / a;
                    if !interval_surrounds(ray_t, root) {
                        continue;
                    }
                }

                rec.t = root;
                rec.p = ray_at(r, rec.t);
                let outward_normal = (rec.p - current_center) / radius;
                hit_record_set_face_normal(&rec, r, outward_normal);
                rec.uv = get_sphere_uv(outward_normal);
                rec.mat_id = u32(p.mat_id);

                hit = true;
                ray_t = Interval(ray_t.min, root);
            }
            case 2u: { // quad
                let q = p.data0.xyz;
                let u = p.data1.xyz;
                let v = p.data2.xyz;
                let normal = p.data3.xyz;
                let d = p.data3.w;
                let w = p.data4.xyz;

                // Plane test
                let denom = dot(normal, r.dir);
                if abs(denom) < 1e-8 { continue; }
                let t = (d - dot(normal, r.orig)) / denom;
                if !interval_contains(ray_t, t) { continue; }

                // Interior test
                let intersection = ray_at(r, t);
                let planar_hitpt_vector = intersection - q;
                let alpha = dot(w, cross(planar_hitpt_vector, v));
                let beta = dot(w, cross(u, planar_hitpt_vector));
                let unit_interval = Interval(0.0, 1.0);
                if !interval_contains(unit_interval, alpha) || !interval_contains(unit_interval, beta) { continue; }

                rec.t = t;
                rec.p = intersection;
                rec.mat_id = u32(p.mat_id);
                hit_record_set_face_normal(&rec, r, normal);
                rec.uv = vec2(alpha, beta);

                hit = true;
                ray_t = Interval(ray_t.min, t);
            }
            case 3u: { // translate
                let offset = p.data0.xyz;

                switch this_entry.stage {
                    case 0u: {
                        stack[stack_top] = HitStackEntry(this_entry.pid, 1u, hit);
                        stack_top += 1u;

                        r.orig -= offset;
                        hit = false;
                        stack[stack_top] = HitStackEntry(u32(p.right_id), 0u, hit);
                        stack_top += 1u;
                    }
                    case 1u: {
                        if hit {
                            rec.p += offset;
                            hit = true;
                        } else {
                            hit = this_entry.cur_hit;
                        }
                        r.orig += offset;
                    }
                    default: {}
                }
            }
            case 4u: { // rotate_y
                let sin_theta = p.data0.x;
                let cos_theta = p.data0.y;

                switch this_entry.stage {
                    case 0u: {
                        stack[stack_top] = HitStackEntry(this_entry.pid, 1u, hit);
                        stack_top += 1u;

                        let origin = vec3(
                            cos_theta * r.orig.x - sin_theta * r.orig.z,
                            r.orig.y,
                            sin_theta * r.orig.x + cos_theta * r.orig.z,
                        );
                        let direction = vec3(
                            cos_theta * r.dir.x - sin_theta * r.dir.z,
                            r.dir.y,
                            sin_theta * r.dir.x + cos_theta * r.dir.z,
                        );
                        r = Ray(origin, direction, r.tm);
                        hit = false;

                        stack[stack_top] = HitStackEntry(u32(p.right_id), 0u, hit);
                        stack_top += 1u;
                    }
                    case 1u: {
                        if hit {
                            rec.p = vec3(
                                cos_theta * rec.p.x + sin_theta * rec.p.z,
                                rec.p.y,
                                -sin_theta * rec.p.x + cos_theta * rec.p.z,
                            );
                            rec.normal = vec3(
                                cos_theta * rec.normal.x + sin_theta * rec.normal.z,
                                rec.normal.y,
                                -sin_theta * rec.normal.x + cos_theta * rec.normal.z,
                            );
                            hit = true;
                        } else {
                            hit = this_entry.cur_hit;
                        }
                        let origin = vec3(
                            cos_theta * r.orig.x + sin_theta * r.orig.z,
                            r.orig.y,
                            -sin_theta * r.orig.x + cos_theta * r.orig.z,
                        );
                        let direction = vec3(
                            cos_theta * r.dir.x + sin_theta * r.dir.z,
                            r.dir.y,
                            -sin_theta * r.dir.x + cos_theta * r.dir.z,
                        );
                        r = Ray(origin, direction, r.tm);
                    }
                    default: {}
                }
            }
            case 5u: { // constant medium
                let neg_inv_density = p.data0.x;

                switch this_entry.stage {
                    case 0u: {
                        prev_ray_t = ray_t;
                        prev_rec = rec;

                        stack[stack_top] = HitStackEntry(this_entry.pid, 1u, hit);
                        stack_top += 1u;

                        ray_t = Interval(-INF, INF);
                        rec = hit_record_new();
                        hit = false;

                        stack[stack_top] = HitStackEntry(u32(p.right_id), 0u, hit);
                        stack_top += 1u;
                    }
                    case 1u: {
                        if hit {
                            prev_rec1_t = rec.t;
                            stack[stack_top] = HitStackEntry(
                                this_entry.pid,
                                2u,
                                this_entry.cur_hit,
                            );
                            stack_top += 1u;

                            ray_t = Interval(rec.t + 0.0001, INF);
                            rec = hit_record_new();
                            hit = false;

                            stack[stack_top] = HitStackEntry(u32(p.right_id), 0u, hit);
                            stack_top += 1u;
                        } else {
                            ray_t = prev_ray_t;
                            rec = prev_rec;
                            hit = this_entry.cur_hit;
                        }
                    }
                    case 2u: {
                        var this_hit = false;
                        if hit {
                            var rec1_t = prev_rec1_t;
                            var rec2 = rec;
                            if rec1_t < prev_ray_t.min { rec1_t = prev_ray_t.min; }
                            if rec2.t > prev_ray_t.max { rec2.t = prev_ray_t.max; }
                            if rec1_t < rec2.t {
                                if rec1_t < 0.0 { rec1_t = 0.0; }
                                let ray_length = length(r.dir);
                                let distance_inside_boundary = (rec2.t - rec1_t) * ray_length;
                                let hit_distance = neg_inv_density * log(random_f32(rng_state));

                                if hit_distance <= distance_inside_boundary {
                                    rec.t = rec1_t + hit_distance / ray_length;
                                    rec.p = ray_at(r, rec.t);
                                    rec.normal = vec3(1.0, 0.0, 0.0);
                                    rec.front_face = 1u;
                                    rec.mat_id = u32(p.mat_id);
                                    this_hit = true;
                                }
                            }
                        }

                        if this_hit {
                            ray_t = Interval(prev_ray_t.min, rec.t);
                            hit = true;
                        } else {
                            ray_t = prev_ray_t;
                            rec = prev_rec;
                            hit = this_entry.cur_hit;
                        }
                    }
                    default: {}
                }
            }
            default: {}
        }
    }

    (*_rec) = rec;
    return hit;
}

fn primitive_pdf_value(pid: u32, origin: vec3<f32>, direction: vec3<f32>, rng_state: ptr<function, u32>) -> f32 {
    let prim = primitive_list[pid];
    switch prim.type_id {
        case 2u: { // quad
            let area = prim.data4.w;

            var rec = hit_record_new();
            if !primitive_hit(pid, Ray(origin, direction, 0.0), Interval(0.001, INF), &rec, rng_state) {
                return 0.0;
            }
            let distance_squared = rec.t * rec.t * dot(direction, direction);
            let cosine = abs(dot(direction, rec.normal) / length(direction));
            return distance_squared / (cosine * area);
        }
        default: { return 0.0; }
    }
}

fn primitive_random(pid: u32, origin: vec3<f32>, rng_state: ptr<function, u32>) -> vec3<f32> {
    let prim = primitive_list[pid];
    switch prim.type_id {
        case 2u: { // quad
            let q = prim.data0.xyz;
            let u = prim.data1.xyz;
            let v = prim.data2.xyz;

            let p = q + (random_f32(rng_state) * u) + (random_f32(rng_state) * v);
            return p - origin;
        }
        default: { return vec3(0.0); }
    }
}

fn primitive_hit_list(r: Ray, ray_t: Interval, rec: ptr<function, HitRecord>, rng_state: ptr<function, u32>) -> bool {
    var temp_rec = hit_record_new();
    var hit_anything = false;
    var closest_so_far = ray_t.max;
    for (var i = 0u; i < arrayLength(&primitive_list); i = i + 1u) {
        let hit = primitive_hit(i , r, Interval(ray_t.min, closest_so_far), &temp_rec, rng_state);
        if hit {
            hit_anything = true;
            closest_so_far = temp_rec.t;
            (*rec) = temp_rec;
        }
    }

    return hit_anything;
}

fn get_sphere_uv(p: vec3<f32>) -> vec2<f32> {
    let theta = acos(-p.y);
    let phi = atan2(-p.z, p.x) + PI;

    return vec2(
        phi / (2.0 * PI),
        theta / PI,
    );
}

// Material
struct ScatterRecord {
    scatter_direction: vec3<f32>,
    scatter_pdf: f32,
    attenuation: vec3<f32>,
    sample_pdf: f32,
};

fn scatter_record_new() -> ScatterRecord {
    return ScatterRecord(
        vec3(0.0),
        1.0,
        vec3(1.0),
        1.0,
    );
}

struct Material {
    type_id: u32,
    tex_id: i32,
    data0: vec4<f32>,
}

fn material_scatter(
    mat_id: u32,
    r_in: Ray,
    rec: HitRecord,
    srec: ptr<function, ScatterRecord>,
    rng_state: ptr<function, u32>,
) -> bool {
    let mat = material_list[mat_id];
    switch mat.type_id {
        case 0u: { // lambertian
            let uvw = onb_new(rec.normal);
            var scatter_direction = normalize(onb_transform(uvw, random_cosine_direction(rng_state)));

            (*srec).scatter_direction = scatter_direction;
            (*srec).scatter_pdf = dot(uvw.w, scatter_direction) / PI;
            (*srec).attenuation = texture_value(u32(mat.tex_id), rec.uv, rec.p);
            return true;
        }
        case 1u: { // metal
            let albedo = mat.data0.xyz;
            let fuzz = mat.data0.w;

            var reflected = reflect(r_in.dir, rec.normal);
            reflected = normalize(reflected) + (fuzz * random_vec3_unit(rng_state));

            (*srec).scatter_direction = reflected;
            (*srec).scatter_pdf = 1.0;
            (*srec).attenuation = albedo;
            return true;
        }
        case 2u: { // dielectric
            let refraction_index = mat.data0.x;

            var ri = refraction_index;
            if rec.front_face == 1u {
                ri = 1.0 / refraction_index;
            }
            let unit_direction = normalize(r_in.dir);
            let cos_theta = min(dot(-unit_direction, rec.normal), 1.0);
            let sin_theta = sqrt(1.0 - cos_theta * cos_theta);
            let cannot_refract = ri * sin_theta > 1.0;
            var direction = vec3(0.0);
            if cannot_refract || dielectric_reflectance(cos_theta, ri) > random_f32(rng_state) {
                direction = reflect(unit_direction, rec.normal);
            } else {
                direction = refract(unit_direction, rec.normal, ri);
            }

            (*srec).scatter_direction = direction;
            (*srec).scatter_pdf = 1.0;
            (*srec).attenuation = vec3(1.0);
            return true;
        }
        case 3u: { // diffuse light
            return false;
        }
        case 4u: { // isotropic
            (*srec).scatter_direction = random_vec3_unit(rng_state);
            (*srec).scatter_pdf = 1.0 / (4.0 * PI);
            (*srec).attenuation = texture_value(u32(mat.tex_id), rec.uv, rec.p);
            return true;
        }
        default: {
            return false;
        }
    }
}

fn material_scatter_pdf(
    mat_id: u32,
    r_in: Ray,
    rec: HitRecord,
    scattered_dir: vec3<f32>,
) -> f32 {
    let mat = material_list[mat_id];
    switch mat.type_id {
        case 0u: { // Lambertian
            let cos_theta = dot(rec.normal, normalize(scattered_dir));
            if cos_theta < 0.0 {
                return 0.0;
            } else {
                return cos_theta / PI;
            }
        }
        case 1u: {
            return 1.0;
        }
        case 2u: {
            return 1.0;
        }
        case 4u: { // isotropic
            return 1.0 / (4.0 * PI);
        }
        default: { return 0.0; }
    }
}

fn material_emitted(
    mat_id: u32,
    rec: HitRecord,
    uv: vec2<f32>,
    p: vec3<f32>,
) -> vec3<f32> {
    let mat = material_list[mat_id];
    switch mat.type_id {
        case 3u: { // diffuse light
            if rec.front_face == 0u {
                return vec3(0.0);
            }
            return texture_value(u32(mat.tex_id), uv, p);
        }
        default: {
            return vec3(0.0);
        }
    }
}

fn dielectric_reflectance(cosine: f32, refraction_index: f32) -> f32 {
    // Use Schlick's approximation for reflectance.
    var r0 = (1.0 - refraction_index) / (1.0 + refraction_index);
    r0 = r0 * r0;
    return r0 + (1.0 - r0) * pow((1.0 - cosine), 5.0);
}

// Texture
struct Texture {
    type_id: u32,
    start: u32,
    end: u32,

    data0: vec4<f32>,
}

fn texture_value(
    tex_id: u32,
    uv: vec2<f32>,
    p: vec3<f32>,
) -> vec3<f32> {
    var stack: array<u32, 32>;
    var stack_top = 1u;
    stack[0] = tex_id;

    while stack_top > 0u {
        stack_top -= 1u;
        let tex = texture_list[stack[stack_top]];

        switch tex.type_id {
            case 0u: { // solid color
                return tex.data0.xyz;
            }
            case 1u: { // checker
                let x = i32(floor(tex.data0.z * p.x));
                let y = i32(floor(tex.data0.z * p.y));
                let z = i32(floor(tex.data0.z * p.z));

                let is_even = (x + y + z) % 2 == 0;
                if is_even {
                    stack[stack_top] = u32(tex.data0.x);
                    stack_top += 1u;
                } else {
                    stack[stack_top] = u32(tex.data0.y);
                    stack_top += 1u;
                }
            }
            case 2u: { // image
                let img_w = tex.data0.x;
                let img_h = tex.data0.y;
                if img_h <= 0.0 {
                    return vec3(0.0, 1.0, 1.0);
                }
                let it = Interval(0.0, 1.0);
                let clamp_uv = vec2(
                    interval_clamp(it, uv.x),
                    1.0 - interval_clamp(it, uv.y),
                );
                let i = u32(clamp_uv.x * img_w);
                let j = u32(clamp_uv.y * img_h);
                let index = 4u * (j * u32(img_w) + i);
                let r = tex_data[tex.start + index];
                let g = tex_data[tex.start + index + 1u];
                let b = tex_data[tex.start + index + 2u];

                return srgb_to_linear(vec3(r, g, b));
            }
            case 3u: { // noise
                let point_count = u32(tex.data0.x);
                let scale = tex.data0.y;

                return vec3(0.5) * (1.0 + sin(scale * p.z + 10.0 * noise_turb(tex.start, point_count, p, 7u)));
            }
            default: {
                return vec3(0.0);
            }
        }
    }

    return vec3(0.0);
}

fn noise(tex_start: u32, point_count: u32, p: vec3<f32>) -> f32 {
    var uvw = p - floor(p);
    let i = i32(floor(p.x));
    let j = i32(floor(p.y));
    let k = i32(floor(p.z));
    var c = array<vec3<f32>, 8>();
    for (var di = 0u; di < 2u; di += 1u) {
        for (var dj = 0u; dj < 2u; dj += 1u) {
            for (var dk = 0u; dk < 2u; dk += 1u) {
                let perm_x = bitcast<u32>(tex_data[tex_start + point_count * 3u + u32((i+i32(di)) & 255)]);
                let perm_y = bitcast<u32>(tex_data[tex_start + point_count * 4u + u32((j+i32(dj)) & 255)]);
                let perm_z = bitcast<u32>(tex_data[tex_start + point_count * 5u + u32((k+i32(dk)) & 255)]);
                c[di * 4u + dj * 2u + dk] = vec3(
                    tex_data[tex_start + 3u * (perm_x ^ perm_y ^ perm_z)],
                    tex_data[tex_start + 3u * (perm_x ^ perm_y ^ perm_z) + 1u],
                    tex_data[tex_start + 3u * (perm_x ^ perm_y ^ perm_z) + 2u],
                );
            }
        }
    }

    let rand_float = trilinear_interp(c, uvw);
    return rand_float;
}

fn noise_turb(tex_start: u32, point_count: u32, p: vec3<f32>, depth: u32) -> f32 {
    var accum = 0.0;
    var temp_p = p;
    var weight = 1.0;

    for (var i = 0u; i < depth; i += 1u) {
        accum += weight * noise(tex_start, point_count, temp_p);
        weight *= 0.5;
        temp_p *= 2.0;
    }

    return abs(accum);
}

struct ONB {
    u: vec3<f32>,
    v: vec3<f32>,
    w: vec3<f32>,
}

fn onb_new(n: vec3<f32>) -> ONB {
    let w = normalize(n);
    var a = vec3(1.0, 0.0, 0.0);
    if abs(w.x) > 0.9 {
        a = vec3(0.0, 1.0, 0.0);
    }
    let v = normalize(cross(w, a));
    let u = cross(w, v);
    return ONB(u, v, w);
}

fn onb_transform(onb: ONB, v: vec3<f32>) -> vec3<f32> {
    return (v.x * onb.u) + (v.y * onb.v) + (v.z * onb.w);
}

// Utils
const INF: f32 = 3.402823e38;
const PI: f32 = 3.141592653589793;

fn linear_to_srgb(c: vec3<f32>) -> vec3<f32> {
    var srgb_c = c;
    for (var i = 0u; i < 3u; i += 1u) {
        if c[i] <= 0.0031308 {
            srgb_c[i] *= 12.92;
        } else {
            srgb_c[i] = 1.055 * pow(c[i], 1.0 / 2.4) - 0.055;
        }
    }
    return srgb_c;
}

fn srgb_to_linear(c: vec3<f32>) -> vec3<f32> {
    var linear_c = c;
    for (var i = 0u; i < 3u; i += 1u) {
        if c[i] <= 0.04045 {
            linear_c[i] /= 12.92;
        } else {
            linear_c[i] = pow(((c[i] + 0.055) / 1.055), 2.4);
        }
    }
    return linear_c;
}

fn vec3_near_zero(v: vec3<f32>) -> bool {
    let s = 1e-8;
    return (abs(v.x) < s) && (abs(v.y) < s) && (abs(v.z) < s);
}

fn trilinear_interp(c: array<vec3<f32>, 8>, uvw: vec3<f32>) -> f32 {
    let new_uvw = uvw * uvw * (3.0 - 2.0 * uvw);
    var accum = 0.0;
    for (var i = 0u; i < 2u; i += 1u) {
        for (var j = 0u; j < 2u; j += 1u) {
            for (var k = 0u; k < 2u; k += 1u) {
                let weight_v = vec3(
                    uvw.x - f32(i),
                    uvw.y - f32(j),
                    uvw.z - f32(k),
                );
                accum += (f32(i)*new_uvw.x + f32(1u-i)*(1.0-new_uvw.x))
                    * (f32(j)*new_uvw.y + f32(1u-j)*(1.0-new_uvw.y))
                    * (f32(k)*new_uvw.z + f32(1u-k)*(1.0-new_uvw.z))
                    * dot(c[i * 4u + j * 2u + k], weight_v);
            }
        }
    }

    return accum;
}

// Random
// https://nelari.us/post/weekend_raytracing_with_wgpu_1/
fn jenkins_hash(input: u32) -> u32 {
    var x = input;
    x += x << 10u;
    x ^= x >> 6u;
    x += x << 3u;
    x ^= x >> 11u;
    x += x << 15u;
    return x;
}

fn random_init(pixel: vec2<u32>, resolution: vec2<u32>, frame: u32) -> u32 {
    // Adapted from https://github.com/boksajak/referencePT
    let seed = dot(pixel, vec2<u32>(1u, resolution.x)) ^ jenkins_hash(frame);
    return jenkins_hash(seed);
}

fn random_u32(state: ptr<function, u32>) -> u32 {
    // PCG random number generator
    // Based on https://www.shadertoy.com/view/XlGcRh
    let newState = *state * 747796405u + 2891336453u;
    *state = newState;
    let word = ((newState >> ((newState >> 28u) + 4u)) ^ newState) * 277803737u;
    return (word >> 22u) ^ word;
}

fn random_f32(state: ptr<function, u32>) -> f32 {
    let x = random_u32(state);
    return f32(x) / f32(0xffffffffu);
}

fn random_f32_range(state: ptr<function, u32>, r_min: f32, r_max: f32) -> f32 {
    return r_min + (r_max - r_min)*random_f32(state);
}

fn sample_square(state: ptr<function, u32>) -> vec2<f32> {
    let x = random_f32(state);
    let y = random_f32(state);
    return vec2(x, y);
}

fn random_vec3_unit(state: ptr<function, u32>) -> vec3<f32> {
    let cosTheta = 1f - 2f * random_f32(state);
    let sinTheta = sqrt(1f - cosTheta * cosTheta);
    let phi = 2f * PI * random_f32(state);

    let x = sinTheta * cos(phi);
    let y = sinTheta * sin(phi);
    let z = cosTheta;

    return vec3(x, y, z);
}

fn random_vec3_unit_hemisphere(state: ptr<function, u32>, normal: vec3<f32>) -> vec3<f32> {
    let on_unit_sphere = random_vec3_unit(state);
    if dot(on_unit_sphere, normal) > 0.0 {
        return on_unit_sphere;
    } else {
        return -on_unit_sphere;
    }
}

fn random_vec3_in_unit_disk(state: ptr<function, u32>) -> vec3<f32> {
    // r^2 ~ U(0, 1)
    let r = sqrt(random_f32(state));
    let alpha = 2f * PI * random_f32(state);

    let x = r * cos(alpha);
    let y = r * sin(alpha);

    return vec3(x, y, 0f);
}

fn random_cosine_direction(state: ptr<function, u32>) -> vec3<f32> {
    let r1 = random_f32(state);
    let r2 = random_f32(state);
    let phi = 2.0 * PI * r1;
    let x = cos(phi) * sqrt(r2);
    let y = sin(phi) * sqrt(r2);
    let z = sqrt(1.0 - r2);

    return vec3(x, y, z);
}