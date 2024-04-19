use super::particles::Particle;
use cgmath::Vector3;

pub fn get_particles_3d(
    bottom_left: (f32, f32, f32),
    top_right: (f32, f32, f32),
    is_fluid: bool,
    density: f32,
    velocity: Option<Vector3<f32>>,
    diameter: f32,
) -> Vec<Particle> {
    let mut particles = Vec::new();

    let num_x = ((top_right.0 - bottom_left.0) / diameter).ceil() as u32;
    let num_y = ((top_right.1 - bottom_left.1) / diameter).ceil() as u32;
    let num_z = ((top_right.2 - bottom_left.2) / diameter).ceil() as u32;
    for i in 0..num_x {
        for j in 0..num_y {
            for k in 0..num_z {
                let x_coord = bottom_left.0 + i as f32 * diameter;
                let y_coord = bottom_left.1 + j as f32 * diameter;
                let z_coord = bottom_left.2 + k as f32 * diameter;
                let p = Particle {
                    position: Vector3::new(x_coord, y_coord, z_coord),
                    ptype: if is_fluid { 0 } else { 1 },
                    density,
                    velocity: velocity.unwrap_or(Vector3::new(0.0, 0.0, 0.0)),
                    ..Default::default()
                };

                particles.push(p);
            }
        }
    }

    particles
}

/// fill in particles in the given range
#[allow(dead_code)]
pub fn get_particles_2d(
    bottom_left: (f32, f32),
    top_right: (f32, f32),
    is_fluid: bool,
    density: f32,
    velocity: Option<Vector3<f32>>,
    diameter: f32,
) -> Vec<Particle> {
    let mut particles = Vec::new();

    let num_x = ((top_right.0 - bottom_left.0) / diameter).ceil() as u32;
    let num_y = ((top_right.1 - bottom_left.1) / diameter).ceil() as u32;
    for i in 0..num_x {
        for j in 0..num_y {
            let x_coord = bottom_left.0 + i as f32 * diameter;
            let y_coord = bottom_left.1 + j as f32 * diameter;
            let p = Particle {
                position: Vector3::new(x_coord, y_coord, 0.0),
                ptype: if is_fluid { 0 } else { 1 },
                density,
                velocity: velocity.unwrap_or(Vector3::new(0.0, 0.0, 0.0)),
                ..Default::default()
            };

            particles.push(p);
        }
    }

    particles
}
