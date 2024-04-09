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
}

const GRAVITY: vec3f = vec3f(0.0, -9.8, 0.0);
const time_step: f32 = 1e-6;

@group(0) @binding(0)
var<storage, read_write> particles_in: array<SphParticle>;

fn get_mass(particle: SphParticle) -> f32 {
    let density = particle.density;
    let r = particle.particle_radius;
    let volume = 4.0 * M_PI * r * r * r / 3.0;
    return density * volume;
}

@compute
@workgroup_size(256, 1, 1)
fn cs_main(@builtin(global_invocation_id) gid: vec3<u32>, @builtin(local_invocation_id) lid: vec3<u32>, @builtin(workgroup_id) wid: vec3<u32>) {
    // do some computation
    let id = gid.x;
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
        particles_in[id] = p_next;
    } // else don't update
}

fn solve_boundary_constraints(p_in: SphParticle) -> SphParticle {
    // implement the native boundary constraint that remove the perpendicular velocity
    var p_out: SphParticle = p_in;
    var vel = p_in.velocity;

    let bound_x: vec2f = world_boundary_x();
    let bound_y: vec2f = world_boundary_y();

    if p_in.position.x < bound_x.x {
        p_out.position.x = bound_x.x;
        vel.x = 0.0;
        vel.y = -vel.y;
        p_out.velocity = vel;
    }
    if p_in.position.x > bound_x.y {
        p_out.position.x = bound_x.y;
        vel.x = 0.0;
        vel.y = -vel.y;
        p_out.velocity = vel;
    }
    if p_in.position.y < bound_y.x {
        p_out.position.y = bound_y.x;
        vel.y = 0.0;
        vel.x = -vel.x;
        p_out.velocity = vel;
    }
    if p_in.position.y > bound_y.y {
        p_out.position.y = bound_y.y;
        vel.y = 0.0;
        vel.x = -vel.x;
        p_out.velocity = vel;
    }

    return p_out;
}

