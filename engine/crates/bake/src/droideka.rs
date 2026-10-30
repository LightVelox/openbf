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
    crate::walker::skin_glb_of(skel, bank, &ENTRIES, &[""], |_, _| {})
}
