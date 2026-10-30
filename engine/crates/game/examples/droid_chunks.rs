//! Offscreen paired droid deaths: `droid_chunks <outdir> <bf1|bf2> <class>`.
//! Uses the normal class, death event, soldier and chunk render paths.
use anyhow::{anyhow, Context, Result};
use core::unit::UnitSpec;
use game::walk::{ClassVisual, TeamTemplate, WalkSim};
use glam::Vec3;
use render::{Camera, Renderer};
use std::path::PathBuf;

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let out = PathBuf::from(
        args.next()
            .context("droid_chunks <outdir> <game> <class>")?,
    );
    let game = args.next().context("game")?;
    let name = args.next().context("class")?;
    let root = game::assets::baked_root();
    let map = root.join(format!("{game}_geo1"));
    let (scene, _) = game::assets::load_battle_scene(&map, "con")?;
    let db = game::roster::load_classdb(&root.join("classes"), &game, &["rep", "cis"])?;
    let roster = game::roster::ClassRoster::build(&db, &name, 0, None)?;
    let mut sim = core::sim::Sim::empty(7);
    sim.ground_rules = if game == "bf1" {
        core::soldier::GroundRules::Bf1
    } else {
        core::soldier::GroundRules::Bf2
    };
    let class = roster.register(&mut sim, &mut Vec::new());
    sim.classes[1] = vec![class.clone()];
    let mut team = TeamTemplate::from_roster(
        &game::roster::FactionRoster {
            faction: "cis".into(),
            classes: vec![roster.clone()],
        },
        &class,
        vec![ClassVisual::from_roster(&roster)],
    );
    team.faction = "cis".into();
    let pos = Vec3::new(
        0.0,
        scene
            .heightfield()
            .as_ref()
            .map_or(0.0, |h| h.height_at(0.0, 0.0)),
        0.0,
    );
    let mut walk = WalkSim::with_teams(sim, scene.heightfield(), pos, [team.clone(), team], None);
    scene.install_collision(&mut walk.sim);
    let spec = UnitSpec::trooper(1, pos);
    let unit = walk.player;
    walk.sim.units[unit.index as usize] = spec.build(unit.gen, &walk.sim.weapon_classes);
    let mut r = Renderer::new_headless(960, 720).map_err(|e| anyhow!("renderer: {e}"))?;
    let gpu = game::viewer::upload_scene(&mut r, &scene)?;
    let mut vis = game::soldier_vis::SoldierVisuals::for_walk(&mut r, &root, &scene, &walk);
    vis.update(&walk, 0.0);
    let mut fx = game::fx::Fx::for_walk(&root, &map, &game, &walk);
    let eye = pos + Vec3::new(3.4, 2.2, -5.2);
    let look = (pos + Vec3::Y * 1.0 - eye).normalize();
    let cam = Camera {
        position: eye,
        yaw: (-look.x).atan2(-look.z),
        pitch: look.y.asin(),
        fov_y: 48f32.to_radians(),
        near: 0.1,
        far: 2000.0,
        roll: 0.0,
    };
    std::fs::create_dir_all(&out)?;
    for t in [0, 1, 4, 10, 20, 40] {
        if t > 0 {
            if t == 1 {
                // A lethal hit's death-block inputs, forced burst to pair images.
                // The separate seeded referee checks the unforced authored chance.
                let u = &mut walk.sim.units[unit.index as usize];
                u.health.cur = -50.0;
                u.alive = false;
                u.body = true;
                u.chunk_mark = true;
            }
            while walk.sim.tick < t {
                walk.sim.step(&[]);
                fx.ingest_posed(&walk, Some(&vis));
                fx.update(core::TICK_DT);
                vis.update(&walk, core::TICK_DT);
            }
        }
        let draws = gpu.view_draws(&walk.sim, walk.player);
        fx.submit(&mut r, &cam);
        r.render_frame(&cam, &draws, &vis.draws(&walk), gpu.clear, None)
            .map_err(|e| anyhow!("render: {e}"))?;
        let (rgba, w, h) = r.capture_rgba().map_err(|e| anyhow!("capture: {e}"))?;
        std::fs::write(
            out.join(format!("{game}_{name}_{t:03}.png")),
            render::encode_png(&rgba, w, h).map_err(|e| anyhow!("png: {e}"))?,
        )?;
        println!(
            "{game} {name} tick {t}: burst={} fx={}",
            walk.sim.units[unit.index as usize].burst,
            fx.len()
        );
    }
    Ok(())
}
