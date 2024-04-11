// This is useful because we can't use, say, vec2<array<i32>> because
// of array<T> being unsized. Normally we would interweave them or use
// and array of structs but this is just for the sake of demonstration.

@group(0) @binding(0)
var<storage, read_write> arr: array<f32>;

@compute @workgroup_size(1, 1, 1)
fn main(@builtin(local_invocation_id) lid: vec3<u32>, @builtin(workgroup_id) wid: vec3<u32>) {
    arr[wid.x] += 1.0;
}
