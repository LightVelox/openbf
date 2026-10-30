use anyhow::{Context, Result};
use game::{assets, viewer};
use glam::Vec3;
use render::{Camera, Renderer};
use std::path::PathBuf;

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    let map = PathBuf::from(args.get(1).context("map directory")?);
    let output = PathBuf::from(args.get(2).context("output directory")?);
    std::fs::create_dir_all(&output)?;
    let (scene, _) = assets::load_battle_scene(&map, "con")?;
    let mut renderer = Renderer::new_headless(960, 540)?;
    let gpu = viewer::upload_scene(&mut renderer, &scene)?;
    let ground = scene.heightfield();
    let objects = viewer::leaf::objects(
        &scene.manifest,
        &viewer::foliage::records(&scene),
        ground.as_ref(),
    );
    let mut camera = if let Some(object) = objects
        .iter()
        .find(|o| scene.manifest.leaf[o.class as usize].falling_count > 0)
    {
        let target = object.matrix.w_axis.truncate();
        let height = ground
            .as_ref()
            .map_or(target.y, |g| g.height_at(target.x, target.z));
        Camera {
            position: Vec3::new(target.x + 4.0, height + 2.0, target.z + 7.0),
            yaw: 20f32.to_radians(),
            pitch: -12f32.to_radians(),
            ..Default::default()
        }
    } else {
        viewer::camera_from(viewer::GOLDEN_CAM)
    };
    if let Some(pose) = args.get(3) {
        let p: Vec<f32> = pose
            .split(',')
            .map(str::parse)
            .collect::<std::result::Result<_, _>>()?;
        anyhow::ensure!(p.len() == 5, "camera requires x,y,z,yaw,pitch");
        camera = viewer::camera_from((p[0], p[1], p[2], p[3], p[4]));
    }
    for step in 0..=920 {
        renderer.set_leaf_time(Some(f64::from(step) / 30.0));
        renderer.render(&camera, &[], gpu.clear)?;
        if step == 620 {
            renderer.render(&camera, &gpu.draws, gpu.clear)?;
            let (rgba, width, height) = renderer.capture_rgba()?;
            image::save_buffer(
                output.join("at-20-seconds.png"),
                &rgba,
                width,
                height,
                image::ColorType::Rgba8,
            )?;
        }
    }
    renderer.render(&camera, &gpu.draws, gpu.clear)?;
    let (after, width, height) = renderer.capture_rgba()?;
    image::save_buffer(
        output.join("after.png"),
        &after,
        width,
        height,
        image::ColorType::Rgba8,
    )?;
    renderer.set_leaf_fall(Vec::new(), None, None);
    renderer.render(&camera, &gpu.draws, gpu.clear)?;
    let (before, _, _) = renderer.capture_rgba()?;
    image::save_buffer(
        output.join("before.png"),
        &before,
        width,
        height,
        image::ColorType::Rgba8,
    )?;
    let changed = before
        .chunks_exact(4)
        .zip(after.chunks_exact(4))
        .filter(|(a, b)| a != b)
        .count();
    println!(
        "{}: {changed} changed pixels, camera {:?}",
        map.display(),
        camera
    );
    if !scene
        .manifest
        .leaf
        .iter()
        .any(|leaf| leaf.falling_count > 0)
    {
        anyhow::ensure!(before == after, "non-emitting map changed");
    }
    Ok(())
}
