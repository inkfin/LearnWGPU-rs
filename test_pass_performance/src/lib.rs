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

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub async fn run() {
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

    let mut local_a = [0i32; 100];
    for (i, e) in local_a.iter_mut().enumerate() {
        *e = i as i32;
    }
    // log::info!("Input a: {local_a:?}");
    let mut local_b = [0i32; 100];
    for (i, e) in local_b.iter_mut().enumerate() {
        *e = i as i32 * 2;
    }
    // log::info!("Input b: {local_b:?}");

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

    let storage_buffer_a = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: bytemuck::cast_slice(&local_a[..]),
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
    });
    let storage_buffer_b = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: bytemuck::cast_slice(&local_b[..]),
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
    });
    let output_staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: std::mem::size_of_val(&local_a) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
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
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: storage_buffer_a.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: storage_buffer_b.as_entire_binding(),
            },
        ],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: None,
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: "main",
    });

    let pipeline2 = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Pipeline 2"),
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: "main2",
    });

    //----------------------------------------------------------

    let timer = Instant::now();

    for _ in 1..100 {
        let mut command_encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut compute_pass =
                command_encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: None,
                    timestamp_writes: None,
                });
            compute_pass.set_pipeline(&pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            /* Note that since each workgroup will cover both arrays, we only need to
            cover the length of one array. */
            compute_pass.dispatch_workgroups(local_a.len() as u32, 1, 1);
        }

        queue.submit(Some(command_encoder.finish()));
    }

    let elapsed_time = timer.elapsed().as_secs_f32();
    log::info!("100 separate iters cost: {elapsed_time}");

    //----------------------------------------------------------

    let timer = Instant::now();

    let mut command_encoder =
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    {
        let mut compute_pass = command_encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: None,
            timestamp_writes: None,
        });
        for _ in 1..100 {
            compute_pass.set_pipeline(&pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            /* Note that since each workgroup will cover both arrays, we only need to
            cover the length of one array. */
            compute_pass.dispatch_workgroups(local_a.len() as u32, 1, 1);
        }
    }

    queue.submit(Some(command_encoder.finish()));

    let elapsed_time = timer.elapsed().as_secs_f32();
    log::info!("100 inner iters cost: {elapsed_time}");

    //----------------------------------------------------------

    let timer = Instant::now();

    for _ in 0..100 {
        let mut command_encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut compute_pass =
                command_encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: None,
                    timestamp_writes: None,
                });
            compute_pass.set_pipeline(&pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            /* Note that since each workgroup will cover both arrays, we only need to
            cover the length of one array. */
            compute_pass.dispatch_workgroups(local_a.len() as u32, 1, 1);
        }

        queue.submit(Some(command_encoder.finish()));

        let mut command_encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut compute_pass =
                command_encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: None,
                    timestamp_writes: None,
                });
            compute_pass.set_pipeline(&pipeline2);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            /* Note that since each workgroup will cover both arrays, we only need to
            cover the length of one array. */
            compute_pass.dispatch_workgroups(local_a.len() as u32, 1, 1);
        }

        queue.submit(Some(command_encoder.finish()));
    }

    let elapsed_time = timer.elapsed().as_secs_f32();
    log::info!("200 inters separate submits cost: {elapsed_time}");

    //----------------------------------------------------------

    let timer = Instant::now();

    let mut command_encoder =
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    {
        let mut compute_pass = command_encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: None,
            timestamp_writes: None,
        });

        for _ in 0..100 {
            compute_pass.set_pipeline(&pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            /* Note that since each workgroup will cover both arrays, we only need to
            cover the length of one array. */
            compute_pass.dispatch_workgroups(local_a.len() as u32, 1, 1);

            compute_pass.set_pipeline(&pipeline2);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            compute_pass.dispatch_workgroups(local_a.len() as u32, 1, 1);
        }
    }

    queue.submit(Some(command_encoder.finish()));

    let elapsed_time = timer.elapsed().as_secs_f32();
    log::info!("200 inters combined cost: {elapsed_time}");

    //----------------------------------------------------------

    let timer = Instant::now();

    for _ in 0..100 {
        let mut command_encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut compute_pass =
                command_encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: None,
                    timestamp_writes: None,
                });
            compute_pass.set_pipeline(&pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            /* Note that since each workgroup will cover both arrays, we only need to
            cover the length of one array. */
            compute_pass.dispatch_workgroups(local_a.len() as u32, 1, 1);
        }
        queue.submit(Some(command_encoder.finish()));
    }

    let elapsed_time = timer.elapsed().as_secs_f32();
    log::info!("100 inters cost: {elapsed_time}");

    //----------------------------------------------------------

    get_data(
        &mut local_a[..],
        &storage_buffer_a,
        &output_staging_buffer,
        &device,
        &queue,
    )
    .await;
    get_data(
        &mut local_b[..],
        &storage_buffer_b,
        &output_staging_buffer,
        &device,
        &queue,
    )
    .await;

    // log::info!("Output in A: {local_a:?}");
    // log::info!("Output in B: {local_b:?}");
}

async fn get_data<T: bytemuck::Pod>(
    output: &mut [T],
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
        std::mem::size_of_val(output) as u64,
    );
    queue.submit(Some(command_encoder.finish()));
    let buffer_slice = staging_buffer.slice(..);
    let (sender, receiver) = futures_intrusive::channel::shared::oneshot_channel();
    buffer_slice.map_async(wgpu::MapMode::Read, move |r| sender.send(r).unwrap());
    device.poll(wgpu::Maintain::Wait);
    receiver.receive().await.unwrap().unwrap();
    output.copy_from_slice(bytemuck::cast_slice(&buffer_slice.get_mapped_range()[..]));
    staging_buffer.unmap();
}
