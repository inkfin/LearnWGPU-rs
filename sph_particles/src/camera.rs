use cgmath::{SquareMatrix, Transform};
use winit::{
    event::{ElementState, KeyEvent, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
};

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
);

#[derive(Debug)]
pub struct Camera {
    pub eye: cgmath::Point3<f32>,
    pub target: cgmath::Point3<f32>,
    pub up: cgmath::Vector3<f32>,
    pub aspect: f32,
    pub fovy: f32,
    pub znear: f32,
    pub zfar: f32,
    // render
}

impl Camera {
    pub fn new(aspect: f32) -> Self {
        Self {
            // position the camera 1 unit up and 2 units back
            // +z is out of the screen
            eye: (15.0, 10.0, 20.0).into(),
            // have it look at the origin
            target: (5.0, 5.0, 5.0).into(),
            // which way is "up"
            up: cgmath::Vector3::unit_y(),
            aspect,
            fovy: 45.0,
            znear: 10.0,
            zfar: 100.0,
        }
    }

    pub fn build_view_matrix(&self) -> cgmath::Matrix4<f32> {
        let view = cgmath::Matrix4::look_at_rh(self.eye, self.target, self.up);
        view
    }

    pub fn build_projection_matrix(&self) -> cgmath::Matrix4<f32> {
        let proj = cgmath::perspective(cgmath::Deg(self.fovy), self.aspect, self.znear, self.zfar);
        OPENGL_TO_WGPU_MATRIX * proj
    }

    pub fn get_camera_intrinsic(&self, w: u32, h: u32) -> cgmath::Vector4<f32> {
        let proj = self.build_projection_matrix();
        let tan_half_fovx = 1.0 / proj[0][0];
        let tan_half_fovy = 1.0 / proj[1][1];

        cgmath::Vector4::new(
            2.0 * tan_half_fovx / w as f32, // fxInv
            2.0 * tan_half_fovy / h as f32, // fyInv
            w as f32 / 2.0,                 // cx
            h as f32 / 2.0,                 // cy
        )
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    mat_view: [[f32; 4]; 4],
    mat_proj: [[f32; 4]; 4],
    mat_view_inv: [[f32; 4]; 4],
    mat_proj_inv: [[f32; 4]; 4],
    eye_pos: [f32; 4],
}

impl CameraUniform {
    pub fn new() -> Self {
        Self {
            mat_view: cgmath::Matrix4::identity().into(),
            mat_proj: cgmath::Matrix4::identity().into(),
            mat_view_inv: cgmath::Matrix4::identity().into(),
            mat_proj_inv: cgmath::Matrix4::identity().into(),
            eye_pos: [0.0; 4],
        }
    }

    pub fn update_view_proj(&mut self, camera: &Camera) {
        self.mat_view = camera.build_view_matrix().into();
        self.mat_proj = camera.build_projection_matrix().into();
        self.mat_view_inv = camera
            .build_view_matrix()
            .inverse_transform()
            .unwrap()
            .into();
        self.mat_proj_inv = camera
            .build_projection_matrix()
            .inverse_transform()
            .unwrap()
            .into();
        self.eye_pos = [camera.eye.x, camera.eye.y, camera.eye.z, 1.0];
    }
}

pub struct CameraController {
    speed: f32,
    is_forward_pressed: bool,
    is_backward_pressed: bool,
    is_left_pressed: bool,
    is_right_pressed: bool,
    is_up_pressed: bool,
    is_down_pressed: bool,
}

impl CameraController {
    pub fn new(speed: f32) -> Self {
        Self {
            speed,
            is_forward_pressed: false,
            is_backward_pressed: false,
            is_left_pressed: false,
            is_right_pressed: false,
            is_up_pressed: false,
            is_down_pressed: false,
        }
    }

    pub fn process_events(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state,
                        physical_key: PhysicalKey::Code(keycode),
                        ..
                    },
                ..
            } => {
                let is_pressed = *state == ElementState::Pressed;
                match keycode {
                    KeyCode::KeyW | KeyCode::ArrowUp => {
                        self.is_forward_pressed = is_pressed;
                        true
                    }
                    KeyCode::KeyA | KeyCode::ArrowLeft => {
                        self.is_left_pressed = is_pressed;
                        true
                    }
                    KeyCode::KeyS | KeyCode::ArrowDown => {
                        self.is_backward_pressed = is_pressed;
                        true
                    }
                    KeyCode::KeyD | KeyCode::ArrowRight => {
                        self.is_right_pressed = is_pressed;
                        true
                    }
                    KeyCode::KeyQ => {
                        self.is_up_pressed = is_pressed;
                        true
                    }
                    KeyCode::KeyE => {
                        self.is_down_pressed = is_pressed;
                        true
                    }
                    _ => false,
                }
            }
            _ => false,
        }
    }

    pub fn update_camera_state(&self, camera: &mut Camera, delta_time: f32) {
        use cgmath::InnerSpace;
        let forward = camera.target - camera.eye;
        let forward_norm = forward.normalize();
        let forward_mag = forward.magnitude();

        // Prevents glitching when the camera gets too close to the
        // center of the scene.
        if self.is_forward_pressed && forward_mag > self.speed {
            camera.eye += forward_norm * self.speed * 2.0 * delta_time;
        }
        if self.is_backward_pressed {
            camera.eye -= forward_norm * self.speed * 2.0 * delta_time;
        }

        let right = forward_norm.cross(camera.up);

        // Redo radius calc in case the forward/backward is pressed.
        let forward = camera.target - camera.eye;
        let forward_norm = forward.normalize();
        let forward_mag = forward.magnitude();

        if self.is_right_pressed {
            // Rescale the distance between the target and the eye so
            // that it doesn't change. The eye, therefore, still
            // lies on the circle made by the target and eye.
            camera.eye = camera.target
                - (forward_norm - right * self.speed * delta_time).normalize() * forward_mag;
        }
        if self.is_left_pressed {
            camera.eye = camera.target
                - (forward_norm + right * self.speed * delta_time).normalize() * forward_mag;
        }

        let forward = camera.target - camera.eye;
        let forward_norm = forward.normalize();
        let forward_mag = forward.magnitude();

        if self.is_up_pressed {
            if (forward_norm).dot(camera.up) > 0.8 {
                return;
            }
            camera.eye = camera.target
                - (forward_norm + camera.up * self.speed * delta_time).normalize() * forward_mag;
        }
        if self.is_down_pressed {
            if (forward_norm).dot(camera.up) < -0.8 {
                return;
            }
            camera.eye = camera.target
                - (forward_norm - camera.up * self.speed * delta_time).normalize() * forward_mag;
        }
    }
}
