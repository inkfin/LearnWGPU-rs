struct type_1_block_0Compute {
    _group_0_binding_0_cs: array<u32>,
}

@group(0) @binding(0) 
var<storage, read_write> global: type_1_block_0Compute;
var<private> gl_GlobalInvocationID: vec3<u32>;

fn collatz_iterations(n_base: u32) -> u32 {
    var n_base_1: u32;
    var n: u32 = 0u;
    var i: u32 = 0u;
    var _e4_: u32;
    var _e7_: u32;
    var _e12_: u32;
    var _e15_: u32;
    var _e20_: u32;
    var _e24_: u32;
    var _e27_: u32;

    n_base_1 = n_base;
    let _e8 = n_base_1;
    n = _e8;
    loop {
        if false {
            break;
        }
        {
            let _e11 = n;
            _e4_ = _e11;
            let _e13 = _e4_;
            if (_e13 <= 1u) {
                {
                    break;
                }
            }
            let _e16 = n;
            _e7_ = _e16;
            let _e18 = _e7_;
            if ((_e18 % 2u) == 0u) {
                {
                    let _e23 = n;
                    _e12_ = _e23;
                    let _e25 = _e12_;
                    n = (_e25 / 2u);
                }
            } else {
                {
                    let _e28 = n;
                    _e15_ = _e28;
                    let _e30 = _e15_;
                    if (_e30 >= 1431655765u) {
                        {
                            return 4294967295u;
                        }
                    }
                    let _e34 = n;
                    _e20_ = _e34;
                    let _e37 = _e20_;
                    n = ((3u * _e37) + 1u);
                }
            }
            let _e41 = i;
            _e24_ = _e41;
            let _e43 = _e24_;
            i = (_e43 + 1u);
        }
    }
    let _e46 = i;
    _e27_ = _e46;
    let _e48 = _e27_;
    return _e48;
}

fn main_1() {
    var global_id: vec3<u32>;
    var _e7_1: u32;
    var _e8_: u32;

    let _e3 = gl_GlobalInvocationID;
    global_id = _e3;
    let _e5 = global_id;
    let _e8 = global._group_0_binding_0_cs[_e5.x];
    _e7_1 = _e8;
    let _e11 = _e7_1;
    let _e12 = collatz_iterations(_e11);
    _e8_ = _e12;
    let _e14 = global_id;
    let _e17 = _e8_;
    global._group_0_binding_0_cs[_e14.x] = _e17;
    return;
}

@compute @workgroup_size(1, 1, 1) 
fn main(@builtin(global_invocation_id) param: vec3<u32>) {
    gl_GlobalInvocationID = param;
    main_1();
    return;
}
