// particles.rs Particle
struct SphParticle {
    position: vec3<f32>,
    density: f32,
    velocity: vec3<f32>,
    pressure: f32,
    ptype: u32, // 0 for fluid, 1 for boundary
    cell_id: u32,
    _pad: array<f32, 2>,
}

