//! The per-scenario MP rules (402 mp-match-rules) — every rule here carries its game address in
//! `docs/systems/mp-match-rules.md`; this file implements, in order: kill scoring
//! (`MP_PlayerKilled` 0x188f70), the assassin reset (0x18aa70), the friendly-fire gate
//! (`MP_RegisterBulletHit` 0x1857c0), the respawn rules (`MP_ReSpawn` 0x185368), the who-won
//! decision (`MP_SortOutWhoWon` 0x1848d0), the round restart (`MP_RestartScenario` 0x183fc0), the
//! mode-object creation picks (`MP_Start` 0x183a50:77-114) and the per-frame scenario updates
//! (`MP_FlagUpdate` 0x1861a8, `MP_UplinkUpdate` 0x186d98, `MP_KOHUpdate` 0x186ae0,
//! `MP_DemolitionProtectionUpdate` 0x1870a0, `MP_GoldenEyeUpdate` 0x187568,
//! `MP_BluePrintUpdate` 0x1882d8 / `MP_BluePrintReachedBase` 0x188970).
//!
//! Timers are **frames** at the fixed 60 Hz (`FRAME_RATE_INT`); the drip scores use the game's
//! `REC_FRAME_RATE * 0.2` = 0.2 points/s. Touch events (`touch_*`) stand in for the game's
//! `MP_HitBy` 0x1830e0 overlap walk — the rules (team filters, state gates) are here, the spatial
//! test is the caller's.

use super::{scenario, MpEndStatus, MpMatch};
use crate::drone::Rng;

/// 60 Hz — the fixed tick (`FRAME_RATE_INT`; the game's timers count frames [M 0x1861a8:226-229]).
pub const FRAME_RATE_INT: u32 = 60;
/// `FRAME_RATE_INT * 0x1e` — the 30 s dropped-object return [M 0x1861a8:226-229].
pub const RETURN_FRAMES: u32 = FRAME_RATE_INT * 30;
/// The capture/base reach: sq-dist < 2.0 [M 0x1861a8:74-86, disasm `c.olt.s` 2.0].
pub const CAPTURE_SQ_DIST: f32 = 2.0;

/// One MP object's live state — the port's stand-in for `obj_tag+0xF4/+0xF6` and the
/// `MPOBJECT+0x02/+0x04/+0x08/+0x50/+0x52` fields [M 0x185de8:129-131, 0x1861a8, 0x1870a0:81-91].
#[derive(Clone, Debug, Default)]
pub struct MpObjState {
    /// `obj+0xF4` (i16): the scenario state word — per-type meaning (flag 0/1/2, uplink 2/0/1,
    /// site HP 2000.., GE item 0/1/2/3, blueprint 0/1/2).
    pub state: i16,
    /// `obj+0xF6` (i16): the sub-index / done flag (GE item 0/1, site done = 1).
    pub sub: i16,
    /// `MPOBJECT+0x04` (u16): the return counter (frames, counts up past `RETURN_FRAMES`).
    pub counter: u16,
    /// `MPOBJECT+0x08` / `obj+0x1C`: the carrying player's slot.
    pub carrier: Option<usize>,
    /// `MPOBJECT+0x50` (i16): the last hitter slot (the demolition site).
    pub last_hitter: i16,
    /// `MPOBJECT+0x52` (i16): the controlling player slot (uplink), −1 = none.
    pub controller: i16,
    /// `MPOBJECT+0x02` (u16): the team word (the flag's owner; the GE/blueprint carrying team).
    pub team: u16,
    /// `obj+0xC0`: the object's current world position (the drop spot when dropped).
    pub pos: [f32; 3],
    /// The home/spawn position (`MPOBJECT+0x10`'s matrix translation) — where the object returns
    /// to [M 0x1861a8:99-112].
    pub home: [f32; 3],
}

/// The scenario objects' live state — the port's `Flags`[2] @0x317210, `Bases`[2] @0x317330,
/// `Uplinks`[8] @0x317450, `Hill` @0x318CE0, the demolition/protection site, `GoldenEye`[2]
/// @0x317FF0/0x318080, `BluePrint` @0x318830 and `EsponageBase`[2] [M 0x185de8:133-178].
/// The getter family over these records: `MP_getFlagObj` 0x18b978 / `MP_getBaseObj` 0x18b9a0 /
/// `MP_getUplinkObj` 0x18b9c8 / `MP_getHillObj` 0x18bb50 / `MP_getDemolitionObj` 0x18bab8 /
/// `MP_getProtectionObj` 0x18bac8 / `MP_getGoldenEyeObj` 0x18baf0 (gates on state ∈ {0, 2}) /
/// `MP_getBlueprintObj` 0x18bad8 / `MP_getEsponageBaseObj` 0x18a338 / `MP_getObjExtFromMPOBJECT`
/// 0x18a210. `MP_recalcObjExtPaths` 0x18b888's position-refresh half is the carrier updates
/// below; its AI-emitter re-walk is not ported.
#[derive(Clone, Debug, Default)]
pub struct MpObjectsRt {
    /// The two CTF flags (index = owning team).
    pub flags: [Option<MpObjState>; 2],
    /// The two CTF bases' capture points (the base obj's +0xC0 position).
    pub bases: [Option<[f32; 3]>; 2],
    /// The uplink terminals (≤ 8 [M 0x185990:55-63]).
    pub uplinks: Vec<MpObjState>,
    /// The KOTH hill.
    pub hill: Option<MpObjState>,
    /// The one demolition/protection site.
    pub site: Option<MpObjState>,
    /// The two GoldenEye items (sub 0/1).
    pub ge: [Option<MpObjState>; 2],
    /// The GoldenEye strike's target slot (global from `MP_GetTarget` at launch [M 0x187568:362-367]).
    pub ge_target: Option<usize>,
    /// The blueprint item.
    pub blueprint: Option<MpObjState>,
    /// The two espionage bases (the blueprint delivery points).
    pub esp_bases: [Option<[f32; 3]>; 2],
}

impl MpObjectsRt {
    fn obj_at(records: &[super::MpObjectRecord], i: usize) -> [f32; 3] {
        [records[i].matrix[3][0], records[i].matrix[3][1], records[i].matrix[3][2]]
    }

    fn config_team(records: &[super::MpObjectRecord], i: usize) -> u16 {
        (records[i].config[1] & 0xffff) as u16
    }
}

/// The who-won decision of `MP_SortOutWhoWon` 0x1848d0 (numbers only — the game's texts are the
/// renderer's). `Winner::None` for team games: the function never compares the team scores there
/// [M 0x1848d0:87-92]; the scoreboard judges.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Winner {
    /// Nobody qualified / the generic team text.
    None,
    /// The unique winner's slot.
    Slot(usize),
    /// The draw text (a tie at the minimum, or 0/≥2 qualifiers).
    Draw,
}

fn sq_dist(a: [f32; 3], b: [f32; 3]) -> f32 {
    let d = [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    d[0] * d[0] + d[1] * d[1] + d[2] * d[2]
}

impl MpMatch {
    // ---------------------------------------------------------------- kill scoring

    /// `MP_PlayerKilled` 0x188f70 — the full scoring path. `killer` is the resolved credit slot
    /// (the game walks `HitByList`; the caller resolves), `explosive` = the killing hurt type was
    /// 4..7 [M 0x188f70:193-198] (preloads the carried objects' return counters), `golden_gun` =
    /// the bot golden-gun special (+2.0 [M :170-178]).
    pub fn player_killed(&mut self, victim: usize, killer: Option<usize>, explosive: bool, golden_gun: bool, rng: &mut Rng) {
        if self.settings.master == 0 {
            return; // [M 0x188f70:84-86]
        }
        self.game.slots[victim].deaths += 1; // every scenario [M :92-94]
        if self.settings.scenario == scenario::DEATH_LIMIT_FFA {
            self.game.slots[victim].score -= 1.0; // the lives copy [M :96-98]
        }
        self.game.slots[victim].dead = true; // the port's obj+0xF4 ∈ {2,3} stand-in
        let mut delta = 0.0f32;
        if let Some(k) = killer {
            if k == victim {
                let s = &mut self.game.slots[victim];
                s.streak = 0;
                s.kills -= 1;
                s.last_killer = 0xffff; // [M :151-155]
            } else {
                let same_team = self.settings.players[victim].team == self.settings.players[k].team;
                if self.settings.team_game == 0 || !same_team {
                    self.game.slots[k].kills += 1; // [M :157-163]
                    self.game.slots[k].streak += 1;
                }
                self.game.slots[victim].last_killer = k as u16; // [M :164]
            }
            delta = if k == victim {
                -1.0 // self-kill [M :167-169]
            } else if golden_gun {
                2.0 // [M :170-178]
            } else {
                1.0
            };
            // the personal gate: bit30 == 0 AND one of the four scenarios [M :179-184]
            let s = self.settings.scenario;
            if self.settings.bit30 == 0
                && (s == scenario::DEATHMATCH
                    || s == scenario::TEAM_DEATHMATCH
                    || s == scenario::KOTH_FFA
                    || s == scenario::KOTH_TEAM)
            {
                self.game.slots[k].score += delta;
            }
            // the team effects — scenario 0x20000002 only [M :246-274]
            if self.settings.scenario == scenario::TEAM_DEATHMATCH {
                let vt = self.settings.players[victim].team as usize;
                if self.settings.players[k].team == self.settings.players[victim].team {
                    // same team (incl. self): the victim's team pays |Δ| [M :248-252]
                    self.game.team_scores[vt] -= delta.abs();
                } else {
                    // enemy kill: the opposing team gains |Δ| [M :261-273]
                    self.game.team_scores[self.settings.players[k].team as usize] += delta.abs();
                }
            }
        } else {
            // no player credit (uVar16 == 0): the victim's team pays |Δ| = 0 [M :254-259,273]
            if self.settings.scenario == scenario::TEAM_DEATHMATCH {
                self.game.team_scores[self.settings.players[victim].team as usize] -= 0.0;
            }
        }
        // the assassin block — scenario 0x400 [M :223-245]
        if self.settings.scenario == scenario::ASSASSIN {
            let killer_slot = killer.filter(|k| *k != victim);
            if let Some(t) = self.assassin_target {
                if victim == t && killer_slot == self.assassin {
                    self.game.slots[killer_slot.unwrap()].score += 5.0; // [M :229-233]
                }
            }
            if self.assassin == Some(victim) {
                if let (Some(t), Some(k)) = (self.assassin_target, killer_slot) {
                    if k == t {
                        self.game.slots[k].score += 3.0; // [M :234-242]
                    }
                }
                self.assassin_reset(rng); // [M :243]
            }
        }
        // the carried objects drop [M :276-303]
        self.drop_carried(victim, explosive);
    }

    /// The carried-object drop of `MP_PlayerKilled` 0x188f70:276-303 — flag (type 0), blueprint
    /// (type 5), GE items (types 6/7) drop on their carrier's death (the object lands at the
    /// death spot — the game's floor-snapped `obj+0xC0` [M 0x1861a8:129-136]); the return counter
    /// preloads to `RETURN_FRAMES` when the kill was explosive.
    fn drop_carried(&mut self, victim: usize, explosive: bool) {
        let drop_pos = self.game.slots[victim].pos;
        for f in self.objects_rt.flags.iter_mut().flatten() {
            if f.carrier == Some(victim) {
                f.state = 2;
                f.carrier = None;
                f.pos = drop_pos.unwrap_or(f.pos);
                if explosive {
                    f.counter = RETURN_FRAMES as u16; // [M 0x188f70:276-299]
                }
            }
        }
        for it in self.objects_rt.ge.iter_mut().flatten() {
            if it.carrier == Some(victim) {
                it.state = 2;
                it.carrier = None;
                it.pos = drop_pos.unwrap_or(it.pos);
                if explosive {
                    it.counter = RETURN_FRAMES as u16;
                }
            }
        }
        if let Some(bp) = &mut self.objects_rt.blueprint {
            if bp.carrier == Some(victim) {
                bp.state = 2;
                bp.carrier = None;
                bp.pos = drop_pos.unwrap_or(bp.pos);
                if explosive {
                    bp.counter = RETURN_FRAMES as u16;
                }
            }
        }
        self.game.slots[victim].status_bits &= !1; // the carrying bit clears [M 0x18a518]
    }

    /// `MP_assassinReset` 0x18aa70 — the new assassin is the dead assassin's last-killer when
    /// there is one (the kill just wrote it), else a random pick; the new target is a random pick
    /// among the other present slots (`MP_GetTarget` 0x1832b8 + `Rand_Rand`); the assassin's
    /// settings record team word = 1, everyone else's 0 [M 0x18aa70:12-41].
    fn assassin_reset(&mut self, rng: &mut Rng) {
        let dead = match self.assassin {
            Some(d) => d,
            None => return,
        };
        for p in &mut self.settings.players {
            p.team = 0; // the team words zero [M :12-19]
        }
        let lk = self.game.slots[dead].last_killer;
        self.assassin = None;
        self.assassin_target = None;
        let present: Vec<usize> = (0..8).filter(|i| self.game.slots[*i].pos.is_some()).collect();
        let new_a = if lk != 0xffff && lk as usize != dead && present.contains(&(lk as usize)) {
            Some(lk as usize) // [M :24-28]
        } else if present.is_empty() {
            None
        } else {
            Some(present[rng.rand(present.len() as u32) as usize]) // [M :35-37]
        };
        self.assassin = new_a;
        let cands: Vec<usize> =
            present.iter().copied().filter(|i| Some(*i) != self.assassin).collect();
        self.assassin_target = if cands.is_empty() {
            None
        } else {
            Some(cands[rng.rand(cands.len() as u32) as usize]) // [M :29-34]
        };
        if let Some(a) = self.assassin {
            self.settings.players[a].team = 1; // [M :38-41]
        }
    }

    // --------------------------------------------------------------- friendly fire

    /// `MP_RegisterBulletHit` 0x1857c0 — returns whether the hit is allowed. MP off → allowed; no
    /// attacker → blocked; the team game's friendly-fire gate (`friendly_fire == 0` blocks, the
    /// warn timers at +0x22); the victim's last-shooter word (+0x20) records the attacker in every
    /// allowed case [M 0x1857c0:33-57].
    pub fn register_bullet_hit(&mut self, victim: usize, attacker: usize) -> bool {
        if self.settings.master == 0 {
            return true; // [M 0x1857c0:15-17]
        }
        if self.game.slots[attacker].pos.is_none() {
            return false; // [M :18-20]
        }
        if self.settings.team_game != 0 {
            let same = self.settings.players[victim].team == self.settings.players[attacker].team;
            if same && victim != attacker {
                if self.settings.friendly_fire == 0 {
                    return false; // [M :43-45]
                }
                if self.game.slots[victim].timers[0] == 0 {
                    self.game.slots[victim].timers[0] = (3 * FRAME_RATE_INT) as u16; // [M :56]
                }
            }
        }
        self.game.slots[victim].status = attacker as u16; // [M :34]
        true
    }

    // ------------------------------------------------------------------- respawn

    /// `MP_ReSpawn` 0x185368:16-20 — the gates only (the position/health reset is
    /// [`MpMatch::do_respawn`]; the 5 s death delay is the caller's `Player_HandleDeath` gate [M
    /// 0x190bb0:41-54]).
    pub fn respawn_allowed(&self, slot: usize) -> bool {
        if self.settings.master == 0 || self.game.state != 0 {
            return false;
        }
        if self.settings.scenario == scenario::DEATH_LIMIT_FFA
            && self.game.slots[slot].deaths >= self.settings.frag_limit
        {
            return false; // out of lives [M 0x185368:16-20]
        }
        true
    }

    /// `MP_ReSpawn` 0x185368 — the whole per-slot respawn the live host calls: the gates
    /// ([`MpMatch::respawn_allowed`], [M 0x185368:16-20]), the spawn pick through the policy
    /// ([`MpMatch::get_spawn_point`] [M 0x185368:31 — the game passes the slot's team word from
    /// `MPSettings + slot·0x30 + 0x20`]) and the apply ([`MpMatch::do_respawn`] — the position
    /// reset, the streak/last-shooter/status words, the alive flag). Returns
    /// `Some((spawn_idx, health))` when the respawn ran, `None` on the gates. The caller moves the
    /// body and ends its death sequence (the game's `Player_SetHealth`/`Player_ChangeState(0)`/
    /// `obj+0xff = 3` half [M 0x185368:76-108] is the host's player pipeline).
    pub fn mp_respawn(&mut self, slot: usize, rng: &mut Rng) -> Option<(u32, f32)> {
        if !self.respawn_allowed(slot) {
            return None;
        }
        let team = self.settings.players[slot].team as u16;
        let idx = self.get_spawn_point(team, slot, rng);
        let hp = self.do_respawn(slot, idx);
        Some((idx, hp))
    }

    /// `MP_ReSpawn` 0x185368:22-91 — applies the spawn reset at `spawn_idx` (chosen by the
    /// caller's `MP_GetSpawnPoint`) and returns the human health (`armor_bonus + 100.0` [M
    /// :65-73]).
    pub fn do_respawn(&mut self, slot: usize, spawn_idx: u32) -> f32 {
        let p = self.spawns.points[spawn_idx as usize].pos;
        let s = &mut self.game.slots[slot];
        s.pos = Some([p[0], p[1], p[2]]);
        s.streak = 0; // [M 0x185368:55-57]
        s.status = 0xfffe;
        s.status_bits = 0;
        s.dead = false;
        self.settings.players[slot].armor_bonus as f32 + 100.0
    }

    // --------------------------------------------------------------- who won / restart

    /// `MP_SortOutWhoWon` 0x1848d0 — the decision (state 3) + the end status (`MpGame::end_status`,
    /// the status sprite's text data [M :21-92]). 0x10: the fewest deaths among the live slots
    /// under the lives limit, with the count/tie quirk [M :23-52]; FFA: exactly one slot at/over
    /// the limit, else a draw [M :55-84]; team games always report the draw (the team winner is
    /// the debriefing's line) [M :87-92]. The `Winner` return is the flow result (402's API); the
    /// status carries the codename for the draw code.
    pub fn sort_out_who_won(&mut self) -> Winner {
        self.game.state = 3; // [M 0x1848d0:20]
        let limit = self.settings.frag_limit;
        if self.settings.scenario == scenario::DEATH_LIMIT_FFA {
            // The survivor count increments BEFORE the min-compare and a tie at the running
            // minimum zeroes it [M 0x1848d0:32-40] — so the draw fires only when the count is
            // zero at the END, i.e. the LAST qualifier tied the minimum and nothing followed
            // (a later qualifier re-increments: [3,3,1] crowns the 1). 402's tie-flag version
            // missed the re-increment; mp-debriefing-data2 settled this from the decomp.
            let mut count: i32 = 0; // iVar10 [M 0x1848d0:23-52]
            let mut best = 30000i32; // iVar2's start [M :23]
            let mut winner: Option<usize> = None;
            for (i, slot) in self.game.slots.iter().enumerate() {
                if slot.pos.is_none() {
                    continue; // the obj gate [M :29]
                }
                let d = slot.deaths;
                if d >= limit {
                    continue; // [M :31]
                }
                count += 1;
                if d < best {
                    best = d;
                    winner = Some(i);
                } else if d == best {
                    count = 0; // the tie quirk [M :38-40]
                }
            }
            self.game.end_status = match winner {
                Some(w) if count != 0 => MpEndStatus::Won(self.settings.players[w].codename.clone()),
                _ => MpEndStatus::Draw, // [M :49-52]
            };
            match winner {
                Some(w) if count != 0 => Winner::Slot(w),
                _ => Winner::Draw,
            }
        } else if self.settings.team_game == 0 {
            // NO obj gate — the game's FFA qualifier loop reads only the slot scores
            // [M 0x1848d0:56-71; the decomp has no `obj != 0` test in this branch]. 402 filtered
            // on `pos.is_some()`; mp-debriefing-data2 removed it (a qualifier needs only
            // `frag_limit <= score`).
            let f_limit = limit as f32;
            let mut count = 0usize;
            let mut winner: Option<usize> = None;
            for (i, slot) in self.game.slots.iter().enumerate() {
                if f_limit <= slot.score {
                    count += 1; // [M :62-67]
                    winner = Some(i);
                }
            }
            self.game.end_status = match winner {
                Some(w) if count == 1 => MpEndStatus::Won(self.settings.players[w].codename.clone()),
                _ => MpEndStatus::Draw, // [M :79-84]
            };
            match winner {
                Some(w) if count == 1 => Winner::Slot(w),
                _ => Winner::Draw,
            }
        } else {
            // the team branch compares nothing — the status is the draw text (the team winner
            // is announced on the debriefing) [M :87-92]; `Winner::None` keeps 402's flow result.
            self.game.end_status = MpEndStatus::Draw;
            Winner::None
        }
    }

    /// `MP_RestartScenario` 0x183fc0:40-82 — the round clock back to 0, a new random site/item
    /// pick (a `Rand_Rand` draw on the shared RNG), the done flags clear, and every present slot's
    /// last-killer/last-shooter words reset to 0xffff. Scores are **not** cleared (multi-round
    /// Demolition/Protection); the players' respawn is the caller's (`MP_ReSpawn` per slot).
    pub fn restart_scenario(&mut self, rng: &mut Rng) {
        self.game.clock = 0.0; // [M 0x183fc0:40-53]
        self.game.restart_timer = 0.0;
        let s = self.settings.scenario;
        if s == scenario::DEMOLITION || s == scenario::PROTECTION {
            let pool: Vec<usize> = match s {
                scenario::DEMOLITION => (0..self.objects.demolition.len()).collect(),
                _ => (0..self.objects.protection.len()).collect(),
            };
            if let Some(site) = &mut self.objects_rt.site {
                site.sub = 0;
                site.state = 2000; // the HP re-inits [M 0x185de8:174]
                site.counter = 0;
                if !pool.is_empty() {
                    let pick = pool[rng.rand(pool.len() as u32) as usize] as usize; // [M 0x183fc0:40-53]
                    site.pos = MpObjectsRt::obj_at(
                        if s == scenario::DEMOLITION { &self.objects.demolition } else { &self.objects.protection },
                        pick,
                    );
                }
            }
        }
        for slot in &mut self.game.slots {
            slot.last_killer = 0xffff; // [M 0x183fc0:55-82]
            slot.status = 0xffff;
        }
    }

    // ------------------------------------------------------- mode-object creation

    /// `MP_Start` 0x183a50:77-114 — builds the runtime objects from the registered placements.
    /// Flags/bases/uplinks/hill/espionage-bases link directly (the game creates them at
    /// registration); the demolition/protection site, the blueprint item and the two GoldenEye
    /// items pick a random registered place each (`Rand_Rand` — the game's start draws).
    /// Post-load resolve: `MP_initObjExt` (0x18b7a0) + `MP_PostLoad_Init` (0x183718) — the game
    /// re-resolves scripted site objects and re-emits AI paths for the mode objects here; the
    /// port links placements once at start [S partial].
    pub fn create_mode_objects(&mut self, rng: &mut Rng) {
        let s = self.settings.scenario;
        // the direct links from `created` (config word 0 = the type, word 1 = the team)
        for i in 0..self.objects.created.len() {
            let (ty, team, pos) = {
                let r = &self.objects.created[i];
                (r.config[0], (r.config[1] & 0xffff) as u16, MpObjectsRt::obj_at(&self.objects.created, i))
            };
            match ty {
                0 if s == scenario::CAPTURE_THE_FLAG => {
                    self.objects_rt.flags[team as usize % 2] =
                        Some(MpObjState { state: 0, team, pos, home: pos, ..Default::default() });
                }
                1 if s == scenario::CAPTURE_THE_FLAG => {
                    self.objects_rt.bases[team as usize % 2] = Some(pos);
                }
                2 if s == scenario::UPLINK => {
                    // the neutral init [M 0x185de8:146]
                    self.objects_rt.uplinks.push(MpObjState { state: 2, team, pos, ..Default::default() });
                }
                4 if s == scenario::BLUEPRINT => {
                    self.objects_rt.esp_bases[team as usize % 2] = Some(pos);
                }
                9 if s == scenario::KOTH_FFA || s == scenario::KOTH_TEAM => {
                    self.objects_rt.hill = Some(MpObjState { pos, home: pos, ..Default::default() });
                }
                _ => {}
            }
        }
        // the random picks [M 0x183a50:77-114]
        if s == scenario::DEMOLITION || s == scenario::PROTECTION {
            let (records, n) = if s == scenario::DEMOLITION {
                (0, self.objects.demolition.len())
            } else {
                (1, self.objects.protection.len())
            };
            if n > 0 {
                let pick = rng.rand(n as u32) as usize;
                let pos = if records == 0 {
                    MpObjectsRt::obj_at(&self.objects.demolition, pick)
                } else {
                    MpObjectsRt::obj_at(&self.objects.protection, pick)
                };
                self.objects_rt.site = Some(MpObjState {
                    state: 2000, // the HP init [M 0x185de8:174]
                    team: if s == scenario::DEMOLITION { 0 } else { 1 },
                    pos,
                    home: pos,
                    ..Default::default()
                });
            }
        }
        if s == scenario::BLUEPRINT && !self.objects.blueprint_items.is_empty() {
            let n = self.objects.blueprint_items.len();
            let pick = rng.rand(n as u32) as usize;
            let pos = MpObjectsRt::obj_at(&self.objects.blueprint_items, pick);
            self.objects_rt.blueprint = Some(MpObjState { state: 0, pos, home: pos, ..Default::default() });
        }
        if s == scenario::GOLDENEYE {
            for (i, pool) in self.objects.goldeneye.iter().enumerate() {
                if !pool.is_empty() {
                    let pick = rng.rand(pool.len() as u32) as usize; // [M 0x183a50:101-114]
                    let pos = MpObjectsRt::obj_at(pool, pick);
                    self.objects_rt.ge[i] =
                        Some(MpObjState { state: 0, sub: i as i16, pos, home: pos, ..Default::default() });
                }
            }
        }
    }

    // ------------------------------------------------------------- per-frame updates

    /// The scenario-object updates of one frame — the port's `MP_ObjectUpdate` 0x18b198 walk.
    /// The game's object system runs every frame regardless of the match state (the Protection
    /// timeout scores the frame after the state-6 latch — the site's own done flag stops it), so
    /// this is gated on the master/started flags only. `on_hill` is the caller's KOTH volume test
    /// (the game's trigger box is level data [M 0x186ae0:88-93]); `rng` is drawn only by the
    /// GoldenEye/blueprint respawn re-picks.
    pub fn update_objects(&mut self, dt: f32, on_hill: &[bool; 8], rng: &mut Rng) {
        if self.settings.master == 0 || !self.settings.started {
            return;
        }
        self.uplink_update(dt);
        self.koth_update(dt, on_hill);
        self.site_update();
        self.flag_update();
        self.ge_update(dt, rng);
        self.blueprint_update(dt, rng);
    }

    /// `MP_UplinkUpdate` 0x186d98:59-98 — every owned terminal drips 0.2/s into its team score and
    /// (when recorded) its controller's personal score; in team games the owning team's score is
    /// then recomputed as Σ that team's personal scores [M :66-80].
    fn uplink_update(&mut self, dt: f32) {
        if self.settings.scenario != scenario::UPLINK {
            return;
        }
        for ui in 0..self.objects_rt.uplinks.len() {
            let (state, controller) = {
                let u = &self.objects_rt.uplinks[ui];
                (u.state, u.controller)
            };
            if state != 0 && state != 1 {
                continue;
            }
            let t = state as usize;
            self.game.team_scores[t] += dt * 0.2; // [M 0x186d98:59-60]
            if controller >= 0 {
                let c = controller as usize;
                self.game.slots[c].score += dt * 0.2; // [M :61-65]
            }
            if self.settings.team_game != 0 {
                let mut sum = 0.0f32;
                for i in 0..8 {
                    if self.settings.players[i].team as usize == t {
                        sum += self.game.slots[i].score;
                    }
                }
                self.game.team_scores[t] = sum; // [M :66-80]
            }
        }
    }

    /// `MP_UplinkUpdate` 0x186d98:24-114 — the touch rule: a neutral terminal is claimed by the
    /// first toucher; an owned terminal flips at once to an enemy toucher (the controller follows).
    pub fn touch_uplink(&mut self, term: usize, slot: usize) {
        if self.settings.scenario != scenario::UPLINK {
            return;
        }
        let team = self.settings.players[slot].team as i16;
        let u = &mut self.objects_rt.uplinks[term];
        if u.state != team {
            u.state = team;
            u.controller = slot as i16; // [M 0x186d98:88-98, 24-52]
            u.sub = 0;
        }
    }

    /// `MP_KOHUpdate` 0x186ae0:78-151 — every alive player on the hill drips 0.2/s personally; in
    /// team games the scoring player's team score is recomputed as Σ that team's personals [M
    /// :122-137]. (The 5-point milestone sound and the on-hill status bits are presentation.)
    fn koth_update(&mut self, dt: f32, on_hill: &[bool; 8]) {
        let s = self.settings.scenario;
        if s != scenario::KOTH_FFA && s != scenario::KOTH_TEAM {
            return;
        }
        for slot in 0..8 {
            if !on_hill[slot] || self.game.slots[slot].pos.is_none() || self.game.slots[slot].dead {
                continue; // the alive gate [M 0x186ae0:86]
            }
            self.game.slots[slot].score += dt * 0.2; // [M :118-121]
            if self.settings.team_game != 0 {
                let t = self.settings.players[slot].team as usize;
                let mut sum = 0.0f32;
                for i in 0..8 {
                    if self.settings.players[i].team as usize == t {
                        sum += self.game.slots[i].score;
                    }
                }
                self.game.team_scores[t] = sum; // [M :122-137]
            }
        }
    }

    /// `MP_DemolitionProtectionUpdate` 0x1870a0:112-124 — the timeout scores the **defending**
    /// team (0 for Demolition, 1 for Protection — the dispatch's param_3 [M 0x18b198:22-24,35-37]).
    fn site_update(&mut self) {
        let s = self.settings.scenario;
        if s != scenario::DEMOLITION && s != scenario::PROTECTION {
            return;
        }
        let defending = if s == scenario::DEMOLITION { 0usize } else { 1usize };
        let done = self.objects_rt.site.as_ref().map(|x| x.sub == 1).unwrap_or(true);
        if done {
            return; // [M 0x1870a0:77-79]
        }
        if self.game.limit > 0.0 && self.game.limit < self.game.clock {
            self.game.team_scores[defending] += 1.0; // [M :112-124]
            if let Some(site) = &mut self.objects_rt.site {
                site.sub = 1;
            }
        }
    }

    /// `MP_DemolitionProtectionUpdate` 0x1870a0:81-165 — the site takes damage (HP = obj+0xF4,
    /// init 2000 [M 0x185de8:174]); HP < 1: the attacking team +1.0, the last hitter +1.0
    /// personally — an own-goal (the destroyer defending) instead subtracts the defender team and
    /// the hitter's point — the site is done and the round enters state 6 at once [M :125-165].
    pub fn damage_site(&mut self, dmg: f32, hitter: usize) {
        let s = self.settings.scenario;
        if s != scenario::DEMOLITION && s != scenario::PROTECTION {
            return;
        }
        let defending = if s == scenario::DEMOLITION { 0usize } else { 1usize };
        {
            let Some(site) = &mut self.objects_rt.site else { return };
            if site.sub == 1 {
                return;
            }
            site.last_hitter = hitter as i16; // [M 0x1870a0:84-91]
            site.state -= dmg as i16; // the (short) damage [M :81-83]
            if site.state >= 1 {
                return;
            }
        }
        let attacking = 1 - defending;
        self.game.team_scores[attacking] += 1.0; // [M :125-132]
        let h = self.objects_rt.site.as_ref().unwrap().last_hitter as usize;
        self.game.slots[h].score += 1.0; // [M :133-140]
        if self.settings.players[h].team as usize == defending {
            self.game.team_scores[defending] -= 1.0; // the own-goal
            self.game.slots[h].score -= 1.0;
        }
        let site = &mut self.objects_rt.site;
        site.as_mut().unwrap().sub = 1;
        self.game.state = 6; // the round is over [M :160]
    }

    /// `MP_FlagUpdate` 0x1861a8 — the carried flag's capture check (sq-dist < 2.0 of the enemy
    /// base +1.0 y → the taker's team +1.0 and the carrier +1.0, the flag resets home [M
    /// :74-112]) and the dropped flag's 30 s return timer [M :226-259].
    fn flag_update(&mut self) {
        if self.settings.scenario != scenario::CAPTURE_THE_FLAG {
            return;
        }
        for ti in 0..2 {
            let (state, carrier, counter) = match &self.objects_rt.flags[ti] {
                Some(f) => (f.state, f.carrier, f.counter),
                None => continue,
            };
            match state {
                1 => {
                    let Some(c) = carrier else { continue };
                    let Some(cpos) = self.game.slots[c].pos else { continue };
                    let Some(base) = self.objects_rt.bases[1 - ti] else { continue };
                    let point = [base[0], base[1] + 1.0, base[2]]; // [M 0x1861a8:74-86]
                    if sq_dist(cpos, point) < CAPTURE_SQ_DIST {
                        self.game.team_scores[1 - ti] += 1.0; // [M :89-93]
                        self.game.slots[c].score += 1.0; // [M :94-98]
                        let home = self.objects_rt.flags[ti].as_ref().unwrap().home;
                        let f = self.objects_rt.flags[ti].as_mut().unwrap();
                        f.state = 0;
                        f.carrier = None;
                        f.counter = 0;
                        f.pos = home;
                        self.game.slots[c].status_bits &= !1; // the carrying bit clears [M :87-88]
                    }
                }
                2 => {
                    let t = counter;
                    if t > RETURN_FRAMES as u16 {
                        // the 30 s return [M :226-259]
                        let home = self.objects_rt.flags[ti].as_ref().unwrap().home;
                        let f = self.objects_rt.flags[ti].as_mut().unwrap();
                        f.state = 0;
                        f.carrier = None;
                        f.counter = 0;
                        f.pos = home;
                    } else {
                        self.objects_rt.flags[ti].as_mut().unwrap().counter = t + 1;
                    }
                }
                _ => {}
            }
        }
    }

    /// `MP_FlagUpdate` 0x1861a8:170-263 — the touch rule: an enemy takes the flag (from the base
    /// or dropped); a teammate touching a dropped flag returns it at once. The port's stand-in for
    /// the `MP_HitBy` walk with the enemy-team filter.
    pub fn touch_flag(&mut self, slot: usize) {
        if self.settings.scenario != scenario::CAPTURE_THE_FLAG {
            return;
        }
        let my_team = self.settings.players[slot].team;
        for ti in 0..2 {
            let (state, fteam) = match &self.objects_rt.flags[ti] {
                Some(f) => (f.state, f.team),
                None => continue,
            };
            if state == 2 && my_team == fteam as u32 {
                // the teammate return [M :195-221]
                let home = self.objects_rt.flags[ti].as_ref().unwrap().home;
                let f = self.objects_rt.flags[ti].as_mut().unwrap();
                f.state = 0;
                f.carrier = None;
                f.counter = 0;
                f.pos = home;
                return;
            }
            if (state == 0 || state == 2) && my_team != fteam as u32 {
                // the pickup [M :170-190, 223-263]
                let f = self.objects_rt.flags[ti].as_mut().unwrap();
                f.state = 1;
                f.carrier = Some(slot);
                f.counter = 0;
                self.game.slots[slot].status_bits |= 1; // [M 0x18a518:88-96]
                return;
            }
        }
    }

    /// `MP_GoldenEyeUpdate` 0x187568 — the dropped items' 30 s respawn [M :187-228] and the strike
    /// launch when both items are carried by the same team (each carrier +1.0; the target is a
    /// random opposing player via `MP_GetTarget` [M :356-384]).
    fn ge_update(&mut self, dt: f32, rng: &mut Rng) {
        if self.settings.scenario != scenario::GOLDENEYE {
            return;
        }
        // the strike launch [M :356-384]
        let (s0, s1, c0, c1) = match (&self.objects_rt.ge[0], &self.objects_rt.ge[1]) {
            (Some(a), Some(b)) => (a.state, b.state, a.carrier, b.carrier),
            _ => return,
        };
        if s0 == 1 && s1 == 1 {
            let (Some(c0), Some(c1)) = (c0, c1) else { return };
            let ct = self.settings.players[c0].team;
            if ct == self.settings.players[c1].team {
                for c in [c0, c1] {
                    self.game.slots[c].score += 1.0; // [M :368-377]
                }
                // the target: a random player of the opposing team [M :362-367]
                let filter = if ct == 0 { 1u32 } else { 0u32 };
                let cands: Vec<usize> = (0..8)
                    .filter(|i| {
                        self.game.slots[*i].pos.is_some()
                            && self.settings.players[*i].team == filter
                    })
                    .collect();
                self.objects_rt.ge_target = if cands.is_empty() {
                    None
                } else {
                    Some(cands[rng.rand(cands.len() as u32) as usize])
                };
                for i in 0..2 {
                    self.objects_rt.ge[i].as_mut().unwrap().state = 3; // [M :379-384]
                }
                return;
            }
        }
        // the dropped items' respawn timers [M :187-228]
        for i in 0..2 {
            let (state, counter) = match &self.objects_rt.ge[i] {
                Some(it) => (it.state, it.counter),
                None => continue,
            };
            if state != 2 {
                continue;
            }
            if counter > RETURN_FRAMES as u16 {
                let n = self.objects.goldeneye[i].len();
                if n > 0 {
                    let pick = rng.rand(n as u32) as usize;
                    let pos = MpObjectsRt::obj_at(&self.objects.goldeneye[i], pick);
                    let it = self.objects_rt.ge[i].as_mut().unwrap();
                    it.state = 0;
                    it.carrier = None;
                    it.counter = 0;
                    it.pos = pos;
                }
            } else {
                self.objects_rt.ge[i].as_mut().unwrap().counter = counter + dt_frame(dt);
            }
        }
    }

    /// `MP_GoldenEyeUpdate` 0x187568:96-179 — the strike completes: the target dies (the port
    /// marks the death; the game's `Player_Kill`/`BOT_SetHealth(−1)` runs the normal death path —
    /// the port counts the death, no kill credit), the **opposite** team's score += 1.0, and both
    /// items respawn at random pool spawns (a draw each).
    pub fn ge_strike_complete(&mut self, rng: &mut Rng) {
        if self.settings.scenario != scenario::GOLDENEYE {
            return;
        }
        if let Some(t) = self.objects_rt.ge_target.take() {
            let tt = self.settings.players[t].team as usize;
            self.game.team_scores[1 - tt] += 1.0; // [M 0x187568:110-119]
            self.game.slots[t].status = 0xfffe; // [M :118]
            self.game.slots[t].deaths += 1; // the strike kill's death, no credit
            self.game.slots[t].dead = true;
        }
        for i in 0..2 {
            let n = self.objects.goldeneye[i].len();
            if n > 0 {
                let pick = rng.rand(n as u32) as usize;
                let pos = MpObjectsRt::obj_at(&self.objects.goldeneye[i], pick);
                let it = self.objects_rt.ge[i].as_mut().unwrap();
                it.state = 0;
                it.carrier = None;
                it.counter = 0;
                it.pos = pos;
            }
        }
    }

    /// `MP_GoldenEyeUpdate` 0x187568:231-253 — the item touch: any player of any team picks a free
    /// or dropped item (the carrying team is recorded).
    pub fn touch_ge_item(&mut self, item: usize, slot: usize) {
        if self.settings.scenario != scenario::GOLDENEYE {
            return;
        }
        let it = &mut self.objects_rt.ge[item];
        let Some(it) = it else { return };
        if it.state == 0 || it.state == 2 {
            it.state = 1;
            it.carrier = Some(slot);
            it.counter = 0;
            it.team = self.settings.players[slot].team as u16; // the MP_HitBy out-param [M :241]
        }
    }

    /// `MP_BluePrintUpdate` 0x1882d8 + `MP_BluePrintReachedBase` 0x188970 — the carrier reaching
    /// their team's base scores team +1.0 and carrier +1.0 and respawns the blueprint at a random
    /// registered spawn; the dropped item resets to free where it lies after 30 s [M
    /// 0x1882d8:147-180] (the base check is the port's radius stand-in for the room-list overlap
    /// — *Deviation*).
    fn blueprint_update(&mut self, dt: f32, rng: &mut Rng) {
        if self.settings.scenario != scenario::BLUEPRINT {
            return;
        }
        let (state, carrier, counter) = match &self.objects_rt.blueprint {
            Some(bp) => (bp.state, bp.carrier, bp.counter),
            None => return,
        };
        match state {
            1 => {
                let Some(c) = carrier else { return };
                let Some(cpos) = self.game.slots[c].pos else { return };
                let team = self.settings.players[c].team as usize;
                let Some(Some(base)) = self.objects_rt.esp_bases.get(team).copied() else { return };
                if sq_dist(cpos, base) < CAPTURE_SQ_DIST {
                    // MP_BluePrintReachedBase [M 0x188970:27-55]
                    self.game.team_scores[team] += 1.0;
                    self.game.slots[c].score += 1.0;
                    self.game.slots[c].status_bits &= !1;
                    let n = self.objects.blueprint_items.len();
                    let bp = self.objects_rt.blueprint.as_mut().unwrap();
                    bp.state = 0;
                    bp.carrier = None;
                    bp.counter = 0;
                    if n > 0 {
                        let pick = rng.rand(n as u32) as usize;
                        bp.pos = MpObjectsRt::obj_at(&self.objects.blueprint_items, pick);
                    }
                }
            }
            2 => {
                if counter > RETURN_FRAMES as u16 {
                    let bp = self.objects_rt.blueprint.as_mut().unwrap();
                    bp.state = 0; // resets to free where it lies [M 0x1882d8:147-180]
                    bp.carrier = None;
                    bp.counter = 0;
                } else {
                    self.objects_rt.blueprint.as_mut().unwrap().counter = counter + dt_frame(dt);
                }
            }
            _ => {}
        }
    }

    /// `MP_BluePrintUpdate` 0x1882d8:124-145,182-198 — the touch rule: any player picks the free
    /// or dropped blueprint.
    pub fn touch_blueprint(&mut self, slot: usize) {
        if self.settings.scenario != scenario::BLUEPRINT {
            return;
        }
        let Some(bp) = &mut self.objects_rt.blueprint else { return };
        if bp.state == 0 || bp.state == 2 {
            bp.state = 1;
            bp.carrier = Some(slot);
            bp.counter = 0;
            bp.team = self.settings.players[slot].team as u16;
            self.game.slots[slot].status_bits |= 1;
        }
    }
}

/// The frame count a dt contributes (the fixed 60 Hz — dt is 1/60, so one frame).
fn dt_frame(dt: f32) -> u16 {
    let f = (dt * FRAME_RATE_INT as f32).round() as u16;
    if f == 0 {
        1
    } else {
        f
    }
}
