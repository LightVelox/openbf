use std::path::{Path, PathBuf};

const ALIASES: [(u32, u32); 3] = [
    (2_298_015_460, 2_911_796_988),
    (2_314_793_079, 2_928_574_607),
    (117_175_253, 2_945_352_226),
];

fn pcm(banks: &[mungefs::soundbank::SampleBank<'_>], id: u32) -> (u32, Vec<i16>) {
    let (bank, sample) = banks
        .iter()
        .find_map(|bank| {
            bank.samples
                .iter()
                .find(|s| s.id_hash == id && s.alias_of.is_none())
                .map(|sample| (bank, sample))
        })
        .unwrap_or_else(|| panic!("original backing sample {id:08x} missing"));
    (
        sample.frequency,
        bake::adpcm::decode(&bank.data[sample.offset..sample.offset + sample.size]),
    )
}

struct Cleanup(PathBuf);
impl Drop for Cleanup {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn bf2_tantive_female_hurt_aliases_reuse_identical_original_ps2_pcm() {
    let root = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../extracted/bf2"
    ));
    let sounds = root.join("sound/DATA/_LVL_PS2/SOUND");
    let tan = sounds.join("TAN.LVL");
    if !tan.is_file() {
        eprintln!("SKIP: original BF2 sound LVLs required");
        return;
    }
    let files = ["TAN", "TAT", "MYG"].map(|name| {
        std::fs::read(sounds.join(format!("{name}.LVL"))).expect("original reference LVL")
    });
    let banks = files
        .each_ref()
        .map(|bytes| mungefs::soundbank::find_banks_deep(mungefs::open(bytes).unwrap()));
    for (alias, backing) in ALIASES {
        assert!(banks[1]
            .iter()
            .flat_map(|bank| &bank.samples)
            .any(|s| s.id_hash == alias && s.alias_of == Some(backing)));
        let expected = pcm(&banks[0], backing);
        assert_eq!(expected, pcm(&banks[1], backing), "TAN/TAT {backing:08x}");
        assert_eq!(expected, pcm(&banks[2], backing), "TAN/MYG {backing:08x}");
        assert!(!expected.1.is_empty());
    }
    let out = Cleanup(std::env::temp_dir().join(format!(
        "openbf-tan-alias-repair-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    )));
    let core = root.join("lvl/DATA/_LVL_PS2/CORE.LVL");
    let baked = bake::sounds::bake_sounds(
        tan.to_str().unwrap(),
        out.0.to_str().unwrap(),
        Some(core.to_str().unwrap()),
    )
    .unwrap();
    let mut actual = Vec::new();
    let mut expected = Vec::new();
    for (alias, backing) in ALIASES {
        let file = baked
            .samples
            .iter()
            .find(|(id, _)| *id == backing)
            .unwrap()
            .1
            .clone();
        let original = pcm(&banks[0], backing);
        assert_eq!(
            std::fs::read(out.0.join("sounds").join(&file)).unwrap(),
            bake::adpcm::wav_mono_s16(original.0, &original.1)
        );
        actual.push(
            baked
                .samples
                .iter()
                .find(|(id, _)| *id == alias)
                .map(|(_, p)| p.clone()),
        );
        expected.push(Some(file));
    }
    // Filename alone must not inject aliases into a different/empty source.
    let empty = out.0.join("empty/TAN.LVL");
    std::fs::create_dir_all(empty.parent().unwrap()).unwrap();
    std::fs::write(&empty, b"ucfb\0\0\0\0").unwrap();
    let untouched = bake::sounds::bake_sounds(
        empty.to_str().unwrap(),
        out.0.join("empty_output").to_str().unwrap(),
        None,
    )
    .unwrap();
    assert!(untouched.samples.is_empty());
    assert_eq!(
        actual, expected,
        "all three aliases must reuse the original backing WAVs"
    );
}
