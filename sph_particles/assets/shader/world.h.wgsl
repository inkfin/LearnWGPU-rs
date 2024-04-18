// file: world.h
// defines the world constants

struct WorldUniforms {
    boundary_upper: vec3<f32>,
    dx: f32,
    boundary_lower: vec3<f32>,
    dh: f32, // kernel radius
};

