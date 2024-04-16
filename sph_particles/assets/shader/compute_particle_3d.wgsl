// particles.rs Particle
struct SphParticle {
    position: vec3<f32>, // only use xy
    density: f32,
    velocity: vec3<f32>,
    support_radius: f32,
    pressure: f32,
    ptype: u32, // 0 for fluid, 1 for boundary
    _pad: array<f32, 2>,
}

@group(0) @binding(0)
var<storage, read_write> particles_in: array<SphParticle>;

@compute
@workgroup_size(1, 1, 1)
fn cs_main(@builtin(global_invocation_id) gid: vec3<u32>, @builtin(local_invocation_id) lid: vec3<u32>, @builtin(workgroup_id) wid: vec3<u32>) {
    // do some computation
    let position = particles_in[0].position;
    let id = i32(gid.x);
    particles_in[id].position = particles_in[id].position + 0.00001;
}
