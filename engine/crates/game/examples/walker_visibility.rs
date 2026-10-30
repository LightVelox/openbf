#[path = "support/walker_battle.rs"]
mod walker_battle;
use anyhow::{anyhow, Context, Result};
use glam::Vec3;
use render::{Camera, Renderer};
use std::path::PathBuf;
use walker_battle::battle;

fn main() -> Result<()> {
    let out = PathBuf::from(
        std::env::args()
            .nth(1)
            .context("walker_visibility <out_dir>")?,
    );
    std::fs::create_dir_all(&out)?;
    let root = game::assets::baked_root();
    let (mut walk, scene) = battle(&root)?;
    let class = walk
        .vehicle_classes
        .iter()
        .position(|c| c == "rep_walk_atte")
        .context("AT-TE class")?;
    let (slot, v) = walk
        .sim
        .vehicles
        .iter()
        .enumerate()
        .find(|(_, v)| v.alive && v.class as usize == class)
        .context("AT-TE")?;
    let pos = v.pos;
    let h = core::vehicle::VehicleHandle {
        index: slot as u32,
        gen: v.gen,
    };
    let mut near = core::collision::Aabb {
        min: Vec3::splat(f32::INFINITY),
        max: Vec3::splat(f32::NEG_INFINITY),
    };
    for b in walk.sim.collision.hull(h) {
        near.min = near.min.min(b.aabb.min);
        near.max = near.max.max(b.aabb.max);
    }
    println!("AT-TE at {pos:?}; bounds {near:?}");
    for (i, v) in walk.sim.vehicles.iter_mut().enumerate() {
        v.alive = i == slot;
    }
    let mut renderer = Renderer::new_headless(960, 720).map_err(|e| anyhow!("{e}"))?;
    let gpu = game::viewer::upload_scene(&mut renderer, &scene)?;
    let mut vehicles =
        game::viewer::VehicleVisuals::load(&mut renderer, &scene, &root, &walk.vehicle_geometry);
    for (label, offset) in [
        ("belly", Vec3::new(0.0, 1.8, 3.0)),
        ("between_legs", Vec3::new(-4.0, 1.8, 0.0)),
        ("approach", Vec3::new(-7.0, 1.8, 0.0)),
    ] {
        let eye = pos + offset;
        let sights = [eye + Vec3::new(0.0, -0.3, -2.0)];
        let fixed = game::camera_collide::see_through_vehicles(
            &walk.sim.collision,
            std::iter::once(h),
            eye,
            &sights,
            None,
        );
        let old = if eye
            .cmpge(near.min - Vec3::splat(game::camera_collide::CAMERA_RADIUS))
            .all()
            && eye
                .cmple(near.max + Vec3::splat(game::camera_collide::CAMERA_RADIUS))
                .all()
        {
            vec![(h, 0.0)]
        } else {
            fixed.clone()
        };
        println!("{label}: old={old:?} fixed={fixed:?}");
        let dir = (pos + Vec3::Y * 4.0 - eye).normalize();
        let cam = Camera {
            roll: 0.0,
            position: eye,
            yaw: (-dir.x).atan2(-dir.z),
            pitch: dir.y.asin(),
            fov_y: 70.0f32.to_radians(),
            near: 0.1,
            far: 2000.0,
        };
        for (version, fades) in [("before", old), ("after", fixed)] {
            let mut draws = gpu.view_draws(&walk.sim, walk.player);
            if version != "before" || !fades.iter().any(|(_, opacity)| *opacity == 0.0) {
                draws.extend(vehicles.draws(&mut renderer, &walk.sim, &fades));
            }
            renderer
                .render_frame(&cam, &draws, &[], gpu.clear, None)
                .map_err(|e| anyhow!("{e}"))?;
            let (rgba, w, h) = renderer.capture_rgba().map_err(|e| anyhow!("{e}"))?;
            std::fs::write(
                out.join(format!("{label}_{version}.png")),
                render::encode_png(&rgba, w, h)?,
            )?;
        }
    }
    Ok(())
}
