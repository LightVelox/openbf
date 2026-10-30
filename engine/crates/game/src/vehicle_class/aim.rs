use super::{float, floats, h};

#[derive(Clone, Debug, PartialEq)]
pub struct AimerSpec {
    pub node: String,
    pub weapon: usize,
    pub fire_points: Vec<String>,
    pub yaw: [f32; 2],
    pub pitch: [f32; 2],
    pub barrels: Vec<(String, f32)>,
}

impl AimerSpec {
    pub(super) fn new(node: &str, weapon: usize) -> Self {
        Self {
            node: node.trim().into(),
            weapon,
            fire_points: Vec::new(),
            yaw: [-30.0, 30.0],
            pitch: [-45.0, 45.0],
            barrels: Vec::new(),
        }
    }

    pub(super) fn set(&mut self, k: u32, v: &str) {
        if k == h("FirePointName") {
            self.fire_points.push(v.trim().into());
        } else if k == h("AimerYawLimits") {
            if let Some(l) = floats(v) {
                self.yaw = l;
            }
        } else if k == h("AimerPitchLimits") {
            if let Some(l) = floats(v) {
                self.pitch = l;
            }
        } else if k == h("BarrelNodeName") {
            self.barrels.push((v.trim().into(), 0.0));
        } else if k == h("BarrelRecoil") || k == h("BarrelLength") {
            if let (Some(b), Some(x)) = (self.barrels.last_mut(), float(v)) {
                b.1 = if k == h("BarrelLength") { x * 0.25 } else { x };
            }
        }
    }
}
