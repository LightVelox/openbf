//! BF2's shared custom-animation descriptors. The first actual SCOP locks a
//! name/layer; forward declarations do not (0x307a80, 0x279780, 0x279498).

use super::aim::AimKind;
use crate::mission::MissionDef;
use anyhow::{anyhow, bail, Context, Result};
use manifest::combo::{ComboDef, ComboField, ComboKeyword as K, ComboSet};
use std::{collections::BTreeMap, path::Path};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Descriptor {
    pub aim: AimKind,
    pub blend_in: f32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Descriptors {
    layers: BTreeMap<String, [Option<Descriptor>; 2]>,
}

fn layer(name: &str) -> (String, Option<usize>) {
    let name = name.to_ascii_lowercase();
    for (suffix, layer) in [("_upper", 0), ("_lower", 1)] {
        if let Some(base) = name.strip_suffix(suffix) {
            return (base.into(), Some(layer));
        }
    }
    (name, None)
}

// Scoped to the descriptor's numeric packing, not ordinary presentation math.
// MIPS279bb4/279bcc and27f458 use EE round-toward-zero multiplication.
fn ee_mul(a: f32, b: f32) -> f32 {
    let exact = f64::from(a) * f64::from(b);
    let rounded = exact as f32;
    if f64::from(rounded.abs()) > exact.abs() {
        f32::from_bits(rounded.to_bits() - 1)
    } else {
        rounded
    }
}

fn descriptor(field: &ComboField) -> Descriptor {
    let mut descriptor = Descriptor::default();
    // FUN279780 applies each recognized property in authored order. Unknown
    // AimType strings do not overwrite a prior recognized value.
    for property in &field.children {
        match property.keyword {
            K::AimType => {
                if let Some(name) = property.text(0) {
                    if name.eq_ignore_ascii_case("Torso") {
                        descriptor.aim = AimKind::Torso;
                    } else if name.eq_ignore_ascii_case("Head") {
                        descriptor.aim = AimKind::Head;
                    } else if name.eq_ignore_ascii_case("None")
                        || name.eq_ignore_ascii_case("FullBody")
                    {
                        // Native FullBody becomes flag4 and skips the
                        // head/torso adjustment in0x280158.
                        descriptor.aim = AimKind::None;
                    }
                }
            }
            K::BlendInTime => {
                if let Some(mut seconds) = property.number(0) {
                    if property
                        .text(1)
                        .is_some_and(|unit| unit.eq_ignore_ascii_case("Frames"))
                    {
                        seconds = ee_mul(seconds, f32::from_bits(0x3d08_8889));
                    }
                    // 279bcc..279bf0: multiply100, clamp0..255, CVT, storebyte.
                    // 27f458 decodes that byte with the ELF's literal0.01.
                    let packed = ee_mul(seconds, 100.0).clamp(0.0, 255.0) as u8;
                    descriptor.blend_in = ee_mul(f32::from(packed), 0.01);
                }
            }
            // BlendTimeIn is a distinct, unrecognized key in279780; it is
            // not an alias for BlendInTime.
            _ => {}
        }
    }
    descriptor
}

impl Descriptors {
    pub fn upper(&self, name: &str) -> Descriptor {
        self.layers
            .get(&layer(name).0)
            .and_then(|l| l[0])
            .unwrap_or_default()
    }

    fn field(&mut self, field: &ComboField) {
        if matches!(
            field.keyword,
            K::Animation | K::OffhandAnimation | K::ThrowAnimation
        ) && field.scoped
        {
            if let Some(name) = field.text(0) {
                let (name, which) = layer(name);
                let descriptor = descriptor(field);
                let pair = self.layers.entry(name).or_default();
                match which {
                    Some(i) => {
                        pair[i].get_or_insert(descriptor);
                    }
                    None => {
                        pair[0].get_or_insert(descriptor);
                        // Bare scopes copy to the lower descriptor with AimType cleared.
                        let upper = pair[0].unwrap();
                        pair[1].get_or_insert(Descriptor {
                            aim: AimKind::None,
                            ..upper
                        });
                    }
                }
            }
        }
        if field.keyword == K::State {
            for child in &field.children {
                self.field(child);
            }
        }
    }

    pub fn register(&mut self, program: &ComboDef) {
        for field in &program.fields {
            self.field(field);
        }
    }
}

/// Resolve only the SIDE sublevels the PS2 mission actually requests, in call and
/// argument order. This table is presentation data and never enters the sim.
pub fn for_mission(classes_dir: &Path, mission: &MissionDef) -> Result<Descriptors> {
    if !mission.source_game.eq_ignore_ascii_case("bf2") {
        return Ok(Descriptors::default());
    }
    if mission.class_load_order.is_empty() {
        bail!(
            "{}: missing PS2 class load order; regenerate missions",
            mission.map
        );
    }
    let root = classes_dir
        .parent()
        .ok_or_else(|| anyhow!("classes directory has no asset root"))?;
    let mut sets: BTreeMap<String, ComboSet> = BTreeMap::new();
    let mut descriptors = Descriptors::default();
    for load in &mission.class_load_order {
        let side = load.side.to_ascii_lowercase();
        if !sets.contains_key(&side) {
            let path = root.join("side_bf2/combos").join(format!("{side}.ron"));
            let set: ComboSet = ron::from_str(&pak::read_to_string(&path)?)
                .with_context(|| path.display().to_string())?;
            if !set.scope_provenance || !set.load_provenance {
                bail!(
                    "{}: missing combo scope/load provenance; rebake combos",
                    path.display()
                );
            }
            sets.insert(side.clone(), set);
        }
        let set = &sets[&side];
        let hash = crate::classdb::fnv1a_lowercase(&load.request);
        let programs = set.loads.get(&hash).ok_or_else(|| {
            anyhow!(
                "{side}: requested sublevel {} has no baked load index",
                load.request
            )
        })?;
        for hash in programs {
            let program = set
                .combos
                .iter()
                .find(|c| c.name_hash == *hash)
                .ok_or_else(|| anyhow!("{side}: loaded combo {hash:08x} not baked"))?;
            descriptors.register(program);
        }
    }
    Ok(descriptors)
}

/// Apply the selected hero defaults after resolving the original mission table.
pub fn for_hero(classes_dir: &Path, mission: &MissionDef, hero: &str) -> Result<Descriptors> {
    let mut descriptors = for_mission(classes_dir, mission)?;
    if hero.eq_ignore_ascii_case("rep_hero_aalya") {
        // [ours] Aayla keeps Hero Assault's unadjusted third-attack pose on every map.
        if let Some(upper) = descriptors
            .layers
            .get_mut("stand_attack1c")
            .and_then(|pair| pair[0].as_mut())
        {
            upper.aim = AimKind::None;
        }
    }
    Ok(descriptors)
}

#[cfg(test)]
mod tests;
