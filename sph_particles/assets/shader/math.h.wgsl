const M_PI: f32 = 3.1415926535897932384626;

const CUBIC_KERNEL_FACTOR: f32 = 10 / (7 * M_PI);

fn cubicKernel(r: vec2f, h: f32) -> f32 {
    let r_len = length(r);
    let half_h = h / 2.0;
    let k = CUBIC_KERNEL_FACTOR / (half_h * half_h);
    let q = max(r_len / half_h, 0.0);

    var res: f32 = 0.0;
    if q <= 1.0 {
        let q2 = q * q;
        res = k * (1.0 - 1.5 * q2 + 0.75 * q * q2);
    } else if q < 2.0 {
        res = k * 0.25 * pow((2.0 - q), 3.0);
    }
    return res;
}
