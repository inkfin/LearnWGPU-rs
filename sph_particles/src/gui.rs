use std::collections::HashMap;

use egui_wgpu_backend::{RenderPass, ScreenDescriptor};
use egui_winit_platform::{Platform, PlatformDescriptor};

pub struct UILayer {
    pub egui_platform: Platform,
    pub egui_rpass: RenderPass,
    pub demo_app: egui_demo_lib::DemoWindows,
    pub display_demo: bool,
    pub window_open: HashMap<String, bool>,
}

impl UILayer {
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
        let egui_rpass = egui_wgpu_backend::RenderPass::new(device, *surface_format, 1);
        let demo_app = egui_demo_lib::DemoWindows::default();
        Self {
            egui_platform,
            egui_rpass,
            demo_app,
            display_demo: false,
            window_open: HashMap::new(),
        }
    }

    pub fn ui(&mut self) {
        let ctx = self.egui_platform.context();

        if !self.display_demo {
            egui::TopBottomPanel::top("menu bar").show(&ctx, |ui| {
                egui::menu::bar(ui, |ui| {
                    ui.menu_button("File", |ui| {
                        ui.set_min_width(220.0);
                        ui.style_mut().wrap = Some(false);

                        // On the web the browser controls the zoom
                        #[cfg(not(target_arch = "wasm32"))]
                        {
                            egui::gui_zoom::zoom_menu_buttons(ui);
                            ui.weak(format!(
                                "Current zoom: {:.0}%",
                                100.0 * ui.ctx().zoom_factor()
                            ))
                            .on_hover_text(
                                "The UI zoom level, on top of the operating system's default value",
                            );
                            ui.separator();
                        }

                        if ui.add(egui::Button::new("Organize Windows")).clicked() {
                            ui.ctx().memory_mut(|mem| mem.reset_areas());
                            ui.close_menu();
                        }

                        if ui
                            .add(egui::Button::new("Reset egui memory"))
                            .on_hover_text("Forget scroll, positions, sizes etc")
                            .clicked()
                        {
                            ui.ctx().memory_mut(|mem| *mem = Default::default());
                            ui.close_menu();
                        }
                    });
                });
            });
        }
        let title = "SPH Particles";
        self.window_open.entry(title.to_owned()).or_insert(true);
        egui::Window::new(title)
            .open(self.window_open.get_mut(title).unwrap_or(&mut false))
            .resizable(true)
            .show(&ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.display_demo, "Display Demo");
                    ui.separator();
                });
            });
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

        self.ui();

        if self.display_demo {
            // Draw the demo application
            self.demo_app.ui(&self.egui_platform.context());
        }

        // End the UI frame. We could now handle the output and draw the UI with the backend.
        let full_output = self.egui_platform.end_frame(Some(window));
        let paint_jobs = self
            .egui_platform
            .context()
            .tessellate(full_output.shapes, full_output.pixels_per_point);

        // Upload all resources for the GPU.
        let screen_descriptor = ScreenDescriptor {
            physical_width: window.inner_size().width,
            physical_height: window.inner_size().height,
            scale_factor: window.scale_factor() as f32,
        };
        let tdelta: egui::TexturesDelta = full_output.textures_delta;

        {
            self.egui_rpass
                .update_buffers(device, queue, &paint_jobs, &screen_descriptor);

            self.egui_rpass
                .add_textures(device, queue, &tdelta)
                .expect("Can't add egui texture");

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
            .expect("Can't remove egui texture");

        egui_encoder.finish()
    }
}
