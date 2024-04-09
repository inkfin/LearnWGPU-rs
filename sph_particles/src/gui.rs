use std::collections::HashMap;

use egui::util::History;
use egui_wgpu_backend::{RenderPass, ScreenDescriptor};
use egui_winit_platform::{Platform, PlatformDescriptor};

pub struct UILayer {
    pub egui_platform: Platform,
    pub egui_rpass: RenderPass,
    pub demo_app: egui_demo_lib::DemoWindows,
    pub display_demo: bool,
    pub window_open: HashMap<String, bool>,

    pub frame_history: FrameHistory,
}

const SCALE: f64 = 0.7;

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
            scale_factor: scale_factor * SCALE,
            font_definitions: egui::FontDefinitions::default(),
            style: Default::default(),
        });
        let egui_rpass = egui_wgpu_backend::RenderPass::new(device, *surface_format, 1);
        let demo_app = egui_demo_lib::DemoWindows::default();

        Self {
            egui_platform,
            egui_rpass,
            demo_app,
            frame_history: FrameHistory::default(),
            display_demo: false,
            window_open: HashMap::new(),
        }
    }

    fn ui(&mut self) {
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
            .default_size((96.0, 300.0))
            .resizable(true)
            .show(&ctx, |ui| {
                ui.vertical(|ui| {
                    ui.checkbox(&mut self.display_demo, "Display Demo");
                    ui.separator();
                    self.frame_history.ui(ui);
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
            scale_factor: window.scale_factor() as f32 * SCALE as f32,
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

/// struct used to record fps
pub struct FrameHistory {
    frame_times: History<f32>,
}

impl Default for FrameHistory {
    fn default() -> Self {
        let max_age: f32 = 1.0;
        let max_len = (max_age * 300.0).round() as usize;
        Self {
            frame_times: History::new(0..max_len, max_age),
        }
    }
}

impl FrameHistory {
    // Called first
    pub fn on_new_frame(&mut self, now: f64, previous_frame_time: Option<f32>) {
        let previous_frame_time = previous_frame_time.unwrap_or_default();
        if let Some(latest) = self.frame_times.latest_mut() {
            *latest = previous_frame_time; // rewrite history now that we know
        }
        self.frame_times.add(now, previous_frame_time); // projected
    }

    fn mean_frame_time(&self) -> f32 {
        self.frame_times.average().unwrap_or_default()
    }

    #[allow(dead_code)]
    fn fps(&self) -> f32 {
        1.0 / self.frame_times.mean_time_interval().unwrap_or_default()
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.label(format!(
            "Mean CPU usage: {:.2} ms / frame",
            1e3 * self.mean_frame_time()
        ))
        .on_hover_text(
            "Includes all app logic, egui layout, tessellation, and rendering.\n\
            Does not include waiting for vsync.",
        );
        egui::warn_if_debug_build(ui);

        if !cfg!(target_arch = "wasm32") {
            egui::CollapsingHeader::new("📊 CPU usage history")
                .default_open(false)
                .show(ui, |ui| {
                    self.graph(ui);
                });
        }
    }

    fn graph(&mut self, ui: &mut egui::Ui) -> egui::Response {
        use egui::*;

        ui.label("egui CPU usage history");

        let history = &self.frame_times;

        // TODO(emilk): we should not use `slider_width` as default graph width.
        let height = ui.spacing().slider_width;
        let size = vec2(ui.available_size_before_wrap().x, height);
        let (rect, response) = ui.allocate_at_least(size, Sense::hover());
        let style = ui.style().noninteractive();

        let graph_top_cpu_usage = 0.1;
        let graph_rect = Rect::from_x_y_ranges(history.max_age()..=0.0, graph_top_cpu_usage..=0.0);
        let to_screen = emath::RectTransform::from_to(graph_rect, rect);

        let mut shapes = Vec::with_capacity(3 + 2 * history.len());
        shapes.push(Shape::Rect(epaint::RectShape::new(
            rect,
            style.rounding,
            ui.visuals().extreme_bg_color,
            ui.style().noninteractive().bg_stroke,
        )));

        let rect = rect.shrink(4.0);
        let color = ui.visuals().text_color();
        let line_stroke = Stroke::new(1.0, color);

        if let Some(pointer_pos) = response.hover_pos() {
            let y = pointer_pos.y;
            shapes.push(Shape::line_segment(
                [pos2(rect.left(), y), pos2(rect.right(), y)],
                line_stroke,
            ));
            let cpu_usage = to_screen.inverse().transform_pos(pointer_pos).y;
            let text = format!("{:.1} ms", 1e3 * cpu_usage);
            shapes.push(ui.fonts(|f| {
                Shape::text(
                    f,
                    pos2(rect.left(), y),
                    egui::Align2::LEFT_BOTTOM,
                    text,
                    TextStyle::Monospace.resolve(ui.style()),
                    color,
                )
            }));
        }

        let circle_color = color;
        let radius = 2.0;
        let right_side_time = ui.input(|i| i.time); // Time at right side of screen

        for (time, cpu_usage) in history.iter() {
            let age = (right_side_time - time) as f32;
            let pos = to_screen.transform_pos_clamped(Pos2::new(age, cpu_usage));

            shapes.push(Shape::line_segment(
                [pos2(pos.x, rect.bottom()), pos],
                line_stroke,
            ));

            if cpu_usage < graph_top_cpu_usage {
                shapes.push(Shape::circle_filled(pos, radius, circle_color));
            }
        }

        ui.painter().extend(shapes);

        response
    }
}
