//! The MP remote vehicles (playtest 1016-1355: "remote tanks and like helicopters that you
//! should be able to pilot") — the `Car_*` family of ACTION.ELF, class 0x28.
//!
//! The disc's 0xDA placements (`Car_Create`) exist on every arena except Sub Pen [D census,
//! mp_map_objects_live::probe_cars_on_all_arenas]. The Enviro-Mods menu row (`MPSettings+0x460`,
//! `P_MPENVIROMODS_Handler` 0x210030, options Off/Tanks/Helicopters/Random, docs/systems/
//! mp-menus.md:186) picks the variant: the "Helicopters" the playtest names are the SAME object
//! with the model swapped to 0x2000194 and +1.5 bounds [M 0x1305b0:53-58].
//!
//! Status: the data model, the create gates, the activate/deactivate state machine and the
//! state-99 respawn timer are ported here with their addresses. The drive physics of
//! `Car_Update` 0x1307d8 (605 decompile lines: the tank branch, the heli branch's matrix math,
//! the shell/MG fire with cooldown + overheat) is NEXT-session work — only the heli branch's
//! input→attitude step is modelled (`drive_step_heli`), read at 0x1307d8:157-246; the tank
//! branch is NOT modelled and no assertion below pins it.
//!
//! Wiring (the remaining sim work, not in this module): the 0xDA placement walk must call
//! [`Car::create`] in placement order (it draws `Rand_Rand(100)` ONLY on option 3 — the shared
//! RNG stream, placement order, like every creator), `Player_Activate`'s class-0x28 arm
//! (0x1a5170:142-145 — the use-key sphere cast) calls [`Car::activate`], and the control walk
//! calls [`Car::tick`]. The renderer reads `obj+0x90` (+ the 3 turret sub-objects at
//! CAR_INFO+0xe0/e4/e8, all `View_SetDrawInAllViews` [M 0x1305b0:66-70]).

use crate::drone::Rng;
use nf_formats::anim::Mat4;

/// The Enviro-Mods vehicles option (`MPSettings+0x460`): 0 Off, 1 Tanks, 2 Helicopters, 3 Random
/// [M P_MPENVIROMODS_Handler 0x210030:16 — options in menu order].
pub const VEHICLES_OFF: u32 = 0;
pub const VEHICLES_TANKS: u32 = 1;
pub const VEHICLES_HELICOPTERS: u32 = 2;
pub const VEHICLES_RANDOM: u32 = 3;

/// `Car_Update`'s state words (`obj+0xf4`) [M 0x1307d8:100/116/122/141, 0x131c48:31,
/// 0x131d88:24-26]: 0 idle (usable), 1 driven, 2 frozen (the create arm's brief state),
/// 5 the activate tick's arming value (collapsed to 1 before the next drive input by the
/// state walk), 99 respawning (hidden, 0x708-tick timer).
pub const CAR_IDLE: i16 = 0;
pub const CAR_DRIVEN: i16 = 1;
pub const CAR_FROZEN: i16 = 2;
pub const CAR_ARMING: i16 = 5;
pub const CAR_RESPAWNING: i16 = 99;

/// The respawn timer `Car_Deactivate` loads into `obj+0xf6` [M 0x131d88:79] — 0x708 ticks =
/// 30 s at the fixed 60 Hz.
pub const RESPAWN_TICKS: i16 = 0x708;

/// The activate sounds [M 0x131c48:44-52]: tank 0x60a, helicopter 0x10b; the spin-up timers
/// stored at CAR_INFO+0x110: tank 10, helicopter 4.
pub const ACTIVATE_SOUND_TANK: u32 = 0x60a;
pub const ACTIVATE_SOUND_HELI: u32 = 0x10b;

/// `Car_Create`'s cap [M 0x1305b0:24 — `7 < NumTanks` returns].
pub const MAX_TANKS: usize = 8;

/// One remote vehicle (`obj+0xe0`'s CAR_INFO + the obj fields the state machine reads), the
/// parts the ported state machine needs. Field names carry the game offsets.
#[derive(Clone, Debug, PartialEq)]
pub struct Car {
    /// `obj+0xf4` — the state word.
    pub state: i16,
    /// `CAR_INFO+0x10e` — the TankSpawns slot (== the creation order).
    pub slot: u16,
    /// `CAR_INFO+0x112` — the variant byte: 0 tank, 1 helicopter [M 0x1305b0:29-43].
    pub heli: bool,
    /// `obj+0xf6` — the piloting player's slot while driven; the respawn countdown while
    /// state 99 [M 0x131c48:32, 0x131d88:79].
    pub timer_or_pilot: i16,
    /// `CAR_INFO+0xec` — the driving player's obj handle (the port models the slot).
    pub driver: Option<usize>,
    /// `CAR_INFO+0xa8` — the throttle accumulator (action 2, clamp ±0.125·FRM) [M 0x1307d8:196-214].
    pub throttle: f32,
    /// `CAR_INFO+0xa0` — the steer accumulator (action 1, clamp ±0.1·FRM) [M 0x1307d8:215-246].
    pub steer: f32,
    /// `CAR_INFO+0x104` — the weapon overheat accumulator (0..100, +10 per shell) [M 0x1307d8
    /// shell fire tail; 0x131c48 resets it].
    pub heat: f32,
    /// `CAR_INFO+0xfc` — the shell cooldown [M 0x1307d8 fire arm — set 0x40c00000 = 6.0 s].
    pub shell_cooldown: f32,
    /// `CAR_INFO+0x113` — the overheat latch (blocks fire while 1) [M 0x1307d8:100-108].
    pub overheated: bool,
    /// `obj+0x90` — the live matrix; `TankSpawns[slot]` on the port side is `spawn_matrix`
    /// (the game copies the spawn pose in at create and back at deactivate — the two
    /// `Mat_Copy(obj+0x90, TankSpawns+slot*0x40)` call sites [M 0x1305b0:74, 0x131d88:74];
    /// the decompiler's arg order is ambiguous, disasm check listed in the report).
    pub matrix: Mat4,
    pub spawn_matrix: Mat4,
}

impl Car {
    /// `Car_Create` 0x1305b0's gates and variant pick. `mp_enabled` = `MPSettings+0x384 != 0`,
    /// `vehicles_option` = `MPSettings+0x460`; `level` = `GameState+0x12` (cars are blocked on
    /// Ravine 0x0700004b [M 0x1305b0:19-22]); `rng` draws ONLY on the Random option
    /// (`Rand_Rand(100) < 0x33` → helicopter, 51 %) [M 0x1305b0:36-43]. Returns None where the
    /// game returns 0 (no object created).
    pub fn create(mp_enabled: bool, vehicles_option: u32, level: u32, rng: &mut Rng) -> Option<(Car, bool)> {
        if !mp_enabled || vehicles_option == VEHICLES_OFF {
            return None;
        }
        if level == 0x0700_004b {
            return None;
        }
        let heli = match vehicles_option {
            VEHICLES_HELICOPTERS => true,
            VEHICLES_RANDOM => rng.rand(100) < 0x33,
            VEHICLES_TANKS => false,
            _ => true,
        };
        let ident: Mat4 = [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]];
        Some((
            Car {
                state: CAR_IDLE,
                slot: 0,
                heli,
                timer_or_pilot: 0,
                driver: None,
                throttle: 0.0,
                steer: 0.0,
                heat: 0.0,
                shell_cooldown: 0.0,
                overheated: false,
                matrix: ident,
                spawn_matrix: ident,
            },
            heli,
        ))
    }

    /// `Car_Activate` 0x131c48: the use key's sphere cast reached a class-0x28 object while it
    /// was idle. Returns the (prev_substate, cam_mode, sound id, spin-up timer) the caller must
    /// apply to the player — the port's `Player_Activate` arm owns the BLData writes
    /// (`+0x942` prev substate, `+0x878` camera mount = the turret sub-object for the tank and
    /// the body for the helicopter, cam 0xD, `Player_Disable(1)`, `Player_WeaponNone`).
    pub fn activate(&mut self, player_slot: usize) -> Option<(i16, u16, u32, i16)> {
        if self.state != CAR_IDLE {
            return None;
        }
        let (sound, spin) = if self.heli {
            (ACTIVATE_SOUND_HELI, 4)
        } else {
            (ACTIVATE_SOUND_TANK, 10)
        };
        self.state = CAR_ARMING; // 0x131c48:31 — collapsed to CAR_DRIVEN by the next tick
        self.timer_or_pilot = player_slot as i16; // 0x131c48:32 — the player slot short
        self.driver = Some(player_slot); // CAR_INFO+0xec
        self.heat = 0.0; // 0x131c48:36
        self.overheated = false; // 0x131c48:37
        Some((-1, 0xD, sound, spin)) // the prev substate comes from Player_ChangeSubState
    }

    /// `Car_Deactivate` 0x131d88: action 0xE while driven (or death/collision force). The
    /// vehicle explodes (`Explode_Create` 10/10/50, blast script 0x06000052, colour 0x2d6210,
    /// radius 100 [M 0x131d88:66-68]), the pose restores from `TankSpawns[slot]`, state 99 with
    /// the 0x708 respawn timer [M 0x131d88:74-79]. The caller restores the player (substate +
    /// `Player_Enable`); `Car_PlayerHasDied` 0x132e98 and `Car_CollisionHandler` 0x131838 call
    /// the same.
    pub fn deactivate(&mut self) {
        self.state = CAR_RESPAWNING;
        self.timer_or_pilot = RESPAWN_TICKS;
        self.driver = None;
        self.matrix = self.spawn_matrix;
    }

    /// The heli drive step [M Car_Update 0x1307d8:157-246, the `CAR_INFO+0x112 != 0` branch]:
    /// yaw from action 0 (`+input·FRAME_RATE_MUL·0.05`), pitch from action 5 (negated), the
    /// throttle (action 2) and steer (action 1) lerp toward `2·input·FRAME_RATE_MUL` at 0.15
    /// with the ±0.125 (throttle) / ±0.1 (steer) clamps and the ±0.0002/0.01 dead-zone zero.
    /// FRAME_RATE_MUL is 1 at the port's fixed 60 Hz [M the game's frame-rate multiplier; the
    /// port's fixed tick]. Returns the new (yaw_step, pitch_step, throttle, steer) the caller
    /// applies to the matrix/velocity; the matrix composition itself stays with the full
    /// Car_Update port (RotMatrixZYX + the quat integrator, next session).
    pub fn drive_step_heli(&mut self, yaw_in: f32, pitch_in: f32, throttle_in: f32, steer_in: f32)
        -> (f32, f32, f32, f32) {
        if self.state != CAR_DRIVEN {
            return (0.0, 0.0, 0.0, 0.0);
        }
        const FRM: f32 = 1.0;
        let yaw_step = yaw_in * FRM * 0.05;
        let pitch_step = -pitch_in * FRM * 0.05;
        let throttle_target = (throttle_in + throttle_in) * FRM;
        let mut throttle = self.throttle + (throttle_target - self.throttle) * 0.15;
        if (throttle_target - self.throttle).abs() > 0.0
            && ((throttle_target >= self.throttle && throttle > throttle_target)
                || (throttle_target < self.throttle && throttle < throttle_target))
        {
            throttle = throttle_target;
        }
        if (-0.0002..=0.0002).contains(&throttle_target) && throttle.abs() < 0.01 {
            throttle = 0.0;
        }
        throttle = throttle.clamp(-0.125 * FRM, 0.125 * FRM);
        let steer_target = (steer_in + steer_in) * FRM;
        let mut steer = self.steer + (steer_target - self.steer) * 0.15;
        if ((steer_target >= self.steer && steer > steer_target)
            || (steer_target < self.steer && steer < steer_target))
            && (steer_target - self.steer).abs() > 0.0
        {
            steer = steer_target;
        }
        if (-0.0002..=0.0002).contains(&steer_target) && steer.abs() < 0.01 {
            steer = 0.0;
        }
        steer = steer.clamp(-0.1 * FRM, 0.1 * FRM);
        self.throttle = throttle;
        self.steer = steer;
        (yaw_step, pitch_step, throttle, steer)
    }

    /// One `Car_Update` tick of the state machine [M 0x1307d8:100-155]: state 99 hides and
    /// counts the respawn timer down by FRAME_RATE_MUL (state 0 when it lands); the overheat
    /// accumulator cools by FRAME_RATE_MUL and clears the latch at 0; the arming state
    /// collapses to driven.
    pub fn tick(&mut self) {
        match self.state {
            CAR_RESPAWNING => {
                self.timer_or_pilot -= 1;
                if self.timer_or_pilot <= 0 {
                    self.state = CAR_IDLE;
                    self.timer_or_pilot = 0;
                }
            }
            CAR_ARMING => {
                self.state = CAR_DRIVEN;
            }
            _ => {}
        }
        if self.heat > 0.0 {
            self.heat -= 1.0;
            if self.heat <= 0.0 {
                self.heat = 0.0;
                self.overheated = false;
            }
        }
        // The shell cooldown counts down by FRAME_RATE_MUL while non-zero [M 0x1307d8:254-266
        // — the +0xfc branch; the fire test is `<= 0.0`, so the decrement may run past 0].
        if self.shell_cooldown != 0.0 {
            self.shell_cooldown -= 1.0;
        }
    }

    /// The shell fire arm [M 0x1307d8 fire tail]: +10 heat per shell, the 6.0 s shell cooldown
    /// (0x40c00000), the overheat latch at > 100.
    pub fn fire_shell(&mut self) -> bool {
        if self.overheated || self.shell_cooldown > 0.0 || self.state != CAR_DRIVEN {
            return false;
        }
        self.shell_cooldown = 6.0;
        self.heat += 10.0;
        if self.heat > 100.0 {
            self.overheated = true;
        }
        true
    }
}
