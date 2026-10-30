//! BF2 282700 measures the first half, then caches signed15-bit cm/s.
//! Only hero setup uses this EE rounding; ordinary animation math is unchanged.
use super::*;

fn rz(value: f64) -> f32 {
    let nearest = value as f32;
    if f64::from(nearest.abs()) > value.abs() {
        f32::from_bits(nearest.to_bits() - 1)
    } else {
        nearest
    }
}
fn add(a: f32, b: f32) -> f32 {
    rz(f64::from(a) + f64::from(b))
}
fn sub(a: f32, b: f32) -> f32 {
    rz(f64::from(a) - f64::from(b))
}
fn mul(a: f32, b: f32) -> f32 {
    rz(f64::from(a) * f64::from(b))
}
fn div(a: f32, b: f32) -> f32 {
    rz(f64::from(a) / f64::from(b))
}

pub(super) fn packed(speed: [f32; 2]) -> Result<[f32; 2]> {
    if speed.iter().any(|v| !v.is_finite()) {
        bail!("nonfinite physical root speed");
    }
    Ok(speed.map(|v| {
        let biased = add(add(mul(v, 100.0), 16383.0), 0.5).clamp(0.0, 32767.0);
        let bits = (biased as i32 - 16383) & 0x7fff;
        let signed = (bits << 17) >> 17;
        mul(signed as f32, f32::from_bits(0x3c23d70a))
    }))
}

pub(super) fn measure(root: &[[f32; 3]]) -> Result<[f32; 2]> {
    if root.iter().flatten().any(|v| !v.is_finite()) {
        bail!("nonfinite original root key");
    }
    // A stationary one-key/bind pose has no elapsed interval to divide by.
    if root.len() <= 1 {
        return Ok([0.0; 2]);
    }
    let frames = (root.len() - 1) as f32;
    let half_seconds = mul(mul(frames, f32::from_bits(0x3d088889)), 0.5);
    // 2783b8→532b28 stores phase; 532c80 reconstructs the sample frame.
    let phase = div(mul(half_seconds, 30.0), frames);
    let at = mul(phase, frames);
    let index = at as usize;
    let fraction = sub(at, index as f32);
    let sample = |axis| {
        let a = root[index][axis];
        let b = root[(index + 1).min(root.len() - 1)][axis];
        a + (b - a) * fraction
    };
    packed([
        div(sub(sample(0), root[0][0]), half_seconds),
        div(sub(sample(2), root[0][2]), half_seconds),
    ])
}
