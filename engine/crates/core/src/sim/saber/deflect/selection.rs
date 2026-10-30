//! Direction buckets and fallback masks in FUN_003074a8.

use super::*;
use glam::Mat4;

// FUN_003074a8 loads this literal as 0xbf350529 at 3076f4..3076fc.
// Using -FRAC_1_SQRT_2 would change the native bucket boundary.
#[allow(clippy::approx_constant)]
const REAR_CUTOFF: f32 = -0.70711;

pub(in crate::sim::saber) fn direction_mask(
    available: u8,
    incoming: Vec3,
    facing: Vec3,
    velocity: Vec3,
) -> u8 {
    let horizontal = Vec3::new(velocity.x, 0.0, velocity.z);
    let axis = if horizontal.length_squared() > 0.09 {
        let axis = horizontal.normalize();
        if axis.dot(facing) <= 0.0 {
            -axis
        } else {
            axis
        }
    } else {
        facing
    };
    let dot = -incoming.dot(axis);
    let right = incoming.cross(axis).y < 0.0;
    let (first, second, third) = if dot > 0.92388 {
        if right {
            (1, 3, 15)
        } else {
            (1, 9, 27)
        }
    } else if dot > 0.38268 {
        if right {
            (2, 6, 7)
        } else {
            (8, 24, 25)
        }
    } else if dot <= REAR_CUTOFF {
        if right {
            (32, 36, 54)
        } else {
            (32, 48, 60)
        }
    } else if right {
        (4, 6, 38)
    } else {
        (16, 24, 56)
    };
    [first, second, third]
        .into_iter()
        .find(|mask| mask & available != 0)
        .unwrap_or(63)
}

pub(in crate::sim::saber) fn reflected_origin(
    class: &crate::hero_weapon::SaberClass,
    animation: Option<&str>,
    basis: Mat4,
    impact: Vec3,
) -> Vec3 {
    let Some(attachment) = animation.and_then(|a| class.deflect_attachment(a)) else {
        return impact;
    };
    let world = basis * attachment;
    // 33ecf0..33ed64 dots the offset against each composed basis column.
    // Its rigid transpose must not become a general inverse: retained-input
    // caps and sampled animation matrices need not be exactly orthonormal.
    let offset = impact - world.w_axis.truncate();
    let local = Vec3::new(
        offset.dot(world.x_axis.truncate()),
        offset.dot(world.y_axis.truncate()),
        offset.dot(world.z_axis.truncate()),
    );
    let nearest = class.deflect_edges.iter().min_by(|a, b| {
        let distance = |edge: &crate::hero_weapon::BladeSegment| {
            ((edge.start + edge.end) * 0.5 - local).length_squared()
        };
        distance(a).total_cmp(&distance(b))
    });
    let Some(edge) = nearest else {
        return world.w_axis.truncate();
    };
    let delta = edge.end - edge.start;
    let length = delta.length();
    if length == 0.0 {
        return world.transform_point3(edge.start);
    }
    let direction = delta / length;
    // The native fraction is centred and halved, not a nearest projection.
    let fraction = (((local - edge.start).dot(direction) / length + 1.0) * 0.5).clamp(0.0, 1.0);
    world.transform_point3(edge.start + delta * fraction)
}
