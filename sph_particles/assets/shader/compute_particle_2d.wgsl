//!include world.h.wgsl math.h.wgsl

// particles.rs Particle
struct SphParticle {
    position: vec3<f32>, // only use xy
    density: f32,
    velocity: vec3<f32>,
    support_radius: f32,
    pressure: vec3<f32>,
    particle_radius: f32,
    ptype: u32, // 0 for fluid, 1 for boundary
    _pad: array<f32, 3>,
}

const GRAVITY: vec3f = vec3f(0.0, -9.8, 0.0);
const time_step: f32 = 1e-4;

@group(0) @binding(0)
var<storage, read_write> particles_in: array<SphParticle>;

@group(0) @binding(1)
var<storage, read_write> particles_out: array<SphParticle>;

@compute
@workgroup_size(256, 1, 1)
fn cs_main(
    @builtin(global_invocation_id) gid: vec3<u32>,
    @builtin(local_invocation_id) lid: vec3<u32>,
    @builtin(workgroup_id) wid: vec3<u32>
) {
    // WORKGROUP_SIZE: (4096, 1024, 1)
    let id = (gid.x + gid.y * 4096u);
    if id >= arrayLength(&particles_in) {
        return;
    }

    let y_lower = world_boundary_y().x;
    let p = particles_in[id];
    var p_next = p;

    if p.ptype == 0 { // fluid
        // 1. Apply gravity
        p_next.velocity += GRAVITY;
        p_next.position += p_next.velocity * time_step;
        // 2. Solve boundary constraints
        p_next = solve_boundary_constraints(p_next);

        // last: update particles
    } // else don't update

    particles_out[id] = p_next;
}


// =========================================================
//  WCSPH implementation

const dx = 0.1;
const h_fac = 3.0;
const dh: f32 = dx * h_fac; // kernel radius
const alpha: f32 = 0.5;
const c_s: f32 = 100.0;

fn get_mass(p: SphParticle) -> f32 {
    var density_sum = 0.0;

    return p.density / density_sum;

    // let density = p.density;
    // let r = p.particle_radius;
    // let volume = 4.0 * M_PI * r * r * r / 3.0;
    // return density * volume;
}

fn rho_sum(p: SphParticle, x_ab: vec2f) -> f32 {
    let m = get_mass(p);
    return m * cubicKernel(x_ab, dh);
}

fn rho_dt(p: SphParticle, v_ab: vec2f, x_ab: vec2f) -> f32 {
    let m = get_mass(p);
    return dot(v_ab, cubicGrad(x_ab, dh));
}

fn vel_dt_from_pressure(
    p: SphParticle,
    Pa: f32,
    Pb: f32,
    rho_a: f32,
    rho_b: f32,
    x_ab: vec2f
) -> vec2f {
    // Compute the pressure force contribution, Symmetric Formula
    let m = get_mass(p);
    let res = -m * (Pa / (rho_a * rho_a) + Pb / (rho_b * rho_b) * cubicGrad(x_ab, dh));
    return res;
}

fn vel_dt_from_viscosity(
    p: SphParticle,
    rho_a: f32,
    rho_b: f32,
    v_ab: vec2f,
    x_ab: vec2f
) -> vec2f {
    // Compute the viscosity force contribution, artificial viscosity
    let m = get_mass(p);

    var res = vec2f(0.0);
    let v_dot_x = dot(v_ab, x_ab);
    if v_dot_x < 0 {
        // artificial viscosity
        let mu = 2.0 * alpha * dh * c_s / (rho_a + rho_b);
        let x_ab_norm = normalize(x_ab);
        let x_ab_norm_2 = x_ab_norm * x_ab_norm;
        let PI_ab = -mu * (v_dot_x / x_ab_norm_2 + 0.01 * dh * dh);
        res = -m * PI_ab * cubicGrad(x_ab, dh);
    }

    return res;
}

// end WCSPH
// =========================================================

fn solve_boundary_constraints(p_in: SphParticle) -> SphParticle {
    // implement the native boundary constraint that remove the perpendicular velocity
    var p_out: SphParticle = p_in;
    var vel = p_in.velocity;

    let bound_x: vec2f = world_boundary_x();
    let bound_y: vec2f = world_boundary_y();

    let c_f: f32 = 0.3; // collision factor

    if p_in.position.x < bound_x.x {
        p_out.position.x = bound_x.x;
        vel.y = 0.0;
        vel.x = (c_f - 1.0) * vel.x;
        p_out.velocity = vel;
    }
    if p_in.position.x > bound_x.y {
        p_out.position.x = bound_x.y;
        vel.y = 0.0;
        vel.x = (c_f - 1.0) * vel.x;
        p_out.velocity = vel;
    }
    if p_in.position.y < bound_y.x {
        p_out.position.y = bound_y.x;
        vel.x = 0.0;
        vel.y = (c_f - 1.0) * vel.y;
        p_out.velocity = vel;
    }
    if p_in.position.y > bound_y.y {
        p_out.position.y = bound_y.y;
        vel.x = 0.0;
        vel.y = (c_f - 1.0) * vel.y;
        p_out.velocity = vel;
    }

    return p_out;
}

