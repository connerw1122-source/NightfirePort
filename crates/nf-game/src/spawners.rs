//! Drone spawners (placement type 0xF1, `DroneSpawner_Create` 0x1378D0 / `DroneSpawner_Control` 0x137E68;
//! docs/systems/triggers.md "Spawners"): the drones of a group are templates that a spawner brings in.

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
    /// Drone indices of the group's templates and which of them were used.
    pub templates: Vec<usize>,
    pub used: Vec<bool>,
    /// Drones it spawned (indices).
    pub spawned: Vec<usize>,
    pub active: bool,
    pub immediate: bool,
    pub killing: bool,
    /// `DroneSpawner_Create` +0x14 = placement param 7: the **ARM channel** — on (latched) the spawner
    /// proceeds without the trigger channel [M 0x137e68: `if (ch[p7]) armed(+0x20) = 1`]. The port
    /// previously read it as a kill channel and removed the group's drones when it fired — the reverse
    /// of the game (map-tower2's stairwell spawners died instead of arriving; fix-actors).
    pub arm: u32,
    pub armed: bool,
    pub wave: u32,
    pub this_wave: usize,
    pub gone: bool,
}

/// What a spawner asks of the drone list this frame.
#[derive(Clone, Debug, PartialEq)]
pub enum Action { None, Spawn(usize), Remove(Vec<usize>) }

impl Spawner {
    /// From a placement (`p0` > 1 required) and the indices of the drones in its group.
    pub fn new(placement: usize, param: impl Fn(u32) -> u32, templates: Vec<usize>, ch: &Channels) -> Option<Spawner> {
        if param(0) <= 1 { return None; }
        let n = templates.len();
        let max = param(2) as usize;
        let start = param(4);
        let immediate = (start == 0 || ch.on(start)) && !ch.on(param(5));
        let min = f32::from_bits(param(8));
        Some(Spawner {
            placement, mode: param(0), group: param(1), max_alive: if max == 0 || max > n { n } else { max },
            waves: if param(3) == 0 { 32000 } else { param(3) }, start, stop: param(5), done: param(6),
            arm: param(7), armed: param(7) != 0 && ch.on(param(7)),
            min_dist: if min > 0.0 && min.is_finite() { min } else { 10.0 },
            used: vec![false; n], templates, spawned: Vec::new(), active: start == 0, immediate, killing: false, wave: 0, this_wave: 0, gone: false,
        })
    }

    /// One `DroneSpawner_Control` update. `alive(i)`: drone i exists and is not dead; `spot_ok(i)`: template i's
    /// spot is at least `min_dist` from the player and not visible to him; `rand(n)`: `Rand_Rand`.
    pub fn update(&mut self, ch: &mut Channels, frame: u32, alive: impl Fn(usize) -> bool, spot_ok: impl Fn(usize, f32) -> bool, rand: &mut impl FnMut(u32) -> u32) -> Action {
        if self.gone { return Action::None; }
        // `DroneSpawner_Control` 0x137e68 [M]: the arm channel latches `armed` (+0x20); the stop
        // channel (+0x10 = param 5) un-triggers, pauses an ARMED spawner for the tick, and deletes an
        // un-armed one; the kill path (the level-end slot scan) stays the `killing` latch via the
        // caller (the port has no other writer).
        if self.arm != 0 && ch.on(self.arm) { self.armed = true; }
        if !self.active {
            // The trigger channel (+0x0E = param 4) is consumed when it fires [M: `+0x0E = 0,
            // +0x1E = 1`]; an armed spawner needs no trigger [M: the `armed` branch skips the wait].
            if self.start != 0 && ch.on(self.start) { self.active = true; self.start = 0; }
            else if self.armed { self.active = true; }
            else { return Action::None; }
        }
        if self.stop != 0 && ch.on(self.stop) {
            self.active = false; // un-trigger [M 0x137e68: `+0x1E = 0`]
            if !self.armed { self.gone = true; return Action::None; }
            return Action::None; // armed: pauses this tick
        }
        if self.killing {
            self.gone = true;
            return Action::Remove(self.spawned.iter().copied().filter(|&i| alive(i)).collect());
        }
        let living = self.spawned.iter().filter(|&&i| alive(i)).count();
        if self.wave >= self.waves || self.used.iter().all(|&u| u) {
            // All waves spawned: the done channel waits for the spawned drones to die, unless the
            // spawner was never armed/triggered-gated (no done and no kill latch) [M 0x137e68: the
            // `puVar1[3] <= puVar1[0xb]` branch — the done write needs the alive count 0 when the
            // arm channel (param 7) is set].
            if self.done != 0 && self.arm != 0 && living > 0 { return Action::None; }
            if self.done != 0 { ch.set(self.done, 1, frame); }
            self.gone = true;
            return Action::None;
        }
        if self.this_wave >= self.max_alive { self.wave += 1; self.this_wave = 0; return Action::None; }
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
