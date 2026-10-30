use manifest::ManifestDroideka;
use mungefs::anim::AnimBank;
use mungefs::coll::Primitives;
use mungefs::entity::EntityClass;
use mungefs::skel::Skeleton;

pub const ENTRIES: [(&str, bool); 14] = [
    ("idle", false),
    ("turnl", false),
    ("turnr", false),
    ("death01", false),
    ("", false),
    ("idle_to_leftfoot", false),
    ("", false),
    ("walk_leftfoot_rightfoot", true),
    ("walk_rightfoot_leftfoot", true),
    ("leftfoot_to_idle", false),
    ("rightfoot_to_idle", false),
    ("fold", false),
    ("roll", true),
    ("unfold", false),
];

pub fn is_droideka_base(base: &str) -> bool {
    base.eq_ignore_ascii_case("walkerdroid")
}

pub fn bake(
    e: &EntityClass,
    geometry: &str,
    bank_name: &str,
    banks: &[AnimBank<'_>],
    prims: Option<&Primitives>,
    skel: Option<&Skeleton>,
    skin: Option<String>,
) -> ManifestDroideka {
    let bank = banks
        .iter()
        .find(|b| b.name.eq_ignore_ascii_case(bank_name));
    ManifestDroideka {
        class: e.name.clone(),
        geometry: geometry.to_owned(),
        bank: bank_name.to_owned(),
        clips: bank.map_or_else(
            || vec![None; ENTRIES.len()],
            |b| crate::walker::clips_of(b, &ENTRIES),
        ),
        skin,
        prims: prims.map_or_else(Vec::new, |p| crate::walker::prim_points(p, skel)),
    }
}

pub fn skin_glb(skel: &Skeleton, bank: Option<&AnimBank<'_>>) -> anyhow::Result<Vec<u8>> {
    let mut clips = Vec::new();
    for (name, _) in ENTRIES.iter().filter(|(n, _)| !n.is_empty()) {
        let Some(bank) = bank else { break };
        let Some(k) = bank
            .clip_index(name)
            .or_else(|| bank.clip_index(&format!("{}_{name}", bank.name)))
        else {
            continue;
        };
        let Ok(d) = bank.decode_clip(k) else { continue };
        let (tracks, _) = crate::soldier::clip_to_tracks(&d, skel, None);
        let mut kept: Vec<crate::skin_glb::ClipTrack> = Vec::new();
        for t in tracks {
            kept.retain(|k| k.bone != t.bone);
            kept.push(t);
        }
        clips.push(crate::skin_glb::Clip {
            name: (*name).to_owned(),
            fps: 30.0,
            frames: u32::from(d.frames),
            tracks: kept,
        });
    }
    let model = crate::skin_glb::SkinnedModel {
        name: skel.name.clone(),
        bones: crate::soldier::bones_of(skel),
        segments: Vec::new(),
    };
    crate::skin_glb::skinned_to_glb(&model, &clips)
}
