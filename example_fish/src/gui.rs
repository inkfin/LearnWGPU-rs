use egui_wgpu_backend::{RenderPass, ScreenDescriptor};
use egui_winit_platform::{Platform, PlatformDescriptor};

pub struct UIState {
    pub egui_platform: Platform,
    pub egui_rpass: RenderPass,
    pub demo_app: egui_demo_lib::DemoWindows,

    size: winit::dpi::PhysicalSize<u32>,
    scale_factor: f64,
}

impl UIState {
    pub fn new(
        device: &wgpu::Device,
        surface_format: &wgpu::TextureFormat,
        size: winit::dpi::PhysicalSize<u32>,
        scale_factor: f64,
    ) -> Self {
        let egui_platform = Platform::new(PlatformDescriptor {
            physical_width: size.width,
            physical_height: size.height,
            scale_factor,
            font_definitions: egui::FontDefinitions::default(),
            style: Default::default(),
        });
        let egui_rpass = egui_wgpu_backend::RenderPass::new(&device, *surface_format, 1);
        let demo_app = egui_demo_lib::DemoWindows::default();
        Self {
            egui_platform,
            egui_rpass,
            demo_app,
            size,
            scale_factor,
        }
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>, scale_factor: Option<f64>) {
        self.size = new_size;

        if let Some(sf) = scale_factor {
            self.scale_factor = sf;
        }
    }

    pub fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        window: &winit::window::Window,
        view: &wgpu::TextureView,
    ) -> wgpu::CommandBuffer {
        // use different encoder
        let mut egui_encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder (UI)"),
        });

        // Begin to draw the UI frame
        self.egui_platform.begin_frame();

        // Draw the demo application
        self.demo_app.ui(&self.egui_platform.context());

        // End the UI frame. We could now handle the output and draw the UI with the backend.
        let full_output = self.egui_platform.end_frame(Some(window));
        let paint_jobs = self.egui_platform.context().tessellate(full_output.shapes);

        // Upload all resources for the GPU.
        let screen_descriptor = ScreenDescriptor {
            physical_width: self.size.width,
            physical_height: self.size.height,
            scale_factor: self.scale_factor as f32,
        };
        let tdelta: egui::TexturesDelta = full_output.textures_delta;

        {
            self.egui_rpass
                .update_buffers(device, queue, &paint_jobs, &screen_descriptor);

            self.egui_rpass
                .add_textures(device, queue, &tdelta)
                .expect("add texture ok");

            // Record all render passes.
            self.egui_rpass
                .execute(
                    &mut egui_encoder,
                    view,
                    &paint_jobs,
                    &screen_descriptor,
                    // Some(wgpu::Color::BLACK),
                    None,
                )
                .unwrap();
        }

        self.egui_rpass
            .remove_textures(tdelta)
            .expect("remove texture ok");

        egui_encoder.finish()
    }
}
