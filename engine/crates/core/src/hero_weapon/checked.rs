use super::*;

pub(super) fn number(src: &dyn PropSource, key: &str, default: Option<f32>) -> Option<f32> {
    src.prop(key)
        .map_or(default, |s| s.split_whitespace().next()?.parse().ok())
        .filter(|n: &f32| n.is_finite())
}

pub(super) fn hash(src: &dyn PropSource, key: u32, default: Option<f32>) -> Option<f32> {
    src.prop_hash(key)
        .map_or(default, |s| s.trim().parse().ok())
        .filter(|n: &f32| n.is_finite())
}
