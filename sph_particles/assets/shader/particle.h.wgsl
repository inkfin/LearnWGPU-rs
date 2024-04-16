// particles.rs Particle
struct SphParticle {
    position: vec3<f32>,
    density: f32,
    velocity: vec3<f32>,
    support_radius: f32,
    mass: f32,
    pressure: f32,
    ptype: u32, // 0 for fluid, 1 for boundary
    _pad: array<f32, 1>,
}

