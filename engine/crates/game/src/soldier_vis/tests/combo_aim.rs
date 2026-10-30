use super::*;
use crate::roster::FactionRoster;
use crate::walk::ClassVisual;
use manifest::combo::ComboKeyword as K;

// Stage the actual authored state and clock to isolate its presentation
// metadata. Aayla uses the Hero Assault aim default; a separate raw
// Felucia descriptor control retains the original torso adjustment.
// This does not claim that these attacks were unlocked by input.
#[test]
fn aayla_uses_hero_assault_aiming_on_felucia() {
    let root = crate::assets::baked_root();
    if !root
        .join("side_bf2/soldiers/rep_inf_aaylasecura.glb")
        .is_file()
    {
        eprintln!("SKIP: actual BF2 Aayla model/classes required");
        return;
    }
    let mission = crate::mission::MissionDef::load(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../missions/bf2_fel1_con_cw.ron"),
    )
    .unwrap();
    let mut faction = FactionRoster {
        faction: "rep".into(),
        classes: Vec::new(),
    };
    assert_eq!(
        crate::heroes::append(&mut faction, &root.join("classes"), Some(&mission), 0),
        Some(0)
    );
    let roster = faction.classes[0].clone();
    let mut native = roster.clone();
    native.anim.custom =
        crate::animator::custom::for_mission(&root.join("classes"), &mission).unwrap();
    check_aayla_poses(&root, roster, false);
    check_aayla_poses(&root, native, true);
}

fn check_aayla_poses(root: &std::path::Path, roster: crate::roster::ClassRoster, aims_third: bool) {
    let mut sim = core::sim::Sim::empty(4);
    sim.ground_rules = core::soldier::GroundRules::Bf2;
    let registered = roster.register(&mut sim, &mut Vec::new());
    sim.classes[0] = vec![registered.clone()];
    crate::heroes::install(&mut sim, [Some(0), None]);
    let faction = FactionRoster {
        faction: "rep".into(),
        classes: vec![roster.clone()],
    };
    let team = TeamTemplate::from_roster(
        &faction,
        &registered,
        vec![ClassVisual::from_roster(&roster)],
    );
    let mut walk = WalkSim::with_teams(
        sim,
        None,
        Vec3::ZERO,
        [team, TeamTemplate::default_trooper()],
        None,
    );
    let mut vis = SoldierVisuals::load_cpu(root, "bf2", &walk.teams);
    let unit = walk.player.index as usize;
    let weapon = walk.sim.units[unit].weapon_class;
    let combo = walk.sim.weapon_classes[weapon]
        .saber
        .as_ref()
        .unwrap()
        .combo
        .clone();
    let cases = [
        ("ATTACK3", Some("Torso")),
        ("ATTACK3_LAND", Some("Torso")),
        ("RECOVER3", None), // Same locked descriptor as ATTACK3 in original FeluciaCW.
        ("ATTACK1", Some("None")),
        ("DASHATTACK", Some("FullBody")),
    ];
    let mut observed = Vec::new();
    for (state_name, expected) in cases {
        let (index, state) = combo
            .states()
            .enumerate()
            .find(|(_, s)| s.text(0) == Some(state_name))
            .unwrap();
        let animation = state.find(K::Animation).unwrap();
        assert_eq!(animation.find(K::AimType).and_then(|f| f.text(0)), expected);
        let u = &mut walk.sim.units[unit];
        u.soldier.pitch = 0.5;
        u.soldier.yaw = 0.0;
        u.soldier.stance = core::soldier::Stance::Stand;
        let melee = u.weapon.melee.get_or_insert_default();
        *melee = core::hero_weapon::MeleeState::default();
        melee.runtime.state = index;
        melee.runtime.time = 0.1;
        melee.animation = Some(animation.text(0).unwrap().into());
        melee.aligned_to_thrust = state.find(K::AlignedToThrust).is_some();
        melee.retained_thrust = [0.0, 1.0];
        melee.current_thrust_forward = 1.0;
        vis.sync_units(&walk);
        vis.units[unit].animator = Animator::new();
        for _ in 0..12 {
            walk.sim.tick += 1;
            vis.update(&walk, core::TICK_DT);
        }
        let asset = vis.asset_for(0, 0).unwrap();
        let skel = &asset.asset.skeleton;
        let a = &vis.units[unit].animator;
        let raw = a.kept_pose(skel);
        let posed = a.pose(skel);
        let spine = skel.index_of("bone_ribcage").unwrap();
        let raw_rotation = crate::animator::aim::global(&raw, skel, spine);
        let aimed_rotation = crate::animator::aim::global(&posed, skel, spine);
        // cos(0.05 / 2) keeps the angular threshold without host trig.
        let changed = raw_rotation.dot(aimed_rotation).abs() < 0.999_687_5;
        let hp = skel.index_of("hp_weapons").unwrap();
        let raw_hp = skel.palette(&raw.locals())[hp] * skel.global_bind[hp];
        let posed_hp = skel.palette(&posed.locals())[hp] * skel.global_bind[hp];
        assert_eq!(
            !raw_hp.abs_diff_eq(posed_hp, 0.01),
            changed,
            "{state_name}: the weapon follows the aimed spine"
        );
        let root = skel.index_of("DummyRoot").unwrap();
        assert_eq!(
            raw.0[root].rotation, posed.0[root].rotation,
            "{state_name}: no custom root twist"
        );
        println!("Aayla {state_name} AimType={expected:?} torso_changed={changed}");
        observed.push((state_name, changed));
    }
    assert_eq!(
        observed,
        [
            ("ATTACK3", aims_third),
            ("ATTACK3_LAND", aims_third),
            ("RECOVER3", aims_third),
            ("ATTACK1", false),
            ("DASHATTACK", false),
        ]
    );
}
