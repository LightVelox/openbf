use super::{pf_hash, PropSource};

const HASH_MAX_DAMAGE: u32 = 0x47d0_e794;
const HASH_MIN_DAMAGE: u32 = 0xfe54_4f96;
const HASH_DAMAGE_DEDUCTION: u32 = 0xaf92_2a6b;
const HASH_RAY_RADIUS: u32 = 0xa4c5_3efd;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FatRayClass {
    pub max_damage: f32,
    pub min_damage: f32,
    pub damage_deduction: f32,
    pub radius: f32,
}

impl Default for FatRayClass {
    fn default() -> Self {
        Self {
            max_damage: 20.0,
            min_damage: 10.0,
            damage_deduction: 5.0,
            radius: 3.0,
        }
    }
}

impl FatRayClass {
    pub fn from_props(src: &dyn PropSource) -> Self {
        let d = Self::default();
        Self {
            max_damage: pf_hash(src, HASH_MAX_DAMAGE).unwrap_or(d.max_damage),
            min_damage: pf_hash(src, HASH_MIN_DAMAGE).unwrap_or(d.min_damage),
            damage_deduction: pf_hash(src, HASH_DAMAGE_DEDUCTION).unwrap_or(d.damage_deduction),
            radius: pf_hash(src, HASH_RAY_RADIUS).unwrap_or(d.radius),
        }
    }

    pub fn damage(&self, charge: f32) -> f32 {
        self.min_damage + (self.max_damage - self.min_damage) * charge
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::weapon::test_props::List;

    #[test]
    fn the_sonic_blast_reads_its_class_over_the_ctor() {
        let props: &[(u32, &str)] = &[
            (2_945_591_915, "5.0"),
            (2_933_939_609, "0.0"),
            (1_053_891_267, "0.0"),
            (1_487_395_364, "1.0"),
            (2_764_390_141, "1.0"),
            (1_204_873_108, "100.0"),
        ];
        let f = FatRayClass::from_props(&List(props));
        assert_eq!(
            f,
            FatRayClass {
                max_damage: 100.0,
                min_damage: 10.0,
                damage_deduction: 5.0,
                radius: 1.0,
            }
        );
        assert_eq!(f.damage(1.0), 100.0);
        assert_eq!(f.damage(0.0), 10.0);
        assert_eq!(FatRayClass::from_props(&List(&[])), FatRayClass::default());
    }
}
