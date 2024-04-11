// This is useful because we can't use, say, vec2<array<i32>> because
// of array<T> being unsized. Normally we would interweave them or use
// and array of structs but this is just for the sake of demonstration.

@group(0) @binding(0)
var<storage, read_write> arr: array<f32>;

fn biotomic_sort(ix: u32, low: u32, size: u32, asscent: bool) {
}


fn sort_biotomic(ix: u32) {
    biotomic_sort(ix, 0, arrayLength(arr), true);
}

fn sort_bubble(ix: u32) {
    if ix % 2 == 0 {
        if arr[ix] > arr[ix + 1] {
            let tmp = arr[ix];
            arr[ix] = arr[ix + 1];
            arr[ix + 1] = tmp;
        }
    }
}

/// Grantee input size is power of 2
    @compute @workgroup_size(1, 1, 1)
fn main(
    @builtin(local_invocation_id) lid: vec3<u32>,
    @builtin(global_invocation_id) gid: vec3<u32>,
    @builtin(workgroup_id) wid: vec3<u32>
) {

    let ix = gid.x; // denotes index in array
    let iy = gid.y; // denotes iteration in loop
    if ix >= arrayLength(arr) {
        return;
    }
    sort_bubble(ix);
}
