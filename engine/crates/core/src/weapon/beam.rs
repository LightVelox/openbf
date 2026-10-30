use super::{pf, pf_hash, PropSource};

const HASH_RANGE: u32 = 0xfadc_0cd2;
const HASH_PIERCE: u32 = 0x604e_2441;
const HASH_IMPACT_EFFECT: u32 = 0x1172_7a92;

#[derive(Clone, Debug, PartialEq)]
pub struct BeamClass {
    pub range: f32,
    pub life: f32,
    pub pierce: bool,
    pub impact: Option<String>,
}

impl BeamClass {
    pub fn from_props(src: &dyn PropSource) -> Self {
        Self {
            range: pf_hash(src, HASH_RANGE).unwrap_or(150.0),
            life: pf(src, "LifeSpan").unwrap_or(0.0),
            pierce: pf_hash(src, HASH_PIERCE).is_some_and(|v| v != 0.0),
            impact: src
                .prop_hash(HASH_IMPACT_EFFECT)
                .map(|s| s.trim().to_owned())
                .filter(|s| !s.is_empty()),
        }
    }

    pub fn share(&self, dt: f32) -> f32 {
        if self.life > 0.0 {
            dt / self.life
        } else {
            1.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::weapon::test_props::List;

    #[test]
    fn the_award_sniper_rifle_reads_its_beam() {
        let props: &[(u32, &str)] = &[
            (0x7c7c_544b, "0.15"),
            (0xfadc_0cd2, "500.0"),
            (0x604e_2441, "1"),
        ];
        let b = BeamClass::from_props(&List(props));
        assert_eq!(
            b,
            BeamClass {
                range: 500.0,
                life: 0.15,
                pierce: true,
                impact: None,
            }
        );
        assert!((b.share(0.05) - 1.0 / 3.0).abs() < 1e-6);
    }

    #[test]
    fn the_ctor_defaults_stand_without_keys() {
        let b = BeamClass::from_props(&List(&[(0x1172_7a92, "com_sfx_weap_beam_exp")]));
        assert_eq!(b.range, 150.0);
        assert_eq!(b.life, 0.0);
        assert!(!b.pierce);
        assert_eq!(b.impact.as_deref(), Some("com_sfx_weap_beam_exp"));
        assert_eq!(b.share(0.05), 1.0);
    }
}
