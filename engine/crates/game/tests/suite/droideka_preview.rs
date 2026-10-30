use core::sim::Sim;
use game::walk::{ClassVisual, TeamTemplate, WalkSim};
use glam::Vec3;
use render::{Camera, DrawCmd, Renderer, SkinnedDrawCmd};
use std::path::Path;

fn fixture(root: &Path, game_id: &str, map: &str) -> (WalkSim, game::assets::Scene) {
    let (scene, _) = game::assets::load_battle_scene(&root.join(map), "con").unwrap();
    let mission = game::mission::MissionDef::load(
        &root
            .parent()
            .unwrap()
            .join("missions")
            .join(format!("{map}_con_cw.ron")),
    )
    .unwrap();
    let (mut rosters, _) = game::roster::FactionRoster::pair(
        &root.join("classes"),
        game_id,
        [
            mission.teams[0].faction.as_str(),
            mission.teams[1].faction.as_str(),
        ],
        Some(&mission),
    )
    .unwrap();
    game::droideka::install(&mut rosters, &scene.dir, game_id);
    let mut sim = Sim::empty(42);
    let mut names = Vec::new();
    let mut team = 0;
    let teams = rosters.map(|roster| {
        let classes: Vec<_> = roster
            .classes
            .iter()
            .map(|c| c.register(&mut sim, &mut names))
            .collect();
        let visuals = roster
            .classes
            .iter()
            .map(ClassVisual::from_roster)
            .collect();
        let template = TeamTemplate::from_roster(&roster, &classes[0], visuals);
        sim.classes[team] = classes;
        team += 1;
        template
    });
    (
        WalkSim::with_teams(sim, scene.heightfield(), Vec3::ZERO, teams, None),
        scene,
    )
}

fn pixels(r: &mut Renderer, cam: &Camera, body: &[SkinnedDrawCmd], models: &[DrawCmd]) -> Vec<u8> {
    r.render_frame(cam, models, body, [0.08, 0.08, 0.08, 1.0], None)
        .unwrap();
    r.capture_rgba().unwrap().0
}

fn save(name: &str, pixels: &[u8]) {
    if let Some(dir) = std::env::var_os("OPENBF_TEST_ARTIFACTS") {
        let dir = std::path::PathBuf::from(dir);
        std::fs::create_dir_all(&dir).unwrap();
        let png = render::encode_png(pixels, 640, 480).unwrap();
        std::fs::write(dir.join(format!("{name}.png")), png).unwrap();
    }
}

#[test]
#[ignore = "requires a GPU adapter and newly baked local BF1/BF2 CIS assets"]
fn real_droideka_carousels_replace_the_infantry_body_and_weapon() {
    let root = game::assets::baked_root();
    for (game_id, map) in [("bf1", "bf1_nab1"), ("bf2", "bf2_kam1")] {
        let (walk, scene) = fixture(&root, game_id, map);
        let team = walk.teams.iter().position(|t| t.faction == "cis").unwrap();
        let class = walk.sim.classes[team]
            .iter()
            .position(|c| c.droideka.is_some())
            .unwrap();
        let definition = walk.sim.classes[team][class].droideka.as_deref().unwrap();
        let page = ui::spawn_draw::SpawnPage::Class {
            title: "Droideka".into(),
            lines: Vec::new(),
            back: String::new(),
            accept: String::new(),
            team: team as u8,
            class,
            classes: (0..walk.sim.classes[team].len() as u8).collect(),
            refused: 0,
        };
        let cam = Camera {
            fov_y: 2.0 * 0.34131f32.atan(),
            ..Default::default()
        };
        let mut r = Renderer::new_headless(640, 480).unwrap();
        let vis = game::soldier_vis::SoldierVisuals::for_walk(&mut r, &root, &scene, &walk);
        let mut dekas = game::viewer::DroidekaVisuals::load(&mut r, &scene, &root, &walk);
        let mut fallback = game::spawn_preview::Preview::default();
        let (old_body, old_models) = fallback.draws(&vis, Some(&page), &cam, 0.0);
        assert!(
            !old_body.is_empty(),
            "the real infantry fallback control must draw"
        );
        let old = pixels(&mut r, &cam, &old_body, &old_models);
        save(&format!("{game_id}_preview_infantry_control"), &old);
        let mut preview = game::spawn_preview::Preview::default();
        let mut called = 0;
        let mut selected_draws = 0;
        let hash = walk.sim.hash();
        let (body, models) = preview.draws_with(&vis, Some(&page), &cam, 0.0, |t, c, model, dt| {
            if usize::from(t) != team || usize::from(c) != class {
                return None;
            }
            called += 1;
            let draws = dekas.preview_draws(&mut r, t, c, definition, model, dt);
            assert!(
                !draws.is_empty(),
                "the newly baked deployed droideka must draw"
            );
            selected_draws = draws.len();
            Some(draws)
        });
        assert_eq!(
            called, 1,
            "the carousel must dispatch the specialized droideka once"
        );
        assert!(
            body.len() < old_body.len(),
            "droideka fallback infantry survived in the carousel"
        );
        assert!(models
            .iter()
            .all(|d| d.light == render::LightPath::Iface && d.cull.is_none()));
        let correct = pixels(&mut r, &cam, &body, &models);
        let changed = old
            .chunks_exact(4)
            .zip(correct.chunks_exact(4))
            .filter(|(a, b)| a != b)
            .count();
        assert!(
            changed > 500,
            "{game_id}: replacing infantry with the droideka changed only {changed} pixels"
        );
        save(&format!("{game_id}_preview_droideka"), &correct);
        // A known specialized class with a missing bake stays absent. The
        // ordinary classes retain exactly their prior bodies and weapons.
        let (missing_body, missing_models) = game::spawn_preview::Preview::default().draws_with(
            &vis,
            Some(&page),
            &cam,
            0.0,
            |t, c, _, _| (usize::from(t) == team && usize::from(c) == class).then(Vec::new),
        );
        assert_eq!(
            missing_body.len(),
            body.len(),
            "missing droideka fell back to infantry"
        );
        for (actual, expected) in missing_body.iter().zip(&body) {
            assert_eq!(actual.mesh, expected.mesh, "ordinary class body changed");
            assert_eq!(actual.model, expected.model, "ordinary class place changed");
        }
        assert_eq!(
            missing_models.len() + selected_draws,
            models.len(),
            "ordinary class weapon draws changed"
        );
        save(
            &format!("{game_id}_preview_missing_bake"),
            &pixels(&mut r, &cam, &missing_body, &missing_models),
        );
        assert_eq!(
            walk.sim.hash(),
            hash,
            "preview must not advance the live sim"
        );
    }
}

/// The maintainer: "The shield needs a hud to show it is the secondary
/// weapon." The player's rows on both games, the gun's in the primary.
#[test]
fn a_player_droidekas_rows_are_its_gun_and_the_bf2_shield() {
    let root = game::assets::baked_root();
    if !root.join("classes/bf2_cis.ron").exists() {
        eprintln!("SKIP: no baked classes");
        return;
    }
    for (game_id, map) in [("bf1", "bf1_nab1"), ("bf2", "bf2_kam1")] {
        let (mut walk, _scene) = fixture(&root, game_id, map);
        let team = walk.teams.iter().position(|t| t.faction == "cis").unwrap();
        let class = walk.sim.classes[team]
            .iter()
            .position(|c| c.droideka.is_some())
            .unwrap();
        let spec = walk.sim.classes[team][class].spec(
            team as u8,
            Vec3::ZERO,
            0.0,
            core::unit::Controller::Idle,
            class as u8,
            0,
        );
        walk.player = walk.sim.spawn(&spec).unwrap();
        let [primary, secondary] = game::hud_feed::weapon_rows(&walk);
        assert_eq!(primary.unwrap().key, "hud_vehicle_lasers", "{game_id}");
        let shield = secondary.unwrap();
        assert_eq!(shield.key, "hud_droideka_shield", "{game_id}");
        assert_eq!(shield.icon, None, "{game_id}");
        assert_eq!(shield.total, None, "{game_id}");
    }
}
