//! Prop effect attachments: native property order and full hardpoint frames.
//! BF2 FUN_001d5260 / FUN_001d4db8 encode optional repeat-delay bounds.
use crate::{
    hardpoints::Skeletons,
    leafpatch::{ATTACH_EFFECT, ATTACH_ODF, ATTACH_TO_HARDPOINT},
};
use manifest::{ManifestAttachedEffect, ManifestClass};

pub const EMITTER: u32 = 0x576b_09cd;

pub fn is_prop(key: u32) -> bool {
    matches!(
        key,
        ATTACH_EFFECT | ATTACH_ODF | ATTACH_TO_HARDPOINT | EMITTER
    )
}

pub fn attach(
    entcs: &[(String, Vec<(u32, String)>)],
    skels: &Skeletons,
    classes: &mut [ManifestClass],
) {
    let mut current = None;
    for (name, props) in entcs {
        let Some(class) = classes
            .iter_mut()
            .find(|c| c.name.eq_ignore_ascii_case(name))
        else {
            continue;
        };
        let skel = class.geometry.as_deref().and_then(|g| skels.find(g));
        let binds = skel.and_then(|s| s.bind_matrices().ok());
        class.effects.clear();
        for (key, value) in props {
            let (effect, point) = match *key {
                ATTACH_EFFECT => {
                    current = Some(value.trim().to_owned());
                    continue;
                }
                ATTACH_ODF => {
                    current = None;
                    continue;
                }
                ATTACH_TO_HARDPOINT => {
                    let Some(effect) = &current else { continue };
                    (effect.clone(), value.as_str())
                }
                EMITTER => {
                    let mut words = value.split_whitespace();
                    let (Some(effect), Some(point)) = (words.next(), words.next()) else {
                        continue;
                    };
                    (effect.to_owned(), point)
                }
                _ => continue,
            };
            let mut words = point.split_whitespace();
            let Some(hardpoint) = words.next() else {
                continue;
            };
            let delay = words
                .next()
                .and_then(|v| v.parse::<f32>().ok())
                .filter(|v| v.is_finite())
                .map(|lo| {
                    let hi = words
                        .next()
                        .and_then(|v| v.parse::<f32>().ok())
                        .filter(|v| v.is_finite())
                        .unwrap_or(lo);
                    [lo, hi]
                });
            let matrix = skel
                .zip(binds.as_ref())
                .and_then(|(s, m)| s.index_of(hardpoint).map(|k| m[k]));
            // [ours] BF2's origin fallback and 64 slots on both games.
            let (pos, axes) = matrix.map_or(
                (
                    [0.0; 3],
                    [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
                ),
                |m| {
                    (
                        [m[3][0], m[3][1], m[3][2]],
                        [0, 1, 2].map(|k| [m[k][0], m[k][1], m[k][2]]),
                    )
                },
            );
            if class.effects.len() < 64 {
                class.effects.push(ManifestAttachedEffect {
                    effect,
                    hardpoint: hardpoint.into(),
                    pos,
                    axes,
                    delay,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn effect_selection_carries_between_classes_until_an_odf_replaces_it() {
        let make = |n: &str| {
            ron::from_str::<ManifestClass>(&format!("(name:\"{n}\",base:\"prop\",geometry:None)"))
                .unwrap()
        };
        let mut classes = [make("a"), make("b"), make("c")];
        let props = vec![
            (
                "a".into(),
                vec![
                    (ATTACH_EFFECT, "steam".into()),
                    (ATTACH_TO_HARDPOINT, "hp 2 5".into()),
                ],
            ),
            ("b".into(), vec![(ATTACH_TO_HARDPOINT, "other".into())]),
            (
                "c".into(),
                vec![
                    (ATTACH_ODF, "light".into()),
                    (ATTACH_TO_HARDPOINT, "hp".into()),
                ],
            ),
        ];
        attach(&props, &Skeletons::of(&[]), &mut classes);
        assert_eq!(classes[0].effects[0].delay, Some([2.0, 5.0]));
        assert_eq!(classes[0].effects[0].pos, [0.0; 3]);
        assert_eq!(classes[1].effects[0].effect, "steam");
        assert_eq!(classes[1].effects[0].delay, None);
        assert!(classes[2].effects.is_empty());
    }
    #[test]
    fn inline_emitter_records_do_not_need_a_previous_effect_selection() {
        let mut classes: Vec<ManifestClass> =
            vec![ron::from_str(r#"(name:"a",base:"prop",geometry:None)"#).unwrap()];
        attach(
            &[("a".into(), vec![(EMITTER, "steam hp".into())])],
            &Skeletons::of(&[]),
            &mut classes,
        );
        assert_eq!(classes[0].effects[0].effect, "steam");
        assert_eq!(classes[0].effects[0].hardpoint, "hp");
        assert_eq!(classes[0].effects[0].delay, None);
    }
}
