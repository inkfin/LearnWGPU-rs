// Reference: https://github.com/InteractiveComputerGraphics/SPlisHSPlasH/blob/master/SPlisHSPlasH/SPHKernels.h
const M_PI: f32 = 3.1415926535897932384626;

//====================================================
// Cubic Kernel 3D
//====================================================

// smooth kernel function
fn cubicKernel3D(r: vec3<f32>, h: f32) -> f32 {
    // value of cubic spline smoothing kernel
    let r_len = length(r);
    let k = 8.0 / (M_PI * h * h * h);
    let q = max(r_len / h, 0.0);

    // assert q > 0.0
    var res: f32 = 0.0;
    if q <= 1.0 {
        if q <= 0.5 {
            res = k * (6.0 * q * q * q - 6.0 * q * q + 1.0);
        } else {
            res = k * 2.0 * pow(1.0 - q, 3.0);
        }
    }
    return res;
}

fn cubicGrad3D(r: vec3<f32>, h: f32) -> vec3<f32> {
    // derivative of cubic spline smoothing kernel
    let r_len = length(r);
    let r_dir = normalize(r);

    let l = 48.0 / (M_PI * h * h * h);
    let q = r_len / h;
    let gradq = r_dir / h;

    // assert q > 0.0
    var res = vec3<f32>(0.0);
    if q > 1e-9 && q <= 1.0 {
        if q <= 0.5 {
            res = l * q * (3.0 * q - 2.0) * gradq;
        } else {
            res = l * (q - 1.0) * (1.0 - q) * gradq;
        }
    }
    return res;
}

//====================================================
// Cubic Kernel 2D
//====================================================

// smooth kernel function
fn cubicKernel2D(r: vec3<f32>, h: f32) -> f32 {
    // value of cubic spline smoothing kernel
    let r_len = length(r);
    let k = 40.0 / (7.0 * M_PI * h * h);
    let q = max(r_len / h, 0.0);

    // assert q > 0.0
    var res: f32 = 0.0;
    if q <= 1.0 {
        if q <= 0.5 {
            res = k * (6.0 * q * q * q - 6.0 * q * q + 1.0);
        } else {
            res = k * 2.0 * pow(1.0 - q, 3.0);
        }
    }
    return res;
}

fn cubicGrad2D(r: vec3<f32>, h: f32) -> vec3<f32> {
    // derivative of cubic spline smoothing kernel
    let r_len = length(r);
    let r_dir = normalize(r);

    let k = 240.0 / (7.0 * M_PI * h * h);
    let q = r_len / h;
    let gradq = r_dir / h;

    // assert q > 0.0
    var res = vec3<f32>(0.0);
    if q > 1e-9 && q <= 1.0 {
        if q <= 0.5 {
            res = k * q * (3.0 * q - 2.0) * gradq;
        } else {
            res = k * (q - 1.0) * (1.0 - q) * gradq;
        }
    }
    return res;
}

// End Cubic Kernel 2D
//====================================================
