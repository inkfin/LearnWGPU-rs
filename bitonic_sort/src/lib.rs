//! This example assumes that you've seen hello-compute and or repeated-compute
//! and thus have a general understanding of what's going on here.
//!
//! There's an explainer on what this example does exactly and what workgroups
//! are and the meaning of `@workgroup(size_x, size_y, size_z)` in the
//! README. Also see commenting in shader.wgsl as well.
//!
//! Only parts specific to this example will be commented.

use std::time::Instant;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
use wgpu::util::DeviceExt;

// buffer maximum size 2^25
const ARRAY_LENGTH: usize = 1usize << 25;

// Bubble sort
// #[repr(C)]
// #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
// struct Uniforms {
//     sort_even: u32,
// }

// Bitonic sort
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    log_len: u32,
    log_group_init: u32,
    log_group_curr: u32,
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub async fn run() {
    let mut time_list = vec![];

    let timer = Instant::now();
    #[cfg(target_arch = "wasm32")]
    {
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        console_log::init_with_level(log::Level::Debug).expect("Couldn't initialize logger");
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let env = env_logger::Env::default()
            .filter_or("MY_LOG_LEVEL", "info")
            .write_style_or("MY_LOG_STYLE", "always");
        env_logger::init_from_env(env);
    }

    let mut local_input = vec![0f32; ARRAY_LENGTH];
    for e in local_input.iter_mut() {
        *e = rand::random::<f32>();
    }
    // log::info!("Input a: {local_input:?}");

    let instance = wgpu::Instance::default();
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions::default())
        .await
        .unwrap();
    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_defaults(),
            },
            None,
        )
        .await
        .unwrap();

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!("shader.wgsl"))),
    });

    // ---------------------------------------------------------
    // initialize uniform buffers
    // let mut uniforms_data = Uniforms { sort_even: 0 };
    let mut uniforms_data = Uniforms {
        log_len: (ARRAY_LENGTH as f32).log2() as u32,
        log_group_init: (ARRAY_LENGTH as f32).log2() as u32 - 1,
        log_group_curr: 1,
    };

    let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: bytemuck::cast_slice(&[uniforms_data]),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    // ---------------------------------------------------------
    // initialize storage buffer
    let storage_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: bytemuck::cast_slice(&local_input[..]),
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
    });

    let output_staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: (std::mem::size_of::<f32>() * local_input.len()) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    // ---------------------------------------------------------

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Bind Group Layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::all(),
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    });

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Bind Group"),
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: storage_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: uniform_buffer.as_entire_binding(),
            },
        ],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Pipeline Layout"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Compute Pipeline"),
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: "main",
    });

    time_list.push(timer.elapsed().as_secs_f64()); // 0
    let timer = Instant::now();

    //----------------------------------------------------------

    let len = local_input.len() as u32;
    // Bubble sort
    // for i in 0..len - 1 {
    //     // update uniform buffer
    //     uniforms_data.sort_even = i % 2;
    //     queue.write_buffer(&uniform_buffer, 0, bytemuck::cast_slice(&[uniforms_data]));

    //     let mut command_encoder =
    //         device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    //     {
    //         let mut compute_pass =
    //             command_encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
    //                 label: Some("Compute Pass"),
    //                 timestamp_writes: None,
    //             });
    //         compute_pass.set_pipeline(&pipeline);
    //         compute_pass.set_bind_group(0, &bind_group, &[]);

    //         compute_pass.dispatch_workgroups((len / 128).max(1), 1, 1);
    //     }
    //     queue.submit(Some(command_encoder.finish()));
    // }

    // Bitonic sort
    let log_len = (local_input.len() as f32).log2() as u32;
    for num_stage in 1..=log_len {
        let log_num_group_init = log_len - num_stage;
        uniforms_data.log_group_init = log_num_group_init;

        for num_step in 0..num_stage {
            let log_num_group = log_num_group_init + num_step;

            // update uniform buffer
            uniforms_data.log_group_curr = log_num_group;
            queue.write_buffer(&uniform_buffer, 0, bytemuck::cast_slice(&[uniforms_data]));

            let mut command_encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
            {
                let mut compute_pass =
                    command_encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                        label: Some("Compute Pass"),
                        timestamp_writes: None,
                    });
                compute_pass.set_pipeline(&pipeline);
                compute_pass.set_bind_group(0, &bind_group, &[]);

                let (x, y, z) = if log_len <= 8 {
                    (1, 1, 1)
                } else {
                    let log_len_global = log_len - 8;
                    let len_global_div2 = 1 << (log_len_global / 2);
                    (len_global_div2 * 2, len_global_div2, 1)
                };
                compute_pass.dispatch_workgroups(x, y, z);
            }
            queue.submit(Some(command_encoder.finish()));
        }
    }

    time_list.push(timer.elapsed().as_secs_f64()); // 1
    let timer = Instant::now();

    //----------------------------------------------------------

    let mut bitonic_sorted = vec![0f32; ARRAY_LENGTH];

    get_data(
        &mut bitonic_sorted,
        &storage_buffer,
        &output_staging_buffer,
        &device,
        &queue,
    )
    .await;

    time_list.push(timer.elapsed().as_secs_f64()); // 2
    let timer = Instant::now();

    let mut sorted = local_input.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    // bubble_sort(&mut sorted);

    time_list.push(timer.elapsed().as_secs_f64()); // 3
    let timer = Instant::now();

    // log::info!("Output of merge sort: {sorted:?}");
    // log::info!("Output of bitonic_sort: {bitonic_sorted:?}");

    assert!(sorted == bitonic_sorted);
    log::info!("Bitonic sort successful!");
    log::info!("Initialization takes: {}s", time_list[0]);
    log::info!("Wgpu computation takes: {}s", time_list[1]);
    log::info!("Data transfer takes: {}s", time_list[2]);
    log::info!("CPU sorting takes: {}s", time_list[3]);
}

async fn get_data<T: bytemuck::Pod>(
    output: &mut Vec<T>,
    storage_buffer: &wgpu::Buffer,
    staging_buffer: &wgpu::Buffer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) {
    let mut command_encoder =
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    command_encoder.copy_buffer_to_buffer(
        storage_buffer,
        0,
        staging_buffer,
        0,
        (std::mem::size_of::<f32>() * output.len()) as u64,
    );
    queue.submit(Some(command_encoder.finish()));
    let buffer_slice = staging_buffer.slice(..);
    let (sender, receiver) = futures_intrusive::channel::shared::oneshot_channel();
    buffer_slice.map_async(wgpu::MapMode::Read, move |r| sender.send(r).unwrap());
    device.poll(wgpu::Maintain::Wait);
    receiver.receive().await.unwrap().unwrap();
    *output = bytemuck::cast_slice(&buffer_slice.get_mapped_range()).to_vec();
    staging_buffer.unmap();
}

#[allow(dead_code, clippy::ptr_arg)]
fn bubble_sort(arr: &mut Vec<f32>) {
    for i in 0..arr.len() {
        for j in 0..arr.len() - i - 1 {
            if arr[j] > arr[j + 1] {
                arr.swap(j, j + 1);
            }
        }
    }
}
