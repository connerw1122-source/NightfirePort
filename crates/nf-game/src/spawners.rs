//! Drone spawners (placement type 0xF1, `DroneSpawner_Create` 0x1378D0 / `DroneSpawner_Control` 0x137E68;
//! docs/systems/triggers.md "Spawners"): the drones of a group are templates that a spawner brings in.
//!
//! Param 7 (+0x14) is the **KILL channel** [L-verified 07000005, docs/fields/hitdata-spawner-ally.md
//! "Live result"]: `Control` latches `ch[p7]` into the byte at +0x20 (0x137e68:50-52) and the latched
//! path skips the spawn machinery entirely and runs the cleanup block (0x137e68:235-288 — the disasm
//! 0x137fbc-0x1380b4): spawned drones are removed (`obj+0xFE |= 1`), the slots are zeroed, and the
//! spawner disables itself — with a level split: on levels outside {0x07000011..0x07000013, 0x07000016,
//! 0x0700004A} every spawned drone is removed; on those levels only drones whose `obj+0xFC` byte (the
//! draw counter — 0 = not drawn recently) is 0 are removed and the spawner waits for the rest. While a
//! script has `Drone_EnableAll(0)` active (`NPCGlobals == 0`) `Control` returns at its first line
//! (0x137e68:21-23), so the latch only fires once the script frees the NPCs — measured live: ch 0x43's
//! wall-hug script held the latch off until `Script_Free`.

use crate::triggers::Channels;

#[derive(Clone, Debug)]
pub struct Spawner {
    pub placement: usize,
    /// 2: spawn while alive < max; 3: only when none is alive.
    pub mode: u32,
    pub group: u32,
    pub max_alive: usize,
    pub waves: u32,
    pub start: u32,
    pub stop: u32,
    pub done: u32,
    pub min_dist: f32,
    /// Drone indices of the group's templates and which of them were used this epoch.
    pub templates: Vec<usize>,
    pub used: Vec<bool>,
    /// Drones it spawned (indices) — the game's live-slot array [max_alive] [M 0x137aa0:46-51].
    pub spawned: Vec<usize>,
    pub active: bool,
    pub immediate: bool,
    /// p7 (+0x14): the **kill channel** [L-verified; M 0x137e68:50-52]. `Control` latches `ch[p7]` into
    /// +0x20; the latched path runs the cleanup block and never spawns again. (fix-actors' "ARM channel"
    /// reading was refuted by the live measurement.)
    pub kill: u32,
    /// +0x20, the p7 latch [M 0x137e68:50-52; L: +0x20 → 1 only after the ch-0x43 script freed the NPCs].
    pub kill_latch: bool,
    pub wave: u32,
    pub this_wave: usize,
    pub gone: bool,
}

/// What a spawner asks of the drone list this frame.
#[derive(Clone, Debug, PartialEq)]
pub enum Action { None, Spawn(usize), Remove(Vec<usize>) }

impl Spawner {
    /// From a placement (`p0` > 1 required, `p1` group non-zero — `Control`'s early-outs at
    /// 0x137e68:44-46/38-40) and the indices of the drones in its group.
    pub fn new(placement: usize, param: impl Fn(u32) -> u32, templates: Vec<usize>, ch: &Channels) -> Option<Spawner> {
        if param(0) <= 1 { return None; }
        if param(1) == 0 { return None; }
        let n = templates.len();
        let max = param(2) as usize;
        let start = param(4);
        // +0x1F "no stealth checks": Init sets it when (p4 = 0 or on) and (p5 = 0 or off) [M 0x137aa0:33-45];
        // cleared at wave rollover [M 0x137e68:148].
        let immediate = (start == 0 || ch.on(start)) && !ch.on(param(5));
        let min = f32::from_bits(param(8));
        Some(Spawner {
            placement, mode: param(0), group: param(1), max_alive: if max == 0 || max > n { n } else { max },
            waves: if param(3) == 0 { 32000 } else { param(3) }, start, stop: param(5), done: param(6),
            kill: param(7), kill_latch: false,
            min_dist: if min > 0.0 && min.is_finite() { min } else { 10.0 },
            used: vec![false; n], templates, spawned: Vec::new(), active: start == 0, immediate, wave: 0, this_wave: 0, gone: false,
        })
    }

    /// One `DroneSpawner_Control` update. `level` is the current level id (the game's `GameState._12_4_`,
    /// read as DAT_002a3774 at 0x137fc4 [M disasm]); `alive(i)`: drone i is in a live slot; `drawn(i)`:
    /// its `obj+0xFC` draw counter is non-zero (the port's per-tick visibility stand-in); `spot_ok(i)`:
    /// template i's spot is at least `min_dist` from the player and not visible to him; `rand(n)`:
    /// `Rand_Rand`.
    pub fn update(&mut self, ch: &mut Channels, frame: u32, level: u32, alive: impl Fn(usize) -> bool,
                  drawn: impl Fn(usize) -> bool, spot_ok: impl Fn(usize, f32) -> bool,
                  rand: &mut impl FnMut(u32) -> u32) -> Action {
        if self.gone { return Action::None; }
        // The p7 kill latch [M 0x137e68:50-52; L-verified 07000005: +0x20 → 1 once the ch-0x43 script
        // freed the NPCs — the early return below is what held it off while the script ran].
        if self.kill != 0 && ch.on(self.kill) { self.kill_latch = true; }
        if !self.active && !self.kill_latch {
            // The trigger channel (+0x0E = param 4) is consumed when it fires [M: `+0x0E = 0, +0x1E = 1`];
            // the wait is skipped entirely once the kill latch is set [M: the p4 block only runs when
            // +0x20 is clear, 0x137e68:53-64].
            if self.start != 0 && ch.on(self.start) { self.active = true; self.start = 0; }
            else { return Action::None; }
        }
        if self.stop != 0 && ch.on(self.stop) {
            self.active = false; // un-trigger [M 0x137e68: `+0x1E = 0`]
            if self.kill_latch { return Action::None; } // latched: pauses this tick [M :79-81]
            self.gone = true; return Action::None;      // un-latched: deletes itself [M :82]
        }
        if self.kill_latch {
            // The cleanup block [M 0x137e68:235-288; disasm 0x137fbc-0x1380b4; L-verified 07000005:
            // both gate guards removed, the slots zeroed, +0x1E = 0, obj+0xFE |= 1].
            // The level split (the disasm's `s7`, set through the always-executed delay slot at
            // 0x137fe8): kill-all for level < 0x07000011, 0x07000014/0x07000015, and > 0x07000016
            // except 0x0700004A; the obj+0xFC-gated keep for 0x07000011..0x07000013, 0x07000016 and
            // 0x0700004A.
            let kill_all = level < 0x0700_0011
                || (0x0700_0014..=0x0700_0015).contains(&level)
                || (level > 0x0700_0016 && level != 0x0700_004a);
            let mut removed = Vec::new();
            let mut kept = 0usize;
            for &i in &self.spawned {
                if !alive(i) { continue; } // a dead drone's slot is already cleared [M DroneSpawner_DroneDelete 0x13a3b8 / the alive scan]
                if kill_all || !drawn(i) {
                    // obj+0xFC == 0 (not drawn recently) → removed [M `lbu $v0, 0xfc($s0)` 0x13805c;
                    // on 0x07000016 the removal first raises the drone's death channel [M jal 0x13e660
                    // at 0x138074 — the caller writes it, DroneFunc_SetDeathChannel 0x13e660]].
                    removed.push(i);
                } else {
                    kept += 1; // the kept count [M 0x138098: `andi $s6, $v0` with v0 = s6+1]
                }
            }
            self.spawned.retain(|i| !removed.contains(i)); // the slots zeroed [M `sw $zero, ($v1)` 0x138094]
            if kill_all || kept == 0 {
                // every spawned drone gone → the spawner deletes itself [M 0x1380b4-ish: `fe |= 1`,
                // `+0x1E = 0`]; with keep-gated survivors it returns and lingers [M :281-283].
                self.gone = true;
                self.active = false;
            }
            return Action::Remove(removed);
        }
        let living = self.spawned.iter().filter(|&&i| alive(i)).count();
        if self.wave >= self.waves || self.used.iter().all(|&u| u) {
            // All waves spawned: the done write waits for the spawned drones to die when the done
            // channel (p6) OR the kill channel (p7) is set — the game's `p6 == 0 && p7 == 0` immediate
            // disable [M 0x137e68:126-143].
            if (self.done != 0 || self.kill != 0) && living > 0 { return Action::None; }
            if self.done != 0 { ch.set(self.done, 1, frame); }
            self.gone = true;
            self.active = false; // +0x1E = 0, +0x1D = 0 [M 0x137e68:140-142]
            return Action::None;
        }
        if self.this_wave >= self.max_alive {
            // Wave rollover [M 0x137e68:145-172]: the epoch counts reset (+0x28 = 0, +0x2C++), +0x1F
            // cleared and every template record's spawned count zeroed — the same templates can spawn
            // again next wave.
            self.wave += 1;
            self.this_wave = 0;
            self.immediate = false;
            for u in self.used.iter_mut() { *u = false; }
            return Action::None;
        }
        let may = if self.mode == 3 { living == 0 || self.immediate } else { living < self.max_alive };
        if !may { return Action::None; }
        let unused: Vec<usize> = (0..self.templates.len()).filter(|&k| !self.used[k]).collect();
        if unused.is_empty() { return Action::None; }
        let k = unused[(rand(unused.len() as u32) as usize).min(unused.len() - 1)];
        let i = self.templates[k];
        if !self.immediate && !spot_ok(i, self.min_dist) { return Action::None; }
        self.used[k] = true;
        self.spawned.push(i);
        self.this_wave += 1;
        Action::Spawn(i)
    }
}
