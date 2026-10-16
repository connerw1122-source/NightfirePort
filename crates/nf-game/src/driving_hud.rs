//! The driving missions' HUD data model (DRIVING.ELF), headless — the per-frame snapshot of
//! the numbers the game's driving HUD shows, as the game sets them. **The main agent draws
//! it; nothing here renders.**
//!
//! Evidence base (all [M] unless noted — DRIVING.ELF decompile under `work/decomp/DRIVING.ELF/`,
//! addresses = EE; docs/atlas/driving-elf.md is the map):
//!
//! - **The HUD render object exists** ("RRenderHUDView" [D 0x3856c8/0x385878/0x3858c0]):
//!   `FUN_001c3db0` 0x1c3db0:22-34 builds it (88-byte object) into the render-camera manager
//!   at **manager+0x28** (`DAT_00333ed8 = manager`) [M]; the frame render pass `FUN_001c1ec0`
//!   0x1c1ec0:80-81 is the only reader of manager+0x28 and forwards the HUD view through
//!   `FUN_001c1b18` 0x1c1b18:118-144, which calls `FUN_001a4520`, copies a **0xa8-byte
//!   (21-u32) element block** and passes the label-table pointer **&DAT_0037d790** [M]. The
//!   element walk's per-element positions/sheet ids are not decoded yet (report live checks).
//! - **Vehicle numbers** (the sim side, wired below): speed = |velocity| [M `FUN_0018aa88`
//!   0x18aa88:26-29, recomputed per tick in the tyre pass 0x190f88:394]; the gear byte
//!   actor+0x2d8 [M], the previous gear +0x2d9 [M], the shift inhibit +0x2d7 [M 0x18cef8:410-413];
//!   rpm = speed·6000/LIMIT[gear] [M 0x18aa88:62], the neutral rev throttle·7000 [M :32];
//!   the reverse state actor+0x29c [M]. Position = instance+0x10, heading = class+0x00-0x3f's
//!   yaw [M docs/systems/drive-physics.md §2].
//! - **No mph/kmh conversion constant exists anywhere in the binary** [M-negative — byte
//!   search], so `CarReadout::speed` is the game's raw |velocity| units, exactly what the
//!   HUD receives.
//! - **Mission events drive the rest.** The dispatch table is at **0x36fda0** [M]: ids 1-283
//!   are "instant" commands (a handler bump-allocates a command record via `FUN_00175730` and
//!   fills `{vtable, params}`), ids 285+ are "object" commands (accessor/dtor pairs; the
//!   per-command record table is 173 × 0x18 bytes at **0x370214-0x37124c**) [M]. The executor
//!   `FUN_001a35f0` walks event records (stride 0x40, id byte at +0xd, data at +0xe..) and
//!   sets the context statics DAT_0032e708/0c/14 [M 001a35f0_FUN_001a35f0.c:123-161].
//!   Proven ids [M — record-table mapping verified per entry]:
//!   EPowerUpAmmo=56, ESetVisualDamage=188, EObjectiveAdd=225, EObjectiveFail=226,
//!   EObjectiveInsert=227, EObjectivePass=228, ETargetBeaconOnOff=321/323, ETimerDrawOn=309/311,
//!   ETimerDrawOff=315/317, EStopTimer=357/359, EStartTimer=381/383, ESetTimer=507/509.
//! - **Record shapes proven, state structs open**: the EObjectiveAdd command record is
//!   `{vtable 0x370e70, p1..p4}` — 16 bytes, 4 params [M 00151510_FUN_00151510.c]; the
//!   ESetVisualDamage record is `{vtable 0x370558, p1..p5}` [M 00159638_FUN_00159638.c].
//!   The timer/beacon/objective *state* structs (what the HUD finally reads) are not
//!   decompiled yet — the snapshot models them at the command level (the setters below are
//!   the commands; the fields are what the commands would leave behind [S]). Live checks in
//!   the report ask the main agent to pin them.
//! - **Vehicle health**: `EPowerUpHealth`/`ESetVisualDamage` exist [D names 0x372300/0x372790];
//!   the PBondCar ctor `FUN_001862c0` decompiles register-mangled, no health field provable
//!   yet [M-negative]. `DriveHud::health` is `None` until the damage state is ported.
//! - **Weapon + ammo**: the weapon manager init is `FUN_0020bb00` on a 400-byte alloc
//!   [M FUN_00176658's "Init Weapon Manager" block]; `EPowerUpAmmo` id 56 [M],
//!   `ESetSpecialWeapon`/`EUnloadWeapon` [D 0x372720/0x372a30]. The per-weapon ammo source is
//!   not pinned yet — `WeaponReadout` carries the values as set [S].

use crate::driving::Car;

// ---------------------------------------------------------------------------
// The per-frame vehicle readout — wired to `driving::Car` (task 506's physics)
// ---------------------------------------------------------------------------

/// The vehicle numbers the driving HUD shows each frame, exactly as the port's `Car` holds
/// them after a tick ([M] sources per field; `CarReadout::of` copies, it never re-derives).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CarReadout {
    /// |velocity| [M 0x18aa88:26-29] — the game's raw units (no mph constant exists [M-negative]).
    pub speed: f32,
    /// The gear byte actor+0x2d8 [M]; the port's auto walk keeps 1..=5 forward [M 0x18aa88:36-48].
    pub gear: u8,
    /// The rpm `FUN_0018aa88` returns: speed·6000/LIMIT[gear] [M :62]; the neutral rev is
    /// throttle·7000 [M :32].
    pub rpm: f32,
    /// The reverse state actor+0x29c [M].
    pub reversing: bool,
    /// instance+0x10 (x/y/z, y = height) — the minimap's viewer position [M].
    pub pos: [f32; 3],
    /// The class orientation matrix's yaw — the minimap's viewer heading [M].
    pub yaw: f32,
}

impl CarReadout {
    /// One frame's readout of `car` — a plain copy of the fields the HUD would read [M].
    pub fn of(car: &Car) -> CarReadout {
        CarReadout {
            speed: car.speed,
            gear: car.gear,
            rpm: car.rpm,
            reversing: car.reversing,
            pos: car.pos,
            yaw: car.yaw,
        }
    }
}

// ---------------------------------------------------------------------------
// The mission-side groups — command-level model (state structs open, see header)
// ---------------------------------------------------------------------------

/// The mission timer, modelled at its command level: `ESetTimer` (507/509) sets the value,
/// `EStartTimer` (381/383) / `EStopTimer` (357/359) run/hold it, `ETimerDrawOn` (309/311) /
/// `ETimerDrawOff` (315/317) toggle the HUD draw [M ids]. The game's value/draw state struct
/// is not decompiled yet [S]; the field semantics here are the commands', ticked at the fixed
/// 1/60 s ([M 0x2d6b90's {60,60,50,60} table] NTSC).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MissionTimer {
    /// The timer's current value in seconds [S — the unit is not pinned; see the live checks].
    pub remaining: f32,
    /// The last `ESetTimer` value.
    pub initial: f32,
    /// `EStartTimer` sets, `EStopTimer` clears.
    pub running: bool,
    /// `ETimerDrawOn` sets, `ETimerDrawOff` clears.
    pub draw: bool,
}

impl MissionTimer {
    pub fn new(initial: f32) -> MissionTimer {
        MissionTimer { remaining: initial, initial, running: false, draw: false }
    }

    /// `ESetTimer` [M id 507/509]: (re)arm the value; the run/draw flags stand.
    pub fn set(&mut self, seconds: f32) {
        self.remaining = seconds;
        self.initial = seconds;
    }

    /// `EStartTimer` [M id 381/383].
    pub fn start(&mut self) { self.running = true; }

    /// `EStopTimer` [M id 357/359].
    pub fn stop(&mut self) { self.running = false; }

    /// `ETimerDrawOn` (on) / `ETimerDrawOff` (off) [M ids 309/311, 315/317].
    pub fn set_draw(&mut self, on: bool) { self.draw = on; }

    /// One fixed tick: a running timer counts down, floored at zero [S — the countdown
    /// semantic is the race missions' visible behaviour; the exact game field is open].
    pub fn update(&mut self, dt: f32) {
        if self.running {
            self.remaining = (self.remaining - dt).max(0.0);
        }
    }
}

/// The objective states the EObjective* commands leave behind [M names 0x371eb8-0x371f78]:
/// Added (EObjectiveAdd 225) / Failed (226) / Inserted (227) / Passed (228) — the port keeps
/// insert and add distinct so the record order stays inspectable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectiveState {
    Added,
    Inserted,
    Passed,
    Failed,
}

/// One objective record: the game's command record is `{vtable 0x370e70, p1..p4}` — 4 i32
/// params, 16 bytes [M 00151510_FUN_00151510.c]. The port carries the params verbatim plus
/// the state byte's command-level meaning [S].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Objective {
    /// The record's four params, verbatim [M record shape; the meaning of each param is open].
    pub params: [i32; 4],
    /// The command that last touched it [S].
    pub state: ObjectiveState,
}

/// One target beacon: `ETargetBeaconOnOff` (321/323) toggles a beacon on a mission target
/// [M id]; the target struct (and its position field) is not decompiled yet [S], so the
/// position rides the snapshot exactly as the mission sets it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Beacon {
    /// The target's world position [S — source struct open; see the live checks].
    pub pos: [f32; 3],
    /// The beacon's on/off [M the command's toggle].
    pub on: bool,
}

/// The weapon + ammo line, as set [S — the per-weapon ammo source is not pinned; the weapon
/// manager is `FUN_0020bb00`'s 400-byte alloc [M]].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WeaponReadout {
    /// The current weapon id (0 = none) [S].
    pub weapon: u32,
    /// The ammo count [S].
    pub ammo: i32,
}

// ---------------------------------------------------------------------------
// The per-frame snapshot
// ---------------------------------------------------------------------------

/// The driving HUD's per-frame snapshot: the values + label/texture ids + positions the game
/// sets, in one struct the main agent can draw from. The vehicle group is wired to
/// `driving::Car` (506's physics) with `observe`; the mission groups are the command-level
/// state the mission-script task will drive. No drawing, no RNG, no wall-clock.
#[derive(Debug, Clone, PartialEq)]
pub struct DriveHud {
    /// The vehicle numbers (speed/gear/rpm/reverse/pos/yaw) — refreshed per frame from `Car`.
    pub car: CarReadout,
    /// The vehicle's health [M-negative source — `None` until the damage state is ported;
    /// `EPowerUpHealth`/`ESetVisualDamage` exist [D 0x372300/0x372790]].
    pub health: Option<f32>,
    /// The last `ESetVisualDamage` record's five params, verbatim [M 0x159638's record shape;
    /// zeros until a mission sets it].
    pub visual_damage: [f32; 5],
    /// The weapon + ammo line [S].
    pub weapon: WeaponReadout,
    /// The mission timer.
    pub timer: MissionTimer,
    /// The objective records, in game order [M the 0x40-byte event-record walk is the game's
    /// order; the port keeps insertion order].
    pub objectives: Vec<Objective>,
    /// The target beacons (the HUD's diamond markers) [S].
    pub beacons: Vec<Beacon>,
}

impl DriveHud {
    /// A fresh snapshot: the vehicle part at rest, the mission parts unset (the game's
    /// fresh-mission state: no timer running, no draw, no objectives/beacons).
    pub fn new() -> DriveHud {
        DriveHud {
            car: CarReadout {
                speed: 0.0, gear: 1, rpm: 0.0, reversing: false, pos: [0.0; 3], yaw: 0.0,
            },
            health: None,
            visual_damage: [0.0; 5],
            weapon: WeaponReadout { weapon: 0, ammo: 0 },
            timer: MissionTimer::new(0.0),
            objectives: Vec::new(),
            beacons: Vec::new(),
        }
    }

    /// The per-frame vehicle refresh — `CarReadout::of` [M]; the mission groups are untouched
    /// (the game reads them from the mission state, not from the car).
    pub fn observe(&mut self, car: &Car) {
        self.car = CarReadout::of(car);
    }

    /// `EObjectiveAdd` (225) — append a record [M the 4-param record shape].
    pub fn objective_add(&mut self, params: [i32; 4]) {
        self.objectives.push(Objective { params, state: ObjectiveState::Added });
    }

    /// `EObjectiveInsert` (227) — insert a record at `index` [S the position semantic].
    pub fn objective_insert(&mut self, index: usize, params: [i32; 4]) {
        let len = self.objectives.len().min(index);
        self.objectives.insert(len, Objective { params, state: ObjectiveState::Inserted });
    }

    /// `EObjectivePass` (228) / `EObjectiveFail` (226) on record `index` [M ids].
    pub fn objective_pass(&mut self, index: usize) {
        if let Some(o) = self.objectives.get_mut(index) { o.state = ObjectiveState::Passed; }
    }

    pub fn objective_fail(&mut self, index: usize) {
        if let Some(o) = self.objectives.get_mut(index) { o.state = ObjectiveState::Failed; }
    }

    /// `ETargetBeaconOnOff` (321/323) — toggle beacon `index` in place [M the toggle].
    pub fn beacon_toggle(&mut self, index: usize) {
        if let Some(b) = self.beacons.get_mut(index) { b.on = !b.on; }
    }

    /// `ESetVisualDamage` (188) — store the record's five params verbatim [M the record shape].
    pub fn visual_damage_set(&mut self, params: [f32; 5]) {
        self.visual_damage = params;
    }
}

impl Default for DriveHud {
    fn default() -> DriveHud { DriveHud::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The fresh snapshot's vehicle part mirrors a rest car; the mission parts unset.
    #[test]
    fn fresh_snapshot_is_the_fresh_mission_state() {
        let hud = DriveHud::new();
        assert_eq!(hud.car.speed, 0.0);
        assert_eq!(hud.car.gear, 1);
        assert_eq!(hud.car.rpm, 0.0);
        assert!(!hud.car.reversing);
        assert_eq!(hud.health, None);
        assert_eq!(hud.visual_damage, [0.0; 5]);
        assert_eq!(hud.weapon, WeaponReadout { weapon: 0, ammo: 0 });
        assert_eq!(hud.timer, MissionTimer { remaining: 0.0, initial: 0.0, running: false, draw: false });
        assert!(hud.objectives.is_empty() && hud.beacons.is_empty());
    }
}
