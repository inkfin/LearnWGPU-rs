//!include world.h.wgsl math.h.wgsl particle.h.wgsl

const GRAVITY: vec3<f32> = vec3<f32>(0.0, -9.8, 0.0);
const time_step: f32 = 0.01;

struct Uniforms {
    dt: f32,
};

@group(0) @binding(0)
var<storage, read_write> particles_in: array<SphParticle>;

@group(1) @binding(0)
var<storage, read_write> particles_out: array<SphParticle>;

@group(2) @binding(0)
var<uniform> uniforms: Uniforms;

@group(3) @binding(0)
var<uniform> world: WorldUniforms;

const workgroup_size_x: u32 = 256;

fn get_particle_id(gid: vec3<u32>, num_workgroups: vec3<u32>) -> u32 {
    return gid.x + gid.y * workgroup_size_x * num_workgroups.x;
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
    // update pressure, get prepared for pressure force
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

    var p_next = advect(id);

    particles_out[id] = p_next;
}

@compute
@workgroup_size(workgroup_size_x, 1, 1)
fn empty_copy_main(
    @builtin(global_invocation_id) gid: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>,
) {
    let id = get_particle_id(gid, num_workgroups);
    if id >= arrayLength(&particles_in) {
        return;
    }

    var p_next = particles_in[id];

    particles_out[id] = p_next;
}


// =========================================================
//  WCSPH implementation

const dim = 2.0; // dimension
const rho_0 = 1000.0; // reference density
const viscosity: f32 = 0.05;
const c_s: f32 = 100.0;
const gamma: f32 = 7.0;
const stiffness: f32 = 50.0;//rho_0 * c_s * c_s / gamma; // pressure constant

// particle volume 2D
fn get_m_V() -> f32 {
    let diameter = 2.0 * world.dx;
    return 0.8 * diameter * diameter;
}

fn density_kernel(r: vec3<f32>, h: f32) -> f32 {
    return cubicKernel2D(r, h);
}

fn density_grad(r: vec3<f32>, h: f32) -> vec3<f32> {
    return cubicGrad2D(r, h);
}


fn calc_density(pi: u32) -> SphParticle {
    let p_in = particles_in[pi];
    var p_out = p_in;

    let m_V = get_m_V();

    p_out.density = 0.0;
    for (var pj: u32 = 0; pj < arrayLength(&particles_in); pj += 1u) {
        if pj == pi { continue; }

        let p_other: SphParticle = particles_in[pj];
        let x_ij = p_in.position - p_other.position;

        if length(x_ij) < world.dh {
            p_out.density += m_V * density_kernel(x_ij, world.dh);
        }
    }
    // treat as [0, 1]
    p_out.density *= rho_0;
    return p_out;
}

fn calc_non_pressure_force(pi: u32) -> SphParticle {
    let p_in = particles_in[pi];
    var p_out = p_in;

    let m_V = get_m_V();

    if p_in.ptype != 0 {return p_out;}

    var dv = vec3<f32>(0.0);
    dv += GRAVITY;

    for (var pj: u32 = 0; pj < arrayLength(&particles_in); pj += 1u) {
        if pj == pi { continue; }

        let p_other = particles_in[pj];
        let x_ab = p_in.position - p_other.position;
        let v_ab = p_in.velocity - p_other.velocity;
        let r_ab = length(x_ab);
        if r_ab < world.dh {
            let v_dot_x: f32 = dot(v_ab, x_ab);
            dv += 2.0 * (dim + 2.0) * viscosity * ((m_V * rho_0) / p_in.density) * v_dot_x / (r_ab * r_ab + 0.01 * world.dh * world.dh) * density_grad(x_ab, world.dh);
        }
    }

    p_out.velocity += time_step * dv;
    return p_out;
}

// Calculate pressure, WCSPH equation (7)
fn update_pressure(p_in: SphParticle) -> SphParticle {
    var p_out = p_in;
    // hard coded free surface solution
    p_out.density = max(p_in.density, rho_0);
    p_out.pressure = stiffness * (pow(p_out.density / rho_0, gamma) - 1.0);
    return p_out;
}

fn calc_pressure_force(pi: u32) -> SphParticle {
    let p_in = particles_in[pi];
    var p_out = p_in;

    let m_V = get_m_V();

    if p_in.ptype != 0 { return p_out; }

    var dv = vec3<f32>(0.0);

    for (var pj: u32 = 0; pj < arrayLength(&particles_in); pj += 1u) {
        if pj == pi { continue; }

        let p_other = particles_in[pj];
        let x_ab = p_in.position - p_other.position;
        let v_ab = p_in.velocity - p_other.velocity;
        let r_ab = length(x_ab);
        if r_ab < world.dh {
            let Pa = p_in.pressure;
            let Pb = p_other.pressure;
            let rho_a = p_in.density;
            let rho_b = p_other.density;

            dv += - rho_0 * m_V * (Pa / (rho_a * rho_a) + Pb / (rho_b * rho_b)) * density_grad(x_ab, world.dh);
        }
    }

    p_out.velocity += time_step * dv;

    return p_out;
}

fn advect(pi: u32) -> SphParticle {
    let p_in = particles_in[pi];
    var p_out = p_in;

    if p_in.ptype != 0 { return p_out; }

    p_out.position += p_in.velocity * time_step;
    p_out = solve_boundary_constraints(p_out);
    return p_out;
}

// end WCSPH
// =========================================================

fn solve_boundary_constraints(p_in: SphParticle) -> SphParticle {
    // implement the native boundary constraint that remove the perpendicular velocity
    var p_out: SphParticle = p_in;
    var vel = p_in.velocity;

    if p_in.ptype == 1 { return p_out; }


    let c_f: f32 = 0.3; // collision factor

    if p_in.position.x < world.boundary_lower.x {
        p_out.position.x = world.boundary_lower.x;
        vel.x = (c_f - 1.0) * vel.x;
        p_out.velocity = vel;
    }
    if p_in.position.x > world.boundary_upper.x {
        p_out.position.x = world.boundary_upper.x;
        vel.x = (c_f - 1.0) * vel.x;
        p_out.velocity = vel;
    }
    if p_in.position.y < world.boundary_lower.y {
        p_out.position.y = world.boundary_lower.y;
        vel.y = (c_f - 1.0) * vel.y;
        p_out.velocity = vel;
    }
    if p_in.position.y > world.boundary_upper.y {
        p_out.position.y = world.boundary_upper.y;
        vel.y = (c_f - 1.0) * vel.y;
        p_out.velocity = vel;
    }

    return p_out;
}

