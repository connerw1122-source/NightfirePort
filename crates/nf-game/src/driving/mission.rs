//! The driving missions' mission-script state machine, headless — the `Rule`/`MSet`/`Trgr`
//! chunks of the track `.crp` (inside the mission `.VIV`), the objective-channel tracker, and
//! the mission end with the right PTP result.
//!
//! Evidence base (all [M] unless noted — DRIVING.ELF decompile under `work/decomp/DRIVING.ELF/`,
//! addresses = EE; the research notes are in docs/systems/drive-mission-scripts.md):
//! - Loader: "Init World" `FUN_0022faf8` 0x22faf8 finds `Map ` (pool record) and resolves its
//!   sub-records `Rule` 0x52756c65, `MSet` 0x4d536574, `Trgr` 0x54726772 (`:108-129,187-194`).
//! - Rules: `FUN_00205220` 0x205220 walks the `Rule` payload as **0x20-byte entries**
//!   (`iVar5 += 0x20`, count = record+8) and builds one runtime object per entry through
//!   `FUN_00209390` 0x209390 — a 15-way type switch; the object's group id (entry+8) buckets
//!   it at mgr+0x24+g*4 (mgr = `DAT_0033a024`; g < 0 → the default list mgr+0x20).
//! - Rule base ctor `FUN_00209318` 0x209318: obj+0 = entry+0xc (the name hash), obj+4/8/0xc =
//!   entry+4 bits 0/1/2 (three flag booleans), obj+0x10 = the per-type descriptor.
//! - Type names: GCC typeinfo strings at 0x397c28.. ("12SMissionRule", "11SRuleAlways",
//!   "9SRuleAmmo", … "16SRuleProgCounter") [D]; sizes + inits from the `FUN_00209390` switch.
//! - Mission set: `FUN_00204d18` 0x204d18 names the set from the MSet payload (defaults
//!   "lotus"/"playervan"/"playerlimo"/"playertaxi"/"playersmart" by `DAT_0033a010..020`);
//!   `FUN_00205498` 0x205498 registers up to 0x20 objective channels from the MSet table
//!   (enabled byte at +0x2c+i, target at +0x4c+i*4, params at +0xcc/+0x14c+i*4, skipped when
//!   target >= 0x2711) and sets the active channel through `FUN_0020d488` 0x20d488
//!   (display+4 = index; the first enabled non-zero index, else 0).
//! - Channel entries: `FUN_0020d378` 0x20d378 — array at display+0x10, stride 0x54:
//!   +0x14 |= 1 (enabled), +0x18 += target, +0x1c/+0x20 = progress (min'ed), +0x34 = param.
//! - Fail/damage setup: `FUN_00206ec0` 0x206ec0 reads MSet +0x27c/+0x280/+0x284/+0x288 (health
//!   values, defaults 10000.0/1000.0 = 0x461c4000/0x447a0000), scales by the PTP cheat /
//!   difficulty ints (MSet +0x294 gate, +0x298/+0x29c/+0x2a0/+0x2a4/+0x2a8 percent tables).
//! - Mission end: DRIVING main's exit tail maps the outcome to the PTP result
//!   (`crates/nf-game/src/ptp.rs` — result 1 → `Mission_SetStatus(6)`, 0 → status 3).
//!
//! [S] boundaries (each listed in the doc): the per-frame evaluation order inside the
//! descriptors' +0xc functions (`FUN_0020b120`/`0020b1a0`/… — one per type, not decomposed
//! here), which rule fire advances vs completes a channel, and the trigger-record field
//! semantics beyond the leading position floats.

use crate::ptp::{drive_exit, DriveResult};

// ---------------------------------------------------------------------------
// The `Rule` chunk — 0x20-byte file entries [M FUN_00205220's stride + FUN_00209318's fields]
// ---------------------------------------------------------------------------

/// One 0x20-byte `Rule` entry [M FUN_00205220 (`iVar5 + 0x20`) + FUN_00209318's reads].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuleEntry {
    /// +0x00: the rule type 0..=14 (`FUN_00209390`'s switch; >= 0xf → the entry is dropped,
    /// the decompile's `default: return 0`).
    pub kind: u32,
    /// +0x04: flag bits 0/1/2 → the base object's +4/+8/+0xc booleans [M FUN_00209318:15-17].
    pub flags: u32,
    /// +0x08: the group/channel id — `i32`, < 0 → the default bucket [M FUN_00205220:29-51].
    pub group: i32,
    /// +0x0c: the rule's name hash [M FUN_00209318:14 (`*puVar1 = *(param_2 + 0xc)`)].
    pub hash: u32,
    /// +0x10..0x20: the type payload (read by the per-type ctors, see [`RuleKind`]).
    pub payload: [u8; 16],
}

impl RuleEntry {
    pub fn parse(data: &[u8]) -> Option<RuleEntry> {
        if data.len() < 0x20 { return None; }
        let le = |o: usize| u32::from_le_bytes(data[o..o + 4].try_into().unwrap());
        let mut payload = [0u8; 16];
        payload.copy_from_slice(&data[0x10..0x20]);
        Some(RuleEntry { kind: le(0), flags: le(4), group: le(8) as i32, hash: le(0xc), payload })
    }

    pub fn u32_at(&self, off: usize) -> u32 {
        u32::from_le_bytes(self.payload[off..off + 4].try_into().unwrap())
    }
}

/// The 15 rule classes — `FUN_00209390`'s switch arms (alloc size → ctor) with the RTTI names
/// [D 0x397c28..0x397d18]. `SRuleAlways` (0) fires unconditionally; the rest test sim state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuleKind {
    Always,      // 0  0x14 SRuleAlways
    Ammo,        // 1  0x24 SRuleAmmo
    Damage,      // 2  0x24 SRuleDamage
    Death,       // 3  0x1c SRuleDeath
    Health,      // 4  0x28 SRuleHealth
    Pip,         // 5  0x20 SRulePIP
    Prog,        // 6  0x18 SRuleProg
    Property,    // 7  0x28 SRuleProperty
    Range,       // 8  0x28 SRuleRange
    Speed,       // 9  0x24 SRuleSpeed
    Timer,       // 10 0x24 SRuleTimer
    Difficulty,  // 11 0x1c SRuleDifficulty
    PlayerDir,   // 12 0x24 SRulePlayerDir
    Collision,   // 13 0x18 SRuleCollision
    ProgCounter, // 14 0x20 SRuleProgCounter
}

impl RuleKind {
    pub fn from_type(kind: u32) -> Option<RuleKind> {
        Some(match kind {
            0 => RuleKind::Always,
            1 => RuleKind::Ammo,
            2 => RuleKind::Damage,
            3 => RuleKind::Death,
            4 => RuleKind::Health,
            5 => RuleKind::Pip,
            6 => RuleKind::Prog,
            7 => RuleKind::Property,
            8 => RuleKind::Range,
            9 => RuleKind::Speed,
            10 => RuleKind::Timer,
            11 => RuleKind::Difficulty,
            12 => RuleKind::PlayerDir,
            13 => RuleKind::Collision,
            14 => RuleKind::ProgCounter,
            _ => return None,
        })
    }

    /// The per-type ctor's object size [M the `FUN_002cfda8` size arg per switch arm].
    pub fn object_size(self) -> usize {
        match self {
            RuleKind::Always => 0x14,
            RuleKind::Ammo | RuleKind::Damage | RuleKind::Speed | RuleKind::Timer | RuleKind::PlayerDir => 0x24,
            RuleKind::Death | RuleKind::Difficulty => 0x1c,
            RuleKind::Health | RuleKind::Property | RuleKind::Range => 0x28,
            RuleKind::Pip | RuleKind::ProgCounter => 0x20,
            RuleKind::Prog | RuleKind::Collision => 0x18,
        }
    }
}

// ---------------------------------------------------------------------------
// The `MSet` chunk — the mission set / objective table [M FUN_00205498 + FUN_00206ec0]
// ---------------------------------------------------------------------------

/// The `MSet` payload: the mission-set name plus the per-objective table [M FUN_00205498's
/// reads at +0x2c+i, +0x4c+i*4, +0xcc+i*4, +0x14c+i*4; FUN_00206ec0's +0x27c..+0x2a8].
#[derive(Clone, Debug)]
pub struct MissionSet {
    /// +0x00: NUL-terminated name ("helicopter" in MIS01) [M FUN_00204d18's hash lookup].
    pub name: String,
    /// +0x2c+i: the enabled byte per objective channel (0x20 channels) [M FUN_00205498].
    pub enabled: [bool; 32],
    /// +0x4c+i*4: the per-channel target count (skipped when >= 0x2711) [M FUN_00205498].
    pub target: [i32; 32],
    /// +0xcc+i*4: per-channel param 2 (into `FUN_0020d378` arg 4 → entry +0x20).
    pub param2: [i32; 32],
    /// +0x14c+i*4: per-channel param 3 (into `FUN_0020d378` arg 5 → entry +0x34).
    pub param3: [i32; 32],
    /// +0x27c/+0x280/+0x284/+0x288: the health/damage thresholds [M FUN_00206ec0:12-21;
    /// defaults 10000.0/1000.0 when 0 (the 0x461c4000/0x447a0000 stores)].
    pub health: [i32; 4],
    /// +0x294: the difficulty-scaling gate (0 = none) [M FUN_00206ec0:52].
    pub diff_gate: i32,
    /// +0x298/+0x29c/+0x2a0/+0x2a4/+0x2a8: the per-difficulty percent tables [M FUN_00206ec0's
    /// /100.0 scalers keyed on mgr+0x3b8 ∈ {1, 2, else}].
    pub diff_pct: [i32; 5],
}

impl MissionSet {
    pub fn parse(data: &[u8]) -> MissionSet {
        let le = |o: usize| i32::from_le_bytes(data.get(o..o + 4).map_or([0; 4], |b| b.try_into().unwrap()));
        let name_end = data.iter().take(0x2c).position(|&b| b == 0).unwrap_or(0x2c);
        let name = String::from_utf8_lossy(&data[..name_end]).to_string();
        let mut enabled = [false; 32];
        for (i, e) in enabled.iter_mut().enumerate() {
            *e = data.get(0x2c + i).copied().unwrap_or(0) != 0;
        }
        let arr = |base: usize| {
            let mut a = [0i32; 32];
            for (i, v) in a.iter_mut().enumerate() { *v = le(base + i * 4); }
            a
        };
        MissionSet {
            name,
            enabled,
            target: arr(0x4c),
            param2: arr(0xcc),
            param3: arr(0x14c),
            health: [le(0x27c), le(0x280), le(0x284), le(0x288)],
            diff_gate: le(0x294),
            diff_pct: [le(0x298), le(0x29c), le(0x2a0), le(0x2a4), le(0x2a8)],
        }
    }
}

// ---------------------------------------------------------------------------
// The `Trgr` chunk — 0x40-byte trigger records (minimal decode)
// ---------------------------------------------------------------------------

/// One 0x40-byte `Trgr` record, minimally decoded: the leading floats and the name hash are
/// [D]-measured over the mission archives (see the doc); the remaining bytes stay raw — the
/// consumer (`FUN_0022cbe0` 0x22cbe0, class `WTrigger` [D]) is not decomposed yet.
#[derive(Clone, Debug, PartialEq)]
pub struct Trigger {
    /// +0x00..+0x10: four f32s (x, y, z + a fourth — radius/height, semantics [S]).
    pub pos: [f32; 4],
    /// +0x18: the u32 at +0x18 — `01 00 6c 65`-shaped name hash in every mission [D].
    pub hash: u32,
    /// The raw 0x40 bytes (for the golden tests).
    pub raw: [u8; 0x40],
}

impl Trigger {
    pub fn parse(data: &[u8]) -> Option<Trigger> {
        if data.len() < 0x40 { return None; }
        let f = |o: usize| f32::from_le_bytes(data[o..o + 4].try_into().unwrap());
        let mut raw = [0u8; 0x40];
        raw.copy_from_slice(&data[..0x40]);
        Some(Trigger { pos: [f(0), f(4), f(8), f(0xc)], hash: u32::from_le_bytes(data[0x18..0x1c].try_into().unwrap()), raw })
    }
}

// ---------------------------------------------------------------------------
// The runtime mission — the channel tracker + the rule buckets
// ---------------------------------------------------------------------------

/// What the headless sim feeds each frame: the car/sim facts the rule types read
/// ([S] aggregation — the real rules read engine globals through their descriptors).
#[derive(Clone, Copy, Debug, Default)]
pub struct SimSnapshot {
    /// The player car's health [S; SRuleHealth reads it].
    pub health: f32,
    /// Accumulated damage [S; SRuleDamage].
    pub damage: f32,
    /// Speed [S; SRuleSpeed].
    pub speed: f32,
    /// Position [S; SRuleRange].
    pub pos: [f32; 3],
    /// The hash of the actor that died this frame, if any [S; SRuleDeath].
    pub died: Option<u32>,
    /// A collision happened this frame [S; SRuleCollision].
    pub collided: bool,
    /// The mission-progress counter [S; SRuleProg/SRuleProgCounter].
    pub progress: u32,
    /// The PTP difficulty [S; SRuleDifficulty gates on it — the PTP block carries the
    /// mission/cheat flags].
    pub difficulty: u32,
}

/// One objective channel — the stride-0x54 display entry, the fields `FUN_0020d378` writes
/// [M: +0x14 flags bit0, +0x18 target, +0x1c done, +0x20 done2, +0x34 param].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Channel {
    pub enabled: bool,
    pub target: i32,
    pub done: i32,
    pub done2: i32,
    pub param: i32,
}

/// What fired this frame (the port's stand-in for the descriptor events the HUD/audio see).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MissionEvent {
    /// `FUN_0020d488` set the active objective [M].
    ObjectiveActivated(usize),
    /// A rule fired into its channel (progress +1) [S].
    ObjectiveProgress(usize),
    /// The channel reached its target [S].
    ObjectiveComplete(usize),
    /// An `SRulePIP` rule fired — the movie request carries the rule's hash [S].
    PipRequested(u32),
    /// All enabled channels completed → `DriveResult::Success` [M ptp mapping].
    Complete,
    /// A fail rule fired (health/damage exhausted) → `DriveResult::Fail` [M ptp mapping].
    Failed,
}

/// The headless mission state machine.
pub struct Mission {
    /// The parsed rules, bucketed by group id exactly like `FUN_00205220` (g < 0 → the
    /// default bucket; else bucket g) [M].
    pub rules: Vec<(RuleEntry, RuleKind)>,
    /// The mission set (`None` when the `.crp` has no `MSet` — `FUN_00205498`'s
    /// `&DAT_004162b8` default path then registers a fixed channel set [M]).
    pub set: Option<MissionSet>,
    /// The 0x20 display channels [M FUN_0020d378's stride-0x54 array].
    pub channels: [Channel; 32],
    /// The active objective index (display+4 [M FUN_0020d488]).
    pub active: i32,
    /// The damage thresholds [M FUN_00206ec0's defaults 10000.0/1000.0].
    pub health_limit: f32,
    pub damage_limit: f32,
    /// Mission timers: (rule index, remaining seconds) — the `SRuleTimer` rules' countdowns [S].
    pub timers: Vec<(usize, f32)>,
    pub state: MissionState,
    /// The PTP drive index for the exit mapping [M ptp::drive_exit].
    pub drive_index: u8,
}

/// The mission's outcome [M the mapping in crates/nf-game/src/ptp.rs].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MissionState {
    Playing,
    Complete,
    Failed,
}

impl Mission {
    /// Parses the `Rule` payload (0x20-byte entries) + optional `MSet` payload and builds the
    /// runtime state — `FUN_00205220` + `FUN_00205498` + `FUN_00206ec0` [M].
    pub fn load(rule: &[u8], mset: Option<&[u8]>, drive_index: u8) -> Mission {
        let mut rules = Vec::new();
        let mut timers = Vec::new();
        for (i, c) in rule.chunks_exact(0x20).enumerate() {
            let Some(e) = RuleEntry::parse(c) else { continue };
            let Some(k) = RuleKind::from_type(e.kind) else { continue }; // the switch default [M]
            if k == RuleKind::Timer {
                // SRuleTimer's countdown seconds [S — the ctor keeps entry+0x10/0x14 through
                // FUN_00209738 and entry+0x18 raw; the seconds live in the payload].
                let secs = i32::from_le_bytes(e.payload[0..4].try_into().unwrap()).max(0) as f32;
                timers.push((i, secs));
            }
            rules.push((e, k));
        }
        let set = mset.map(MissionSet::parse);
        let mut m = Mission {
            rules,
            set: None,
            channels: [Channel::default(); 32],
            active: 0,
            health_limit: 10_000.0,
            damage_limit: 1_000.0,
            timers,
            state: MissionState::Playing,
            drive_index,
        };
        match set.as_ref() {
            Some(s) => {
                // FUN_00205498's MSet path: register every enabled channel whose target
                // survived the < 0x2711 gate, then activate the first enabled non-zero one.
                for i in 0..32usize {
                    if s.enabled[i] && s.target[i] < 0x2711 {
                        m.channels[i] = Channel {
                            enabled: true,
                            target: s.target[i],
                            done: 0,
                            done2: 0,
                            param: s.param3[i],
                        };
                    }
                }
                m.activate_first();
                m.health_limit = if s.health[0] != 0 { s.health[0] as f32 } else { 10_000.0 };
                m.damage_limit = if s.health[1] != 0 { s.health[1] as f32 } else { 1_000.0 };
            }
            None => {
                // The no-MSet default path's fixed set [M FUN_00205498:11-20]: count-99
                // helper channels 0x15/0x16/3/7 (the 0x397540 key compare branch).
                for i in [0x15usize, 0x16, 3, 7] {
                    m.channels[i] = Channel { enabled: true, target: 99, ..Default::default() };
                }
                m.activate_first();
            }
        }
        m.set = set;
        m
    }

    /// `FUN_00205498`'s tail: the first enabled channel whose index != 0, else 0 [M
    /// iVar5's -1 scan + the fallback registration of channel 0], through `FUN_0020d488`.
    fn activate_first(&mut self) {
        let mut pick: i32 = -1;
        for i in 1..32usize {
            if self.channels[i].enabled { pick = i as i32; break; }
        }
        self.active = if pick < 0 { 0 } else { pick };
        self.channels[self.active as usize].enabled = true;
    }

    /// The rules whose group matches the active channel (bucket walk) [S order; M buckets].
    fn active_rules(&self) -> impl Iterator<Item = (usize, &RuleEntry, RuleKind)> {
        let active = self.active;
        self.rules.iter().enumerate().filter_map(move |(i, (e, k))| {
            (e.group == active).then_some((i, e, *k))
        })
    }

    /// One 60 Hz frame: tick timers, evaluate the active channel's rules against `sim`,
    /// advance the channel counters. Fixed timestep [M 0x2d6b90's 60 Hz table].
    pub fn step(&mut self, dt: f32, sim: &SimSnapshot) -> Vec<MissionEvent> {
        let mut events = Vec::new();
        if self.state != MissionState::Playing { return events; }

        // SRuleTimer: the countdown ticks every frame [S].
        let mut fired: Vec<usize> = Vec::new();
        for (idx, t) in self.timers.iter_mut() {
            *t -= dt;
            if *t <= 0.0 { fired.push(*idx); }
        }

        // The frame evaluates the channel active at frame start — an advance mid-frame
        // only takes effect next frame [S; keeps one-frame-one-evaluation semantics].
        let active = self.active;
        for i in 0..self.rules.len() {
            let (e, k) = self.rules[i];
            if e.group != active { continue; }
            let fire = match k {
                RuleKind::Always => true, // SRuleAlways: unconditional [D name; S semantics]
                RuleKind::Timer => fired.contains(&i),
                RuleKind::Health => sim.health <= 0.0,
                RuleKind::Damage => sim.damage >= self.damage_limit,
                RuleKind::Death => sim.died == Some(e.hash),
                RuleKind::Collision => sim.collided,
                RuleKind::Prog | RuleKind::ProgCounter => sim.progress >= e.u32_at(0),
                RuleKind::Speed => sim.speed >= f32::from_le_bytes(e.payload[0..4].try_into().unwrap()),
                RuleKind::Range => {
                    let x = f32::from_le_bytes(e.payload[0..4].try_into().unwrap());
                    let z = f32::from_le_bytes(e.payload[8..12].try_into().unwrap());
                    let r = f32::from_le_bytes(e.payload[12..16].try_into().unwrap());
                    let dx = sim.pos[0] - x;
                    let dz = sim.pos[2] - z;
                    dx * dx + dz * dz <= r * r
                }
                RuleKind::Pip => false, // PIP is script-driven (see fire())
                RuleKind::Ammo => false,
                RuleKind::Property => false,
                RuleKind::Difficulty => sim.difficulty == e.u32_at(0),
                RuleKind::PlayerDir => false,
            };
            if fire { self.fire(i, &e, k, &mut events); }
        }
        events
    }

    /// A rule fired: bump its channel's done counter and complete/advance [S; the counters
    /// mirror FUN_0020d378's +0x1c/+0x20 fields]. `SRulePIP` fires a movie request instead.
    pub fn fire(&mut self, _rule_idx: usize, e: &RuleEntry, k: RuleKind, events: &mut Vec<MissionEvent>) {
        if k == RuleKind::Pip {
            events.push(MissionEvent::PipRequested(e.hash));
            return;
        }
        let ch = e.group.clamp(0, 31) as usize;
        if !self.channels[ch].enabled { self.channels[ch].enabled = true; }
        self.channels[ch].done += 1;
        events.push(MissionEvent::ObjectiveProgress(ch));
        if self.channels[ch].target > 0 && self.channels[ch].done >= self.channels[ch].target {
            events.push(MissionEvent::ObjectiveComplete(ch));
            self.advance(ch, events);
        }
    }

    /// The channel completed: activate the next enabled channel, or end the mission [S —
    /// the game's advance is inside the display update; the last-channel → end mapping is
    /// what the tests pin].
    fn advance(&mut self, from: usize, events: &mut Vec<MissionEvent>) {
        let next = ((from + 1)..32).find(|&i| self.channels[i].enabled);
        match next {
            Some(i) => {
                self.active = i as i32;
                events.push(MissionEvent::ObjectiveActivated(i));
            }
            None => {
                self.state = MissionState::Complete;
                events.push(MissionEvent::Complete);
            }
        }
    }

    /// The fail path: health/damage exhausted [M FUN_00206ec0's thresholds; SRuleHealth/
    /// SRuleDamage fire the fail].
    pub fn fail(&mut self, events: &mut Vec<MissionEvent>) {
        if self.state == MissionState::Playing {
            self.state = MissionState::Failed;
            events.push(MissionEvent::Failed);
        }
    }

    /// The mission end → the PTP return [M crates/nf-game/src/ptp.rs's mapping: Success →
    /// result 1 → Mission_SetStatus(6); Fail → result 0 → status 3]. `difficulty` is the PTP
    /// difficulty the drive session is running with — the block's +0x144 byte, which DRIVING
    /// preserves and `Boot_LoadPTPData` writes back to GameState+0x28 [M 0x1c8198:15-20,
    /// 0x1c8560:35-36].
    pub fn drive_return(&self, stats: crate::ptp::DrivingStats, difficulty: u32) -> crate::ptp::DriveReturn {
        let result = match self.state {
            MissionState::Complete => DriveResult::Success,
            _ => DriveResult::Fail,
        };
        drive_exit(result, self.drive_index, stats, difficulty)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds one 0x20-byte Rule entry like the shipped payloads [D MIS01.VIV#Rule].
    fn entry(kind: u32, flags: u32, group: i32, hash: u32, payload: [u8; 16]) -> [u8; 0x20] {
        let mut b = [0u8; 0x20];
        b[0..4].copy_from_slice(&kind.to_le_bytes());
        b[4..8].copy_from_slice(&flags.to_le_bytes());
        b[8..12].copy_from_slice(&group.to_le_bytes());
        b[12..16].copy_from_slice(&hash.to_le_bytes());
        b[16..32].copy_from_slice(&payload);
        b
    }

    fn f32b(v: f32) -> [u8; 4] { v.to_le_bytes() }

    /// A 3-objective set: channel 1 target 2, channel 2 target 1, channel 3 target 1.
    fn mset3() -> Vec<u8> {
        let mut m = vec![0u8; 0x2f8];
        m[..10].copy_from_slice(b"helicopter");
        m[0x2c + 1] = 1;
        m[0x2c + 2] = 1;
        m[0x2c + 3] = 1;
        m[0x4c + 4..0x4c + 8].copy_from_slice(&2i32.to_le_bytes());
        m[0x4c + 8..0x4c + 12].copy_from_slice(&1i32.to_le_bytes());
        m[0x4c + 12..0x4c + 16].copy_from_slice(&1i32.to_le_bytes());
        m
    }

    #[test]
    fn rule_entry_round_trips_the_file_layout() {
        let mut payload = [0u8; 16];
        payload[0..4].copy_from_slice(&0x258u32.to_le_bytes());
        let e = RuleEntry::parse(&entry(6, 2, 5, 0x656c_02f4, payload)).unwrap();
        assert_eq!((e.kind, e.flags, e.group, e.hash), (6, 2, 5, 0x656c_02f4));
        assert_eq!(e.u32_at(0), 0x258);
        assert_eq!(RuleKind::from_type(6), Some(RuleKind::Prog));
        assert_eq!(RuleKind::from_type(6).unwrap().object_size(), 0x18); // [M 00209390 case 6]
        assert_eq!(RuleKind::from_type(15), None); // the switch default drops it [M]
        assert!(RuleEntry::parse(&[0u8; 0x1f]).is_none());
    }

    #[test]
    fn mission_set_parses_name_channels_and_thresholds() {
        let mut m = mset3();
        m[0x27c..0x280].copy_from_slice(&9000i32.to_le_bytes()); // health [M 00206ec0]
        m[0x280..0x284].copy_from_slice(&800i32.to_le_bytes()); // damage
        m[0x294..0x298].copy_from_slice(&1i32.to_le_bytes()); // diff gate
        let s = MissionSet::parse(&m);
        assert_eq!(s.name, "helicopter");
        assert!(s.enabled[1] && s.enabled[2] && s.enabled[3] && !s.enabled[0]);
        assert_eq!((s.target[1], s.target[2], s.target[3]), (2, 1, 1));
        assert_eq!((s.health[0], s.health[1]), (9000, 800));
        assert_eq!(s.diff_gate, 1);
    }

    #[test]
    fn scripted_run_reaches_complete_in_order() {
        // Always(0) x1 in group 1; Prog(6) progress>=10 in group 2; Range(8) in group 3.
        let mut rule = Vec::new();
        rule.extend_from_slice(&entry(0, 0, 1, 1, [0; 16]));
        let mut p2 = [0u8; 16];
        p2[0..4].copy_from_slice(&10u32.to_le_bytes());
        rule.extend_from_slice(&entry(6, 0, 2, 2, p2));
        let mut p3 = [0u8; 16];
        p3[0..4].copy_from_slice(&f32b(10.0));
        p3[8..12].copy_from_slice(&f32b(20.0));
        p3[12..16].copy_from_slice(&f32b(5.0));
        rule.extend_from_slice(&entry(8, 0, 3, 3, p3));
        let mset = mset3();
        let mut m = Mission::load(&rule, Some(&mset), 0);
        assert_eq!(m.active, 1, "the first enabled non-zero channel [M 00205498's scan]");
        assert_eq!((m.channels[1].target, m.channels[2].target, m.channels[3].target), (2, 1, 1));

        let far = SimSnapshot { pos: [1000.0, 0.0, 1000.0], ..Default::default() };
        // Frame 1: the Always rule fires ch1 (1/2).
        let ev = m.step(crate::driving::DT, &far);
        assert_eq!(ev, vec![MissionEvent::ObjectiveProgress(1)]);
        assert_eq!(m.channels[1].done, 1);
        // Frame 2: ch1 completes → ch2 activates.
        let ev = m.step(crate::driving::DT, &far);
        assert_eq!(ev, vec![MissionEvent::ObjectiveProgress(1), MissionEvent::ObjectiveComplete(1), MissionEvent::ObjectiveActivated(2)]);
        // Out of range: ch2's Prog stays unfired.
        let ev = m.step(crate::driving::DT, &far);
        assert!(ev.is_empty());
        // Progress 10 → ch2 completes → ch3 activates.
        let ev = m.step(crate::driving::DT, &SimSnapshot { progress: 10, ..Default::default() });
        assert_eq!(ev, vec![MissionEvent::ObjectiveProgress(2), MissionEvent::ObjectiveComplete(2), MissionEvent::ObjectiveActivated(3)]);
        // Into the range circle → ch3 completes → mission complete (no channel 4).
        let ev = m.step(crate::driving::DT, &SimSnapshot { pos: [11.0, 0.0, 21.0], ..Default::default() });
        assert_eq!(ev, vec![MissionEvent::ObjectiveProgress(3), MissionEvent::ObjectiveComplete(3), MissionEvent::Complete]);
        assert_eq!(m.state, MissionState::Complete);
        assert_eq!(m.active, 3);
        // A finished mission steps to nothing.
        assert!(m.step(crate::driving::DT, &far).is_empty());
    }

    #[test]
    fn fail_path_maps_to_the_ptp_fail_return() {
        let mut rule = Vec::new();
        rule.extend_from_slice(&entry(4, 0, 1, 9, [0; 16])); // SRuleHealth in ch1
        let mut m = Mission::load(&rule, Some(&mset3()), 0);
        assert_eq!(m.state, MissionState::Playing);
        m.step(crate::driving::DT, &SimSnapshot { health: 500.0, ..Default::default() });
        assert_eq!(m.state, MissionState::Playing, "health above 0 keeps playing");
        let ev = m.step(crate::driving::DT, &SimSnapshot { health: 0.0, ..Default::default() });
        assert_eq!(ev, vec![MissionEvent::ObjectiveProgress(1)]);
        m.fail(&mut Vec::new());
        let stats = crate::ptp::DrivingStats { ids: [7; 12], values: [1234; 12] };
        let ret = m.drive_return(stats, 2);
        assert_eq!((ret.result, ret.status_byte), (0, 4)); // FUN_00300058(0x100000, 0) [M]
        assert_eq!(ret.mission_state(), 3); // Mission_SetStatus(3) [M 0x1c8560]
    }

    #[test]
    fn complete_path_maps_to_the_ptp_success_return() {
        let mut rule = Vec::new();
        rule.extend_from_slice(&entry(0, 0, 1, 1, [0; 16])); // Always in ch1
        rule.extend_from_slice(&entry(6, 0, 2, 2, [0; 16])); // Prog in ch2
        rule.extend_from_slice(&entry(0, 0, 3, 3, [0; 16])); // Always in ch3
        let mut m = Mission::load(&rule, Some(&mset3()), 0);
        // ch1 target 2: two Always fires complete ch1 → ch2 activates.
        m.step(crate::driving::DT, &SimSnapshot::default());
        m.step(crate::driving::DT, &SimSnapshot::default());
        assert_eq!(m.active, 2);
        let mut ev = Vec::new();
        let (r1, k1) = m.rules[1];
        m.fire(1, &r1, k1, &mut ev); // the Prog rule in ch2 fires → complete → ch3
        assert_eq!(ev, vec![MissionEvent::ObjectiveProgress(2), MissionEvent::ObjectiveComplete(2), MissionEvent::ObjectiveActivated(3)]);
        let (r2, k2) = m.rules[2];
        m.fire(2, &r2, k2, &mut ev); // ch3 completes → no channel 4 → mission complete
        assert!(ev.contains(&MissionEvent::Complete));
        let ret = m.drive_return(crate::ptp::DrivingStats::default(), 2);
        assert_eq!((ret.result, ret.status_byte), (1, 0)); // FUN_00300058(0x100000, 1) [M]
        assert_eq!(ret.mission_state(), 6); // Mission_SetStatus(6) [M 0x1c8560]
        assert_eq!(ret.next_key, crate::ptp::key_for_index(0));
    }

    #[test]
    fn timer_rule_counts_down_and_fires() {
        let mut p = [0u8; 16];
        p[0..4].copy_from_slice(&2i32.to_le_bytes()); // 2 seconds
        let rule = entry(10, 0, 1, 5, p); // SRuleTimer in ch1
        let mut m2 = [0u8; 0x2f8];
        m2[0x2c + 1] = 1;
        m2[0x4c + 4..0x4c + 8].copy_from_slice(&1i32.to_le_bytes());
        let mut m = Mission::load(&rule, Some(&m2), 0);
        assert_eq!(m.timers, vec![(0usize, 2.0f32)]);
        let ev = m.step(crate::driving::DT, &SimSnapshot::default());
        assert!(ev.is_empty(), "the timer has not expired yet");
        for _ in 0..119 { m.step(crate::driving::DT, &SimSnapshot::default()); }
        let ev = m.step(crate::driving::DT, &SimSnapshot::default()); // 120 ticks = 2 s
        assert_eq!(ev, vec![MissionEvent::ObjectiveProgress(1), MissionEvent::ObjectiveComplete(1), MissionEvent::Complete]);
    }

    #[test]
    fn no_mset_default_registers_the_fixed_helper_channels() {
        let rule = entry(0, 0, 0x15, 1, [0; 16]);
        let mut m = Mission::load(&rule, None, 0);
        // [M 00205498:12-19: channels 0x15/0x16/3/7 with count 99]
        for i in [0x15usize, 0x16, 3, 7] {
            assert!(m.channels[i].enabled, "channel {i} enabled");
            assert_eq!(m.channels[i].target, 99);
        }
        assert_eq!(m.active, 3, "the first enabled non-zero channel (3, 7, 0x15, 0x16)");
        // The rule targets channel 0x15, the active is 3 → the frame fires nothing.
        let ev = m.step(crate::driving::DT, &SimSnapshot::default());
        assert!(ev.is_empty());
        // Firing the rule directly (its channel is 0x15) bumps that channel.
        let (r0, k0) = m.rules[0];
        let mut ev = Vec::new();
        m.fire(0, &r0, k0, &mut ev);
        assert_eq!(ev, vec![MissionEvent::ObjectiveProgress(0x15)]);
        assert_eq!(m.channels[0x15].done, 1);
    }

    #[test]
    fn death_and_difficulty_rules_match_their_inputs() {
        let mut rule = Vec::new();
        rule.extend_from_slice(&entry(3, 0, 1, 0x656c_02f3, [0; 16])); // SRuleDeath hash
        let mut p = [0u8; 16];
        p[0..4].copy_from_slice(&2u32.to_le_bytes());
        rule.extend_from_slice(&entry(11, 0, 1, 12, p)); // SRuleDifficulty == 2
        let mut m = Mission::load(&rule, Some(&mset3()), 0);
        // Wrong hash + wrong difficulty: nothing.
        let ev = m.step(crate::driving::DT, &SimSnapshot { died: Some(1), difficulty: 1, ..Default::default() });
        assert!(ev.is_empty());
        // Both match: two progresses on ch1; the second reaches its target of 2 →
        // complete + advance to ch2 in the same frame.
        let ev = m.step(crate::driving::DT, &SimSnapshot { died: Some(0x656c_02f3), difficulty: 2, ..Default::default() });
        assert_eq!(ev, vec![MissionEvent::ObjectiveProgress(1), MissionEvent::ObjectiveProgress(1), MissionEvent::ObjectiveComplete(1), MissionEvent::ObjectiveActivated(2)]);
        assert_eq!(m.channels[1].done, 2);
    }

    #[test]
    fn pip_rule_reports_the_hash_without_progress() {
        let rule = entry(5, 0, 1, 0x656c_0abc, [0; 16]);
        let mut m = Mission::load(&rule, Some(&mset3()), 0);
        let mut ev = Vec::new();
        let (r0, k0) = m.rules[0];
        m.fire(0, &r0, k0, &mut ev);
        assert_eq!(ev, vec![MissionEvent::PipRequested(0x656c_0abc)]);
        assert_eq!(m.channels[1].done, 0, "PIP requests do not advance the channel");
    }
}
