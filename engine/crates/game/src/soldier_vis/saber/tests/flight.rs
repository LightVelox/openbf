use super::*;
use crate::{roster::ClassRoster, walk::ClassVisual};
use core::sim::{buttons, PadState};

fn thrown_scene() -> Option<(WalkSim, SoldierVisuals, Renderer)> {
    let root = crate::assets::baked_root();
    if !root
        .join("side_bf2/soldiers/rep_inf_ep3anakin.glb")
        .is_file()
    {
        eprintln!("SKIP: actual BF2 Anakin model/classes required");
        return None;
    }
    let db = crate::roster::load_classdb(&root.join("classes"), "bf2", &["rep"]).unwrap();
    let roster = ClassRoster::build(&db, "rep_hero_anakin", 0, Some(1)).unwrap();
    let mut sim = core::sim::Sim::empty(4);
    sim.ground_rules = core::soldier::GroundRules::Bf2;
    let registered = roster.register(&mut sim, &mut Vec::new());
    sim.classes[0] = vec![registered.clone()];
    crate::heroes::install(&mut sim, [Some(0), None]);
    let faction = crate::roster::FactionRoster {
        faction: "rep".into(),
        classes: vec![roster.clone()],
    };
    let mut visual = ClassVisual::from_roster(&roster);
    visual.held_hud = roster
        .held
        .iter()
        .map(|s| (s.slot, s.hud.clone()))
        .collect();
    visual.held_models = roster
        .held
        .iter()
        .map(|s| (s.slot, s.geometry.clone()))
        .collect();
    let team = TeamTemplate::from_roster(&faction, &registered, vec![visual]);
    let ground = core::heightfield::Heightfield {
        size: 2,
        unit: 4000.0,
        offset_x: 2000.0,
        offset_z: 2000.0,
        heights: vec![0.0; 4],
    };
    let mut walk = WalkSim::with_teams(
        sim,
        Some(ground),
        Vec3::ZERO,
        [team, TeamTemplate::default_trooper()],
        None,
    );
    let mut renderer =
        Renderer::new_headless(512, 384).expect("headless renderer for submission test");
    let mut vis = SoldierVisuals::load(&mut renderer, &root, "bf2", &walk.teams);
    for _ in 0..60 {
        walk.step_tick(&[PadState {
            buttons: buttons::SECONDARY_FIRE,
            ..Default::default()
        }]);
        vis.update(&walk, core::TICK_DT);
        if vis
            .flying_transform(&walk, walk.player.index as usize)
            .is_some()
        {
            return Some((walk, vis, renderer));
        }
    }
    panic!("actual Anakin secondary input did not launch a saber");
}

#[test]
fn a_physical_throw_draws_independently_of_hidden_mounted_and_dead_owner_bodies() {
    let Some((mut walk, mut vis, mut renderer)) = thrown_scene() else {
        return;
    };
    let i = walk.player.index as usize;
    let expected = (vis.saber_draws(&walk).len(), vis.saber_hilts(&walk).len());
    assert!(
        expected.0 > 0 && expected.1 > 0,
        "visible flight positive control"
    );
    capture(&walk, &mut vis, &mut renderer, "visible_control");
    let eye = walk.sim.units[i].section().eye_y;
    walk.cam_place = Some((eye, Vec3::ZERO));
    assert!(
        walk.player_body_hidden(),
        "camera at the soldier's pull point"
    );
    assert!(vis.posed(&walk).iter().all(|(n, ..)| *n != i));
    let hidden = (vis.saber_draws(&walk).len(), vis.saber_hilts(&walk).len());
    capture(&walk, &mut vis, &mut renderer, "hidden_owner");
    walk.cam_place = None;
    let vehicle = walk.sim.spawn_vehicle(&core::vehicle::VehicleSpec {
        kind: core::vehicle::VehicleKind::Hover,
        class: 0,
        home_team: Some(0),
        neutral_when_empty: false,
        spawn_pad: None,
        pos: Vec3::ZERO,
        yaw: 0.0,
        max_health: 500.0,
        health_type: core::weapon::HealthType::Vehicle,
        seats: vec![core::vehicle::SeatKind::Pilot],
    });
    assert!(walk.sim.seat_unit(walk.player, vehicle, 0));
    vis.update(&walk, core::TICK_DT);
    assert!(
        vis.flying_transform(&walk, i).is_some(),
        "boarding has not destroyed the flight"
    );
    let mounted = (vis.saber_draws(&walk).len(), vis.saber_hilts(&walk).len());
    capture(&walk, &mut vis, &mut renderer, "mounted_owner");
    assert_eq!(
        [hidden, mounted],
        [expected, expected],
        "independent flight draw submission"
    );
    walk.sim.unseat_unit(walk.player);
    let damage = core::weapon::ExplosionClass {
        damage: walk.sim.units[i].health.max * 2.0,
        damage_inner: 5.0,
        damage_outer: 5.0,
        ..Default::default()
    };
    let blast = walk.sim.units[i].soldier.pos + Vec3::Y * 0.9;
    walk.sim.explode(&damage, blast, None, Some(1));
    assert!(
        !walk.sim.units[i].alive && walk.sim.units[i].body,
        "health damage leaves the legal corpse control"
    );
    walk.step_tick(&[PadState::default()]);
    vis.update(&walk, core::TICK_DT);
    assert!(!walk.sim.units[i].alive);
    let u = &walk.sim.units[i];
    assert_eq!(walk.sim.hero_class(u.team), Some(u.spec.class));
    let asset = vis.asset_for(u.team, u.spec.class).unwrap();
    let death_clip = vis.units[i].animator.lower_clip().unwrap();
    assert!(asset.asset.clips[death_clip]
        .name
        .contains("stand_deadhero"));
    assert!(
        vis.flying_transform(&walk, i).is_some(),
        "same-generation flight survives owner death"
    );
    let dead = (vis.saber_draws(&walk).len(), vis.saber_hilts(&walk).len());
    capture(&walk, &mut vis, &mut renderer, "dead_owner_inflight");
    assert_eq!(dead, expected, "independent flight remains lit until catch");
    let mut caught = false;
    for _ in 0..90 {
        walk.step_tick(&[PadState::default()]);
        vis.update(&walk, core::TICK_DT);
        if vis.flying_transform(&walk, i).is_none() {
            caught = true;
            break;
        }
    }
    assert!(caught, "the same-generation flight naturally returns");
    capture(&walk, &mut vis, &mut renderer, "dead_owner_after_catch");
    assert!(!walk.sim.units[i].alive, "control remains a corpse");
    assert!(
        vis.saber_draws(&walk).is_empty(),
        "caught corpse has no lit blade"
    );
    assert!(
        !vis.saber_hilts(&walk).is_empty(),
        "caught hilt follows the visible corpse again"
    );
}

fn capture(walk: &WalkSim, vis: &mut SoldierVisuals, renderer: &mut Renderer, phase: &str) {
    let Some(dir) = std::env::var_os("OPENBF_HERO_FLIGHT_CAPTURE_DIR") else {
        return;
    };
    let dir = std::path::PathBuf::from(dir);
    std::fs::create_dir_all(&dir).unwrap();
    let i = walk.player.index as usize;
    let unit = &walk.sim.units[i];
    let flight = vis.flying_transform(walk, i);
    let mut focus = unit.soldier.pos + Vec3::Y * 0.7;
    if let (Some(at), Some(visual)) = (flight, vis.saber_visual(unit.team, unit.spec.class)) {
        if let Some(binding) = visual
            .bindings
            .iter()
            .find(|b| b.blade.bone.eq_ignore_ascii_case("hp_weapons"))
        {
            let segment = binding.segment(at);
            focus = (segment.start + segment.end) * 0.5;
        }
    }
    let eye = focus + Vec3::new(2.0, 1.1, 3.5);
    let delta = focus - eye;
    let camera = render::Camera {
        position: eye,
        yaw: core::fmath::atan2(-delta.x, -delta.z),
        pitch: core::fmath::atan2(delta.y, Vec3::new(delta.x, 0.0, delta.z).length()),
        ..Default::default()
    };
    let body = vis.draws(walk);
    let mut draws = vis.weapon_draws(walk);
    draws.extend(vis.cloth_draws(renderer, walk));
    let particles = vis.saber_draws(walk);
    let death_clip = vis.asset_for(unit.team, unit.spec.class).and_then(|a| {
        vis.units[i]
            .animator
            .lower_clip()
            .map(|c| a.asset.clips[c].name.as_str())
    });
    let receipt = serde_json::json!({"phase":phase,"tick":walk.sim.tick,"alive":unit.alive,"hero_class":walk.sim.hero_class(unit.team)==Some(unit.spec.class),"lower_clip":death_clip,"mounted":unit.mount.is_some(),"player_body_hidden":walk.player_body_hidden(),"flight":flight.map(|m|m.to_cols_array()),"body_draws":body.len(),"weapon_draws":draws.len(),"blade_particles":particles.len(),"scope":"diagnostic actual Anakin throw with hero manager metadata, core seat API test vehicle; free inspection camera with normal body visibility gates"});
    renderer.set_particles(particles);
    renderer
        .render_frame(&camera, &draws, &body, [0.03, 0.03, 0.03, 1.0], None)
        .unwrap();
    let (rgba, width, height) = renderer.capture_rgba().unwrap();
    std::fs::write(
        dir.join(format!("{phase}.png")),
        render::encode_png_rgb(&rgba, width, height).unwrap(),
    )
    .unwrap();
    std::fs::write(
        dir.join(format!("{phase}.json")),
        serde_json::to_string_pretty(&receipt).unwrap(),
    )
    .unwrap();
}

/// Playtest 2026-10-30: "the lightsaber throw animation is not showing".
/// The real secondary fire of the real class: the clip starts at the
/// trigger and is at InitialSalvoDelay when the saber leaves; the catch is
/// the combo's CATCH_SABRE and its clip; both poses differ from the idle
/// pose. Mutation control: without the rising-edge start the cast stays None.
#[test]
fn the_real_throw_and_catch_play_their_authored_clips() {
    let root = crate::assets::baked_root();
    if !root
        .join("side_bf2/soldiers/rep_inf_ep3anakin.glb")
        .is_file()
    {
        eprintln!("SKIP: actual BF2 Anakin model/classes required");
        return;
    }
    let Some((mut walk, mut vis, _renderer)) = thrown_scene() else {
        return;
    };
    let i = walk.player.index as usize;
    let clips = |vis: &SoldierVisuals, walk: &WalkSim| {
        let u = &walk.sim.units[i];
        let asset = vis.asset_for(u.team, u.spec.class).unwrap();
        let class = &vis.anim[usize::from(u.team.min(1))][usize::from(u.spec.class)];
        let mut clock = vis.units[i].cast.clone();
        clock.clone_frame(u, &walk.sim.weapon_classes, &asset.asset.clips, class)
    };
    // thrown_scene ran to the launch: the clip began at the trigger.
    let (name, seconds) = clips(&vis, &walk).expect("the throw clip is playing at the release");
    assert_eq!(name, "stand_throw");
    assert!(
        (0.2..0.4).contains(&seconds),
        "the release frame: {seconds}"
    );
    let mut caught = false;
    for _ in 0..150 {
        walk.step_tick(&[PadState::default()]);
        vis.update(&walk, core::TICK_DT);
        let m = walk.sim.units[i].weapon.melee.as_ref().unwrap();
        if m.animation.as_deref() == Some("stand_catch") {
            caught = true;
            break;
        }
    }
    assert!(caught, "CATCH_SABRE's clip plays");
    assert!(vis.flying_transform(&walk, i).is_none());
}

/// Playtest 2026-10-30: "it instantly shows your character change to the
/// hero". A soldier taken off the field with no death (a hero Yes keeps his
/// generation and takes the hero's class) is not drawn: no corpse, no model
/// swap in his place.
#[test]
fn a_soldier_removed_without_a_death_is_not_drawn() {
    let Some((mut walk, mut vis, _renderer)) = thrown_scene() else {
        return;
    };
    let i = walk.player.index as usize;
    assert!(
        vis.posed(&walk).iter().any(|(n, ..)| *n == i),
        "drawn alive"
    );
    assert!(walk.sim.open_spawn_screens(0, false));
    vis.update(&walk, core::TICK_DT);
    assert!(!walk.sim.units[i].alive);
    assert!(
        vis.posed(&walk).iter().all(|(n, ..)| *n != i),
        "gone at once"
    );
}
