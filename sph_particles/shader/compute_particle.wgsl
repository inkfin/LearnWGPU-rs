// particles.rs Particle
struct SphParticle {
    position: vec3<f32>,
    velocity: vec3<f32>,
    force: vec3<f32>,
    density: f32,
    support_radius: f32,
    particle_radius: f32,
}

@compute
@workgroup_size(16, 16, 1)
fn cs_main(@builtin(local_invocation_id) lid: vec3<u32>, @builtin(workgroup_id) wid: vec3<u32>) {
    // do some computation
}
