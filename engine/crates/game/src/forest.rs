//! Forest presentation setup and read-only actor footprints.
use core::sim::Sim;
use glam::Vec3;
use render::{LeafActor, Renderer};

pub fn submit_actors(renderer: &mut Renderer, sim: &Sim) {
    renderer.set_leaf_time(Some(sim.tick as f64 / f64::from(core::TICK_HZ)));
    let mut actors = Vec::new();
    for (index, unit) in sim.units.iter().enumerate() {
        if !unit.alive || unit.mount.is_some() {
            continue;
        }
        actors.push(LeafActor {
            id: (u64::from(unit.gen) << 32) | index as u64,
            pos: unit.soldier.pos,
            velocity: unit.soldier.vel,
            radius: core::unit::SOLDIER_RADIUS,
            height: core::unit::capsule_height(unit.soldier.stance),
        });
    }
    for (index, vehicle) in sim.vehicles.iter().enumerate() {
        if !vehicle.alive {
            continue;
        }
        let Some(setup) = sim
            .seat_setup
            .get(vehicle.class as usize)
            .and_then(Option::as_ref)
        else {
            continue;
        };
        actors.push(LeafActor {
            id: (1 << 63) | (u64::from(vehicle.gen) << 32) | index as u64,
            pos: vehicle.pos - Vec3::Y * setup.radius,
            velocity: vehicle.vel,
            radius: setup.radius,
            height: setup.radius * 2.0,
        });
    }
    renderer.set_leaf_actors(actors);
}

pub fn configure(
    renderer: &mut Renderer,
    scene: &crate::assets::Scene,
    mission: Option<&crate::mission::MissionDef>,
) -> anyhow::Result<()> {
    let mut types = Vec::new();
    if scene
        .manifest
        .leaf
        .iter()
        .any(|leaf| leaf.falling_count > 0)
    {
        if let Some(mission) = mission {
            for (size, name) in mission.birds.iter().take(3) {
                let texture = scene
                    .textures
                    .get(&name.to_ascii_lowercase())
                    .ok_or_else(|| anyhow::anyhow!("forest bird texture {name} is absent"))?;
                types.push(render::ForestBirdType {
                    size: *size,
                    texture: crate::viewer::upload_cpu_texture(renderer, texture, name)?,
                });
            }
        }
    }
    renderer.set_forest_birds(
        types,
        if scene.manifest.source_game == "bf1" {
            30.0
        } else {
            20.0
        },
    );
    Ok(())
}

pub fn configure_default(
    renderer: &mut Renderer,
    scene: &crate::assets::Scene,
) -> anyhow::Result<()> {
    if !scene
        .manifest
        .leaf
        .iter()
        .any(|leaf| leaf.falling_count > 0)
    {
        renderer.set_forest_birds(Vec::new(), 30.0);
        return Ok(());
    }
    let mission = scene
        .dir
        .parent()
        .and_then(std::path::Path::parent)
        .zip(scene.dir.file_name())
        .map(|(root, key)| {
            root.join("missions")
                .join(format!("{}_con.ron", key.to_string_lossy()))
        })
        .filter(|path| pak::is_file(path))
        .map(|path| crate::mission::MissionDef::load(&path))
        .transpose()?;
    configure(renderer, scene, mission.as_ref())
}
