#[path = "support/walker_battle.rs"]
mod walker_battle;
use anyhow::{anyhow, Context, Result};
use core::sim::{buttons, PadState};
use core::unit::{Controller, UnitSpec};
use core::vehicle::VehicleHandle;
use render::Renderer;
use std::path::PathBuf;

fn main() -> Result<()> {
    let out = PathBuf::from(
        std::env::args()
            .nth(1)
            .context("walker_controls <out_dir>")?,
    );
    std::fs::create_dir_all(&out)?;
    let root = game::assets::baked_root();
    let (mut walk, scene) = walker_battle::battle(&root)?;
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
    let h = VehicleHandle {
        index: slot as u32,
        gen: v.gen,
    };
    let pos = v.pos;
    anyhow::ensure!(walk.sim.seat_unit(walk.player, h, 0), "pilot");
    for seat in [1, 2] {
        let bot = walk
            .sim
            .spawn(&UnitSpec {
                controller: Controller::Ai,
                ..UnitSpec::trooper(0, pos)
            })
            .context("gunner")?;
        anyhow::ensure!(walk.sim.seat_unit(bot, h, seat), "gunner seat");
    }
    for (i, v) in walk.sim.vehicles.iter_mut().enumerate() {
        v.alive = i == slot;
    }
    let mut renderer = Renderer::new_headless(960, 720).map_err(|e| anyhow!("{e}"))?;
    let gpu = game::viewer::upload_scene(&mut renderer, &scene)?;
    let mut vehicles =
        game::viewer::VehicleVisuals::load(&mut renderer, &scene, &root, &walk.vehicle_geometry);
    let mut soldiers =
        game::soldier_vis::SoldierVisuals::for_walk(&mut renderer, &root, &scene, &walk);
    for (label, ticks, mouse, switch) in [
        ("pilot_front", 0, 0.0, false),
        ("pilot_look_115", 1, 2.0, false),
        ("pilot_body_follow", 180, 0.0, false),
        ("main_gun", 2, 0.0, true),
        ("rear_guns", 60, 0.0, true),
    ] {
        for tick in 0..ticks {
            let pad = if tick == 0 {
                PadState {
                    mx: PadState::quantize_look(mouse),
                    buttons: if switch {
                        buttons::SWITCH_WEAPON
                    } else {
                        buttons::MOUSE_AIM
                    },
                    ..Default::default()
                }
            } else {
                PadState::default()
            };
            walk.step_tick(&[pad]);
            soldiers.update(&walk, core::TICK_DT);
        }
        let camera = walk.camera();
        let fades = walk.camera_see_through();
        let mut draws = gpu.view_draws(&walk.sim, walk.player);
        draws.extend(vehicles.draws(&mut renderer, &walk.sim, &fades.vehicles));
        let skinned = soldiers.draws(&walk);
        renderer
            .render_frame(&camera, &draws, &skinned, gpu.clear, None)
            .map_err(|e| anyhow!("{e}"))?;
        let (rgba, width, height) = renderer.capture_rgba().map_err(|e| anyhow!("{e}"))?;
        std::fs::write(
            out.join(format!("{label}.png")),
            render::encode_png(&rgba, width, height)?,
        )?;
        let v = walk.sim.vehicle(h).context("vehicle")?;
        let look = walk.sim.seat_look(walk.player).context("seat look")?;
        println!(
            "{label}: seat={} body={:?} view={:?} aim={:?} camera={camera:?} riders={:?}",
            walk.sim.unit(walk.player).unwrap().mount.unwrap().seat,
            v.rot,
            look.look,
            look.aim,
            v.seats.iter().map(|s| s.occupant).collect::<Vec<_>>()
        );
    }
    Ok(())
}
