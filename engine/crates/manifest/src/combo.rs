//! BF2 `comb` data, retained in authored order without runtime defaults.
//! A side's combo hash can differ from the same hash on another side.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// `side_bf2/combos/<side>.ron`, read by weapons from that side.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ComboSet {
    /// True when every field's SCOP presence was retained by the checked bake.
    /// Legacy sets load with false and need a rebake for native metadata use.
    #[serde(default)]
    pub scope_provenance: bool,
    /// True when `loads` records the original hashed LVL resource boundaries.
    #[serde(default)]
    pub load_provenance: bool,
    /// Resource request FNV -> COMB name hashes in on-disc order. Empty
    /// resources stay present; dictionary hashes are checked side-locally.
    #[serde(default)]
    pub loads: BTreeMap<u32, Vec<u32>>,
    pub combos: Vec<ComboDef>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComboDef {
    pub name_hash: u32,
    pub fields: Vec<ComboField>,
}

/// Scope nesting retains transitions' separate If/Or condition groups.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComboField {
    pub keyword: ComboKeyword,
    pub args: Vec<ComboValue>,
    /// An actual SCOP followed this DATA, including an empty scope. Native
    /// animation metadata locks on the first scope, not the first reference.
    /// Older RON remains readable, but must be rebaked for that provenance.
    #[serde(default)]
    pub scoped: bool,
    pub children: Vec<ComboField>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ComboValue {
    Number(f32),
    Text(String),
}

impl ComboField {
    pub fn number(&self, index: usize) -> Option<f32> {
        match self.args.get(index)? {
            ComboValue::Number(n) => Some(*n),
            ComboValue::Text(_) => None,
        }
    }

    pub fn text(&self, index: usize) -> Option<&str> {
        match self.args.get(index)? {
            ComboValue::Number(_) => None,
            ComboValue::Text(s) => Some(s),
        }
    }

    pub fn find(&self, keyword: ComboKeyword) -> Option<&ComboField> {
        self.children.iter().find(|f| f.keyword == keyword)
    }

    pub fn all(&self, keyword: ComboKeyword) -> impl Iterator<Item = &ComboField> {
        self.children.iter().filter(move |f| f.keyword == keyword)
    }
}

impl ComboDef {
    pub fn states(&self) -> impl Iterator<Item = &ComboField> {
        self.fields
            .iter()
            .filter(|f| f.keyword == ComboKeyword::State)
    }

    pub fn state(&self, name: &str) -> Option<&ComboField> {
        self.states()
            .find(|f| f.text(0).is_some_and(|n| n.eq_ignore_ascii_case(name)))
    }

    pub fn find(&self, keyword: ComboKeyword) -> Option<&ComboField> {
        self.fields.iter().find(|f| f.keyword == keyword)
    }
}

/// Hashes are decoded and checked by the offline reader. Unknown keywords fail
/// the bake instead of quietly weakening a hero's combo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComboKeyword {
    RemapReloadButton,
    Animation,
    State,
    ThrowAnimation,
    OffhandAnimation,
    Loop,
    AimType,
    SyncType,
    BlendInTime,
    BlendOutTime,
    BlendTimeIn,
    BlendTimeOut,
    BlendTimeTo,
    LowResPose,
    RestartAnimation,
    PlayExplosion,
    MustShowOneFrame,
    Duration,
    Posture,
    Sound,
    InputLock,
    AlignedToThrust,
    TurnOffLightsaber,
    TurnFactor,
    StrafeFactor,
    ThrustFactor,
    EnergyRestoreRate,
    AnimatedMove,
    VelocityX,
    VelocityY,
    VelocityZ,
    VelocityXZ,
    VelocityXZAngle,
    VelocityYAbs,
    VelocityFromThrust,
    VelocityFromStrafe,
    Gravity,
    GravityVelocityTarget,
    Until,
    Attack,
    DamageTime,
    DamageLength,
    DamageWidth,
    Damage,
    DamageDistance,
    Push,
    Edge,
    AttackId,
    Deflect,
    DeflectAngle,
    EnergyCost,
    DeflectAnimation,
    Transition,
    If,
    Or,
    Break,
    TimeStart,
    TimeEnd,
    Button,
    ButtonsPressed,
    ButtonsReleased,
    Energy,
    Thrust,
    ThrustAngle,
    TimeInPosture,
}
