use std::{
    fs,
    io::{self, Write},
    path,
};

use super::resources::load_string;

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
