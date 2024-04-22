@group(0) @binding(0) var inputImage: texture_storage_2d<f32, read>;
@group(0) @binding(1) var outputImage: texture_storage_2d<f32, write>;
@group(0) @binding(2) var weightBuffer: texture_2d<f32>;

struct Uniforms {
    sigma1: f32;
    sigma2: f32;
    indexesSize: i32;
    filterInterval: i32;
};

@group(0) @binding(3) var<uniform> uniforms: Uniforms;

@group(1) @binding(0) buffer Indexs {
    indexes: array<vec2<i32>>,
};

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let curPixelId = vec2<i32>(global_id.xy);
    let originDepth = textureLoad(inputImage, curPixelId, 0);

    if (originDepth.r > 0.0) {
        textureStore(outputImage, curPixelId, vec4<f32>(1.0, 0.0, 0.0, 0.0));
        return;
    }

    var blureDepth: f32 = 0.0;
    var weightSum: f32 = 0.0;

    for (var i = 0; i < uniforms.indexesSize; i++) {
        let index = uniforms.indexes[i] * uniforms.filterInterval;
        let sampleColor = textureLoad(inputImage, curPixelId + index, 0);
        if (sampleColor.r < 0.0) {
            let w = Weight(length(vec2<f32>(index)), abs(originDepth.r - sampleColor.r));
            blureDepth += w * sampleColor.r;
            weightSum += w;
        }
    }

    if (weightSum != 0.0) {
        blureDepth /= weightSum;
    }

    textureStore(outputImage, curPixelId, vec4<f32>(blureDepth, 0.0, 0.0, 0.0));
}

fn Weight(d1: f32, d2: f32) -> f32 {
    let texCoord = vec2<f32>(d1 / (3.0 * uniforms.sigma1), d2 / (3.0 * uniforms.sigma2));
    return textureSample(weightBuffer, sampler(weightBuffer), texCoord).r;
}