// TODO: Use grid to accelerate neighbor search later
use cgmath::Vector3;

const MAX_NUM_PARTICLES_PER_CELL: u32 = 500;
const MAX_NUM_NEIGHBORS: u32 = 500;

pub struct Grid2D {
    pub cell_nums: Vector3<u32>,
    /// single cell size
    pub cell_size: Vector3<f32>,

    pub center: Vector3<f32>,

    pub boundary_upper: Vector3<f32>,
    pub boundary_lower: Vector3<f32>,
}

impl Grid2D {
    pub fn new(corner: Vector3<f32>, cell_nums: Vector3<u32>, cell_size: Vector3<f32>) -> Self {
        let cell_total_size = Vector3::new(
            cell_size.x * cell_nums.x as f32,
            cell_size.y * cell_nums.y as f32,
            0.0,
        );
        let center = corner + cell_total_size / 2.0;

        let boundary_upper = corner + cell_total_size;
        let boundary_lower = corner;

        Self {
            cell_nums,
            cell_size,
            center,
            boundary_upper,
            boundary_lower,
        }
    }
}
