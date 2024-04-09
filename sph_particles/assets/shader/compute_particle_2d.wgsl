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

fn get_mass(particle: SphParticle) -> f32 {
    let density = particle.density;
    let r = particle.particle_radius;
    let volume = 4.0 * M_PI * r * r * r / 3.0;
    return density * volume;
}

@group(0) @binding(0)
var<storage, read_write> particles_in: array<SphParticle>;

const GRAVITY: vec3f = vec3f(0.0, -0.000098, 0.0);

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
    if p.ptype == 0 {
        var velocity = p.velocity + GRAVITY;
        if (p.position + velocity).y > y_lower {
            particles_in[id].position = p.position + velocity;
            particles_in[id].velocity = velocity;
        } else {
            particles_in[id].velocity = -p.velocity;
            particles_in[id].position = vec3f(p.position.x, y_lower, p.position.z);
        }
    }
}
