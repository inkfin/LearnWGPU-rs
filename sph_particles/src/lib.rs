mod camera;
mod compute;
mod grid_2d;
mod grid_3d;
mod gui;
mod model;
mod particles;
mod render;
mod resources;
mod texture;
mod timer;
mod uniforms;
mod vertex_data;

use std::sync::Arc;

use camera::{Camera, CameraController};
use gui::UILayer;
use instant::Instant;
use model::Model;

use render::BindGroupLayoutCache;
use tracing::{error, info, warn};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

use winit::{
    dpi::PhysicalSize,
    event::*,
    event_loop::EventLoop,
    keyboard::{Key, NamedKey},
    window::{Window, WindowBuilder},
};

use crate::timer::Timer;

struct State {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface_config: wgpu::SurfaceConfiguration,
    screen_size: winit::dpi::PhysicalSize<u32>,
    scale_factor: f64,
    // The window must be declared after the surface so
    // it gets dropped after it as the surface contains
    // unsafe references to the window's resources.
    window: Arc<Window>,
    camera: Camera,
    camera_controller: CameraController,

    particle_state: particles::ParticleState,

    bind_group_layout_cache: BindGroupLayoutCache,
    compute_state: compute::ComputeState,
    render_state: render::RenderState,
    ui_state: UILayer,

    fish_model: Model,
    // particle_model: Model,
}

impl State {
    // Creating some of the wgpu types requires async code
    async fn new(window: Arc<Window>) -> Self {
        let size = window.inner_size();

        let scale_factor = window.scale_factor();

        let instance = wgpu::Instance::default();

        let surface = instance.create_surface(window.clone()).unwrap();

        // Disable this because the unstable api is different
        #[cfg(target_arch = "wasm32")]
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        #[cfg(not(target_arch = "wasm32"))]
        let adapter = match instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
        {
            Some(adapter) => adapter,
            None => instance
                .enumerate_adapters(wgpu::Backends::all())
                .into_iter()
                .find(|adapter| {
                    // Check if this adapter supports our surface
                    adapter.is_surface_supported(&surface)
                })
                .expect("No suitable adapter found"),
        };

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::empty(),
                    // NOTE: WebGL doesn't support all of wgpu's features, so if
                    // we're building for the web, we'll have to disable some.
                    required_limits: if cfg!(target_arch = "wasm32") {
                        wgpu::Limits::downlevel_defaults()
                    } else {
                        wgpu::Limits::default()
                    },
                    label: None,
                },
                None, // Trace path
            )
            .await
            .unwrap();

        // Get features
        info!("adapter features: {:?}", adapter.features());
        info!("device limits: {:?}", device.limits());

        // may fail on some devices
        info!(
            "flags.contains VERTEX_STORAGE: {:?}",
            adapter
                .get_downlevel_capabilities()
                .flags
                .contains(wgpu::DownlevelFlags::VERTEX_STORAGE)
        );
        info!(
            "max_storage_buffers_per_shader_stage: {:?}",
            device.limits().max_storage_buffers_per_shader_stage
        );

        let surface_caps = surface.get_capabilities(&adapter);
        // Shader code in this tutorial assumes an sRGB surface texture. Using a different
        // one will result in all the colors coming out darker. If you want to support non
        // sRGB surfaces, you'll need to account for that when drawing to the frame.
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let bind_group_layout_cache = BindGroupLayoutCache::new(&device);

        let particle_state = particles::ParticleState::new(&device, &bind_group_layout_cache);
        let compute_state = compute::ComputeState::new(&device, &bind_group_layout_cache).await;
        let render_state =
            render::RenderState::new(&device, &config, &bind_group_layout_cache).await;

        let ui_state = UILayer::new(&device, &surface_format, size, scale_factor);

        let obj_model = resources::load_model(
            "Amago0.obj",
            &device,
            &queue,
            &bind_group_layout_cache.texture_bind_group_layout,
        )
        .await
        .unwrap();

        let aspect = config.width as f32 / config.height as f32;
        let camera = Camera::new(aspect);

        let camera_controller = CameraController::new(2.0);

        let modes = &surface_caps.present_modes;

        dbg!(modes);
        Self {
            window: window.clone(),
            surface,
            device,
            queue,
            surface_config: config,
            screen_size: size,
            scale_factor,
            camera,
            camera_controller,
            compute_state,
            render_state,
            fish_model: obj_model,
            ui_state,
            particle_state,
            bind_group_layout_cache,
        }
    }

    pub fn window(&self) -> &Window {
        self.window.as_ref()
    }

    fn resize(&mut self, new_size: PhysicalSize<u32>, new_scale_factor: Option<f64>) {
        // Resize with 0 width and height is used by winit to signal a minimize event on Windows.
        // See: https://github.com/rust-windowing/winit/issues/208
        // This solves an issue where the app would panic when minimizing on Windows.
        if new_size.width == 0 || new_size.height == 0 {
            return;
        }

        // info!(
        //     "resizing to [{},{}]x{:?}",
        //     new_size.width, new_size.height, new_scale_factor
        // );

        self.screen_size.width = new_size.width;
        self.screen_size.height = new_size.height;

        if let Some(value) = new_scale_factor {
            self.scale_factor = value;
        }

        self.surface_config.width = self.screen_size.width;
        self.surface_config.height = self.screen_size.height;
        self.camera.aspect = self.screen_size.width as f32 / self.screen_size.height as f32;
        self.surface.configure(&self.device, &self.surface_config);

        self.render_state.depth_texture = texture::Texture::create_depth_texture(
            &self.device,
            &self.surface_config,
            "depth_texture",
        );
    }

    fn input(&mut self, event: &WindowEvent) -> bool {
        self.camera_controller.process_events(event)
    }

    fn update(&mut self, delta_time: f32) {
        self.camera_controller
            .update_camera_state(&mut self.camera, delta_time);
        self.render_state.update_uniforms(&self.camera, &self.queue);
    }

    fn render(&mut self, dt: f32) -> Result<(), wgpu::SurfaceError> {
        futures::executor::block_on(self.compute_state.sort_particle_data(
            &self.device,
            &self.queue,
            &mut self.particle_state,
        ));
        self.compute_state
            .compute(&self.device, &self.queue, &mut self.particle_state, dt);

        let output = self.surface.get_current_texture()?;

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        self.render_state.render(
            &self.device,
            &self.queue,
            &mut self.ui_state,
            &self.window,
            &view,
            &self.particle_state,
        );

        output.present();
        Ok(())
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub async fn run() {
    #[cfg(target_arch = "wasm32")]
    {
        console_error_panic_hook::set_once();
        tracing_wasm::set_as_global_default();
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        tracing_subscriber::fmt::init();
    }

    info!("info!!!");
    warn!("warning!!!");
    error!("eeeeek");

    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new()
        .with_title("Learn wgpu!")
        .build(&event_loop)
        .unwrap();

    let window = Arc::new(window);

    // Attach canvas
    #[cfg(target_arch = "wasm32")]
    {
        use winit::platform::web::WindowExtWebSys;
        web_sys::window()
            .and_then(|win| win.document())
            .and_then(|doc| {
                let dst = doc.get_element_by_id("wasm-example")?;
                let canvas = web_sys::Element::from(window.clone().canvas().unwrap());
                dst.append_child(&canvas).ok()?;
                Some(())
            })
            .expect("Couldn't append canvas to document body.");
    }

    // Initialize last frame timer
    let mut timer = Timer::new();

    // Need to be created after canvas is attached
    let mut state = State::new(window.clone()).await;
    // Register callbacks
    // These are buggy shift
    #[cfg(target_arch = "wasm32")]
    {
        // Winit prevents sizing with CSS, so we have to set
        // the size manually when on web.
        // use winit::dpi::PhysicalSize;
        // window.set_inner_size(PhysicalSize::new(450, 400));

        // referenced from https://github.com/michaelkirk/abstreet/blob/7b99335cd5325d455140c7595bf0ef3ccdaf93e0/widgetry/src/backend_glow_wasm.rs
        // You need to pass an actual closure to javascript
        let get_full_size = || {
            let scrollbars = 0.0;
            let win = web_sys::window().unwrap();
            // `inner_width` corresponds to the browser's `self.innerwidth` function, which are in
            // logical, not physical, pixels
            winit::dpi::LogicalSize::new(
                win.inner_width().unwrap().as_f64().unwrap() - scrollbars,
                win.inner_height().unwrap().as_f64().unwrap() - scrollbars,
            )
        };

        let _ = window.request_inner_size(get_full_size());

        let window_clone = window.clone();
        let closure = wasm_bindgen::closure::Closure::wrap(Box::new(move |_e: web_sys::Event| {
            let size = get_full_size();
            let _ = window_clone.request_inner_size(size);
        }) as Box<dyn FnMut(_)>);

        web_sys::window()
            .and_then(|win| {
                win.add_event_listener_with_callback("resize", closure.as_ref().unchecked_ref())
                    .unwrap();
                Some(())
            })
            .expect("Couldn't register resize to canvas.");

        closure.forget();
    }

    let mut last_frame_time = timer.now().as_secs_f32();

    event_loop
        .run(move |event, target| {
            // Have the closure take ownership of the resources.
            // `event_loop.run` never returns, therefore we must do this to ensure
            // the resources are properly cleaned up.
            #[allow(unused_parens)]
            let _ = (&state);

            // *control_flow = ControlFlow::Poll;

            let dt = timer.now().as_secs_f32() - last_frame_time;

            state.ui_state.egui_platform.handle_event(&event);
            // update egui
            state
                .ui_state
                .egui_platform
                .update_time(timer.elapse_timer.elapsed().as_secs_f64());
            state
                .ui_state
                .frame_history
                .on_new_frame(timer.now().as_secs_f64(), Some(dt));
            last_frame_time = timer.now().as_secs_f32();

            if state.ui_state.egui_platform.captures_event(&event) {
                return;
            }

            match event {
                Event::WindowEvent {
                    ref event,
                    window_id,
                } if window_id == state.window.id() => {
                    if !state.input(event) {
                        match event {
                            WindowEvent::CloseRequested
                            | WindowEvent::KeyboardInput {
                                event:
                                    KeyEvent {
                                        state: ElementState::Pressed,
                                        logical_key: Key::Named(NamedKey::Escape),
                                        ..
                                    },
                                ..
                            } => target.exit(),
                            WindowEvent::Resized(physical_size) => {
                                state.resize(*physical_size, None);
                            }
                            WindowEvent::RedrawRequested if window_id == state.window().id() => {
                                // Get current time on frame start
                                timer.render_timer = Instant::now();

                                match state.render(dt) {
                                    Ok(_) => {}
                                    // Reconfigure the surface if lost
                                    Err(wgpu::SurfaceError::Lost) => {
                                        state.resize(state.screen_size, None)
                                    }
                                    // The system is out of memory, we should probably quit
                                    Err(wgpu::SurfaceError::OutOfMemory) => {
                                        target.exit();
                                    }
                                    // All other errors (Outdated, Timeout) should be resolved by the next frame
                                    Err(e) => eprintln!("{:?}", e),
                                }

                                timer.get_and_update_render_time();
                            }
                            _ => {}
                        }
                    }
                }
                Event::AboutToWait => {
                    let dt = timer.get_and_update_all_events_time();

                    timer.state_timer = Instant::now();

                    state.update(dt.as_secs_f32());

                    timer.get_and_update_state_time();

                    // RedrawRequested will trigger once we manually request it, or it get resized
                    state.window().request_redraw();
                }
                Event::LoopExiting => {
                    target.exit();
                }
                _ => {}
            }
        })
        .expect("Failed to run event loop!");
}
