//! EE single-precision timing and packed combo numeric operations round toward zero.
//! Keep this local: the sim's ordinary floating-point policy is unchanged.

fn rz(value: f64) -> f32 {
    let nearest = value as f32;
    if f64::from(nearest.abs()) > value.abs() {
        f32::from_bits(nearest.to_bits() - 1)
    } else {
        nearest
    }
}

pub(super) fn add(a: f32, b: f32) -> f32 {
    rz(f64::from(a) + f64::from(b))
}
pub(super) fn sub(a: f32, b: f32) -> f32 {
    rz(f64::from(a) - f64::from(b))
}
pub(super) fn mul(a: f32, b: f32) -> f32 {
    rz(f64::from(a) * f64::from(b))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retained_native_clock_and_transition_cutoff_are_exact() {
        let clock = f32::from_bits(0x4136_f66c);
        let dt = f32::from_bits(0x3d088888);
        let elapsed = add(clock, dt);
        assert_eq!(elapsed.to_bits(), 0x4137_7ef4);
        assert_ne!(elapsed.to_bits(), (clock + dt).to_bits());
        let origin = sub(clock, elapsed);
        let event = sub(clock, origin);
        assert_eq!(sub(add(event, clock), elapsed).to_bits(), clock.to_bits());
        assert_eq!(add(-clock, -dt).to_bits(), (-elapsed).to_bits());
    }
}
