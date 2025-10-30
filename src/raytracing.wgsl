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
var<uniform> camera: Camera;

@group(0) @binding(1)
var<storage, read_write> accum_image: array<PixelData>;

@group(1) @binding(0)
var<uniform> scene_metadata: SceneMetadata;

@group(1) @binding(1)
var<storage, read> primitive_list: array<Primitive>;

@group(1) @binding(2)
var<storage, read> material_list: array<Material>;

@group(1) @binding(3)
var<storage, read> texture_list: array<Texture>;

@group(1) @binding(4)
var<storage, read> tex_data: array<f32>;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let pixel_loc_u32 = vec2(u32(in.clip_position.x), u32(in.clip_position.y));
    var rng_state = random_init(pixel_loc_u32, camera.image_wh, camera.frame_id);
    return camera_render_pixel(in.clip_position, &rng_state);
}

// Ray tracing part

struct SceneMetadata {
    root_id: u32,
    use_bvh: u32,
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
    let pixel_loc_u32 = vec2(u32(pixel_loc.x), u32(pixel_loc.y));
    let pixel_index = pixel_loc_u32.x + pixel_loc_u32.y * camera.image_wh.x;
    let prev_num_samples = min(camera.frame_id * camera.samples_per_frame, camera.samples_per_pixel);
    var pixel_color = vec3(0.0);
    if prev_num_samples > 0u {
        pixel_color = accum_image[pixel_index].rgb * f32(prev_num_samples);
    }
    if prev_num_samples == camera.samples_per_pixel {
        let gamma_color = linear_to_srgb(pixel_color / f32(camera.samples_per_pixel));
        return vec4<f32>(gamma_color, 1.0);
    }

    let now_num_samples = min((camera.frame_id + 1u) * camera.samples_per_frame, camera.samples_per_pixel);
    let pixel_samples_scale = 1.0 / f32(now_num_samples);
    for (var sample = prev_num_samples; sample < now_num_samples; sample += 1u) {
        let r = camera_get_ray(pixel_loc.xy, rng_state);
        pixel_color += camera_ray_color(r, rng_state);
    }

    let linear_color = pixel_color * pixel_samples_scale;
    accum_image[pixel_index] = PixelData(linear_color);
    let gamma_color = linear_to_srgb(linear_color);

    return vec4<f32>(gamma_color, 1.0);
}

fn camera_ray_color(primary_ray: Ray, rng_state: ptr<function, u32>) -> vec3<f32> {
    var rec = hit_record_new();
    var ray_color = vec3(0.0);
    var attenuation = vec3(1.0);
    var emission = vec3(0.0);
    var r = primary_ray;

    for (var depth = 0u; depth < camera.max_depth; depth += 1u) {
        var hit = false;
        if scene_metadata.use_bvh == 1u {
            hit = primitive_hit(scene_metadata.root_id, r, Interval(0.001, INF), &rec);
        } else {
            hit = primitive_hit_list(r, Interval(0.001, INF), &rec);
        }
        if hit {
            var scattered = ray_new();
            var b_att = vec3(0.0);
            let b_emit = material_emitted(rec.mat_id, rec.uv, rec.p);
            if material_scatter(rec.mat_id, r, rec, &b_att, &scattered, rng_state) {
                r = scattered;
                emission += attenuation * b_emit;
                attenuation *= b_att;
            } else {
                emission += attenuation * b_emit;
                break;
            }
        } else {
            ray_color = camera.background;
            break;
        }
    }

    return emission + attenuation * ray_color;
}

fn camera_get_ray(pixel_loc: vec2<f32>, rng_state: ptr<function, u32>) -> Ray {
    let offset = sample_square(rng_state);
    let pixel_sample = camera.pixel00_loc + ((pixel_loc.x + offset.x) * camera.pixel_delta_u) + ((pixel_loc.y + offset.y) * camera.pixel_delta_v);
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
    let p = random_vec3_unit_disk(rng_state);
    return camera.center + (p.x * camera.defocus_disk_u) + (p.y * camera.defocus_disk_v);
}

struct PixelData {
    rgb: vec3<f32>,
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
    normal: vec3<f32>,
    mat_id: u32,
    t: f32,
    uv: vec2<f32>,
    front_face: bool,
}

fn hit_record_new() -> HitRecord {
    return HitRecord(
        vec3(0.0, 0.0, 0.0),
        vec3(0.0, 0.0, 0.0),
        0,
        0.0,
        vec2(0.0, 0.0),
        true,
    );
}

fn hit_record_set_face_normal(hit_record: ptr<function, HitRecord>, r: Ray, outward_normal: vec3<f32>) {
    (*hit_record).front_face = dot(r.dir, outward_normal) < 0.0;
    if (*hit_record).front_face {
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
    hit: bool,
    report_index: i32,
}

fn primitive_hit(pid: u32, _r: Ray, _ray_t: Interval, rec: ptr<function, HitRecord>) -> bool {
    var stack: array<HitStackEntry, 32>;
    var stack_top = 1u;
    var hit = false;
    var r = _r;
    var ray_t = _ray_t;

    stack[0] = HitStackEntry(pid, 0u, false, -1);

    while (stack_top > 0u) {
        stack_top -= 1u;
        var this_entry = stack[stack_top];
        let p = primitive_list[this_entry.pid];

        if p.next_elem_id >= 0 {
            stack[stack_top] = HitStackEntry(u32(p.next_elem_id), 0u, false, this_entry.report_index);
            stack_top += 1u;
        }

        switch p.type_id {
            case 0u: { // bvh node or prim list
                if !aabb_hit(p.aabb, r, ray_t) {
                    continue;
                }

                if p.right_id >= 0 {
                    stack[stack_top] = HitStackEntry(u32(p.right_id), 0u, false, this_entry.report_index);
                    stack_top += 1u;
                }
                if p.left_id >= 0 {
                    stack[stack_top] = HitStackEntry(u32(p.left_id), 0u, false, this_entry.report_index);
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

                (*rec).t = root;
                (*rec).p = ray_at(r, (*rec).t);
                let outward_normal = ((*rec).p - current_center) / radius;
                hit_record_set_face_normal(rec, r, outward_normal);
                (*rec).uv = get_sphere_uv(outward_normal);
                (*rec).mat_id = u32(p.mat_id);

                hit = true;
                this_entry.hit = true;
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

                (*rec).t = t;
                (*rec).p = intersection;
                (*rec).mat_id = u32(p.mat_id);
                hit_record_set_face_normal(rec, r, normal);
                (*rec).uv = vec2(alpha, beta);

                hit = true;
                this_entry.hit = true;
                ray_t = Interval(ray_t.min, t);
            }
            case 3u: { // translate
                let offset = p.data0.xyz;

                switch this_entry.stage {
                    case 0u: {
                        r.orig -= offset;
                        stack[stack_top] = HitStackEntry(this_entry.pid, 1u, false, this_entry.report_index);
                        stack[stack_top + 1u] = HitStackEntry(u32(p.right_id), 0u, false, i32(stack_top));
                        stack_top += 2u;
                    }
                    case 1u: {
                        r.orig += offset;

                        if this_entry.hit {
                            (*rec).p += offset;
                        }
                    }
                    default: {}
                }
            }
            case 4u: { // rotate_y
                let sin_theta = p.data0.x;
                let cos_theta = p.data0.y;

                switch this_entry.stage {
                    case 0u: {
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

                        stack[stack_top] = HitStackEntry(this_entry.pid, 1u, false, this_entry.report_index);
                        stack[stack_top + 1u] = HitStackEntry(u32(p.right_id), 0u, false, i32(stack_top));
                        stack_top += 2u;
                    }
                    case 1u: {
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

                        if this_entry.hit {
                            (*rec).p = vec3(
                                cos_theta * (*rec).p.x + sin_theta * (*rec).p.z,
                                (*rec).p.y,
                                -sin_theta * (*rec).p.x + cos_theta * (*rec).p.z,
                            );
                            (*rec).normal = vec3(
                                cos_theta * (*rec).normal.x + sin_theta * (*rec).normal.z,
                                (*rec).normal.y,
                                -sin_theta * (*rec).normal.x + cos_theta * (*rec).normal.z,
                            );
                        }
                    }
                    default: {}
                }
            }
            default: {}
        }

        if this_entry.hit && this_entry.report_index >= 0 {
            stack[u32(this_entry.report_index)].hit = true;
        }
    }

    return hit;
}

fn primitive_hit_list(r: Ray, ray_t: Interval, rec: ptr<function, HitRecord>) -> bool {
    var temp_rec = hit_record_new();
    var hit_anything = false;
    var closest_so_far = ray_t.max;
    for (var i = 0u; i < arrayLength(&primitive_list); i = i + 1u) {
        let hit = primitive_hit(i , r, Interval(ray_t.min, closest_so_far), &temp_rec);
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
        phi / (2 * PI),
        theta / PI,
    );
}

// Material
struct Material {
    type_id: u32,
    tex_id: i32,
    data0: vec4<f32>,
}

fn material_scatter(
    mat_id: u32,
    r_in: Ray,
    rec: HitRecord,
    attenuation: ptr<function, vec3<f32>>,
    scattered: ptr<function, Ray>,
    rng_state: ptr<function, u32>,
) -> bool {
    let mat = material_list[mat_id];
    switch mat.type_id {
        case 0u: { // lambertian
            var scatter_direction = rec.normal + random_vec3(rng_state);
            if vec3_near_zero(scatter_direction) {
                scatter_direction = rec.normal;
            }

            (*scattered) = Ray(rec.p, scatter_direction, r_in.tm);
            (*attenuation) = texture_value(u32(mat.tex_id), rec.uv, rec.p);
            return true;
        }
        case 1u: { // metal
            let albedo = mat.data0.xyz;
            let fuzz = mat.data0.w;

            var reflected = reflect(r_in.dir, rec.normal);
            reflected = normalize(reflected) + (fuzz * random_vec3(rng_state));

            (*scattered) = Ray(rec.p, reflected, r_in.tm);
            (*attenuation) = albedo;
            return true;
        }
        case 2u: { // dielectric
            let refraction_index = mat.data0.x;

            var ri = refraction_index;
            if rec.front_face {
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

            (*scattered) = Ray(rec.p, direction, r_in.tm);
            (*attenuation) = vec3(1.0);
            return true;
        }
        case 3u: { // diffuse light
            return false;
        }
        default: {
            return false;
        }
    }
}

fn material_emitted(
    mat_id: u32,
    uv: vec2<f32>,
    p: vec3<f32>,
) -> vec3<f32> {
    let mat = material_list[mat_id];
    switch mat.type_id {
        case 3u: { // diffuse light
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

fn random_vec3(state: ptr<function, u32>) -> vec3<f32> {
    // r^3 ~ U(0, 1)
    let r = pow(random_f32(state), 0.33333f);
    let cosTheta = 1f - 2f * random_f32(state);
    let sinTheta = sqrt(1f - cosTheta * cosTheta);
    let phi = 2f * PI * random_f32(state);

    let x = r * sinTheta * cos(phi);
    let y = r * sinTheta * sin(phi);
    let z = cosTheta;

    return vec3(x, y, z);
}

fn random_vec3_hemisphere(state: ptr<function, u32>, normal: vec3<f32>) -> vec3<f32> {
    let on_unit_sphere = random_vec3(state);
    if dot(on_unit_sphere, normal) > 0.0 {
        return on_unit_sphere;
    } else {
        return -on_unit_sphere;
    }
}

fn random_vec3_unit_disk(state: ptr<function, u32>) -> vec3<f32> {
    // r^2 ~ U(0, 1)
    let r = sqrt(random_f32(state));
    let alpha = 2f * PI * random_f32(state);

    let x = r * cos(alpha);
    let y = r * sin(alpha);

    return vec3(x, y, 0f);
}