#[path = "support/walker_battle.rs"]
mod walker_battle;
use anyhow::{Context, Result};
use core::sim::PadState;
use core::unit::{Controller, UnitSpec};
use core::vehicle::VehicleHandle;
fn main() -> Result<()> {
    let root = game::assets::baked_root();
    let (mut w, _) = walker_battle::battle(&root)?;
    let c = w
        .vehicle_classes
        .iter()
        .position(|c| c == "rep_walk_atte")
        .context("class")?;
    let (i, v) = w
        .sim
        .vehicles
        .iter()
        .enumerate()
        .find(|(_, v)| v.alive && v.class as usize == c)
        .context("vehicle")?;
    let h = VehicleHandle {
        index: i as u32,
        gen: v.gen,
    };
    let pos = v.pos;
    let bot = w
        .sim
        .spawn(&UnitSpec {
            controller: Controller::Ai,
            ..UnitSpec::trooper(0, pos)
        })
        .context("bot")?;
    anyhow::ensure!(w.sim.seat_unit(bot, h, 0), "pilot");
    anyhow::ensure!(w.sim.seat_unit(w.player, h, 2), "gunner");
    println!("tick,posx,posz,body_yaw,gun_yaw,cam_x,cam_yaw,aim_input,driver_input,anim");
    for _ in 0..600 {
        let mut b = std::mem::take(&mut w.blend);
        b.on = true;
        b.before_tick(&w);
        w.blend = b;
        w.step_tick(&[PadState::default()]);
        let mut b = std::mem::take(&mut w.blend);
        b.apply(&mut w, 1.0);
        w.blend = b;
        let v = w.sim.vehicle(h).context("alive")?;
        let g = v.guns[2].as_ref().unwrap();
        let f = v.rot * glam::Vec3::Z;
        let cam = w.camera();
        println!(
            "{},{:.5},{:.5},{:.6},{:.6},{:.5},{:.6},{:.5},{:.5},{}",
            w.sim.tick,
            v.pos.x,
            v.pos.z,
            f.x.atan2(f.z),
            g.look.0,
            cam.position.x,
            cam.yaw,
            v.input.turn,
            v.input.strafe,
            v.walker.anim
        );
        let mut b = std::mem::take(&mut w.blend);
        b.restore(&mut w);
        w.blend = b;
    }
    Ok(())
}
