// Sort algorithms in Parallel World
// # Bubble sort
// Bubble sort based on even-odd sort, which is a parallel version of bubble sort.
//     >> miserably slow
//     iter1: sort [even, odd] pairs
//     iter2: sort [odd, even] pairs
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

const workgroup_len: u32 = 256;

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
    compute_mode: u32,
};

const GLOBAL_FLIP: u32 = 0;
const GLOBAL_DISPERSE: u32 = 1;
const LOCAL_MODE: u32 = 2;

@group(0)
@binding(1)
var<uniform> uniforms: Uniforms;

// use workgroup shared memory to accelerate load and write within workgroups
var<workgroup> arr_shared: array<f32, workgroup_len>;

fn compare_and_swap(i: u32, j: u32) {
    if arr[i] > arr[j] {
        let tmp = arr[i];
        arr[i] = arr[j];
        arr[j] = tmp;
    }
}

fn compare_and_swap_local(i: u32, j: u32) {
    if arr_shared[i] > arr_shared[j] {
        let tmp = arr_shared[i];
        arr_shared[i] = arr_shared[j];
        arr_shared[j] = tmp;
    }
}

fn big_disperse(ix: u32, height: u32) {
    let base = ix / height * height;
    let half_height = height >> 1u;
    if ix < base + half_height {
        compare_and_swap(ix, ix + half_height);
    }
}

fn big_flip(ix: u32, height: u32) {
    let base = ix / height * height;
    let offset = ix - base;
    let half_height = height >> 1u;
    if ix < base + half_height {
        compare_and_swap(ix, base + height - 1 - offset);
    }
}

fn local_disperse(ix: u32, height: u32) {
    let base = ix / height * height;
    let half_height = height >> 1u;
    if ix < base + half_height {
        compare_and_swap_local(ix, ix + half_height);
    }
}

fn local_flip(ix: u32, height: u32) {
    let base = ix / height * height;
    let offset = ix - base;
    let half_height = height >> 1u;
    if ix < base + half_height {
        compare_and_swap_local(ix, base + height - 1 - offset);
    }
}

fn sort_bitonic(ix: u32) {
    let size = arrayLength(&arr);
    let num_group_max = 1u << (uniforms.log_len - 1);

    // local workgroup shared memory acceleration
    // 256 threads, can handle group_size 256, 128, 64, 32, 16, 8, 4, 2
    // only use log_group_init, since we're going to apply for loop here
    switch uniforms.compute_mode {
        case LOCAL_MODE: {
            // load data to shared memory
            let local_ix = ix % workgroup_len;
            arr_shared[local_ix] = arr[ix];

            // workgroup shared memory barrier
            workgroupBarrier();

            for (var num_stage = 1u; num_stage <= 8u; num_stage++) {
                let num_group_init = size >> num_stage;
                for (var num_step = 0u; num_step < num_stage; num_step++) {
                    let num_group_curr = num_group_init << num_step;
                    let height = size / num_group_curr;

                    // let log_num_group = uniforms.log_group_init + num_step;
                    let is_first_group = num_group_init == num_group_max;
                    let is_first_step = num_group_init == num_group_curr;

                    if !is_first_group && is_first_step {
                        local_flip(local_ix, height);
                    } else {
                        local_disperse(local_ix, height);
                    }

                    workgroupBarrier();
                }
                // early break
                if num_group_init == 1 {
                    break;
                }
            }

            workgroupBarrier();
            // write back to global memory
            arr[ix] = arr_shared[local_ix];
        }
        case GLOBAL_FLIP: {
            let num_group_curr = 1u << uniforms.log_group_curr;
            let num_group_init = 1u << uniforms.log_group_init;
            let height = size / num_group_curr;
            big_flip(ix, height);
        }
        case GLOBAL_DISPERSE: {
            let num_group_curr = 1u << uniforms.log_group_curr;
            let num_group_init = 1u << uniforms.log_group_init;
            let height = size / num_group_curr;
            big_disperse(ix, height);
        }
        default: {}
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
//     compute_and_swap(i, j);
// }


/// Grantee input size is power of 2
@compute
@workgroup_size(workgroup_len, 1, 1)
fn main(
    @builtin(local_invocation_id) lid: vec3<u32>,
    @builtin(global_invocation_id) gid: vec3<u32>,
    @builtin(workgroup_id) wid: vec3<u32>,
    @builtin(num_workgroups) num_wg: vec3<u32>
) {

    let ix = gid.x + gid.y * num_wg.x * workgroup_len; // denotes index in array
    // let ix = gid.x;
    // if gid.y != 0 { return; }
    if ix >= arrayLength(&arr) {
        return;
    }
    // sort_bubble(ix);
    sort_bitonic(ix);
}
