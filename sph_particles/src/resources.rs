use std::io::{self, BufReader, Cursor, Write};
use std::{fs, path};

use wgpu::util::DeviceExt;

use crate::{model, texture};

#[cfg(target_arch = "wasm32")]
fn format_url(file_name: &str) -> reqwest::Url {
    let window = web_sys::window().unwrap();
    let location = window.location();
    let mut href = location.href().unwrap();
    if !href.ends_with("assets") {
        href = format!("{}/assets", href);
    }
    let base = reqwest::Url::parse(&format!("{}/", href,)).unwrap();
    base.join(file_name).unwrap()
}

pub async fn load_string(file_name: &str) -> anyhow::Result<String> {
    let txt: String;
    #[cfg(target_arch = "wasm32")]
    {
        let url = format_url(file_name);
        txt = reqwest::get(url).await?.text().await?;
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let path = std::path::Path::new(env!("OUT_DIR"))
            .join("assets")
            .join(file_name);
        txt = std::fs::read_to_string(path)?;
    }

    Ok(txt)
}

pub async fn load_binary(file_name: &str) -> anyhow::Result<Vec<u8>> {
    let data: Vec<u8>;
    #[cfg(target_arch = "wasm32")]
    {
        let url = format_url(file_name);
        data = reqwest::get(url).await?.bytes().await?.to_vec();
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let path = std::path::Path::new(env!("OUT_DIR"))
            .join("assets")
            .join(file_name);
        data = std::fs::read(path)?;
    }

    Ok(data)
}

pub async fn load_texture(
    file_name: &str,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> anyhow::Result<texture::Texture> {
    let data = load_binary(file_name).await?;
    texture::Texture::from_bytes(device, queue, &data, file_name)
}

pub async fn load_model(
    file_name: &str,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
) -> anyhow::Result<model::Model> {
    let obj_text = load_string(file_name).await?;
    let obj_cursor = Cursor::new(obj_text);
    let mut obj_reader = BufReader::new(obj_cursor);

    let (models, obj_materials) = tobj::load_obj_buf_async(
        &mut obj_reader,
        &tobj::LoadOptions {
            triangulate: true,
            single_index: true,
            ..Default::default()
        },
        |p| async move {
            let mat_text = load_string(&p)
                .await
                .expect(&format!("Didn't find the material file {}", file_name));
            tobj::load_mtl_buf(&mut BufReader::new(Cursor::new(mat_text)))
        },
    )
    .await?;

    let mut materials = Vec::new();
    for m in obj_materials? {
        let diffuse_texture = load_texture(&m.diffuse_texture.unwrap(), device, queue).await?;
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&diffuse_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&diffuse_texture.sampler),
                },
            ],
            label: None,
        });

        materials.push(model::Material {
            name: m.name,
            diffuse_texture,
            bind_group,
        })
    }

    let meshes = models
        .into_iter()
        .map(|m| {
            let vertices = (0..m.mesh.positions.len() / 3)
                .map(|i| model::ModelVertex {
                    position: [
                        m.mesh.positions[i * 3],
                        m.mesh.positions[i * 3 + 1],
                        m.mesh.positions[i * 3 + 2],
                    ],
                    tex_coords: [m.mesh.texcoords[i * 2], 2.0 - m.mesh.texcoords[i * 2 + 1]],
                    normal: [
                        m.mesh.normals[i * 3],
                        m.mesh.normals[i * 3 + 1],
                        m.mesh.normals[i * 3 + 2],
                    ],
                })
                .collect::<Vec<_>>();

            let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("{:?} Vertex Buffer", file_name)),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
            let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("{:?} Index Buffer", file_name)),
                contents: bytemuck::cast_slice(&m.mesh.indices),
                usage: wgpu::BufferUsages::INDEX,
            });

            model::Mesh {
                name: file_name.to_string(),
                vertex_buffer,
                index_buffer,
                num_elements: m.mesh.indices.len() as u32,
                material: m.mesh.material_id.unwrap_or(0),
            }
        })
        .collect::<Vec<_>>();

    Ok(model::Model { meshes, materials })
}

pub async fn load_shader(name: &str) -> anyhow::Result<wgpu::ShaderModuleDescriptor> {
    let shader_code = load_shader_module(name).await?;

    Ok(wgpu::ShaderModuleDescriptor {
        label: Some(name),
        source: wgpu::ShaderSource::Wgsl(shader_code.into()),
    })
}

async fn load_shader_module(name: &str) -> anyhow::Result<String> {
    let base_path = path::PathBuf::from("shader");
    let module_path = base_path.join(name).with_extension("wgsl");

    let module_source = match load_string(module_path.to_str().unwrap()).await {
        Ok(source) => source,
        Err(e) => panic!(
            "Cannot load shader: {:?} due to error: {:?}",
            module_path, e
        ),
    };

    let mut module_string = String::new();

    let first_line = module_source.lines().next().unwrap();
    if first_line.starts_with("//!include") {
        for include in first_line.split_whitespace().skip(1) {
            module_string.push_str(&Box::pin(load_shader_module(include)).await?);
        }
    }

    module_string.push_str(&module_source);

    #[cfg(not(target_arch = "wasm32"))]
    {
        // debug shader preprocessing
        // write_to_file(base_path.join(format!("{}.o", name)), &module_string).unwrap();
    }

    Ok(module_string)
}

#[allow(dead_code)]
fn write_to_file(path: std::path::PathBuf, source: &String) -> io::Result<()> {
    let mut file = fs::File::create(path)?;
    file.write_all(source.as_bytes())?;
    Ok(())
}
