//! A BF2 vehicle's destruction drawn headless (issue #49, `analysis/re-chunks.md`): on GEO1 the vehicle pads are
//! installed as a battle installs them, the first live vehicle of the named class is destroyed where it stands
//! (`Sim::destroy_vehicle`), and its chunks fly as rigid bodies with their trails, terrain bursts and smoke.
//!
//!   vehicle_chunks <out_dir> [class] [distance_m]
//!
//! Frames: `<out_dir>/<class>_<tick>.png`, ticks after the death, from a fixed camera `distance_m` (default 30)
//! off the vehicle's side. Each frame prints the live chunks.
use anyhow::{anyhow, Context, Result};
use core::sim::{buttons, PadState, Sim, SpawnStage};
use core::vehicle::VehicleHandle;
use game::walk::{ClassVisual, TeamTemplate, WalkSim};
use glam::Vec3;
use render::{Camera, Renderer};
use std::path::{Path, PathBuf};

const TICKS: [u64; 8] = [0, 3, 8, 15, 30, 60, 120, 200];

fn battle(root: &Path) -> Result<(WalkSim, game::assets::Scene)> {
    let map = root.join("bf2_geo1");
    let (scene, _) = game::assets::load_battle_scene(&map, "con")?;
    let mission = game::mission::MissionDef::load(
        &root
            .parent()
            .context("assets root")?
            .join("missions/bf2_geo1_con.ron"),
    )?;
    let (rosters, _) = game::roster::FactionRoster::pair(
        &root.join("classes"),
        "bf2",
        [
            mission.teams[0].faction.as_str(),
            mission.teams[1].faction.as_str(),
        ],
        Some(&mission),
    )?;
    let mut sim = Sim::empty(42);
    sim.ground_rules = core::soldier::GroundRules::Bf2;
    for (team, definition) in mission.teams.iter().enumerate() {
        sim.battle.teams[team] = core::battle::TeamState::new(definition.tickets);
    }
    let mut names = Vec::new();
    let mut team = 0;
    let templates = rosters.map(|roster| {
        let table: Vec<_> = roster
            .classes
            .iter()
            .map(|class| class.register(&mut sim, &mut names))
            .collect();
        let visuals = roster
            .classes
            .iter()
            .map(ClassVisual::from_roster)
            .collect();
        let template = TeamTemplate::from_roster(&roster, &table[0], visuals);
        sim.classes[team] = table;
        team += 1;
        template
    });
    let nav = game::layout::nav_graph(&scene.manifest, &scene.dir);
    let mut walk = WalkSim::with_teams(sim, scene.heightfield(), Vec3::ZERO, templates, nav);
    scene.install_collision(&mut walk.sim);
    let mut layout = game::layout::MapLayout::from_manifest(&scene.manifest);
    walk.sim.death_regions = layout.apply_mission(&mission).death_regions;
    walk.setup_conquest(&layout, [0, 0], true);
    let factions = [walk.teams[0].faction.clone(), walk.teams[1].faction.clone()];
    let pads = game::vehicle_pads::install(
        &mut walk.sim,
        &scene.manifest,
        &scene.dir,
        &root.join("classes"),
        &factions,
    )?;
    walk.vehicle_geometry = pads.geometry;
    walk.vehicle_seats = pads.seats;
    walk.vehicle_riders = pads.riders;
    walk.vehicle_engines = pads.engines;
    walk.vehicle_labels = pads.labels;
    walk.vehicle_classes = pads.classes;
    walk.sim.open_spawn_screens(0, false);
    walk.sim.set_spawn_points(0, 12);
    for _ in 0..2 {
        walk.step_tick(&[PadState {
            buttons: buttons::SPAWN_ACCEPT,
            ..Default::default()
        }]);
        walk.step_tick(&[PadState::default()]);
    }
    for _ in 0..90 {
        walk.step_tick(&[PadState::default()]);
    }
    anyhow::ensure!(walk.sim.spawn_stage[0] == SpawnStage::Accepted, "no spawn");
    Ok((walk, scene))
}

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let out = PathBuf::from(
        args.next()
            .context("vehicle_chunks <out_dir> [class] [distance_m]")?,
    );
    let class = args.next().unwrap_or_else(|| "cis_tread_hailfire".into());
    let distance: f32 = args.next().map_or(Ok(30.0), |d| d.parse())?;
    std::fs::create_dir_all(&out)?;
    let root = game::assets::baked_root();
    let (mut walk, scene) = battle(&root)?;
    let index = walk
        .vehicle_classes
        .iter()
        .position(|c| c.eq_ignore_ascii_case(&class))
        .with_context(|| format!("no pad builds {class}: {:?}", walk.vehicle_classes))?;
    let (slot, v) = walk
        .sim
        .vehicles
        .iter()
        .enumerate()
        .find(|(_, v)| v.alive && v.class as usize == index)
        .with_context(|| format!("no live {class}"))?;
    let h = VehicleHandle {
        index: slot as u32,
        gen: v.gen,
    };
    let (pos, side) = (v.pos, v.rot * Vec3::X);
    let mut r = Renderer::new_headless(960, 540).map_err(|e| anyhow!("renderer: {e}"))?;
    let mut gpu = game::viewer::upload_scene(&mut r, &scene)?;
    let mut vehicles =
        game::viewer::VehicleVisuals::load(&mut r, &scene, &root, &walk.vehicle_geometry);
    let mut fx = game::fx::Fx::for_walk(&root, &scene.dir, "bf2", &walk);
    let eye = pos + side * distance + Vec3::Y * distance * 0.35;
    let look = (pos + Vec3::Y * 2.0 - eye).normalize();
    let cam = Camera {
        position: eye,
        yaw: (-look.x).atan2(-look.z),
        pitch: look.y.asin(),
        fov_y: 70f32.to_radians(),
        near: 0.5,
        far: 2000.0,
    };
    let ground = walk
        .sim
        .heightfield
        .as_ref()
        .map_or(f32::NAN, |hf| hf.height_at(pos.x, pos.z));
    println!("{class} at {pos:.2} over ground {ground:.2}, camera at {eye:.1}");
    walk.sim.destroy_vehicle(h, None);
    fx.ingest(&walk);
    let mut tick = 0;
    for t in TICKS {
        while tick < t {
            walk.step_tick(&[PadState::default()]);
            fx.ingest(&walk);
            fx.update(core::TICK_DT);
            tick += 1;
        }
        r.time = (walk.sim.tick as f32 * core::TICK_DT) % render::TIME_WRAP;
        let mut draws = gpu.view_draws(&walk.sim, walk.player);
        draws.extend(vehicles.draws(&mut r, &walk.sim, &[]));
        fx.submit(&mut r, &cam);
        gpu.animate_water_live(&mut r, cam.position)?;
        r.render_frame(&cam, &draws, &[], gpu.clear, None)
            .map_err(|e| anyhow!("render: {e}"))?;
        let (rgba, w, hgt) = r.capture_rgba().map_err(|e| anyhow!("capture: {e}"))?;
        let png = render::encode_png(&rgba, w, hgt).map_err(|e| anyhow!("png: {e}"))?;
        let name = format!("{class}_{t:03}.png");
        std::fs::write(out.join(&name), png)?;
        println!("{name}: {} live fx", fx.len());
    }
    Ok(())
}
