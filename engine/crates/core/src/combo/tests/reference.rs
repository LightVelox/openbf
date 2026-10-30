// Optional original-content referee. The fixture is supplied externally and
// contains the complete retained observations plus preclassified mixed rows.
#[test]
fn original_bf2_yoda_state_times_and_fifo_replay_all_five_controls() {
    use serde::Deserialize;
    #[derive(Deserialize)]
    struct Reference {
        trials: Vec<Trial>,
    }
    #[derive(Deserialize)]
    struct Trial {
        name: String,
        initial_clock: f32,
        initial_time: f32,
        dt: f32,
        inputs: Vec<u16>,
        observations: Vec<Observation>,
    }
    #[derive(Deserialize)]
    struct Observation {
        step: usize,
        state: usize,
        previous: usize,
        time: f32,
        clock: f32,
        held: u8,
        events: Vec<Event>,
        stable: bool,
    }
    #[derive(Deserialize)]
    struct Event {
        time: f32,
        button: u8,
        flags: u8,
    }
    #[derive(Deserialize)]
    struct Metadata {
        clips: Vec<Clip>,
    }
    #[derive(Deserialize)]
    struct Clip {
        name: String,
        frames: u32,
    }
    let Some(dir) = std::env::var_os("OPENBF_COMBO_REFERENCE_DIR") else {
        eprintln!("SKIP: original BF2 combo reference directory not supplied");
        return;
    };
    let dir = std::path::PathBuf::from(dir);
    let read = |name: &str| std::fs::read_to_string(dir.join(name)).unwrap();
    let reference: Reference = ron::from_str(&read("native_combo_reference.ron")).unwrap();
    let set: manifest::combo::ComboSet =
        ron::from_str(&read("native_combo_yoda.combos.ron")).unwrap();
    let metadata: Metadata = ron::from_str(&read("native_combo_yoda.anims.ron")).unwrap();
    let def = set
        .combos
        .iter()
        .find(|c| c.name_hash == 0x30f6e029)
        .unwrap();
    let clips: Vec<_> = def
        .states()
        .map(|state| {
            animation(state).map_or(0.0, |name| {
                let suffix = format!("_{name}_full");
                let bare = format!("_{name}");
                let clip = metadata
                    .clips
                    .iter()
                    .find(|c| c.name.ends_with(&suffix) || c.name.ends_with(&bare))
                    .unwrap_or_else(|| panic!("missing original clip {name}"));
                clip.frames.saturating_sub(1) as f32 * (1.0 / 30.0)
            })
        })
        .collect();
    assert_eq!(reference.trials.len(), 5);
    let mut total = 0;
    for trial in reference.trials {
        let mut runtime = ComboRuntime {
            state: 0,
            time: trial.initial_time,
            clock: trial.initial_clock,
            start: trial.initial_clock - trial.initial_time,
            ..Default::default()
        };
        let mut states = vec![runtime.clone()];
        for buttons in trial.inputs {
            runtime.step(def, &clips, input(buttons), trial.dt);
            states.push(runtime.clone());
        }
        let mut checked = 0;
        for row in trial.observations {
            if !row.stable {
                continue;
            }
            let r = &states[row.step];
            assert_eq!(r.state, row.state, "{} step{} state", trial.name, row.step);
            let previous = r.retro.as_ref().map_or(r.state, |r| r.state);
            assert_eq!(
                previous, row.previous,
                "{} step{} previous",
                trial.name, row.step
            );
            assert_eq!(
                r.time.to_bits(),
                row.time.to_bits(),
                "{} step{} elapsed port{} native{}",
                trial.name,
                row.step,
                r.time,
                row.time
            );
            assert_eq!(
                r.clock.to_bits(),
                row.clock.to_bits(),
                "{} step{} clock",
                trial.name,
                row.step
            );
            assert_eq!(
                r.history.held, row.held,
                "{} step{} held",
                trial.name, row.step
            );
            assert_eq!(
                r.history.events.len(),
                row.events.len(),
                "{} step{} FIFO count",
                trial.name,
                row.step
            );
            for (a, b) in r.history.events.iter().zip(row.events) {
                assert_eq!(
                    (a.button, a.flags),
                    (b.button, b.flags),
                    "{} step{} FIFO flags",
                    trial.name,
                    row.step
                );
                assert_eq!(
                    a.time.to_bits(),
                    b.time.to_bits(),
                    "{} step{} FIFO time {} != {}",
                    trial.name,
                    row.step,
                    a.time,
                    b.time
                );
            }
            checked += 1;
        }
        assert!(checked >= 80, "{} insufficient stable controls", trial.name);
        println!(
            "{}: {checked} native state/time/FIFO observations matched",
            trial.name
        );
        total += checked;
    }
    assert!(total >= 400);
}
