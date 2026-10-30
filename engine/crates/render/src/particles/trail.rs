//! The native saber trail has a colored sweep between origin and tip,
//! with one transparent authored-width fringe beyond each endpoint.

use super::*;

pub(super) fn vertices(
    sections: &[(Vec3, Vec3, f32, [u8; 4])],
    length: f32,
    camera: &Camera,
) -> Vec<GrassVertex> {
    if !length.is_finite() || length <= 0.0 {
        return Vec::new();
    }
    let mut rows = Vec::new();
    for &(origin, tip, width, color) in sections {
        let axis = (tip - origin) / length;
        let view = camera.forward();
        let projected = axis - view * axis.dot(view);
        let side = if projected.length_squared() > 0.01 {
            projected.normalize()
        } else {
            projected * 10.0
        } * width;
        let c = color.map(gs_unit);
        let clear = [c[0], c[1], c[2], 0.0];
        rows.push([
            vtx(origin - side, [0.0, 0.0], clear),
            vtx(origin, [0.0, 0.0], c),
            vtx(tip, [0.0, 0.0], c),
            vtx(tip + side, [0.0, 0.0], clear),
        ]);
    }
    let mut out = Vec::new();
    for pair in rows.windows(2) {
        for band in 0..3 {
            let (a, b, c, d) = (
                pair[0][band],
                pair[0][band + 1],
                pair[1][band],
                pair[1][band + 1],
            );
            out.extend([a, b, c, c, b, d]);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_blade_sweep_keeps_its_width_fringe_and_temporal_alpha() {
        let rows = [
            (
                Vec3::new(0.0, 0.0, -5.0),
                Vec3::new(0.0, 1.0, -5.0),
                0.08,
                [255, 0, 0, 0],
            ),
            (
                Vec3::new(1.0, 0.0, -5.0),
                Vec3::new(1.0, 1.0, -5.0),
                0.08,
                [255, 0, 0, 255],
            ),
        ];
        let v = vertices(&rows, 1.0, &Camera::default());
        assert_eq!(v.len(), 18, "root fringe, solid sweep, tip fringe");
        assert!(v.iter().any(|v| (v.position[1] + 0.08).abs() < 1e-6));
        assert!(v.iter().any(|v| (v.position[1] - 1.08).abs() < 1e-6));
        assert!(v.iter().any(|v| v.color == [1.0, 0.0, 0.0, 1.0]));
        assert!(v
            .iter()
            .filter(|v| v.position[1] < 0.0 || v.position[1] > 1.0)
            .all(|v| v.color[3] == 0.0));
    }

    #[test]
    fn an_untextured_trail_reaches_the_additive_frame_submission() {
        let draw = ParticleDraw {
            shape: ParticleShape::Trail {
                length: 1.0,
                sections: vec![
                    (
                        Vec3::new(0.0, 0.0, -5.0),
                        Vec3::new(0.0, 1.0, -5.0),
                        0.08,
                        [255, 0, 0, 0],
                    ),
                    (
                        Vec3::new(1.0, 0.0, -5.0),
                        Vec3::new(1.0, 1.0, -5.0),
                        0.08,
                        [255, 0, 0, 255],
                    ),
                ],
            },
            key: Vec3::new(0.0, 0.0, -5.0),
            color: [255; 4],
            texture: None,
            additive: true,
        };
        let frame = build(&[], &[draw], &Camera::default());
        assert_eq!(frame.vertices.len(), 18);
        assert_eq!(frame.runs.len(), 1);
        assert_eq!((frame.runs[0].texture, frame.runs[0].glow), (None, true));
    }

    #[test]
    fn projected_fringe_preserves_the_raw_hermite_direction_magnitude() {
        let origin = Vec3::new(0.0, 0.0, -5.0);
        let rows = [
            (origin, origin + Vec3::X * 0.05, 0.08, [255; 4]),
            (
                origin + Vec3::Y,
                origin + Vec3::Y + Vec3::X * 0.05,
                0.08,
                [255; 4],
            ),
        ];
        let v = vertices(&rows, 1.0, &Camera::default());
        // Raw projected length²=.0025, so the native low-magnitude branch
        // multiplies by10 instead of normalising the Hermite curve first.
        assert!((v[0].position[0] + 0.04).abs() < 1e-6);
    }
}
