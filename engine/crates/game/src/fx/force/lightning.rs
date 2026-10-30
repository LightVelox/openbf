//! Native lightning strip construction, isolated from effect lifecycle.
use super::{Camera, Lcg, ParticleDraw, ParticleShape, TextureHandle, Vec3, SEED_XOR};

pub(super) trait Seeds {
    fn seed(&mut self) -> u32;
}
impl Seeds for Lcg {
    fn seed(&mut self) -> u32 {
        let a = self.step();
        let b = self.step();
        (a >> 16) | (b & 0xffff_0000)
    }
}

pub(super) fn byte(v: f32) -> u8 {
    v.clamp(0.0, 255.0) as u8
}

#[derive(Clone, Copy)]
pub(super) struct Strip {
    pub(super) width: f32,
    pub(super) spread: f32,
    pub(super) phase: f32,
    pub(super) break_distance: f32,
    pub(super) texture_size: f32,
    pub(super) branch_factor: f32,
    pub(super) branch_spread: f32,
    pub(super) branch_length: f32,
    pub(super) max_branches: usize,
}

fn perpendicular(seed: &mut Lcg, axis: Vec3, spread: f32) -> Vec3 {
    let v = Vec3::new(seed.float() - 0.5, seed.float() - 0.5, seed.float() - 0.5) * spread;
    v - axis * v.dot(axis)
}

/// Native strip point count, two seeded jitter streams interpolated at phase,
/// taper, endpoint alpha zero, and one generation of lateral branches.
pub(super) fn strip_draws(
    endpoints: [Vec3; 2],
    seeds: [u32; 2],
    spec: Strip,
    color: [u8; 4],
    texture: Option<TextureHandle>,
    camera: &Camera,
    out: &mut Vec<ParticleDraw>,
) {
    let [from, to] = endpoints;
    let delta = to - from;
    let length = delta.length();
    if !length.is_finite()
        || length <= 0.0
        || spec.break_distance <= 0.0
        || spec.texture_size <= 0.0
    {
        return;
    }
    let n = ((length / spec.break_distance) as usize).max(3);
    if n > 100 {
        return;
    } // Native 0x459c50 skips oversized strips.
    let axis = delta / length;
    let step = length / n as f32;
    let mut a = Lcg(seeds[0] ^ SEED_XOR);
    let mut b = Lcg(seeds[1] ^ SEED_XOR);
    let taper = spec.width / (n + 1) as f32;
    let mut points = vec![(from, spec.width, 0.0, 0)];
    let mut branches = Vec::new();
    let mut width = spec.width;
    for i in 1..n {
        let base = from + axis * (step * i as f32);
        let jitter = perpendicular(&mut a, axis, spec.spread)
            .lerp(perpendicular(&mut b, axis, spec.spread), spec.phase);
        points.push((
            base + jitter,
            width,
            step * i as f32 / spec.texture_size,
            color[3],
        ));
        let branch = a.float() < spec.branch_factor;
        b.float();
        if branch && branches.len() < spec.max_branches {
            branches.push((
                base + jitter,
                (jitter + axis * spec.branch_spread).normalize_or_zero(),
                width,
            ));
        }
        width -= taper;
    }
    points.push((to, width, length / spec.texture_size, 0));
    let mut tris = Vec::new();
    let side = |i: usize| {
        let p = points[i].0;
        let prev = points[i.saturating_sub(1)].0;
        let next = points[(i + 1).min(points.len() - 1)].0;
        ((next - prev).cross(p - camera.position))
            .try_normalize()
            .unwrap_or(Vec3::Y)
            * points[i].1
    };
    for i in 1..points.len() {
        let (p, _, u, alpha) = points[i - 1];
        let (q, _, v, beta) = points[i];
        let ca = [color[0], color[1], color[2], alpha];
        let cb = [color[0], color[1], color[2], beta];
        let sa = side(i - 1);
        let sb = side(i);
        let va = (p + sa, [u, 0.0], ca);
        let vb = (p - sa, [u, 1.0], ca);
        let vc = (q + sb, [v, 0.0], cb);
        let vd = (q - sb, [v, 1.0], cb);
        tris.extend([va, vb, vc, vc, vb, vd]);
    }
    out.push(ParticleDraw {
        shape: ParticleShape::Shaded { tris },
        key: (from + to) * 0.5,
        color,
        texture,
        additive: true,
    });
    for (i, (start, direction, width)) in branches.into_iter().enumerate().rev() {
        strip_draws(
            [start, start + direction * spec.branch_length],
            [
                seeds[0].wrapping_add((i + 1) as u32),
                seeds[1].wrapping_add((i + 1) as u32),
            ],
            Strip {
                width,
                branch_factor: spec.branch_factor * 0.5,
                branch_length: spec.branch_length * 0.5,
                max_branches: 0,
                ..spec
            },
            color,
            texture,
            camera,
            out,
        );
    }
}
