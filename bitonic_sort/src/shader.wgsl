// Sort algorithms in Parallel World
//
// # Bitonic sort
// etc: 16 elements
// stage 1: divide by 2, num_group = (16/2) = 8, each group has 2 elements
//         1.1: sort 1 pairs [0..1]
// stage 2: divide by 4, num_group = (16/4) = 4
//         2.1: sort 2 pairs [0..3], [1..2]
//         divide by 2, num_group = (4/2) = 2, 4 elements, 2 pairs
//         2.2: sort 1 pair [0..1]
// stage 3: divide by 8, num_group = (16/8) = 2
//         3.1: sort 4 pairs [0..7], [1..6], [2..5], [3..4]
//         3.2: sort 2 pairs [0..3], [1..2]
//         3.3: sort 1 pair [0..1]


@group(0)
@binding(0)
var<storage, read_write> arr: array<f32>;

// bubble sort
// struct Uniforms {
//     sort_even: u32, // 0: sort odd, 1: sort even
// }

// bitonic sort
struct Uniforms {
    log_len: u32, // 2^log_len = arrayLength
    log_group_init: u32, // 2^log_group = num_group
    log_group_curr: u32,
};

@group(0)
@binding(1)
var<uniform> uniforms: Uniforms;

// use workgroup shared memory to accelerate load and write within workgroups
// var<workgroup> shared: array<f32, 256>;

fn swap_value(i: u32, j: u32) {
    let tmp = arr[i];
    arr[i] = arr[j];
    arr[j] = tmp;
}

fn compare_and_swap(i: u32, j: u32) {
    if arr[i] > arr[j] {
        swap_value(i, j);
    }
}

fn big_disperse(ix: u32, height: u32) {
    let base = ix / height * height;
    let half_height = height / 2;
    if ix < base + half_height {
        compare_and_swap(ix, ix + half_height);
    }
}

fn big_flip(ix: u32, height: u32) {
    let base = ix / height * height;
    let offset = ix - base;
    let half_height = height / 2;
    if ix < base + half_height {
        compare_and_swap(ix, base + height - 1 - offset);
    }
}

fn sort_bitonic(ix: u32) {
    let size = arrayLength(&arr);
    let num_group_init = 1u << uniforms.log_group_init;
    let num_group_curr = 1u << uniforms.log_group_curr;
    let height = size / num_group_curr;
    let step = uniforms.log_group_init - uniforms.log_group_curr + 1;

    if (step == 1) && (uniforms.log_group_init != uniforms.log_len - 1) {
        big_flip(ix, height);
    } else {
        big_disperse(ix, height);
    }
}


// fn sort_bubble(ix: u32) {
//     if ix % 2 == 1 {
//         return;
//     }
//     var i: u32;
//     var j: u32;
//     if uniforms.sort_even == 1 {
//         i = ix; j = ix + 1;
//     } else {
//         if ix == 0 {
//             return;
//         }
//         i = ix - 1; j = ix;
//     }
//     // swap if left > right
//     if arr[i] > arr[j] {
//         swap_value(i, j);
//     }
// }


/// Grantee input size is power of 2
@compute
@workgroup_size(256, 1, 1)
fn main(
    @builtin(local_invocation_id) lid: vec3<u32>,
    @builtin(global_invocation_id) gid: vec3<u32>,
    @builtin(workgroup_id) wid: vec3<u32>
) {

    let ix = gid.x; // denotes index in array
    if ix >= arrayLength(&arr) {
        return;
    }
    // sort_bubble(ix);
    sort_bitonic(ix);
}
