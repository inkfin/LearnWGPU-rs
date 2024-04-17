//!include world.h.wgsl math.h.wgsl particle.h.wgsl

const GRAVITY: vec3<f32> = vec3<f32>(0.0, -9.8, 0.0);
const time_step: f32 = 0.02;

struct Uniforms {
    dt: f32,
};

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@group(1) @binding(0)
var<storage, read_write> particles_in: array<SphParticle>;

@group(1) @binding(1)
var<storage, read_write> particles_out: array<SphParticle>;

const workgroup_size_x: u32 = 256;

fn get_particle_id(gid: vec3<u32>, num_workgroups: vec3<u32>) -> u32 {
    return (gid.x + gid.y * workgroup_size_x * num_workgroups.x);
}

@compute
@workgroup_size(workgroup_size_x, 1, 1)
fn cs_main(
    @builtin(global_invocation_id) gid: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>,
) {
    // WORKGROUP_SIZE: (4096, 1024, 1)
    let id = get_particle_id(gid, num_workgroups);
    if id >= arrayLength(&particles_in) {
        return;
    }

    let p = particles_in[id];
    var p_next = p;

    // if uniforms.compute_stage == COMPUTE_DENSITIES {
    //     p_next = calc_density(id);
    // } else if uniforms.compute_stage == COMPUTE_NON_PRESSURE_FORCES {
    //     p_next = calc_non_pressure_force(id);
    //     p_next = update_pressure(p_next);
    // } else if uniforms.compute_stage == COMPUTE_PRESSURE_FORCES {
    //     p_next = calc_pressure_force(id);
    // } else if uniforms.compute_stage == ADVECT {
    //     p_next.position += p.velocity * time_step;
    //     p_next = solve_boundary_constraints(p_next);
    // }
    // if p.ptype == 0 {
    //     p_next.velocity = vec3<f32>(0.0);
    //     p_next.velocity += GRAVITY * time_step;
    //     p_next.position += p_next.velocity * time_step;
    //     p_next = solve_boundary_constraints(p_next);
    // }

    particles_out[id] = p_next;
}

@compute
@workgroup_size(workgroup_size_x, 1, 1)
fn compute_density_main(
    @builtin(global_invocation_id) gid: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>,
) {
    let id = get_particle_id(gid, num_workgroups);
    if id >= arrayLength(&particles_in) {
        return;
    }

    let p_next = calc_density(id);

    particles_out[id] = p_next;
}

@compute
@workgroup_size(workgroup_size_x, 1, 1)
fn compute_non_pressure_main(
    @builtin(global_invocation_id) gid: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>,
) {
    let id = get_particle_id(gid, num_workgroups);
    if id >= arrayLength(&particles_in) {
        return;
    }

    var p_next = calc_non_pressure_force(id);
    p_next = update_pressure(p_next);

    particles_out[id] = p_next;
}

@compute
@workgroup_size(workgroup_size_x, 1, 1)
fn compute_pressure_main(
    @builtin(global_invocation_id) gid: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>,
) {
    let id = get_particle_id(gid, num_workgroups);
    if id >= arrayLength(&particles_in) {
        return;
    }

    var p_next = calc_pressure_force(id);

    particles_out[id] = p_next;
}

@compute
@workgroup_size(workgroup_size_x, 1, 1)
fn advect_main(
    @builtin(global_invocation_id) gid: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>,
) {
    let id = get_particle_id(gid, num_workgroups);
    if id >= arrayLength(&particles_in) {
        return;
    }

    var p_next = particles_in[id];
    p_next.position += p_next.velocity * time_step;
    p_next = solve_boundary_constraints(p_next);

    particles_out[id] = p_next;
}


// =========================================================
//  WCSPH implementation

const dim = 2.0; // dimension
const dx = 0.1;
const rho_0 = 1000.0; // reference density
const diameter = 2.0 * dx;
const m_V = 0.8 * diameter * diameter; // particle volume 2D
const dh: f32 = dx * 4.0; // kernel radius
const viscosity: f32 = 0.05;
const c_s: f32 = 100.0;
const gamma: f32 = 7.0;
const stiffness: f32 = 50.0;//rho_0 * c_s * c_s / gamma; // pressure constant

fn density_kernel(r: vec3<f32>, h: f32) -> f32 {
    return cubicKernel2D(r, h);
}

fn density_grad(r: vec3<f32>, h: f32) -> vec3<f32> {
    return cubicGrad2D(r, h);
}


fn calc_density(pi: u32) -> SphParticle {
    let p_in = particles_in[pi];
    var p_out = p_in;

    p_out.density = 0.0;
    for (var pj: u32 = 0; pj < arrayLength(&particles_in); pj += 1u) {
        if pj == pi { continue; }

        let p_other: SphParticle = particles_in[pj];
        let x_ij = p_in.position - p_other.position;

        if length(x_ij) < dh {
            p_out.density += m_V * density_kernel(x_ij, dh);
        }
    }
    // treat as [0, 1]
    p_out.density *= rho_0;
    p_out.density = max(p_out.density, rho_0);
    return p_out;
}

fn calc_viscosity_dv(pi: u32) -> vec3<f32> {
    let p_in = particles_in[pi];
    var dv = vec3<f32>(0.0);

    for (var pj: u32 = 0; pj < arrayLength(&particles_in); pj += 1u) {
        if pj == pi { continue; }

        let p_other = particles_in[pj];
        let x_ab = p_in.position - p_other.position;
        let v_ab = p_in.velocity - p_other.velocity;
        let r_ab = length(x_ab);
        if r_ab < dh {
            let v_dot_x = dot(v_ab, x_ab);
            dv += 2.0 * (dim + 2.0) * viscosity * m_V * v_dot_x / (r_ab * r_ab + 0.01 * dh * dh) * density_grad(x_ab, dh);
        }
    }

    return dv;
}

fn calc_non_pressure_force(pi: u32) -> SphParticle {
    let p_in = particles_in[pi];
    var p_out = p_in;

    if p_in.ptype == 1 {return p_out;}

    var dv = vec3<f32>(0.0);
    dv += calc_viscosity_dv(pi);
    dv += GRAVITY;

    p_out.velocity += time_step * dv;
    return p_out;
}

// Calculate pressure, WCSPH equation (7)
fn update_pressure(p_in: SphParticle) -> SphParticle {
    var p_out = p_in;
    let rho = p_in.density;
    p_out.pressure = stiffness * (pow(rho / rho_0, gamma) - 1.0);
    return p_out;
}

fn calc_pressure_force(pi: u32) -> SphParticle {
    let p_in = particles_in[pi];
    var p_out = p_in;

    if p_in.ptype == 1 { return p_out; }

    var dv = vec3<f32>(0.0);

    for (var pj: u32 = 0; pj < arrayLength(&particles_in); pj += 1u) {
        if pj == pi { continue; }

        let p_other = particles_in[pj];
        let x_ab = p_in.position - p_other.position;
        let v_ab = p_in.velocity - p_other.velocity;
        let r_ab = length(x_ab);
        if r_ab < dh {
            let Pa = p_in.pressure;
            let Pb = p_other.pressure;
            let rho_a = p_in.density;
            let rho_b = p_other.density;

            dv += - rho_0 * m_V * (Pa / (rho_a * rho_a) + Pb / (rho_b * rho_b)) * density_grad(x_ab, dh);
        }
    }

    p_out.velocity += time_step * dv;

    return p_out;
}

// end WCSPH
// =========================================================

fn solve_boundary_constraints(p_in: SphParticle) -> SphParticle {
    // implement the native boundary constraint that remove the perpendicular velocity
    var p_out: SphParticle = p_in;
    var vel = p_in.velocity;

    if p_in.ptype == 1 { return p_out; }


    let bound_x: vec2<f32> = world_boundary_x();
    let bound_y: vec2<f32> = world_boundary_y();

    let c_f: f32 = 0.3; // collision factor

    if p_in.position.x < bound_x.x {
        p_out.position.x = bound_x.x;
        vel.x = (c_f - 1.0) * vel.x;
        p_out.velocity = vel;
    }
    if p_in.position.x > bound_x.y {
        p_out.position.x = bound_x.y;
        vel.x = (c_f - 1.0) * vel.x;
        p_out.velocity = vel;
    }
    if p_in.position.y < bound_y.x {
        p_out.position.y = bound_y.x;
        vel.y = (c_f - 1.0) * vel.y;
        p_out.velocity = vel;
    }
    if p_in.position.y > bound_y.y {
        p_out.position.y = bound_y.y;
        vel.y = (c_f - 1.0) * vel.y;
        p_out.velocity = vel;
    }

    return p_out;
}

