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
//! 6. The menu — the 0x40000042 fail/death menu's handler is `P_ENDMISSION_Handler` 0x216618
//!    (`Handler_HandleMessage` 0x2029e0:498-500 routes menu id 0x40000042 to it; 0x204408 is the
//!    PAUSE menu 0x10000028's handler, :217-219). Its event-0x4b arms over `__Menu_Send`'s row
//!    index [M 0x216618:11-27]: row 0 (Restart) → `ResetMap_LevelToLoad(GameState._12_4_, 1, 1)` +
//!    `Player_RamLoad(Bond, 1)` — the CURRENT level, `IsWarmReset = 1`, no 0x70→0x71 remap;
//!    row 1 → `ResetMap_LevelToLoad(Mission_BaseMapHCode(), 0, 1)` — BaseMap, cold;
//!    row 2 → `GameState+0x28 = 0x4000001c` + `ResetMap_LevelToLoad(0x7000048, 0, 0)` — the front
//!    end. `Mission_BaseMapHCode` returns `BaseMap` — the mission's FIRST level [M 0x1830a0].
//!
//! The Restart is a WARM reload of the CURRENT level: `IsWarmReset = 1` keeps the switch channels
//! (`Init_SwitchChannels` only runs when it is 0 [M 0x1d5080:48-50]) and runs `Player_RamLoad`'s
//! whole body [M 0x1957a8:9-37] — the ENTRY checkpoint (the `Player_RamSave` the previous level's
//! exit made [M 0x1b2318:115]; script checkpoints too, case 0x12 [M 0x1d94c0:194-199]) comes back:
//! Bond's loadout blocks, `PlrMissionStats`, the four channel arrays — whose ch 99/100 are clean
//! because they were saved before this level ever failed — plus the difficulty health floor
//! (`ContinueHealthBoost*`, all 50.0 [L ee_start_05.bin 0x30cc34-3c]) because the handler passes
//! `Player_RamLoad`'s boost arg 1. `Mission_Init(level, 1)` resets the current and future parts'
//! objective states + condition channels (previous parts keep theirs [M 0x182270:68-129]) and
//! `Player_InitWeapon` re-runs the level's entry case (0x07000007: the fixed party loadout, armour 0
//! [M 0x1a7ef8:176-186]). The trigger-exit level change is the other warm arm (`ResetMap_LevelToLoad(
//! next, 1, 0)` + a fresh `Player_RamSave` [M 0x1b2318:111-116] — fix-level2's carry).
//!
//! fix-death-restart had the restart COLD to `Mission_BaseMapHCode()`, citing 0x204408 — that is
//! the PAUSE menu's Restart, not the fail menu's (the handler routing, above). Carrying the DYING
//! state is what looped the 07000007 session (playtest 2026-10-12): a capture taken at the fail
//! brings ch 99 back. The game never does that — the restart restores the ENTRY checkpoint, whose
//! channels predate the fail (fix-restart-target; docs/reports/fix-restart-target.md).

/// The red fade's tick count (`RED_FADE_TICKS`): see the module docs — sprite width 512 [L
/// BloodSprInfo 0x2bcfd8], the cover gate `viewer.h = 448` [M 0x19f4e8:33].
pub const RED_FADE_TICKS: u32 = 448;

/// The death anim script (`Player_ChangeSubState` 0x190630 case 0xd/0xe:
/// `AnimScriptAdd(obj, 0x6000151)`). The 0x60008e0 arm plays only when the gravity global
/// `DAT_002d8894` is 0.0 — retail sets it to 0xc11ccccd (−9.4) in `bootup` 0x1c7e78:73 and
/// `Player_Init` 0x1910f8:153, so 0x6000151 is the only death script that runs [M].
pub const DEATH_SCRIPT: u32 = 0x0600_0151;

/// The dead-tick clock's advance: the script's speed 1.0 [M `AnimScriptInit` 0x1c1600 sets +0x98 =
/// 1.0] × `FRAME_RATE_MUL` 2.0. [L] PCSX2 trace: the death script's +0x90 reads 12, 32, 52 at dead
/// frames 8, 18, 28 — +2.0 per dead tick (`GS_SetRefreshRate` 0x17d2b8:15:
/// `FRAME_RATE_MUL = 60 / FRAME_RATE`; the trace's capture ran the 30 Hz arm).
pub const DEATH_SCRIPT_FRAMES_PER_TICK: f32 = 2.0;

/// One fired op-4 event of the death script (the observability the tests and the viewer read).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DeathEvent {
    /// The command's script frame (op-4 arg 0) [M `AnimProcessScriptCmds` 0x1c1b38 `puVar12[1]`].
    pub frame: u16,
    /// The op-4 kind (arg 1): 0/4/5 the footsteps, 6 the body effect [M 0x1c1b38 case 4's switch].
    pub kind: u16,
    /// The dead tick it fired on (1 = the death tick itself).
    pub dead_tick: u32,
    /// The sound the game queued for it: the surface's footstep (EffectInfo+0x16) / body
    /// (EffectInfo+0x18) column id at the feet-contact material [M `Effect_Player` 0x1cec48,
    /// `Effect_Body` 0x1cf400] — None when the contact material is 0 (`Effect_Create`'s guard
    /// [M 0x1ce688]) or the column holds 0xffff.
    pub sound: Option<u32>,
    /// Kinds 0/4/5: the `0x42 | foot` flags — 8 while `ScriptFootstepFoot` is set, 4 when clear
    /// [M `AnimObjectUpdate` 0x1befd8:190-192]; 0 for the other kinds.
    pub foot: u8,
}

/// The death sequence's state. The game keeps the marker in `obj+0xff`/`obj+0xec`/`obj+0xf4` and
/// the pane in the HUD; the port keeps it here (per sim, fresh at every load — `Player_Init`
/// 0x1910f8:26,30 clears ch 0x62 and the death state is not part of any save [M]).
#[derive(Clone, Debug, PartialEq)]
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
    /// The death script's clock (`sAnimScript +0x90`, frames from 1 [M 0x1c1600]): 1.0 from the
    /// death tick, +2.0 per dead tick [L above], capped at the script's length (end mode 0 finishes
    /// the script there [M `AnimScriptEnd` 0x1c1930]).
    pub script_time: f32,
    /// Every death-script op-4 event fired so far, in order (bounded by the script's op-4 count).
    pub events: Vec<DeathEvent>,
    /// The last footstep foot (`ScriptFootstepFoot` [M 0x1c1b38 case 0/4/5]: kind 5 sets it, 4
    /// clears it, 0 toggles it; the BSS global starts clear [S]).
    pub footstep_right: bool,
}

impl Default for Death {
    fn default() -> Self {
        Death { active: false, ticks: 0, pane_on: false, covered: false, ch62: false, script_time: 0.0, events: Vec::new(), footstep_right: false }
    }
}

impl crate::drone::Drones {
    /// One dead tick's slice of the death script's command run — `AnimScriptTick` 0x1c2630 →
    /// `AnimProcessScriptCmds` 0x1c1b38 over the (from, to] window the dead-tick clock just
    /// crossed. The clock starts at 1.0 on the death tick (the just-started window (1, 1] is empty
    /// for 0x06000151: its first command sits at frame 5) and advances 2.0 frames per dead tick
    /// [L above]; each command fires once when the clock reaches its frame — the game latches the
    /// per-command fired bits in the script's +0x7C [M 0x1c1b38:465-471], the port's monotonic
    /// clock fires each frame once structurally. Effects are the game's chain for the DEAD player
    /// (kind 4/5 the footstep sound, kind 6 the body sound); kinds 1/2/3/7 never occur in
    /// 0x06000151 and kind 2's handler (+0x70) is zeroed for player scripts [M 0x1c1600:51].
    fn death_script_events(&mut self, d: &crate::drone::Data, bond: crate::world::V3) {
        let Some(script) = d.scripts.get(&DEATH_SCRIPT) else { return }; // not in this level's archive
        let len = script.length as f32;
        if self.death.script_time >= len { return; } // the script finished [M 0x1c1930]
        let from = self.death.script_time as u32;
        self.death.script_time = (self.death.script_time + DEATH_SCRIPT_FRAMES_PER_TICK).min(len);
        let to = self.death.script_time as u32;
        // The sounds play at the feet contact with the contact's material: the game reads the anim
        // object's HITDATA shadow (+0x40 the contact point, +0x50 & 0x3f the material) —
        // `Player_FeetOnPoint` 0x194188:183-191 copies the feet ray's 0x60-byte hitdata into it
        // while grounded (skipped while airborne, +0x50 0xd/0xe). The port queries the floor under
        // Bond (the drone-footstep pattern).
        let top = [bond[0], bond[1] - 0.1, bond[2]];
        let bottom = [bond[0], bond[1] - 1.6, bond[2]];
        let (mat, pos) = d.world.segment_surface(top, bottom, true).map_or((0u8, bond), |(t, _, m)| {
            let p = [top[0] + (bottom[0] - top[0]) * t, top[1] + (bottom[1] - top[1]) * t, top[2] + (bottom[2] - top[2]) * t];
            (m, p)
        });
        // `Effect_Create`'s guard [M 0x1ce688]: a 0 contact material (no surface under Bond) mutes
        // the whole effect — the event fires with no sound.
        let footstep_col: &[u16] = d.surfaces.as_ref().map_or(&[], |s| &s.footstep[..]);
        let body_col: &[u16] = d.surfaces.as_ref().map_or(&[], |s| &s.body[..]);
        let column = |col: &[u16]| if mat == 0 { None } else { nf_formats::sfx::SurfaceSounds::get(col, mat) };
        for c in &script.commands {
            let (op, [f, kind, ..]) = (c.op, c.args.as_slice()) else { continue };
            if op != 4 || !(*f as u32 > from && *f as u32 <= to) { continue; }
            let mut event = DeathEvent { frame: *f, kind: *kind, dead_tick: self.death.ticks + 1, sound: None, foot: 0 };
            match *kind {
                // Footsteps [M 0x1c1b38 cases 0/4/5]: kind 5 sets `ScriptFootstepFoot`, 4 clears
                // it, 0 toggles it, and each sets `ScriptFootstepType` = 2 (0x06000151 ∉
                // {0x6000099, 0x60000ae}). The dead player is NOT class 3 — the death marker set
                // obj+0xff = 0x12 [M 0x190810:72] — so `AnimObjectUpdate` 0x1befd8:184-186 takes
                // the ScriptFootstepType arm, sets the anim object's bit 4 and calls
                // `Effect_Create(1.0, animobj, Bond, 0x42 | foot)` [M :190-199]; `Effect_Player`
                // 0x1cec48's 0x40 arm plays the surface's footstep sound (EffectInfo+0x16) at the
                // contact with `Sound_SetAlertness(0, h)` [M 0x1de210] — silent to the AI. (The
                // 0xc decal arm — the footprint — is not ported, as for the drones.)
                0 | 4 | 5 => {
                    self.death.footstep_right = match *kind { 5 => true, 4 => false, _ => !self.death.footstep_right };
                    event.foot = if self.death.footstep_right { 8 } else { 4 };
                    event.sound = column(footstep_col);
                    if let Some(id) = event.sound {
                        // SetAlertness 0: the port plays it and keeps it out of the hearing list.
                        self.audio.play3d(id, pos, None);
                    }
                }
                // The body effect [M 0x1c1b38 case 6]: `Effect_Create(1.0, animobj, Bond, 0xc0)` →
                // the material guard → `Effect_Body` 0x1cf400: the surface's body sound
                // (EffectInfo+0x18) at the contact with `Sound_ModAlertness(0.5, h)` [M 0x1de220] —
                // the node's alertness halves (the port: the hearing entry at half weight).
                6 => {
                    event.sound = column(body_col);
                    if let Some(id) = event.sound {
                        let h = d.sfx.map(|t| t.get(id)).unwrap_or(nf_formats::sfx::SfxHearing { id, inner: 10.0, outer: 20.0, weight: 0.0, looped: false, cull: false });
                        self.audio.play3d(h.id, pos, None);
                        if !h.looped {
                            self.sounds.push(crate::drone::Sound { id: h.id, pos, inner: h.inner, outer: h.outer, weight: h.weight * 0.5 });
                        }
                    }
                }
                _ => {}
            }
            self.death.events.push(event);
        }
    }

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
            self.death.script_time = 1.0; // AnimScriptInit 0x1c1600 (+0x90 = 1.0) on `AnimScriptAdd`
            self.death.events.clear();
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
        // The death script's commands for this tick (`AnimScriptTick` 0x1c2630 →
        // `AnimProcessScriptCmds` 0x1c1b38) — BEFORE the pane/channel logic, the game's order:
        // the dead branch runs `AnimObjectUpdate` then `Player_HandleDeath` [M 0x193600:317-319],
        // and nothing gates the anim afterwards (ch 0x62 / the menu do not stop it).
        self.death_script_events(d, bond);
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

    /// Restart picked (`P_ENDMISSION_Handler` 0x216618:19-21, menu 0x40000042's row 0):
    /// `ResetMap_LevelToLoad(GameState._12_4_, 1, 1)` + `Player_RamLoad(Bond, 1)` — a WARM reload
    /// of the CURRENT level (`GameState+0xC` — `Drones::level_current` [M 0x1d5080:6], the level
    /// Bond is on, NOT the mission's base map; that is the pause menu's Restart, `C_GCPAUSE_Handler`
    /// 0x204408:139) on the ENTRY checkpoint with the 50.0 health floor [L 0x30cc34-3c]. The viewer
    /// re-serves the save this level was loaded with (`level_exit_restart`); it does NOT capture the
    /// dying state — that would carry ch 99 and replay the fail (the old 07000007 loop).
    pub fn fail_choose_restart(&mut self) {
        if !self.mission.menu_open { return; }
        self.mission.menu_open = false;
        self.mission.menu_choice = Some(crate::mission::MenuChoice::Restart);
        self.level_exit = Some(self.level_current); // [M 0x216618:20 GameState._12_4_]
        self.level_exit_cold = false;                // [M 0x216618:20 IsWarmReset = 1]
        self.level_exit_restart = true;              // [M 0x216618:21 Player_RamLoad(Bond, 1)]
    }

    /// Quit picked (menu 0x40000042's row 2, `P_ENDMISSION_Handler` 0x216618:24-27 — the pause
    /// menu's Quit is the same shape, `C_GCPAUSE_Handler` 0x204408:141-149): `GameState._28_4_ =
    /// 0x4000001c` and `ResetMap_LevelToLoad(0x7000048, 0, 0)` — the front end. The port cannot load
    /// 0x07000048 (no front end); the viewer logs the request and holds, like the driving levels.
    pub fn fail_choose_quit(&mut self) {
        if !self.mission.menu_open { return; }
        self.mission.menu_open = false;
        self.mission.menu_choice = Some(crate::mission::MenuChoice::Quit);
        self.level_exit = Some(0x0700_0048);
        self.level_exit_cold = true;
    }
}
