use anyhow::{anyhow, Context, Result};
use core::droideka::Order;
use core::sim::{buttons, PadState, Sim, SpawnStage};
use core::unit::Controller;
use core::weapon::ExplosionClass;
use game::walk::{ClassVisual, TeamTemplate, WalkSim};
use glam::Vec3;
use render::Renderer;
use std::path::{Path, PathBuf};

fn battle(
    root: &Path,
    map: &str,
    mission: &str,
    game_id: &str,
    deka: bool,
) -> Result<(WalkSim, game::assets::Scene)> {
    let (scene, _) = game::assets::load_battle_scene(&root.join(map), "con")?;
    let mission = game::mission::MissionDef::load(
        &root
            .parent()
            .context("assets root")?
            .join("missions")
            .join(mission),
    )?;
    let (mut rosters, _) = game::roster::FactionRoster::pair(
        &root.join("classes"),
        game_id,
        [
            mission.teams[0].faction.as_str(),
            mission.teams[1].faction.as_str(),
        ],
        Some(&mission),
    )?;
    game::droideka::install(&mut rosters, &scene.dir, game_id);
    let mut sim = Sim::empty(42);
    sim.ground_rules = core::soldier::GroundRules::for_game(game_id);
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
    walk.camera_blockers = game::camera_collide::CameraBlockers::from_scene(&scene);
    let mut layout = game::layout::MapLayout::from_manifest(&scene.manifest);
    walk.sim.death_regions = layout.apply_mission(&mission).death_regions;
    walk.setup_conquest(&layout, [0, 0], true);
    walk.sim.open_spawn_screens(0, false);
    walk.sim.set_spawn_points(0, 12);
    if deka {
        let class = walk
            .sim
            .classes
            .iter()
            .find_map(|t| t.iter().position(|c| c.droideka.is_some()))
            .context("no droideka class")?;
        walk.sim.spawn_choice[0] = class as u8;
    }
    for _ in 0..2 {
        walk.step_tick(&[PadState {
            buttons: buttons::SPAWN_ACCEPT,
            ..Default::default()
        }]);
        walk.step_tick(&[PadState::default()]);
    }
    for _ in 0..60 {
        walk.step_tick(&[PadState::default()]);
    }
    anyhow::ensure!(walk.sim.spawn_stage[0] == SpawnStage::Accepted, "no spawn");
    Ok((walk, scene))
}

struct Shot {
    name: &'static str,
    ticks: u32,
    order: Order,
    shield: bool,
    roll: bool,
}

fn run(root: &Path, out: &Path, map: &str, mission: &str, game_id: &str) -> Result<()> {
    let (mut walk, scene) = battle(root, map, mission, game_id, false)?;
    let mut r = Renderer::new_headless(960, 540).map_err(|e| anyhow!("renderer: {e}"))?;
    let mut gpu = game::viewer::upload_scene(&mut r, &scene)?;
    let mut vis = game::soldier_vis::SoldierVisuals::for_walk(&mut r, root, &scene, &walk);
    let mut deka_vis = game::viewer::DroidekaVisuals::load(&mut r, &scene, root, &walk);
    let mut fx = game::fx::Fx::for_walk(root, &scene.dir, game_id, &walk);
    let cis = walk
        .teams
        .iter()
        .position(|t| t.faction == "cis")
        .context("no CIS team")?;
    let class = walk.sim.classes[cis]
        .iter()
        .position(|c| c.droideka.is_some())
        .context("no droideka class on the CIS roster")?;
    let me = walk.soldier().clone();
    let fwd = Vec3::new(-me.yaw.sin(), 0.0, -me.yaw.cos());
    let at = me.pos + fwd * 7.0;
    let spec = walk.sim.classes[cis][class].spec(
        cis as u8,
        at,
        me.yaw + std::f32::consts::FRAC_PI_2,
        Controller::Idle,
        class as u8,
        0,
    );
    let h = walk.sim.spawn(&spec).context("spawn the droideka")?;
    let i = h.index as usize;
    let shots = [
        Shot {
            name: "deployed",
            ticks: 30,
            order: Order::Deploy,
            shield: false,
            roll: false,
        },
        Shot {
            name: "shield",
            ticks: 10,
            order: Order::Deploy,
            shield: true,
            roll: false,
        },
        Shot {
            name: "folding",
            ticks: 8,
            order: Order::Roll,
            shield: false,
            roll: false,
        },
        Shot {
            name: "folded",
            ticks: 12,
            order: Order::Roll,
            shield: false,
            roll: false,
        },
        Shot {
            name: "rolling_a",
            ticks: 6,
            order: Order::Roll,
            shield: false,
            roll: true,
        },
        Shot {
            name: "rolling_b",
            ticks: 3,
            order: Order::Roll,
            shield: false,
            roll: true,
        },
        Shot {
            name: "unfolding",
            ticks: 25,
            order: Order::Deploy,
            shield: false,
            roll: false,
        },
        Shot {
            name: "unfolded",
            ticks: 40,
            order: Order::Deploy,
            shield: false,
            roll: false,
        },
    ];
    for s in &shots {
        for _ in 0..s.ticks {
            let cap = walk.sim.units[i]
                .droideka_class()
                .map_or(0.0, |c| c.ball_omega_cap());
            if let Some(d) = walk.sim.units[i].droideka.as_mut() {
                d.order = s.order;
                d.shield.on = s.shield;
                if s.roll {
                    d.omega = cap;
                }
            }
            walk.step_tick(&[PadState::default()]);
            if !s.roll {
                walk.sim.units[i].soldier.pos.x = at.x;
                walk.sim.units[i].soldier.pos.z = at.z;
            }
            vis.update(&walk, core::TICK_DT);
            fx.ingest(&walk);
            fx.update(core::TICK_DT);
            deka_vis.bubbles(&walk.sim, walk.camera().position);
        }
        frame(
            &mut r,
            &mut gpu,
            &mut vis,
            &mut deka_vis,
            &mut fx,
            &walk,
            out,
            game_id,
            s.name,
        )?;
        let u = &walk.sim.units[i];
        let d = u.droideka.as_ref().context("droideka")?;
        println!(
            "{game_id} {}: state {} clip {:.2} shield {} pos {:.2} spin {:?} lean {:.2}",
            s.name, d.state, d.clip_t, d.shield.on, u.soldier.pos, d.spin, d.lean
        );
    }
    let pos = walk.sim.units[i].soldier.pos;
    let kill = ExplosionClass {
        damage: 20000.0,
        damage_outer: 0.5,
        ..Default::default()
    };
    if let Some(d) = walk.sim.units[i].droideka.as_mut() {
        d.shield.on = false;
    }
    walk.sim.explode(&kill, pos + Vec3::Y, None, Some(0));
    for t in 0..12 {
        walk.step_tick(&[PadState::default()]);
        fx.ingest(&walk);
        fx.update(core::TICK_DT);
        vis.update(&walk, core::TICK_DT);
        if t == 3 || t == 11 {
            let name = if t == 3 { "burst_a" } else { "burst_b" };
            frame(
                &mut r,
                &mut gpu,
                &mut vis,
                &mut deka_vis,
                &mut fx,
                &walk,
                out,
                game_id,
                name,
            )?;
            println!(
                "{game_id} {name}: alive {} burst {} body {}",
                walk.sim.units[i].alive, walk.sim.units[i].burst, walk.sim.units[i].body
            );
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn frame(
    r: &mut Renderer,
    gpu: &mut game::viewer::SceneGpu,
    vis: &mut game::soldier_vis::SoldierVisuals,
    deka_vis: &mut game::viewer::DroidekaVisuals,
    fx: &mut game::fx::Fx,
    walk: &WalkSim,
    out: &Path,
    game_id: &str,
    name: &str,
) -> Result<()> {
    let cam = walk.camera();
    r.time = (walk.sim.tick as f32 * core::TICK_DT) % render::TIME_WRAP;
    let mut draws = gpu.view_draws(&walk.sim, walk.player);
    draws.extend(deka_vis.draws(r, &walk.sim));
    let skinned = vis.draws(walk);
    draws.extend(vis.weapon_draws(walk));
    let mut particles = vis.flash_draws(walk);
    let bubbles = deka_vis.bubbles(&walk.sim, cam.position);
    if let Some(b) = bubbles.first() {
        println!(
            "{game_id} {name}: bubble colour {:?} key {:?}",
            b.color, b.key
        );
    }
    particles.extend(bubbles);
    fx.submit_with(r, &cam, particles);
    gpu.animate_water_live(r, cam.position)?;
    r.render_frame(&cam, &draws, &skinned, gpu.clear, None)
        .map_err(|e| anyhow!("render: {e}"))?;
    let (rgba, w, hgt) = r.capture_rgba().map_err(|e| anyhow!("capture: {e}"))?;
    let png = render::encode_png(&rgba, w, hgt).map_err(|e| anyhow!("png: {e}"))?;
    std::fs::write(out.join(format!("{game_id}_{name}.png")), png)?;
    Ok(())
}

fn player(root: &Path, out: &Path, map: &str, mission: &str, game_id: &str) -> Result<()> {
    let (mut walk, scene) = battle(root, map, mission, game_id, true)?;
    let me = walk.player.index as usize;
    anyhow::ensure!(
        walk.sim.units[me].droideka.is_some(),
        "the player did not spawn as a droideka"
    );
    let mut r = Renderer::new_headless(960, 540).map_err(|e| anyhow!("renderer: {e}"))?;
    let mut gpu = game::viewer::upload_scene(&mut r, &scene)?;
    let mut vis = game::soldier_vis::SoldierVisuals::for_walk(&mut r, root, &scene, &walk);
    let mut deka_vis = game::viewer::DroidekaVisuals::load(&mut r, &scene, root, &walk);
    let mut fx = game::fx::Fx::for_walk(root, &scene.dir, game_id, &walk);
    let pad = |b: u32, ly: f32| PadState {
        buttons: b,
        ly: PadState::quantize(ly),
        ..Default::default()
    };
    let steps: [(&str, u32, PadState, Option<PadState>); 6] = [
        ("player_walk", 40, pad(buttons::FIRE, 1.0), None),
        (
            "player_shield",
            15,
            pad(0, 0.0),
            Some(pad(buttons::SECONDARY_FIRE, 0.0)),
        ),
        (
            "player_fold",
            10,
            pad(0, 0.0),
            Some(pad(buttons::ROLL_STANCE, 0.0)),
        ),
        ("player_ball", 20, pad(0, 0.0), None),
        ("player_roll", 30, pad(0, 1.0), None),
        (
            "player_unfold",
            60,
            pad(0, 0.0),
            Some(pad(buttons::ROLL_STANCE, 0.0)),
        ),
    ];
    for (name, ticks, hold, first) in steps {
        for t in 0..ticks {
            let p = if t == 0 { first.unwrap_or(hold) } else { hold };
            walk.step_tick(&[p]);
            vis.update(&walk, core::TICK_DT);
            fx.ingest(&walk);
            fx.update(core::TICK_DT);
            deka_vis.bubbles(&walk.sim, walk.camera().position);
        }
        frame(
            &mut r,
            &mut gpu,
            &mut vis,
            &mut deka_vis,
            &mut fx,
            &walk,
            out,
            game_id,
            name,
        )?;
        let u = &walk.sim.units[me];
        let d = u.droideka.as_ref().context("droideka")?;
        let cam = walk.camera();
        println!(
            "{game_id} {name}: state {} shield {} pos {:.2} cam {:.2} eye {:.2} section {:?}",
            d.state,
            d.shield.on,
            u.soldier.pos,
            cam.position,
            cam.position - u.soldier.pos,
            u.section()
        );
    }
    Ok(())
}

fn main() -> Result<()> {
    let out = PathBuf::from(std::env::args().nth(1).context("droideka_look <out_dir>")?);
    std::fs::create_dir_all(&out)?;
    let root = game::assets::baked_root();
    run(&root, &out, "bf2_kam1", "bf2_kam1_con_cw.ron", "bf2")?;
    run(&root, &out, "bf1_nab1", "bf1_nab1_con_cw.ron", "bf1")?;
    player(&root, &out, "bf2_kam1", "bf2_kam1_con_cw.ron", "bf2")?;
    player(&root, &out, "bf1_nab1", "bf1_nab1_con_cw.ron", "bf1")
}
