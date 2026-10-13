//! The death sequence and the fail/death restart (`fix-death-restart`, docs/systems/mission.md §8).
//!
//! Game path, end to end:
//! 1. `Player_CheckForDeath` (0x190810): on health ≤ 0 — ch 0x5e, `Music_Event(6,1)`,
//!    `Player_ClearInertia`, `Player_WeaponNone` (the fists), the death marker (`obj+0xff = 0x12`),
//!    the death tick stamp (`obj+0xec`), cam mode 0, health 0, `obj+0xf4 = 2` (dead) and substate
//!    0xd (the death anim; 0xe in a vehicle), the death sound 0x89 [M 0x190810:34-97].
//! 2. `Player_Update` (0x193600): while `obj+0xf4` is 2/3 the player control does NOT run — only
//!    `AnimObjectUpdate` + `Player_HandleDeath` [M 0x193600:317-319].
//! 3. `Player_HandleDeath` (0x190bb0): first call enables the blood pane (`HUD_DisableAll` +
//!    `HUD_Enable(pane 12, 1, 0)` [M 0x190bb0:23-25]); every later call raises the death channel
//!    0x62 once the pane's state is non-zero or the player presses action 25 (✕ / Start, player 0
//!    [L input.md style 0]), unless `Menu_PlayerHasQuitOrRestarted` [M 0x190bb0:29-33].
//! 4. The red fade (`HUD_UpdateBloodPane` 0x19a6a8): while the pane is enabled both blood sprites
//!    grow 1 px per tick (FRAME_RATE_MUL) and at full cover the pane's state word (`HUD_State`,
//!    pane+0x1a) is set to 1 [M 0x19a6a8:23-34]. The gate is `w − viewer.w ≥ viewer.h`; the radar
//!    pane proves viewer.h = 448 [M 0x19f4e8:33] and the templates are 512 wide [L BloodSprInfo
//!    0x2bcfd8], so the cover takes 448 ticks (~7.5 s) — or ✕ skips it.
//! 5. `Mission_Update` (0x182c60) reads ch 0x62 [M 0x182c60:21]: state 1 → 3 (the red `FadeClr`
//!    0xff0000ff, `Music_Event(6,1)`, health 0), 3 → 7 (`TimeOut` 1.0 = 1 tick at FRAME_RATE_MUL
//!    1.0 [L 0x30d0d8], `GS_PausePlayer(1,0)` per tick), 7 → 4, and case 4 opens the 0x40000042
//!    menu [M 0x182c60:77-81].
//! 6. The menu (`C_GCPAUSE_Handler` 0x204408, menu 0x40000042): Restart → `ResetMap_LevelToLoad(
//!    Mission_BaseMapHCode(), 0, 1)` + `user_restarted_or_quit = 1` [M 0x204408:136-155]; Quit →
//!    `GameState._28_4_ = 0x4000001c` + `ResetMap_LevelToLoad(0x7000048, 0, 0)` [M 0x204408:143-149].
//!    `Mission_BaseMapHCode` returns `BaseMap` — the mission's FIRST level [M 0x1830a0].
//!
//! The restart is a COLD load: `ResetMap_LevelToLoad`'s IsWarmReset = 0 → `Init_SwitchChannels`
//! zeroes every switch channel [M 0x1d5080:44-46, 0x134310] and `Player_RamLoad`'s whole body is
//! gated on IsWarmReset [M 0x1957a8] — so no RamSave is carried: fresh loadout, fresh channels, and
//! `Mission_Init(level, 0)` re-announces the mission from its first level. (The trigger-exit level
//! change is the WARM arm: `ResetMap_LevelToLoad(next, 1, 0)` + `Player_RamSave` [M 0x1b2318:111-116]
//! — that carry is fix-level2's.) Carrying the RamSave across a restart is what looped the user's
//! 07000007 session: the saved ch 99 re-fails the mission the tick the level comes back.

/// The red fade's tick count (`RED_FADE_TICKS`): see the module docs — sprite width 512 [L
/// BloodSprInfo 0x2bcfd8], the cover gate `viewer.h = 448` [M 0x19f4e8:33].
pub const RED_FADE_TICKS: u32 = 448;

/// The death sequence's state. The game keeps the marker in `obj+0xff`/`obj+0xec`/`obj+0xf4` and
/// the pane in the HUD; the port keeps it here (per sim, fresh at every load — `Player_Init`
/// 0x1910f8:26,30 clears ch 0x62 and the death state is not part of any save [M]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Death {
    /// Bond is dead and the sequence is running (`obj+0xf4 = 2` [M 0x190810:66]).
    pub active: bool,
    /// Ticks since the death started (`obj+0xec = GameState+0x52` [M 0x190810:59-60]).
    pub ticks: u32,
    /// The blood pane is enabled (one-shot at the first `HandleDeath` [M 0x190bb0:23-25]).
    pub pane_on: bool,
    /// The red fade has covered the screen (`HUD_State(pane 12) != 0` [M 0x19a6a8:24]).
    pub covered: bool,
    /// The death channel 0x62 has been raised [M 0x190bb0:32] (raised once per death).
    pub ch62: bool,
}

impl Default for Death {
    fn default() -> Self {
        Death { active: false, ticks: 0, pane_on: false, covered: false, ch62: false }
    }
}

impl crate::drone::Drones {
    /// `Player_CheckForDeath` (0x190810) + `Player_HandleDeath` (0x190bb0), run from `Drones::tick`
    /// while Bond's health has run out (the port keeps health in `Drones.health`; the game in
    /// BL+0x894). `fire` = this tick's action 25 (✕ / Start for player 0 [L input.md]), read from
    /// the real input path (`Sim::step_bond`'s trigger).
    pub fn death_tick(&mut self, d: &crate::drone::Data, bond: crate::world::V3, frame: u32) {
        if self.health.health > 0.0 {
            // Alive: the sequence is not running. (The game has no revive in SP; a cheat-heal would
            // clear the marker with the next `Player_Init`.)
            if self.death.active { self.death = Death::default(); }
            return;
        }
        if !self.death.active {
            // The death starts: `Player_CheckForDeath`'s effects [M 0x190810:34-97].
            self.death.active = true;
            self.death.ticks = 0;
            self.death.pane_on = true; // HUD_DisableAll + HUD_Enable(pane 12, 1, 0) [M 0x190bb0:23-25]
            self.channels.set(0x5e, 1, frame); // [M 0x190810:40]
            self.music.event(6, 1);            // [M 0x190810:41] (once — the old port replayed it every tick)
            self.bond_weapon = 0x47;           // Player_WeaponNone — the fists [M 0x190810:61]
            // The death sound 0x89 at Bond [M 0x190810:88].
            if let Some(t) = d.sfx { self.play_sound(bond, t.get(0x89)); }
            return;
        }
        // Per dead tick: `Player_HandleDeath` [M 0x190bb0:29-33]. The pane's state word turns 1 at
        // full cover [M 0x19a6a8:24]; the ✕ skip is action 25.
        self.death.ticks += 1;
        if !self.death.covered && self.death.ticks >= RED_FADE_TICKS { self.death.covered = true; }
        if self.death.ch62 { return; } // raised once — the menu path takes over from here
        if self.death.covered || self.bond_fire {
            self.death.ch62 = true;
            self.channels.set(0x62, 1, frame); // [M 0x190bb0:32]
        }
    }

    /// The 0x40000042 fail/death menu is up and waiting for the player
    /// (`Mission_Update` case 4 else [M 0x182c60:77-81]).
    pub fn fail_menu_open(&self) -> bool {
        self.mission.menu_open && self.mission.menu_choice.is_none()
    }

    /// Restart picked (`C_GCPAUSE_Handler` 0x204408:136-140,155): `ResetMap_LevelToLoad(
    /// Mission_BaseMapHCode(), 0, 1)` — the mission's first level [M 0x1830a0], IsWarmReset = 0
    /// (the COLD arm), `user_restarted_or_quit = 1`. The port marks the exit cold so the viewer's
    /// reload drops the RamSave (`level_exit_cold`, consumed by lib.rs).
    pub fn fail_choose_restart(&mut self) {
        if !self.mission.menu_open { return; }
        let to = self.mission.base_map;
        self.mission.menu_open = false;
        self.mission.menu_choice = Some(crate::mission::MenuChoice::Restart);
        self.level_exit = Some(to);
        self.level_exit_cold = true;
    }

    /// Quit picked (`C_GCPAUSE_Handler` 0x204408:141-149): `GameState._28_4_ = 0x4000001c` and
    /// `ResetMap_LevelToLoad(0x7000048, 0, 0)` — the front end. The port cannot load 0x07000048
    /// (no front end); the viewer logs the request and holds, like the driving levels.
    pub fn fail_choose_quit(&mut self) {
        if !self.mission.menu_open { return; }
        self.mission.menu_open = false;
        self.mission.menu_choice = Some(crate::mission::MenuChoice::Quit);
        self.level_exit = Some(0x0700_0048);
        self.level_exit_cold = true;
    }
}
