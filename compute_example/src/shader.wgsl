@group(0)
@binding(0)
var<storage, read_write> s_buffer: array<u32>; // this is used as both input and output for convenience

fn my_func(n: u32) -> u32 {
    var result: u32;
    result = n + 1u;
    return result;
}

@compute
@workgroup_size(1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let gid = global_id.x;
    s_buffer[gid] = my_func(s_buffer[gid]);
}
