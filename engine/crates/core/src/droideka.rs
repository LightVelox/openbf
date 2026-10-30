use glam::Vec3;

use crate::fmath;
use crate::weapon::{pf_hash, PropSource, DEG};

pub const IDLE: u8 = 0;
pub const TURN_LEFT: u8 = 1;
pub const TURN_RIGHT: u8 = 2;
pub const DYING: u8 = 3;
pub const DEAD: u8 = 4;
pub const IDLE_TO_LEFT: u8 = 5;
pub const IDLE_TO_RIGHT: u8 = 6;
pub const LEFT_TO_RIGHT: u8 = 7;
pub const RIGHT_TO_LEFT: u8 = 8;
pub const LEFT_TO_IDLE: u8 = 9;
pub const RIGHT_TO_IDLE: u8 = 10;
pub const IDLE_TO_ROLL: u8 = 11;
pub const ROLLING: u8 = 12;
pub const ROLL_TO_IDLE: u8 = 13;
pub const STATES: usize = 14;

pub const EV_STOP: u8 = 0;
pub const EV_STOP_BACK: u8 = 1;
pub const EV_FORWARD: u8 = 2;
pub const EV_BACK: u8 = 3;
pub const EV_TURN_LEFT: u8 = 4;
pub const EV_TURN_RIGHT: u8 = 5;
pub const EV_DIE: u8 = 6;
pub const EV_ROLL: u8 = 7;

pub const DIR_NONE: u8 = 0;
pub const DIR_FORWARD: u8 = 1;
pub const DIR_BACK: u8 = 2;

pub const TRANSITIONS: [[u8; 8]; STATES] = [
    [0, 0, 5, 9, 1, 2, 3, 11],
    [0, 0, 5, 9, 0, 0, 3, 11],
    [0, 0, 5, 10, 0, 0, 3, 11],
    [4, 4, 4, 4, 4, 4, 4, 11],
    [4, 4, 4, 4, 4, 4, 4, 11],
    [9, 0, 7, 10, 9, 9, 3, 11],
    [10, 0, 8, 9, 10, 10, 3, 11],
    [10, 5, 8, 8, 10, 10, 3, 11],
    [9, 7, 7, 7, 9, 9, 3, 11],
    [0, 5, 5, 8, 1, 2, 3, 11],
    [0, 7, 5, 7, 1, 2, 3, 11],
    [12, 12, 12, 12, 12, 12, 4, 12],
    [13, 13, 12, 12, 12, 12, 4, 12],
    [0, 0, 0, 0, 0, 0, 4, 0],
];

pub const CLIP_NAMES: [Option<&str>; STATES] = [
    Some("idle"),
    Some("turnl"),
    Some("turnr"),
    Some("death01"),
    None,
    Some("idle_to_leftfoot"),
    None,
    Some("walk_leftfoot_rightfoot"),
    Some("walk_rightfoot_leftfoot"),
    Some("leftfoot_to_idle"),
    Some("rightfoot_to_idle"),
    Some("fold"),
    Some("roll"),
    Some("unfold"),
];

pub const CLIP_FRAMES: [u16; STATES] = [46, 9, 9, 81, 0, 17, 0, 17, 17, 24, 24, 31, 11, 101];

pub const CLIP_HZ: f32 = 30.0;
pub const GRAVITY: f32 = 9.8;
pub const THREAD_UPDATES: u32 = 2;
pub const BALL_TURN_DEADZONE: f32 = 0.1;
pub const BALL_TURN_MIN_FWD: f32 = 0.2;
pub const BALL_LEAN_RATE: f32 = 0.5;
pub const BALL_BOOST_STICK: f32 = 0.9;
pub const BALL_JUMP_WINDOW: f32 = 0.1;
pub const WALK_MOVE_DEADZONE_SQ: f32 = 0.01;
pub const WALK_BACK_HYSTERESIS: f32 = 0.3;
pub const WALK_LEG_STOP: f32 = 1.4;
pub const TURN_CLIP_ANGLE: f32 = std::f32::consts::FRAC_PI_6;
pub const TURN_RATE_SHARE: f32 = 0.51;
pub const LEG_UNITS: f32 = 6.0;
pub const ENERGY_RECOVER: f32 = crate::soldier::ENERGY_RECOVER_THRESHOLD;

const H_MAX_SPEED: u32 = 2262762340;
const H_MAX_TURN_SPEED: u32 = 1412691337;
const H_TURN_THRESHOLD: u32 = 1186703061;
const H_BALL_RADIUS: u32 = 2559949690;
const H_BALL_MOVE_SPEED: u32 = 2897009420;
const H_BALL_TURN_SPEED: u32 = 1469016806;
const H_BALL_ACCELERATION: u32 = 3731588408;
const H_BALL_ROLLING_FRICTION: u32 = 1700847207;
const H_BALL_MAX_LEAN: u32 = 3537538910;
const H_BALL_SPEED_SCALE: u32 = 3696381401;
const H_BALL_ZERO_STICK_TURN: u32 = 312993597;
const H_BALL_BOOST_SCALE: u32 = 3199954646;
const H_ROLL_TIME: u32 = 3356991735;
const H_UNROLL_TIME: u32 = 839495124;
const H_BALL_JUMP_HEIGHT: u32 = 766138891;
const H_BALL_JUMP_FORWARD: u32 = 948779316;
const H_ENERGY_BAR: u32 = 2534432106;
const H_ENERGY_RESTORE: u32 = 2486096641;
const H_ENERGY_RESTORE_IDLE: u32 = 137277239;
const H_ENERGY_COST_JUMP: u32 = 189431764;
const H_ENERGY_DRAIN_SPRINT: u32 = 323183033;
const H_WALK_WATER_HEIGHT: u32 = 4028454275;
const H_BALL_WATER_HEIGHT: u32 = 2887948593;
const H_WATER_INTERVAL: u32 = 1531517644;
const H_WATER_AMOUNT: u32 = 2190645677;
const H_PITCH_LIMITS: u32 = 872657209;
const H_MAX_SHIELD: u32 = 3660878268;
const H_ADD_SHIELD: u32 = 3134802965;
const H_ADD_SHIELD_OFF: u32 = 1922955898;
const H_SHIELD_OFFSET: u32 = 494035731;
const H_SHIELD_RADIUS: u32 = 1519518134;
const H_SHIELD_SOUND: u32 = 0xd855_03ff;
const H_SHIELD_OFF_SOUND: u32 = 0x5df6_c1a4;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sphere {
    pub center: Vec3,
    pub radius: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ShieldClass {
    pub max: f32,
    pub rate_on: f32,
    pub rate_off: f32,
    pub sphere: Sphere,
    pub drains_full: bool,
    pub sound: Option<String>,
    pub off_sound: Option<String>,
}

impl Default for ShieldClass {
    fn default() -> Self {
        Self {
            max: 3000.0,
            rate_on: -100.0,
            rate_off: 300.0,
            sphere: Sphere {
                center: Vec3::new(0.0, 1.0, 0.125),
                radius: 1.25,
            },
            drains_full: false,
            sound: None,
            off_sound: None,
        }
    }
}

impl ShieldClass {
    pub fn from_props(src: &dyn PropSource) -> Self {
        let d = Self::default();
        let center = src
            .prop_hash(H_SHIELD_OFFSET)
            .and_then(vec3)
            .unwrap_or(d.sphere.center);
        Self {
            max: pf_hash(src, H_MAX_SHIELD).unwrap_or(d.max),
            rate_on: pf_hash(src, H_ADD_SHIELD).unwrap_or(d.rate_on),
            rate_off: pf_hash(src, H_ADD_SHIELD_OFF).unwrap_or(d.rate_off),
            sphere: Sphere {
                center,
                radius: pf_hash(src, H_SHIELD_RADIUS).unwrap_or(d.sphere.radius),
            },
            drains_full: d.drains_full,
            sound: name_hash(src, H_SHIELD_SOUND),
            off_sound: name_hash(src, H_SHIELD_OFF_SOUND),
        }
    }
}

fn name_hash(src: &dyn PropSource, hash: u32) -> Option<String> {
    let v = src.prop_hash(hash)?.trim().trim_matches('"');
    (!v.is_empty()).then(|| v.to_owned())
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClipInfo {
    pub frames: u16,
    pub root_travel: f32,
}

impl ClipInfo {
    pub fn duration(&self) -> f32 {
        if self.frames < 2 {
            0.0
        } else {
            f32::from(self.frames - 1) / CLIP_HZ
        }
    }

    pub fn natural_speed(&self) -> f32 {
        let d = self.duration();
        if d > 0.0 {
            self.root_travel / d
        } else {
            0.0
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DroidekaClass {
    pub max_speed: f32,
    pub max_turn_speed: f32,
    pub max_pitch_speed: f32,
    pub pitch_limits: (f32, f32),
    pub turn_threshold: f32,
    pub ball_radius: f32,
    pub ball_move_speed: f32,
    pub ball_turn_speed: f32,
    pub ball_zero_stick_turn: f32,
    pub ball_boost_turn: f32,
    pub ball_acceleration: f32,
    pub ball_friction: f32,
    pub ball_max_lean: f32,
    pub ball_speed_scale: f32,
    pub ball_boost_scale: f32,
    pub roll_time: f32,
    pub unroll_time: f32,
    pub jump_vy: f32,
    pub jump_forward: f32,
    pub energy_bar: f32,
    pub energy_restore: f32,
    pub energy_restore_idle: f32,
    pub energy_cost_jump: f32,
    pub energy_drain_boost: f32,
    pub walk_water_height: f32,
    pub ball_water_height: f32,
    pub water_interval: f32,
    pub water_amount: f32,
    pub clips: [ClipInfo; STATES],
    pub rate: [f32; STATES],
    pub speed: [f32; STATES],
    pub duration: [f32; STATES],
    pub shield: ShieldClass,
    pub walk_body: Sphere,
    pub walk_soldier_radius: f32,
    pub walk_height: f32,
    pub walk_ordnance: Vec<Sphere>,
    pub guns: Vec<Vec3>,
    pub eye: Vec3,
    pub explosion: Option<crate::weapon::ExplosionClass>,
    pub cameras: Vec<(u32, crate::unit::CameraSection)>,
}

pub const CAMERA_STAND: u32 = 1_967_731_491;
pub const CAMERA_STAND_ZOOM: u32 = 2_497_170_924;
pub const CAMERA_CROUCH: u32 = 1_313_842_571;

pub const BF1_WALK_NATURAL_SPEED: f32 = 1.13;

pub fn default_clips() -> [ClipInfo; STATES] {
    let mut clips = [ClipInfo {
        frames: 0,
        root_travel: 0.0,
    }; STATES];
    for (s, c) in clips.iter_mut().enumerate() {
        c.frames = CLIP_FRAMES[s];
        if matches!(s as u8, LEFT_TO_RIGHT | RIGHT_TO_LEFT) {
            c.root_travel = BF1_WALK_NATURAL_SPEED * c.duration();
        }
    }
    clips
}

impl Default for DroidekaClass {
    fn default() -> Self {
        let mut c = Self {
            max_speed: 2.5,
            max_turn_speed: 1.25,
            max_pitch_speed: crate::soldier::SoldierConstants::default().max_pitch_speed,
            pitch_limits: (-25.0 * DEG, 25.0 * DEG),
            turn_threshold: 30.0 * DEG,
            ball_radius: 0.65,
            ball_move_speed: 6.0,
            ball_turn_speed: 2.0,
            ball_zero_stick_turn: 2.5,
            ball_boost_turn: 1.0,
            ball_acceleration: 100.0,
            ball_friction: 50.0,
            ball_max_lean: 10.0 * DEG,
            ball_speed_scale: 1.0,
            ball_boost_scale: 2.0,
            roll_time: 0.5,
            unroll_time: 1.66,
            jump_vy: (2.0 * GRAVITY * 1.27f32).sqrt(),
            jump_forward: 1.25,
            energy_bar: 100.0,
            energy_restore: 10.0,
            energy_restore_idle: 15.0,
            energy_cost_jump: 30.0,
            energy_drain_boost: 30.0,
            walk_water_height: 1.5,
            ball_water_height: 1.5,
            water_interval: 1.0,
            water_amount: 10.0,
            clips: default_clips(),
            rate: [1.0; STATES],
            speed: [0.0; STATES],
            duration: [0.0; STATES],
            shield: ShieldClass::default(),
            walk_body: Sphere {
                center: Vec3::new(0.0, 0.992, 0.421),
                radius: 1.0,
            },
            walk_soldier_radius: 1.0,
            walk_height: 2.0,
            walk_ordnance: Vec::new(),
            guns: Vec::new(),
            eye: Vec3::new(0.0, 1.8, 0.0),
            explosion: None,
            cameras: Vec::new(),
        };
        c.derive_rates();
        c
    }
}

impl DroidekaClass {
    pub fn from_props(src: &dyn PropSource, shield: Option<&dyn PropSource>) -> Self {
        let d = Self::default();
        let f = |h: u32, v: f32| pf_hash(src, h).unwrap_or(v);
        let threshold = pf_hash(src, H_TURN_THRESHOLD).map_or(d.turn_threshold, |t| t * DEG);
        let pitch_limits = src
            .prop_hash(H_PITCH_LIMITS)
            .and_then(|s| {
                let mut it = s.split_whitespace().filter_map(|t| t.parse::<f32>().ok());
                Some((it.next()? * DEG, it.next()? * DEG))
            })
            .unwrap_or(d.pitch_limits);
        let jump_height = pf_hash(src, H_BALL_JUMP_HEIGHT);
        let mut c = Self {
            max_speed: f(H_MAX_SPEED, d.max_speed),
            max_turn_speed: f(H_MAX_TURN_SPEED, d.max_turn_speed),
            pitch_limits,
            turn_threshold: threshold,
            ball_radius: f(H_BALL_RADIUS, d.ball_radius),
            ball_move_speed: f(H_BALL_MOVE_SPEED, d.ball_move_speed),
            ball_turn_speed: f(H_BALL_TURN_SPEED, d.ball_turn_speed),
            ball_zero_stick_turn: f(H_BALL_ZERO_STICK_TURN, d.ball_zero_stick_turn),
            ball_acceleration: f(H_BALL_ACCELERATION, d.ball_acceleration),
            ball_friction: f(H_BALL_ROLLING_FRICTION, d.ball_friction),
            ball_max_lean: f(H_BALL_MAX_LEAN, d.ball_max_lean / DEG) * DEG,
            ball_speed_scale: f(H_BALL_SPEED_SCALE, d.ball_speed_scale),
            ball_boost_scale: f(H_BALL_BOOST_SCALE, d.ball_boost_scale),
            roll_time: f(H_ROLL_TIME, d.roll_time),
            unroll_time: f(H_UNROLL_TIME, d.unroll_time),
            jump_vy: jump_height.map_or(d.jump_vy, |h| (2.0 * GRAVITY * h).sqrt()),
            jump_forward: f(H_BALL_JUMP_FORWARD, d.jump_forward),
            energy_bar: f(H_ENERGY_BAR, d.energy_bar),
            energy_restore: f(H_ENERGY_RESTORE, d.energy_restore),
            energy_restore_idle: f(H_ENERGY_RESTORE_IDLE, d.energy_restore_idle),
            energy_cost_jump: f(H_ENERGY_COST_JUMP, d.energy_cost_jump),
            energy_drain_boost: f(H_ENERGY_DRAIN_SPRINT, d.energy_drain_boost),
            walk_water_height: f(H_WALK_WATER_HEIGHT, d.walk_water_height),
            ball_water_height: f(H_BALL_WATER_HEIGHT, d.ball_water_height),
            water_interval: f(H_WATER_INTERVAL, d.water_interval),
            water_amount: f(H_WATER_AMOUNT, d.water_amount),
            shield: shield.map_or(d.shield.clone(), ShieldClass::from_props),
            ..d
        };
        c.derive_rates();
        c
    }

    pub fn with_clips(mut self, clips: [ClipInfo; STATES]) -> Self {
        self.clips = clips;
        self.derive_rates();
        self
    }

    pub fn derive_rates(&mut self) {
        for s in 0..STATES {
            self.rate[s] = 1.0;
            self.duration[s] = self.clips[s].duration();
            self.speed[s] = self.clips[s].natural_speed();
        }
        for s in [TURN_LEFT, TURN_RIGHT] {
            let d = self.duration[usize::from(s)];
            let sign = if s == TURN_LEFT { 1.0 } else { -1.0 };
            self.speed[usize::from(s)] = if d > 0.0 {
                sign * TURN_CLIP_ANGLE / (d * LEG_UNITS)
            } else {
                0.0
            };
        }
        let walk = self.speed[usize::from(LEFT_TO_RIGHT)];
        if walk > 0.0 {
            for s in [
                IDLE_TO_LEFT,
                LEFT_TO_RIGHT,
                RIGHT_TO_LEFT,
                LEFT_TO_IDLE,
                RIGHT_TO_IDLE,
            ] {
                self.rate[usize::from(s)] = self.max_speed / walk;
            }
        }
        let turn = self.speed[usize::from(TURN_LEFT)];
        if turn > 0.0 {
            let r = self.max_turn_speed * TURN_RATE_SHARE / turn;
            self.rate[usize::from(TURN_LEFT)] = r;
            self.rate[usize::from(TURN_RIGHT)] = r;
        }
        if self.roll_time > 0.0 {
            self.rate[usize::from(IDLE_TO_ROLL)] =
                self.duration[usize::from(IDLE_TO_ROLL)] / self.roll_time;
        }
        if self.unroll_time > 0.0 {
            self.rate[usize::from(ROLL_TO_IDLE)] =
                self.duration[usize::from(ROLL_TO_IDLE)] / self.unroll_time;
        }
    }

    pub fn state_time(&self, s: u8) -> f32 {
        let s = usize::from(s);
        if self.rate[s] > 0.0 {
            self.duration[s] / self.rate[s]
        } else {
            0.0
        }
    }

    pub fn camera_section(&self, form: Form, zoomed: bool) -> Option<crate::unit::CameraSection> {
        let want = match (form, zoomed) {
            (Form::Ball, _) => CAMERA_CROUCH,
            (Form::Walk, true) => CAMERA_STAND_ZOOM,
            (Form::Walk, false) => CAMERA_STAND,
        };
        self.cameras
            .iter()
            .find(|(name, _)| *name == want)
            .map(|(_, s)| *s)
    }

    pub fn ball_omega_cap(&self) -> f32 {
        self.ball_move_speed * self.ball_speed_scale / self.ball_radius
    }

    pub fn ball_sphere(&self) -> Sphere {
        Sphere {
            center: Vec3::new(0.0, self.ball_radius + 0.1, -0.2),
            radius: self.ball_radius,
        }
    }

    pub fn gun(&self, barrel: u8) -> Vec3 {
        if self.guns.is_empty() {
            self.shield.sphere.center
        } else {
            self.guns[usize::from(barrel) % self.guns.len()]
        }
    }

    pub fn ordnance_spheres(&self, form: Form) -> Vec<Sphere> {
        match form {
            Form::Ball => vec![self.ball_sphere()],
            Form::Walk if self.walk_ordnance.is_empty() => vec![self.walk_body],
            Form::Walk => self.walk_ordnance.clone(),
        }
    }
}

fn vec3(s: &str) -> Option<Vec3> {
    let mut it = s.split_whitespace().filter_map(|t| t.parse::<f32>().ok());
    Some(Vec3::new(it.next()?, it.next()?, it.next()?))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Form {
    Walk,
    Ball,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Cue {
    Fold,
    Unfold,
    ShieldOn,
    ShieldOff,
    Jump,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum Order {
    Deploy,
    #[default]
    Roll,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Input {
    pub fwd: f32,
    pub strafe: f32,
    pub turn: f32,
    pub fold: bool,
    pub shield: bool,
    pub jump: bool,
    pub boost: bool,
    pub human: bool,
    pub cramped: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Shield {
    pub points: f32,
    pub on: bool,
}

impl Shield {
    pub fn full(c: &ShieldClass) -> Self {
        Self {
            points: c.max,
            on: false,
        }
    }

    pub fn tick(
        &mut self,
        dt: f32,
        press: bool,
        forced_off: bool,
        c: &ShieldClass,
    ) -> Option<bool> {
        let was = self.on;
        if self.points <= 0.0 || forced_off {
            self.on = false;
        } else if press {
            self.on = !self.on;
        }
        let rate = if self.on { c.rate_on } else { c.rate_off };
        if c.drains_full || self.points < c.max {
            self.points = (self.points + dt * rate).clamp(0.0, c.max);
        }
        (was != self.on).then_some(self.on)
    }

    pub fn absorb_bolt(&mut self, damage: f32, shield_scale: f32) {
        self.points = (self.points - damage * shield_scale).max(0.0);
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Body {
    pub pos: Vec3,
    pub vel: Vec3,
    pub yaw: f32,
    pub grounded: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Report {
    pub entered: Option<u8>,
    pub left: Option<u8>,
    pub weapons_off: bool,
    pub weapons_on: bool,
    pub shield: Option<bool>,
    pub jumped: bool,
    pub water_damage: f32,
    pub walk_step: f32,
}

impl Report {
    pub fn cues(&self) -> impl Iterator<Item = Cue> + '_ {
        let fold = (self.entered == Some(IDLE_TO_ROLL)).then_some(Cue::Fold);
        let unfold = (self.entered == Some(ROLL_TO_IDLE)).then_some(Cue::Unfold);
        let shield = self
            .shield
            .map(|on| if on { Cue::ShieldOn } else { Cue::ShieldOff });
        let jump = self.jumped.then_some(Cue::Jump);
        fold.into_iter().chain(unfold).chain(shield).chain(jump)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Droideka {
    pub state: u8,
    pub dir: u8,
    pub clip_t: f32,
    pub legs: f32,
    pub omega: f32,
    pub spin: [f32; 2],
    pub lean: f32,
    pub boosting: bool,
    pub energy: f32,
    pub depleted: bool,
    pub air: f32,
    pub water: f32,
    pub shield: Shield,
    pub order: Order,
    pub prev_buttons: u32,
    pub alive: bool,
    pub barrel: u8,
}

impl Droideka {
    pub fn new(c: &DroidekaClass, order: Order) -> Self {
        Self {
            state: IDLE,
            dir: DIR_NONE,
            clip_t: 0.0,
            legs: 0.0,
            omega: 0.0,
            spin: [1.0, 0.0],
            lean: 0.0,
            boosting: false,
            energy: c.energy_bar,
            depleted: false,
            air: 0.0,
            water: c.water_interval,
            shield: Shield::full(&c.shield),
            order,
            prev_buttons: 0,
            alive: true,
            barrel: 0,
        }
    }

    pub fn form(&self) -> Form {
        if self.state == ROLLING {
            Form::Ball
        } else {
            Form::Walk
        }
    }

    pub fn in_transform(&self) -> bool {
        matches!(self.state, IDLE_TO_ROLL | ROLLING | ROLL_TO_IDLE)
    }

    pub fn no_fire(&self) -> bool {
        self.in_transform() || !self.alive
    }

    pub fn walk_state(&self) -> bool {
        self.state <= RIGHT_TO_IDLE && !matches!(self.state, DYING | DEAD)
    }

    pub fn raise(&mut self, ev: u8, c: &DroidekaClass, body: &mut Body, report: &mut Report) {
        let ev = match ev {
            EV_STOP if self.dir == DIR_BACK => EV_STOP_BACK,
            e => e,
        };
        match ev {
            EV_FORWARD => self.dir = DIR_FORWARD,
            EV_BACK => self.dir = DIR_BACK,
            _ => {}
        }
        if self.state == ROLLING {
            body.vel = Vec3::ZERO;
            self.legs = 0.0;
            self.lean = 0.0;
        }
        let next = TRANSITIONS[usize::from(self.state)][usize::from(ev)];
        self.enter(next, c, report);
        if self.state == ROLLING {
            self.omega = 0.0;
            self.spin = [1.0, 0.0];
            self.legs = 0.0;
        }
    }

    fn enter(&mut self, next: u8, c: &DroidekaClass, report: &mut Report) {
        let prev = self.state;
        if next != DEAD {
            if next <= DEAD || next >= IDLE_TO_ROLL {
                self.dir = DIR_NONE;
            }
            if c.clips[usize::from(next)].frames > 0 {
                self.clip_t = if self.dir == DIR_BACK {
                    c.duration[usize::from(next)]
                } else {
                    0.0
                };
            }
            if prev != next {
                if next == IDLE_TO_ROLL {
                    report.weapons_off = true;
                } else if next == IDLE && prev == ROLL_TO_IDLE {
                    report.weapons_on = true;
                }
            }
        }
        self.state = next;
        if prev != next {
            report.left = Some(prev);
            report.entered = Some(next);
        }
    }

    pub fn turn_body(&mut self, body: &mut Body, delta: f32) {
        body.yaw -= delta;
        if self.state != ROLLING {
            self.legs = wrap(self.legs + delta);
        }
    }

    pub fn step(&mut self, dt: f32, input: &Input, c: &DroidekaClass, body: &mut Body) -> Report {
        let mut report = Report::default();
        if !self.alive {
            if !matches!(self.state, DEAD) {
                self.raise(EV_DIE, c, body, &mut report);
            }
            self.shield.on = false;
            return report;
        }
        if input.human && input.fold {
            if self.state == ROLLING {
                self.order = Order::Deploy;
            } else if self.walk_state() {
                self.order = Order::Roll;
            }
        }
        if self.state == ROLLING {
            if self.order == Order::Deploy && body.grounded && !input.cramped {
                self.raise(EV_STOP, c, body, &mut report);
            }
        } else if self.walk_state() && self.order == Order::Roll {
            self.raise(EV_ROLL, c, body, &mut report);
        }
        if let Some(on) = self
            .shield
            .tick(dt, input.shield, self.in_transform(), &c.shield)
        {
            report.shield = Some(on);
        }
        if self.state == ROLLING {
            self.ball(dt, input, c, body, &mut report);
        } else {
            self.walk(dt, input, c, body, &mut report);
        }
        self.tick_energy(dt, input, c);
        let (sy, cy) = fmath::sin_cos(body.yaw);
        if self.state == ROLLING {
            let v = self.omega * c.ball_radius / c.ball_speed_scale;
            body.vel.x = -sy * v;
            body.vel.z = -cy * v;
            for _ in 0..THREAD_UPDATES {
                body.pos += body.vel * dt;
                body.vel.y -= GRAVITY * dt;
            }
        } else {
            for _ in 0..THREAD_UPDATES {
                body.pos.y += body.vel.y * dt;
                body.vel.y -= GRAVITY * dt;
            }
        }
        report
    }

    pub fn land(&mut self, body: &mut Body, grounded: bool, dt: f32) {
        body.grounded = grounded;
        if grounded {
            if body.vel.y < 0.0 {
                body.vel.y = 0.0;
            }
            self.air = 0.0;
        } else {
            self.air += dt;
        }
    }

    pub fn water_tick(&mut self, dt: f32, depth: Option<f32>, c: &DroidekaClass) -> f32 {
        let limit = if self.state == ROLLING {
            c.ball_water_height
        } else {
            c.walk_water_height
        };
        if depth.is_some_and(|d| d >= limit) {
            if self.water > 0.0 {
                self.water -= dt;
                0.0
            } else {
                c.water_amount
            }
        } else {
            self.water = c.water_interval;
            0.0
        }
    }

    fn tick_energy(&mut self, dt: f32, input: &Input, c: &DroidekaClass) {
        if self.boosting {
            self.energy -= c.energy_drain_boost * dt;
            if self.energy <= 0.0 {
                self.energy = 0.0;
                self.depleted = true;
                self.boosting = false;
            }
        } else {
            let idle = input.fwd == 0.0 && input.strafe == 0.0 && input.turn == 0.0;
            let rate = if idle {
                c.energy_restore_idle
            } else {
                c.energy_restore
            };
            self.energy = (self.energy + rate * dt).min(c.energy_bar);
        }
        if self.depleted && self.energy >= ENERGY_RECOVER {
            self.depleted = false;
        }
    }

    fn pay_energy(&mut self, cost: f32) -> bool {
        if cost <= 0.0 {
            return true;
        }
        if self.depleted {
            return false;
        }
        self.energy -= cost;
        if self.energy < 0.0 {
            self.energy = 0.0;
            self.depleted = true;
        }
        true
    }

    fn boost_on(&mut self) -> bool {
        if self.state != ROLLING || self.depleted {
            return false;
        }
        self.boosting = true;
        true
    }

    fn jump(&mut self, c: &DroidekaClass, body: &mut Body, report: &mut Report) {
        if self.state != ROLLING || self.air >= BALL_JUMP_WINDOW {
            return;
        }
        if !self.pay_energy(c.energy_cost_jump) {
            return;
        }
        self.air = BALL_JUMP_WINDOW;
        body.vel.y += c.jump_vy;
        let (sy, cy) = fmath::sin_cos(body.yaw);
        let facing = Vec3::new(-sy, 0.0, -cy);
        let along = body.vel.dot(facing);
        if along > 0.0 {
            body.vel += facing * along * (c.jump_forward - 1.0);
        }
        report.jumped = true;
    }

    fn ball(
        &mut self,
        dt: f32,
        input: &Input,
        c: &DroidekaClass,
        body: &mut Body,
        report: &mut Report,
    ) {
        let mut fwd = input.fwd;
        let turn = input.turn;
        if input.human && turn * turn > BALL_TURN_DEADZONE * BALL_TURN_DEADZONE {
            if (0.0..BALL_TURN_MIN_FWD).contains(&fwd) {
                fwd = BALL_TURN_MIN_FWD;
            } else if fwd < 0.0 && -fwd < BALL_TURN_MIN_FWD {
                fwd = -BALL_TURN_MIN_FWD;
            }
        }
        let rate = if !input.human {
            c.max_turn_speed
        } else if self.boosting {
            c.ball_boost_turn
        } else {
            c.ball_zero_stick_turn + (c.ball_turn_speed - c.ball_zero_stick_turn) * input.fwd.abs()
        };
        let d = turn * rate * dt;
        body.yaw -= d;
        let sign = if fwd > 0.0 { 1.0 } else { -1.0 };
        let step = dt * BALL_LEAN_RATE;
        let diff = turn * sign * c.ball_max_lean - self.lean;
        let lean = if step < diff {
            self.lean + step
        } else if step < -diff {
            self.lean - step
        } else {
            self.lean + diff
        };
        self.lean = lean / (1.0 + lean * lean).sqrt();
        let accel;
        if input.jump {
            self.boosting = false;
            self.jump(c, body, report);
            accel = 0.0;
        } else if fwd > BALL_BOOST_STICK && (input.boost || self.boosting) && self.boost_on() {
            accel = fwd * c.ball_acceleration * c.ball_boost_scale;
        } else {
            self.boosting = false;
            accel = fwd * c.ball_acceleration;
        }
        self.omega += dt * accel;
        if body.grounded {
            let fr = c.ball_friction * dt;
            self.omega = if fr < self.omega {
                self.omega - fr
            } else if self.omega < -fr {
                self.omega + fr
            } else {
                0.0
            };
        }
        let mut cap = c.ball_omega_cap();
        if self.boosting {
            cap *= c.ball_boost_scale;
        }
        self.omega = self.omega.clamp(-cap, cap);
        let h = dt * self.omega * 0.5;
        let (a, b) = (self.spin[0], self.spin[1]);
        let nb = b + a * h;
        let na = a - b * h;
        let n = (na * na + nb * nb).sqrt();
        self.spin = [na / n, nb / n];
    }

    fn walk(
        &mut self,
        dt: f32,
        input: &Input,
        c: &DroidekaClass,
        body: &mut Body,
        report: &mut Report,
    ) {
        let (mut fwd, mut strafe) = (input.fwd, input.strafe);
        let mut moving = fwd * fwd + strafe * strafe > WALK_MOVE_DEADZONE_SQ;
        let rate = c.max_turn_speed;
        self.turn_body(body, input.turn * dt * rate);
        let mut forward = false;
        if self.state != IDLE {
            if !matches!(self.state, TURN_LEFT | TURN_RIGHT) {
                let mut target = 0.0;
                if moving {
                    let (ls, lc) = fmath::sin_cos(self.legs / LEG_UNITS);
                    let thr = if lc * fwd - ls * strafe > 0.0 {
                        -WALK_BACK_HYSTERESIS
                    } else {
                        WALK_BACK_HYSTERESIS
                    };
                    forward = thr <= fwd;
                    if forward {
                        strafe = -strafe;
                    } else {
                        fwd = -fwd;
                    }
                    target = fmath::atan2(strafe, fwd);
                }
                let diff = wrap(target - self.legs);
                if diff.abs() > WALK_LEG_STOP {
                    forward = false;
                    moving = false;
                }
                let lim = c.max_turn_speed * dt;
                self.legs = wrap(self.legs + diff.clamp(-lim, lim));
            }
        } else {
            let mut target = 0.0;
            if moving {
                let (s, f) = if fwd >= 0.0 {
                    (-strafe, fwd)
                } else {
                    (strafe, -fwd)
                };
                target = fmath::atan2(s, f);
            }
            let diff = target - self.legs;
            if diff > c.turn_threshold {
                self.raise(EV_TURN_LEFT, c, body, report);
            } else if diff < -c.turn_threshold {
                self.raise(EV_TURN_RIGHT, c, body, report);
            } else if moving {
                forward = fwd >= 0.0;
                self.raise(if forward { EV_FORWARD } else { EV_BACK }, c, body, report);
            }
        }
        if dt <= 0.0 {
            return;
        }
        if moving {
            if forward && self.dir == DIR_BACK {
                self.dir = DIR_FORWARD;
            } else if !forward && self.dir == DIR_FORWARD {
                self.dir = DIR_BACK;
            }
        }
        let mut left = dt;
        let mut travel = 0.0;
        let s = usize::from(self.state);
        let rate = c.rate[s];
        let back = self.dir == DIR_BACK;
        let remain = if back {
            self.clip_t
        } else {
            c.duration[s] - self.clip_t
        };
        if rate * left >= remain {
            left -= remain / rate;
            self.clip_t += if back { -remain } else { remain };
            travel += self.advance(remain, back, c);
            if !self.alive {
                self.raise(EV_DIE, c, body, report);
            } else if moving {
                self.raise(if forward { EV_FORWARD } else { EV_BACK }, c, body, report);
            } else {
                self.raise(EV_STOP, c, body, report);
            }
            if self.state == ROLLING {
                return;
            }
        }
        let s = usize::from(self.state);
        let delta = left * c.rate[s];
        let back = self.dir == DIR_BACK;
        if !back || delta <= self.clip_t {
            self.clip_t += if back { -delta } else { delta };
        }
        travel += self.advance(delta, back, c);
        self.apply_walk(travel, dt, body, report);
    }

    fn advance(&mut self, delta: f32, back: bool, c: &DroidekaClass) -> f32 {
        let s = usize::from(self.state);
        match self.state {
            IDLE | DYING | DEAD | ROLLING => 0.0,
            TURN_LEFT | TURN_RIGHT => {
                self.legs = wrap(self.legs + delta * c.speed[s] * LEG_UNITS);
                0.0
            }
            _ => c.speed[s] * delta * if back { -1.0 } else { 1.0 },
        }
    }

    fn apply_walk(&mut self, travel: f32, dt: f32, body: &mut Body, report: &mut Report) {
        let heading = body.yaw + self.legs;
        let (sy, cy) = fmath::sin_cos(heading);
        let dir = Vec3::new(-sy, 0.0, -cy);
        body.pos += dir * travel;
        let v = if dt > 0.0 { travel / dt } else { 0.0 };
        body.vel.x = dir.x * v;
        body.vel.z = dir.z * v;
        report.walk_step = travel;
    }

    pub fn hash_into(&self, eat: &mut impl FnMut(&[u8])) {
        eat(&[
            0xde,
            self.state,
            self.dir,
            self.boosting as u8,
            self.depleted as u8,
            self.shield.on as u8,
            self.order as u8,
            self.alive as u8,
            self.barrel,
        ]);
        for v in [
            self.clip_t,
            self.legs,
            self.omega,
            self.spin[0],
            self.spin[1],
            self.lean,
            self.energy,
            self.air,
            self.water,
            self.shield.points,
        ] {
            eat(&v.to_bits().to_le_bytes());
        }
        eat(&self.prev_buttons.to_le_bytes());
    }
}

pub fn wrap(a: f32) -> f32 {
    let tau = std::f32::consts::TAU;
    let mut a = a % tau;
    if a > std::f32::consts::PI {
        a -= tau;
    } else if a < -std::f32::consts::PI {
        a += tau;
    }
    a
}

pub fn local_to_world(pos: Vec3, yaw: f32, local: Vec3) -> Vec3 {
    let (sy, cy) = fmath::sin_cos(yaw);
    let right = Vec3::new(cy, 0.0, -sy);
    let fwd = Vec3::new(-sy, 0.0, -cy);
    pos + right * local.x + Vec3::Y * local.y + fwd * local.z
}

pub fn ray_sphere(from: Vec3, dir: Vec3, max_t: f32, s: Sphere) -> Option<f32> {
    let oc = from - s.center;
    let b = oc.dot(dir);
    let cc = oc.length_squared() - s.radius * s.radius;
    if cc <= 0.0 {
        return None;
    }
    let disc = b * b - cc;
    if disc < 0.0 {
        return None;
    }
    let t = -b - disc.sqrt();
    (t >= 0.0 && t <= max_t).then_some(t)
}

#[cfg(test)]
mod tests;
