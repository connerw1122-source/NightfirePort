//! Drone (NPC) animation choice: which animation script a drone starts with. Spec: docs/systems/drone-animation.md.
//!
//! The tables (`DroneAnimStates`, `Drone_AnimInfo`, `Drone_AnimTables`) are read from the user's ACTION.ELF through
//! its symbol table; the rules that index them are ported from `NDrone2_DefaultInit` (ACTION.ELF 0x14b300),
//! `DroneAnim_SetDAnimInternal` (0x13aee8) and `DroneAnim_GetDAnimForAnimState` (0x13de40).

use crate::buf::{invalid, Buf, Result};
use crate::elf::Elf;

/// `DroneAnimStates` record count (0x77 states, 12 bytes each; the code clamps state numbers above 0x76 to 0).
pub const STATE_COUNT: usize = 0x77;
/// Anim sets per `Drone_AnimTables` row (0x6C bytes = 27 × {u16 script low half, u16}).
pub const ANIM_SETS: usize = 27;
/// Danim 0x11A: the start animation `NDrone2_DefaultInit` gives an ordinary drone (anim state 0x24, stand idle).
pub const DANIM_STAND_IDLE: u16 = 0x11a;

/// One entry of a state's transition list: requested state `key` → danim.
#[derive(Clone, Debug, PartialEq)]
/// A `DroneAnimStates` transition record: requested state (0 = any), variant count, danim, blend frames (`b4`) and
/// whether the call waits for the blend to end before it completes (`b5`, `Drone+0x578`).
pub struct Transition { pub key: u8, pub count: u8, pub danim: i16, pub b4: u8, pub b5: u8 }

#[derive(Clone, Debug)]
pub struct DroneAnimData {
    /// Per anim state: the state's own danim (`+0`) and its transition list (`+8` pointer, records until key 0xFF).
    pub states: Vec<(u16, Vec<Transition>)>,
    /// Per danim: `Drone_AnimInfo` record (24 bytes); `+8` u16 = the anim state it puts the drone in.
    pub info: Vec<[u8; 24]>,
    /// Per danim: script low halves per anim set (`0x06000000 +` value; 0 = none).
    pub tables: Vec<[u16; ANIM_SETS]>,
}

impl DroneAnimData {
    /// Reads the three tables from ACTION.ELF bytes by symbol name.
    pub fn from_action_elf(data: &[u8]) -> Result<DroneAnimData> {
        let elf = Elf::parse(data)?;
        let syms = elf.symbols(data)?;
        let segs = elf.load_segments(data)?;
        let sym = |name: &str| syms.iter().find(|s| s.name == name).map(|s| (s.value, s.size as usize))
            .ok_or_else(|| crate::buf::FormatError::Invalid(format!("ACTION.ELF has no symbol {name}")));
        let at = |addr: u32, len: usize| -> Result<&[u8]> {
            for (va, bytes, _) in &segs {
                if addr >= *va && (addr - va) as usize + len <= bytes.len() {
                    return Ok(&bytes[(addr - va) as usize..(addr - va) as usize + len]);
                }
            }
            invalid(format!("address {addr:#x}+{len:#x} outside the loaded image"))
        };
        let (sa, ss) = sym("DroneAnimStates")?;
        let (ia, is) = sym("Drone_AnimInfo")?;
        let (ta, ts) = sym("Drone_AnimTables")?;
        if ss != STATE_COUNT * 12 || is % 24 != 0 || ts != is / 24 * 0x6c {
            return invalid(format!("unexpected table sizes: states {ss}, info {is}, tables {ts}"));
        }
        let mut states = Vec::with_capacity(STATE_COUNT);
        for s in 0..STATE_COUNT {
            let r = Buf(at(sa + s as u32 * 12, 12)?);
            let (own, list) = (r.u16(0)?, r.u32(8)?);
            let mut tr = Vec::new();
            if list != 0 {
                let mut p = list;
                loop {
                    let e = Buf(at(p, 6)?);
                    if e.u8(0)? == 0xff { break; }
                    tr.push(Transition { key: e.u8(0)?, count: e.u8(1)?, danim: e.u16(2)? as i16, b4: e.u8(4)?, b5: e.u8(5)? });
                    p += 6;
                    if tr.len() > 256 { return invalid(format!("state {s:#x}: transition list not terminated")); }
                }
            }
            states.push((own, tr));
        }
        let danims = is / 24;
        let info = (0..danims).map(|d| at(ia + d as u32 * 24, 24).map(|b| b.try_into().unwrap())).collect::<Result<_>>()?;
        let tables = (0..danims).map(|d| {
            let row = Buf(at(ta + d as u32 * 0x6c, 0x6c)?);
            let mut t = [0u16; ANIM_SETS];
            for (k, v) in t.iter_mut().enumerate() { *v = row.u16(4 * k)?; }
            Ok(t)
        }).collect::<Result<_>>()?;
        Ok(DroneAnimData { states, info, tables })
    }

    /// `DroneAnim_GetDAnimForAnimState(requested, current, …)`: the danim of the first transition of `current`'s list
    /// whose key is `requested` (or 0, a wildcard); `None` when the list has neither (the game returns −1).
    pub fn danim_for_state(&self, requested: u16, current: u16) -> Option<i16> {
        let cur = if current as usize >= STATE_COUNT { 0 } else { current as usize };
        self.states[cur].1.iter().find(|t| t.key as u16 == requested || t.key == 0).map(|t| t.danim)
    }

    /// `DroneAnim_GetDAnimForCall` (0x13db30): the transition for `requested` from `current`'s list, else from state
    /// 0's list (blend then 8 frames, no wait, unless the state-0 record says otherwise), with the variant added when it
    /// is below the record's count (`GetDAnimForAnimState`). `None` when neither list has it.
    pub fn transition(&self, requested: u16, current: u16, variant: u16) -> Option<Transition> {
        let find = |s: usize| self.states.get(s).and_then(|(_, l)| l.iter().find(|t| t.key as u16 == requested || t.key == 0));
        let cur = if current as usize >= STATE_COUNT { 0 } else { current as usize };
        let t = find(cur).or_else(|| find(0))?;
        let mut t = t.clone();
        if t.danim > 0 && (variant as u8) < t.count { t.danim += variant as i16 }
        Some(t)
    }

    /// The anim state a danim puts the drone in (`Drone_AnimInfo +8`).
    pub fn state_of(&self, danim: u16) -> Option<u16> {
        self.info.get(danim as usize).map(|i| u16::from_le_bytes([i[8], i[9]]))
    }

    /// `DroneAnim_SetDAnimInternal`: the script of `danim` for `anim_set` — the set's entry, or set 0's when the set has
    /// none. `None` = `0x06000000` (no script; the game ends the animation instead).
    pub fn script_for(&self, danim: u16, anim_set: u16) -> Option<u32> {
        let row = self.tables.get(danim as usize)?;
        let set = if row.get(anim_set as usize).copied().unwrap_or(0) == 0 { 0 } else { anim_set as usize };
        let lo = row[set];
        (lo != 0).then_some(0x0600_0000 + lo as u32)
    }
}

/// Drone type (`Drone+0xD8`) and anim set (`Drone+0xDA`) after `NDrone2_DefaultInit`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DroneSetup { pub kind: u16, pub anim_set: u16 }

/// Port of the two `NDrone2_DefaultInit` switches: the skin hashcode (placement parameter 0) gives type and set; the
/// weapon (parameter 9; 1 when absent, 0 for skin 0x0500001E) may replace the set. `level` = `GameState+12`.
/// Multiplayer (`BOT_setGender`) is not modelled.
pub fn drone_setup(skin: u32, weapon_param: u32, level: u32) -> DroneSetup {
    let skin = if level == 0x0700_0014 && skin == 0x0500_00b1 { 0x0500_008c } else { skin };
    let mut weapon: u32 = 1;
    let (kind, mut set) = match skin {
        0x0500_0004 | 0x0500_0006 | 0x0500_0008 | 0x0500_004c => (3, 0xc),
        0x0500_000e | 0x0500_000f => (0x15, 2),
        0x0500_0010 | 0x0500_0011 | 0x0500_0052 => (0x16, 2),
        0x0500_0012..=0x0500_0014 => (0x11, 10),
        0x0500_0015 | 0x0500_0020 | 0x0500_0022 | 0x0500_0023 | 0x0500_0024 => (0xd, 0xe),
        0x0500_0016 | 0x0500_008f | 0x0500_00ac => (0x14, 0xc),
        0x0500_0017 | 0x0500_0018 => (0x12, 0xe), // the LAB_0014b6c8 path stores the pair the other way round
        0x0500_0019 | 0x0500_0021 => (0xe, 0xc),
        0x0500_001b | 0x0500_004f => (9, 3),
        0x0500_001e => { weapon = 0; (0xb, 7) }
        0x0500_0025 => (5, 0xc),
        0x0500_0026 | 0x0500_0066 => (0x10, 0xc),
        0x0500_002b | 0x0500_002d | 0x0500_0034 => (8, 1),
        0x0500_002e..=0x0500_0030 => (2, 0xc),
        0x0500_0035 => (0xf, 0xc),
        0x0500_0036..=0x0500_003a => (1, 0xc),
        0x0500_003b | 0x0500_003c => (0xc, 0xe),
        0x0500_0027..=0x0500_002a | 0x0500_0031..=0x0500_0033 | 0x0500_0053 => (0x17, 0xb),
        0x0500_0054 | 0x0500_006d | 0x0500_006e | 0x0500_007e => (4, 0xc),
        0x0500_005e => (0x13, 0xd),
        0x0500_0078 | 0x0500_0094 | 0x0500_00ba => (0x18, 0x10),
        0x0500_0087 | 0x0500_0090 | 0x0500_0093 => (6, 0xc),
        0x0500_008c | 0x0500_00b1 | 0x0500_00b2 => (7, 0xc),
        _ => (0, 0xc),
    };
    if weapon_param != 0 { weapon = weapon_param; }
    match weapon {
        2 | 6 | 10 | 0xb | 0xe | 0x10 | 0x3b..=0x40 => set = if kind == 0xc { 8 } else { 4 },
        0x12 | 0x14 | 0x15 => if kind != 0xb { set = 6 },
        0x16 | 0x18..=0x1b => set = 5,
        0x1c | 0x1d | 0x2a..=0x2d => set = 9,
        0x1e | 0x1f | 0x24 | 0x25 => set = 0x13,
        0x30 | 0x32 | 0x33 | 0x6a | 0x6e => if kind != 0x18 { set = 6 },
        _ => {}
    }
    DroneSetup { kind, anim_set: set }
}

/// The weapon `NDrone2_DefaultInit` stores in `sAnimObject` (`anim+0x62`): parameter 9, or 1 (0 for skin
/// 0x0500001E) when it is 0. Measured live in 07000005: parameter 9 = 0x16 / 0x24 → anim+0x62 = 0x16 / 0x24.
pub fn weapon_id(skin: u32, weapon_param: u32) -> u32 {
    if weapon_param != 0 { weapon_param } else if skin == 0x0500_001e { 0 } else { 1 }
}

/// Behaviour property decoding (`behaviour_util_getProperty` 0x1c7d98): `(words[desc[2id] & 7] & masks[desc[2id+1]])
/// >> (desc[2id] >> 3)`, tables `bitDescs` (0x2c7250, 2 bytes per id) and `bitMasks` (0x2c7308, pre-shifted u32).
#[derive(Clone, Debug)]
pub struct PropertyTables { pub descs: Vec<[u8; 2]>, pub masks: Vec<u32> }

impl PropertyTables {
    pub fn from_action_elf(data: &[u8]) -> Result<PropertyTables> {
        let elf = Elf::parse(data)?;
        let syms = elf.symbols(data)?;
        let segs = elf.load_segments(data)?;
        let sym = |n: &str| syms.iter().find(|s| s.name == n).map(|s| (s.value, s.size as usize))
            .ok_or_else(|| crate::buf::FormatError::Invalid(format!("ACTION.ELF has no symbol {n}")));
        let at = |addr: u32, len: usize| -> Result<Vec<u8>> {
            for (va, bytes, _) in &segs {
                if addr >= *va && (addr - va) as usize + len <= bytes.len() { return Ok(bytes[(addr - va) as usize..][..len].to_vec()); }
            }
            invalid(format!("{addr:#x} outside the image"))
        };
        let (da, ds) = sym("bitDescs")?;
        let (ma, ms) = sym("bitMasks")?;
        let d = at(da, ds)?;
        let m = at(ma, ms)?;
        Ok(PropertyTables {
            descs: d.chunks(2).map(|c| [c[0], c[1]]).collect(),
            masks: m.chunks(4).map(|c| u32::from_le_bytes(c.try_into().unwrap())).collect(),
        })
    }

    pub fn get(&self, id: usize, words: &[u32; 3]) -> u32 {
        let Some(&[d0, d1]) = self.descs.get(id) else { return 0 };
        let mask = self.masks.get(d1 as usize).copied().unwrap_or(0);
        (words[(d0 & 7) as usize % 3] & mask) >> (d0 >> 3)
    }

    /// `behaviour_util_setProperty` (0x1c7de8): clear the field's mask, then OR in `value << shift` (unmasked, as built).
    pub fn set(&self, id: usize, words: &mut [u32; 3], value: u32) {
        let Some(&[d0, d1]) = self.descs.get(id) else { return };
        let w = (d0 & 7) as usize % 3;
        words[w] &= !self.masks.get(d1 as usize).copied().unwrap_or(0);
        words[w] |= value << (d0 >> 3);
    }
}

/// The two behaviour blocks of a `DoModeSettingsNEW` drone: placement parameters 18..20 (normal) and 23..25 (alert).
pub fn behaviour_words(param: impl Fn(u32) -> u32) -> ([u32; 3], [u32; 3]) {
    ([param(18), param(19), param(20)], [param(23), param(24), param(25)])
}

/// Alertness from property 0x20 (2 bits): the float table at 0x2f5cc8 = {0, 0, 0.66, 1.0} (agent report, to verify).
pub const ALERTNESS_LEVELS: [f32; 4] = [0.0, 0.0, 0.66, 1.0];

/// How a drone placement appears at level start, given the level's drone spawners (object type 0xF1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpawnRole {
    /// Created by `Drone_Create` and left in place.
    Placed,
    /// A spawner template (`DroneSpawner_Init` copies the drone's setup and deletes it) that an immediate spawner
    /// brings straight back at its own position. The game picks which templates at random; we take placement order.
    SpawnedAtStart,
    /// A spawner template not present at start: spawned later (switch channel, or when another dies), out of sight.
    Reinforcement { channel: u16 },
}

/// `(translation, object type, parameters)` of each main-map placement, in order → the role of each drone
/// placement (type 0x0F), in the same order. Rules (*measured*, docs/systems/drone-animation.md "Spawners"):
/// `DroneSpawner_Create` needs parameter 0 > 1 and keys the spawner by parameter 1 (group); drones join group
/// parameter 4 (`NDrone2_AddToGroup`); a group with a spawner becomes templates; a spawner whose start channel
/// (parameter 4) is 0 spawns up to parameter 2 drones at once (0 = the group size), ignoring visibility.
pub fn spawn_roles(placements: &[(u32, Vec<(u32, u32)>)]) -> Vec<SpawnRole> {
    use std::collections::HashMap;
    let param = |p: &Vec<(u32, u32)>, k: u32| p.iter().find(|(i, _)| *i == k).map(|&(_, v)| v).unwrap_or(0);
    // group → (max alive at once, start channel)
    let spawners: HashMap<u32, (u32, u32)> = placements.iter().filter(|(t, p)| *t == 0xf1 && param(p, 0) > 1)
        .map(|(_, p)| (param(p, 1), (param(p, 2), param(p, 4)))).collect();
    let mut spawned: HashMap<u32, u32> = HashMap::new();
    let group_size = |g: u32| placements.iter().filter(|(t, p)| *t == 0x0f && param(p, 4) == g).count() as u32;
    placements.iter().filter(|(t, _)| *t == 0x0f).map(|(_, p)| {
        let g = param(p, 4);
        let Some(&(max_alive, channel)) = spawners.get(&g).filter(|_| g != 0) else { return SpawnRole::Placed };
        let limit = if max_alive == 0 { group_size(g) } else { max_alive };
        let n = spawned.entry(g).or_default();
        if channel == 0 && *n < limit { *n += 1; SpawnRole::SpawnedAtStart } else { SpawnRole::Reinforcement { channel: channel as u16 } }
    }).collect()
}

/// The danim `NDrone2_DefaultInit` starts a drone with (`DroneAnim_SetAnimD`), and the anim set it uses then.
/// `mode` = `Drone+0xC5` (the drone mode; modes 2 and 0x19 with set 5 switch to set 0x12).
pub fn start_danim(setup: DroneSetup, mode: u8) -> (u16, u16) {
    match (setup.kind, mode, setup.anim_set) {
        (0xb, _, s) => (0x1a5, s),
        (0x18, _, s) => (0x82, s),
        (_, 2, 5) => (0x7d, 0x12),
        (_, 0x19, 5) => (0x32, 0x12),
        (_, _, s) => (DANIM_STAND_IDLE, s),
    }
}

/// Where a drone's start script comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StartScript {
    /// Placement parameter 1 names a script the drone plays (`NDrone2_DSTATE_PlayScript` via `Drone+0x554`).
    Custom(u32),
    /// `NDrone2_DefaultInit`'s start danim in the drone's anim set (`None` = the table has no script: the game ends
    /// the animation instead, `DroneAnim_SetEndAnim`).
    Table { danim: u16, anim_set: u16, script: Option<u32> },
}

impl StartScript {
    pub fn script(&self) -> Option<u32> {
        match *self { StartScript::Custom(h) => Some(h), StartScript::Table { script, .. } => script }
    }
}

/// A drone placement's start script from its parameters (`param(k)` = placement parameter k, 0 when absent).
/// Parameter 1 = 0x06000892 / 0x0600089C does not name a script: `NDrone2_DefaultInit` switches the anim set to 0xF
/// (and the mode, `Drone+0xC5`, to 0x1C / 0x1B). The drone mode is otherwise not modelled (0).
pub fn start_script(tables: &DroneAnimData, param: impl Fn(u32) -> u32, level: u32) -> StartScript {
    let custom = param(1);
    if custom >> 24 == 6 && custom & 0xff_ffff != 0 && custom != 0x0600_0892 && custom != 0x0600_089c {
        return StartScript::Custom(custom);
    }
    let mut setup = drone_setup(param(0), param(9), level);
    let mut mode = 0;
    if custom == 0x0600_0892 || custom == 0x0600_089c {
        setup.anim_set = 0xf;
        mode = if custom == 0x0600_0892 { 0x1c } else { 0x1b };
    }
    let (danim, anim_set) = start_danim(setup, mode);
    StartScript::Table { danim, anim_set, script: tables.script_for(danim, anim_set) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn setup_follows_the_default_init_switches() {
        // Skin 0x0500002E (castle guard family): type 2, set 0xC; weapon 0x14 → set 6; weapon 2 → set 4.
        assert_eq!(drone_setup(0x0500_002e, 0, 0x0700_0005), DroneSetup { kind: 2, anim_set: 0xc });
        assert_eq!(drone_setup(0x0500_002e, 0x14, 0x0700_0005).anim_set, 6);
        assert_eq!(drone_setup(0x0500_002e, 2, 0x0700_0005).anim_set, 4);
        // Type 0xC uses set 8 for those weapons; type 0xB ignores 0x12/0x14/0x15.
        assert_eq!(drone_setup(0x0500_003b, 2, 0).anim_set, 8);
        assert_eq!(drone_setup(0x0500_001e, 0x14, 0).anim_set, 7);
        // Level 0x07000014 substitutes skin 0x050000B1.
        assert_eq!(drone_setup(0x0500_00b1, 0, 0x0700_0014), drone_setup(0x0500_008c, 0, 0));
        assert_eq!(drone_setup(0x0500_9999, 0, 0), DroneSetup { kind: 0, anim_set: 0xc });
        assert_eq!(start_danim(DroneSetup { kind: 2, anim_set: 0xc }, 0), (DANIM_STAND_IDLE, 0xc));
        assert_eq!(start_danim(DroneSetup { kind: 0xb, anim_set: 7 }, 0).0, 0x1a5);
    }

    #[test]
    fn spawner_groups_become_templates() {
        // 07000005's shape: spawner A (group 1, one at a time, channel 0), spawner B (group 2, channel 0x22),
        // two drones in each group, one plain drone.
        let d = |g: u32| (0x0f, vec![(0, 0x0500_002e), (4, g)]);
        let s = |group: u32, max: u32, ch: u32| (0xf1, vec![(0, 2), (1, group), (2, max), (4, ch)]);
        let roles = spawn_roles(&[d(2), d(2), d(0), d(1), s(1, 1, 0), s(2, 0, 0x22), d(1)]);
        assert_eq!(roles, vec![
            SpawnRole::Reinforcement { channel: 0x22 }, SpawnRole::Reinforcement { channel: 0x22 }, SpawnRole::Placed,
            SpawnRole::SpawnedAtStart, SpawnRole::Reinforcement { channel: 0 },
        ]);
        // A spawner needs parameter 0 > 1; a group without a spawner stays placed.
        let roles = spawn_roles(&[d(3), (0xf1, vec![(0, 1), (1, 3)])]);
        assert_eq!(roles, vec![SpawnRole::Placed]);
    }

    #[test]
    fn script_falls_back_to_set_zero() {
        let mut row = [0u16; ANIM_SETS];
        row[12] = 0x1bb;
        let d = DroneAnimData { states: vec![(0, vec![]); STATE_COUNT], info: vec![[0; 24]], tables: vec![row] };
        assert_eq!(d.script_for(0, 12), Some(0x0600_01bb));
        assert_eq!(d.script_for(0, 3), None); // set 3 empty → set 0, also empty → no script
        let mut row0 = row;
        row0[0] = 0x55;
        let d0 = DroneAnimData { tables: vec![row0], ..d };
        assert_eq!(d0.script_for(0, 3), Some(0x0600_0055));
    }
}
