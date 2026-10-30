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
            deka_vis.bubbles(&walk.sim, &walk.camera(), r.lighting.sun_dir);
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
    // Same camera and animation time: isolate deployed gun pitch in real content.
    for (name, pitch) in [("aim_level", 0.0), ("aim_up", 0.35), ("aim_down", -0.35)] {
        walk.sim.units[i].soldier.pitch = pitch;
        if let Some(d) = walk.sim.units[i].droideka.as_mut() {
            d.state = core::droideka::IDLE;
            d.clip_t = 0.37;
            d.shield.on = false;
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

fn bubble_list(
    deka_vis: &mut game::viewer::DroidekaVisuals,
    walk: &WalkSim,
    r: &Renderer,
    cam: &render::Camera,
) -> Vec<render::ParticleDraw> {
    deka_vis.bubbles(&walk.sim, cam, r.lighting.sun_dir)
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
    frame_at(r, gpu, vis, deka_vis, fx, walk, out, game_id, name, cam)
}

#[allow(clippy::too_many_arguments)]
fn frame_at(
    r: &mut Renderer,
    gpu: &mut game::viewer::SceneGpu,
    vis: &mut game::soldier_vis::SoldierVisuals,
    deka_vis: &mut game::viewer::DroidekaVisuals,
    fx: &mut game::fx::Fx,
    walk: &WalkSim,
    out: &Path,
    game_id: &str,
    name: &str,
    cam: render::Camera,
) -> Result<()> {
    r.time = (walk.sim.tick as f32 * core::TICK_DT) % render::TIME_WRAP;
    let mut draws = gpu.view_draws(&walk.sim, walk.player);
    draws.extend(deka_vis.draws(r, &walk.sim));
    let skinned = vis.draws(walk);
    draws.extend(vis.weapon_draws(walk));
    let mut points = vis.fire_points(walk);
    deka_vis.fire_points(&walk.sim, &mut points);
    let mut particles = vis.flash_draws_at(walk, &points);
    let bubbles = bubble_list(deka_vis, walk, r, &cam);
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

/// The shield's look from a fixed camera: outside and inside the bubble,
/// idle and under the player's fire, with the bubble list's CPU cost.
fn shield(root: &Path, out: &Path, map: &str, mission: &str, game_id: &str) -> Result<()> {
    let (mut walk, scene) = battle(root, map, mission, game_id, false)?;
    let mut r = Renderer::new_headless(960, 540).map_err(|e| anyhow!("renderer: {e}"))?;
    let mut gpu = game::viewer::upload_scene(&mut r, &scene)?;
    let mut vis = game::soldier_vis::SoldierVisuals::for_walk(&mut r, root, &scene, &walk);
    let mut deka_vis = game::viewer::DroidekaVisuals::load(&mut r, &scene, root, &walk);
    let mut fx = game::fx::Fx::for_walk(root, &scene.dir, game_id, &walk);
    let me = walk.player.index as usize;
    let cis = walk
        .teams
        .iter()
        .position(|t| t.faction == "cis")
        .context("no CIS team")?;
    let class = walk.sim.classes[cis]
        .iter()
        .position(|c| c.droideka.is_some())
        .context("no droideka class")?;
    let s = walk.soldier().clone();
    let fwd = Vec3::new(-s.yaw.sin(), 0.0, -s.yaw.cos());
    let at = s.pos + fwd * 9.0;
    // The droideka fights for its own side's model; the player opposes it.
    walk.sim.units[me].team = (1 - cis) as u8;
    let team = cis;
    let spec = walk.sim.classes[cis][class].spec(
        team as u8,
        at,
        s.yaw + std::f32::consts::PI,
        Controller::Idle,
        class as u8,
        0,
    );
    let i = walk.sim.spawn(&spec).context("spawn")?.index as usize;
    let settle = |walk: &mut WalkSim,
                  vis: &mut game::soldier_vis::SoldierVisuals,
                  fx: &mut game::fx::Fx,
                  pad: PadState,
                  ticks: u32| {
        for _ in 0..ticks {
            if let Some(d) = walk.sim.units[i].droideka.as_mut() {
                d.order = Order::Deploy;
                d.shield.on = true;
            }
            walk.sim.units[i].health.cur = walk.sim.units[i].health.max;
            walk.step_tick(&[pad]);
            vis.update(walk, core::TICK_DT);
            fx.ingest(walk);
            fx.update(core::TICK_DT);
        }
    };
    settle(&mut walk, &mut vis, &mut fx, PadState::default(), 60);
    let centre = walk.sim.units[i]
        .droideka_shield()
        .context("shield up")?
        .center;
    let side = fwd.cross(Vec3::Y).normalize_or_zero();
    let cameras = [
        (
            "outside",
            centre - fwd * 4.5 + side * 1.2 + Vec3::Y * 0.9,
            centre,
        ),
        ("outside_far", centre - fwd * 14.0 + Vec3::Y * 1.5, centre),
        (
            "inside",
            centre - fwd * 0.5 + Vec3::Y * 0.1,
            centre + fwd * 6.0,
        ),
    ];
    println!(
        "{game_id} team {} of {} class {class}, model draws {}",
        walk.sim.units[i].team,
        walk.teams[usize::from(walk.sim.units[i].team.min(1))].faction,
        deka_vis.draws(&mut r, &walk.sim).len()
    );
    // The frame with the shield down: the zero the bubble's add is read from.
    if let Some(d) = walk.sim.units[i].droideka.as_mut() {
        d.shield.on = false;
    }
    walk.step_tick(&[PadState::default()]);
    walk.step_tick(&[PadState::default()]);
    for _ in 0..12 {
        walk.step_tick(&[PadState::default()]);
        deka_vis.bubbles(
            &walk.sim,
            &game::viewer::look_at(cameras[0].1, cameras[0].2),
            r.lighting.sun_dir,
        );
    }
    for (name, eye, look) in cameras {
        let cam = game::viewer::look_at(eye, look);
        let tag = format!("shield_{name}_down");
        frame_at(
            &mut r,
            &mut gpu,
            &mut vis,
            &mut deka_vis,
            &mut fx,
            &walk,
            out,
            game_id,
            &tag,
            cam,
        )?;
    }
    settle(&mut walk, &mut vis, &mut fx, PadState::default(), 1);
    deka_vis.bubbles(
        &walk.sim,
        &game::viewer::look_at(cameras[0].1, cameras[0].2),
        r.lighting.sun_dir,
    );
    settle(&mut walk, &mut vis, &mut fx, PadState::default(), 30);
    for firing in [false, true] {
        let pad = PadState {
            buttons: if firing { buttons::FIRE } else { 0 },
            ..Default::default()
        };
        if firing {
            let d = centre - walk.sim.units[me].fire_point();
            walk.sim.units[me].soldier.yaw = (-d.x).atan2(-d.z);
            walk.sim.units[me].soldier.pitch = (d.y / (d.x * d.x + d.z * d.z).sqrt()).atan();
            settle(&mut walk, &mut vis, &mut fx, pad, 45);
        }
        for (name, eye, look) in cameras {
            let cam = game::viewer::look_at(eye, look);
            let tag = format!("shield_{name}_{}", if firing { "fire" } else { "idle" });
            frame_at(
                &mut r,
                &mut gpu,
                &mut vis,
                &mut deka_vis,
                &mut fx,
                &walk,
                out,
                game_id,
                &tag,
                cam,
            )?;
        }
        println!(
            "{game_id} firing {firing}: charge {:.0} health {:.0}",
            walk.sim.units[i]
                .droideka
                .as_ref()
                .map_or(0.0, |d| d.shield.points),
            walk.sim.units[i].health.cur
        );
    }
    // The draw's cost: eight shields (both teams' authored four) on and off,
    // the same frame, waited for by the readback.
    let mut more = Vec::new();
    for k in 0..7 {
        let a = k as f32 * 0.9;
        let at = at + Vec3::new(a.cos(), 0.0, a.sin()) * 3.5;
        let spec = walk.sim.classes[cis][class].spec(
            team as u8,
            at,
            0.0,
            Controller::Idle,
            class as u8,
            0,
        );
        more.extend(walk.sim.spawn(&spec).map(|h| h.index as usize));
    }
    let cam = game::viewer::look_at(cameras[0].1 - fwd * 3.0, cameras[0].2);
    for up in [false, true, false, true] {
        for &k in more.iter().chain([&i]) {
            if let Some(d) = walk.sim.units[k].droideka.as_mut() {
                d.order = Order::Deploy;
                d.shield.on = up;
            }
        }
        for _ in 0..14 {
            walk.step_tick(&[PadState::default()]);
            deka_vis.bubbles(&walk.sim, &cam, r.lighting.sun_dir);
        }
        let t = std::time::Instant::now();
        for _ in 0..30 {
            let draws = gpu.view_draws(&walk.sim, walk.player);
            let list = deka_vis.bubbles(&walk.sim, &cam, r.lighting.sun_dir);
            fx.submit_with(&mut r, &cam, list);
            r.render_frame(&cam, &draws, &[], gpu.clear, None)
                .map_err(|e| anyhow!("render: {e}"))?;
            r.capture_rgba().map_err(|e| anyhow!("capture: {e}"))?;
        }
        println!(
            "{game_id} {} shields {}: {:.2} ms a frame",
            more.len() + 1,
            if up { "up" } else { "down" },
            t.elapsed().as_secs_f64() * 1e3 / 30.0
        );
    }
    let cam = game::viewer::look_at(cameras[0].1, cameras[0].2);
    let t = std::time::Instant::now();
    let mut verts = 0;
    let n = 2000;
    for _ in 0..n {
        let list = bubble_list(&mut deka_vis, &walk, &r, &cam);
        verts = list
            .iter()
            .map(|p| match &p.shape {
                render::ParticleShape::Mesh { tris } => tris.len(),
                render::ParticleShape::Shaded { tris } => tris.len(),
                _ => 0,
            })
            .sum();
    }
    println!(
        "{game_id} eight shields: {verts} vertices, {:.1} us per frame's CPU build",
        t.elapsed().as_secs_f64() * 1e6 / f64::from(n)
    );
    Ok(())
}

/// The player's droideka under BF2's HUD, shield up and part spent; the bar
/// and rows as the battle's HUD model reads them.
fn hud(root: &Path, out: &Path, map: &str, mission: &str, game_id: &str) -> Result<()> {
    let (mut walk, scene) = battle(root, map, mission, game_id, true)?;
    let me = walk.player.index as usize;
    let mut r = Renderer::new_headless(1280, 720).map_err(|e| anyhow!("renderer: {e}"))?;
    let gpu = game::viewer::upload_scene(&mut r, &scene)?;
    let mut deka_vis = game::viewer::DroidekaVisuals::load(&mut r, &scene, root, &walk);
    let mut view = game::hud_feed::HudView::for_renderer(root, &r)?;
    if let Some(d) = walk.sim.units[me].droideka.as_mut() {
        d.order = Order::Deploy;
        d.shield.on = true;
        d.shield.points = 1800.0;
    }
    for _ in 0..30 {
        walk.step_tick(&[PadState::default()]);
        let cam = walk.camera();
        deka_vis.bubbles(&walk.sim, &cam, r.lighting.sun_dir);
    }
    let u = &walk.sim.units[me];
    let (energy, energy_max) = u.stamina_bar();
    let [weapon, weapon2] = game::hud_feed::weapon_rows(&walk);
    println!(
        "{game_id} hud: bar {energy:.0}/{energy_max:.0} primary {:?} secondary {:?}",
        weapon.as_ref().map(|w| &w.key),
        weapon2.as_ref().map(|w| &w.key),
    );
    let model = ui::hud::HudModel {
        team_names: ["a".into(), "b".into()],
        hp: u.health.cur,
        hp_max: u.health.max,
        energy,
        energy_max,
        weapon,
        weapon2,
        reticle: Some(0),
        reticle_y: walk.reticle_y(),
        player_team: u.team,
        ..Default::default()
    };
    view.update(&model, 0.1, [1280, 720]);
    let cam = walk.camera();
    let mut draws = gpu.view_draws(&walk.sim, walk.player);
    draws.extend(deka_vis.draws(&mut r, &walk.sim));
    let mut fx = game::fx::Fx::for_walk(root, &scene.dir, game_id, &walk);
    let bubbles = deka_vis.bubbles(&walk.sim, &cam, r.lighting.sun_dir);
    fx.submit_with(&mut r, &cam, bubbles);
    r.render_frame(
        &cam,
        &draws,
        &[],
        gpu.clear,
        Some(&mut |dev, q, enc, tgt, size| {
            view.paint(dev, q, enc, tgt, size);
        }),
    )
    .map_err(|e| anyhow!("render: {e}"))?;
    let (rgba, w, h) = r.capture_rgba().map_err(|e| anyhow!("capture: {e}"))?;
    let png = render::encode_png(&rgba, w, h).map_err(|e| anyhow!("png: {e}"))?;
    std::fs::write(out.join(format!("{game_id}_hud.png")), png)?;
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
            deka_vis.bubbles(&walk.sim, &walk.camera(), r.lighting.sun_dir);
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
    if std::env::args().nth(2).as_deref() == Some("hud") {
        hud(&root, &out, "bf2_kam1", "bf2_kam1_con_cw.ron", "bf2")?;
        return hud(&root, &out, "bf1_nab1", "bf1_nab1_con_cw.ron", "bf1");
    }
    if std::env::args().nth(2).as_deref() == Some("shield") {
        shield(&root, &out, "bf2_kam1", "bf2_kam1_con_cw.ron", "bf2")?;
        return shield(&root, &out, "bf1_nab1", "bf1_nab1_con_cw.ron", "bf1");
    }
    run(&root, &out, "bf2_kam1", "bf2_kam1_con_cw.ron", "bf2")?;
    run(&root, &out, "bf1_nab1", "bf1_nab1_con_cw.ron", "bf1")?;
    player(&root, &out, "bf2_kam1", "bf2_kam1_con_cw.ron", "bf2")?;
    player(&root, &out, "bf1_nab1", "bf1_nab1_con_cw.ron", "bf1")
}
