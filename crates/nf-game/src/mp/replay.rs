//! The recorded-input replay harness for the MP core.
//!
//! A lockstep host exchanges only per-tick inputs; every machine must derive the identical match
//! from `(settings, seed, per-tick inputs)`. This module is that contract, executable: an
//! [`MpRecord`] is the complete transcript — the seed, the settings, and per tick the human
//! slot positions (`MpMatch::set_slot` — the pure core's stand-in for applying an agent's input),
//! the ordered cross-sim events (kills, bullet hits, touches), the on-hill mask and the paused
//! flag — and [`run_record`] drives the exact calls the live host makes, in the exact order:
//! positions → events → `update_objects` → `update`, all on one `Rng::new(seed)` stream.
//!
//! The per-tick hashes come back so a test (or a host) can pin the whole run tick-by-tick. The
//! bot brains are *not* inputs and not recorded: they are part of the shared simulation, driven
//! from the same seed (see `mp_lockstep.rs`'s two-sim test, which hashes them alongside the
//! match). `DamageSite`'s `f32` is an input and travels by bit pattern, like every float in
//! `mp::hash`.
//!
//! The tick order matches the established test driver (`mp_match_rules.rs::tick`):
//! `update_objects(DT, …)` before `update(DT, …)` — the game's `MP_ObjectUpdate` and `MP_Update`
//! run in that order on the same frame.

use super::hash::state_hash;
use super::{MpMatch, MpSettings};
use crate::drone::Rng;
use crate::world::CollisionWorld;

/// The fixed sim step. The port's Sim folds this literal every tick (the game samples
/// `psiGetTimeIn100ths`, a wall clock — *Deviation*, the mp module doc); every host folds the
/// same constant in the same order, which is what keeps the f32 timer accumulation bit-stable.
pub const DT: f32 = 1.0 / 60.0;

/// One cross-sim event — the input-derived calls a lockstep host applies for tick N before
/// advancing the sim. Each maps 1:1 onto a public `MpMatch`/rules method.
#[derive(Clone, Debug, PartialEq)]
pub enum MpEvent {
    /// `rules::player_killed`.
    Killed {
        victim: usize,
        killer: Option<usize>,
        explosive: bool,
        golden_gun: bool,
    },
    /// `rules::register_bullet_hit`.
    BulletHit { victim: usize, attacker: usize },
    /// `get_spawn_point` + `rules::do_respawn` — the spawn index is NOT recorded: the real path
    /// picks it through the shared `Rng` (`MP_GetSpawnPoint` 0x185098), so the replay re-picks it
    /// identically from the same stream.
    Respawn { slot: usize },
    /// `rules::touch_flag`.
    TouchFlag { slot: usize },
    /// `rules::touch_uplink`.
    TouchUplink { term: usize, slot: usize },
    /// `rules::damage_site`.
    DamageSite { dmg: f32, hitter: usize },
    /// `rules::touch_ge_item`.
    TouchGeItem { item: usize, slot: usize },
    /// `rules::touch_blueprint`.
    TouchBlueprint { slot: usize },
    /// `rules::ge_strike_complete`.
    GeStrikeComplete,
}

/// One spawn-point registration (`register_spawn_point`) — level-derived (the ISO's 0x25
/// placements), identical on every peer; carried in the record so a headless replay is
/// self-contained (a live host loads them from the shared level, it does not send them).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpawnReg {
    pub pos: [f32; 3],
    pub facing: [f32; 3],
    pub team: u16,
}

/// One MP-object registration (`register_mp_object`) — level-derived like [`SpawnReg`] (the 0x26
/// placements; config word 0 = the type, word 1 = the team [M 0x185990]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ObjReg {
    pub pos: [f32; 3],
    pub facing: [f32; 3],
    pub mp_type: u32,
    pub config: [u32; 7],
    pub cel: u32,
}

/// One tick's inputs.
#[derive(Clone, Debug, Default)]
pub struct MpTickInput {
    /// The KOTH hill occupancy (`update_objects`' per-slot mask).
    pub on_hill: [bool; 8],
    /// `update`'s paused flag.
    pub paused: bool,
    /// Human slot positions applied this tick (`set_slot`), in slot order.
    pub set_slots: Vec<(usize, [f32; 3])>,
    /// The ordered events (kills → hits → touches; the order is the input order).
    pub events: Vec<MpEvent>,
}

/// The complete transcript of a match: `(settings, seed, per-tick inputs)` — everything a
/// lockstep peer needs to reproduce the run bit-for-bit.
#[derive(Clone, Debug)]
pub struct MpRecord {
    /// The match RNG seed (`Rng::new(seed)` — the game's `Rand_*` generator, one stream per match).
    pub seed: u32,
    /// `MPSettings` (the menu-flow `MpSettings` snapshot).
    pub settings: MpSettings,
    /// Whether `create_mode_objects` runs after `start` (the scenario-object build; draws RNG).
    pub with_mode_objects: bool,
    /// The spawn-point registrations (level-derived, see [`SpawnReg`]).
    pub spawn_registrations: Vec<SpawnReg>,
    /// The MP-object registrations (level-derived, see [`ObjReg`]).
    pub object_registrations: Vec<ObjReg>,
    /// The slots to place at match start, in order — each through the real spawn policy
    /// (`get_spawn_point`'s shared-stream draw, then `set_slot`). The spawn INDICES are derived
    /// state, not inputs: every lockstep peer re-picks them identically from the same stream, so
    /// the record carries only the slot list.
    pub slot_spawns: Vec<usize>,
    /// The per-tick inputs.
    pub ticks: Vec<MpTickInput>,
}

/// Applies one recorded event through the real sim method — the single mapping a lockstep host
/// uses for both live play and replay, so the two can never drift apart.
pub fn apply_event(m: &mut MpMatch, ev: &MpEvent, rng: &mut Rng) {
    match ev {
        MpEvent::Killed { victim, killer, explosive, golden_gun } => {
            m.player_killed(*victim, *killer, *explosive, *golden_gun, rng);
        }
        MpEvent::BulletHit { victim, attacker } => {
            let _ = m.register_bullet_hit(*victim, *attacker);
        }
        MpEvent::Respawn { slot } => {
            let team = m.settings.players[*slot].team as u16;
            let idx = m.get_spawn_point(team, *slot, rng);
            m.do_respawn(*slot, idx);
        }
        MpEvent::TouchFlag { slot } => m.touch_flag(*slot),
        MpEvent::TouchUplink { term, slot } => m.touch_uplink(*term, *slot),
        MpEvent::DamageSite { dmg, hitter } => m.damage_site(*dmg, *hitter),
        MpEvent::TouchGeItem { item, slot } => m.touch_ge_item(*item, *slot),
        MpEvent::TouchBlueprint { slot } => m.touch_blueprint(*slot),
        MpEvent::GeStrikeComplete => m.ge_strike_complete(rng),
    }
}

/// Replays a record through the real sim calls and returns the final match plus the per-tick
/// `state_hash`. Pure function of the record: same record → same hashes on any machine.
pub fn run_record(rec: &MpRecord) -> (MpMatch, Vec<u64>) {
    let mut rng = Rng::new(rec.seed);
    let mut m = MpMatch::init(rec.settings.clone());
    let world = CollisionWorld::from_triangles(vec![]);
    for r in &rec.spawn_registrations {
        m.register_spawn_point(r.pos, r.facing, r.team, &world);
    }
    for r in &rec.object_registrations {
        m.register_mp_object(r.pos, r.facing, r.mp_type, r.config, r.cel);
    }
    m.start(&mut rng);
    if rec.with_mode_objects {
        m.create_mode_objects(&mut rng);
    }
    for &slot in &rec.slot_spawns {
        let team = m.settings.players[slot].team as u16;
        let idx = m.get_spawn_point(team, slot, &mut rng);
        let p = m.spawns.points[idx as usize].pos;
        m.set_slot(slot, [p[0], p[1], p[2]]);
    }
    let mut hashes = Vec::with_capacity(rec.ticks.len());
    for t in &rec.ticks {
        for &(slot, pos) in &t.set_slots {
            m.set_slot(slot, pos);
        }
        for ev in &t.events {
            apply_event(&mut m, ev, &mut rng);
        }
        m.update_objects(DT, &t.on_hill, &mut rng);
        m.update(DT, t.paused, &mut rng);
        hashes.push(state_hash(&m));
    }
    (m, hashes)
}
