//! The mission layer (`Mission_*`, docs/systems/mission.md): the MissionData table
//! (ACTION.ELF 0x2a4350, 24 records × 40 bytes with 0x18-byte objective records in the named
//! `.data` tables), `Mission_Init` (0x182270), `Mission_MonitorObjectives` (0x1827d8) and
//! `Mission_Update` (0x182c60).
//!
//! The game's channels of record: **99 (0x63) = mission fail**, **100 (0x64) = mission success**
//! (all objectives' conditions AND-ed), **0x62 = player dead** (`Player_HandleDeath` 0x190bb0:32
//! writes it; `Player_Init` 0x1910f8:26,30 clears it), **0x60 = killed civilians** and
//! **0x61 = alarm raised** (the two standing fail hooks in `Mission_MonitorObjectives`).
//!
//! Every objective record (+0x18 bytes) [L ee_start_05.bin; fields read by 0x1827d8/0x182270]:
//! `+0x00 u32 text label` · `+0x04 u32 fail label (0xffffffff = none)` · `+0x08 u32 hint label` ·
//! `+0x0C u8 condition channel` · `+0x0D u8 its init value` · `+0x0F u8 enable channel (0 = none)` ·
//! `+0x10 u8 flags (bit 1 = the channel being ON means FAIL)` · `+0x14 u32 runtime state`.
//!
//! States [M 0x1827d8 switch on `record+0x14`]: 0 = pending (announce when the record's level is
//! reached), 1 = pending with an enable channel (announce when that channel fires), 2 = announced
//! fail-objective (the mission fails when its channel turns ON), 3 = announced (complete when the
//! channel turns ON), 4 = complete, 5 = failed (records its fail label). `Mission_Init` writes 0/1
//! for the current and future parts and leaves previous parts alone [M 0x182270:92-118].

pub mod c07000001;
pub mod c07000002;
pub mod c07000003;
pub mod c07000004;
pub mod c07000005;
pub mod c07000006;
pub mod c07000007;
pub mod c07000008;
pub mod c07000009;
pub mod c0700000a;
pub mod c0700000b;
pub mod c0700000c;
pub mod c0700000d;
pub mod c0700000e;
pub mod c0700000f;
pub mod c07000010;
pub mod c07000011;
pub mod c07000012;
pub mod c07000013;
pub mod c07000014;
pub mod c07000015;
pub mod c07000016;
pub mod c0700001b;
pub mod c0700004a;

/// One objective record (0x18 bytes, `MissionData`'s `+0x0C` pointer target) [L ee_start_05.bin].
#[derive(Clone, Copy, Debug)]
pub struct Objective {
    /// `+0x00`: the text label (bank 4, e.g. 0x04000000 "Breach the castle walls").
    pub text: u32,
    /// `+0x04`: the fail label (0xffffffff = none in every shipped record).
    pub fail: u32,
    /// `+0x08`: the hint/long-description label (filled but never drawn by ACTION.ELF code).
    pub hint: u32,
    /// `+0x0C`: the condition channel (0 = the raw `switch_channels[0]`, which the disc's out-0
    /// triggers can write — the game has no zero guard [M 0x1827d8:33-37]).
    pub channel: u8,
    /// `+0x0D`: the value `Mission_Init` resets the condition channel to.
    pub init: u8,
    /// `+0x0F`: the enable channel (0 = none; announce without it).
    pub enable: u8,
    /// `+0x10`: flags — bit 1: the channel being ON means the objective FAILS.
    pub flags: u8,
}

/// One `MissionData` record: a level's slice of its mission, with its objective table.
#[derive(Clone, Copy, Debug)]
pub struct LevelDef {
    /// `+0x00`: the level hashcode (the 070000NN.bin archive).
    pub level: u32,
    /// `+0x04`: the mission's first level.
    pub mission: u32,
    /// `+0x08`: the part index within the mission.
    pub part: u32,
    pub objectives: &'static [Objective],
}

/// All 24 records, in table order [L ee_start_05.bin 0x2a4350; also ee_start_07000001.bin — identical].
pub static LEVELS: &[LevelDef] = &[
    c07000005::DEF, c07000006::DEF, c07000007::DEF, c07000008::DEF,
    c07000001::DEF, c07000002::DEF, c07000003::DEF, c07000004::DEF,
    c07000009::DEF, c0700000a::DEF, c0700000b::DEF,
    c0700000c::DEF, c0700000d::DEF,
    c0700000e::DEF, c0700000f::DEF,
    c07000011::DEF, c07000012::DEF, c0700004a::DEF, c07000013::DEF,
    c07000010::DEF,
    c07000014::DEF, c07000015::DEF, c07000016::DEF,
    c0700001b::DEF,
];

/// What the mission layer did this tick — the port's stand-in for the game's `Text_AddMsg` calls,
/// the fade/`GameFlow_PushState` effects and `ResetMap_LevelToLoad` [M 0x1827d8/0x182c60].
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// A type-2 objective notice [M 0x1827d8:41-84]: `label` = the text (0x04000000-style) or
    /// 0x2000053 ("OBJECTIVE COMPLETE"); `is_new` = the game's `ThisOrderNum <= record.part` arg;
    /// `ticks` = the message's life (300 announce / 180 complete+fail); `failed` = the game's
    /// `"%s: %s"` Fail-prefix format (0x30cbb8) instead of plain "%s" (0x30cbb0).
    Objective { label: u32, is_new: bool, ticks: u32, failed: bool },
    /// `FailLabel` changed [M 0x1827d8 case 5 + the 0x60/0x61/level hooks]: the mission banner's
    /// second line (0x4000032 "Alarm Raised", 0x4000033 "Cover Lost", 0x4000034 "Killed Civilians",
    /// 0x2000059/5a).
    FailLabel(u32),
    /// "Mission Complete" (0x2000006, type 3) posted; `next` = `LevelToEndTo` [M 0x182c60 case 6].
    MissionComplete { next: u32 },
    /// "Mission Fail" (0x2000007, type 3) posted [M 0x182c60 case 2].
    MissionFailed,
    /// The player died this tick (ch 0x62 rose) [M 0x182c60 case 1's else branch].
    PlayerDead,
    /// The banner finished: success → `ResetMap_LevelToLoad(LevelToEndTo)` [M 0x182c60 case 4],
    /// failure/death → the 0x40000042 menu, which the port renders as a level restart.
    LevelEnd { next: u32 },
    /// The 0x40000042 fail/death menu opened (`MenuManager_Create` [M 0x182c60:78] — the port's
    /// `Mission.menu_open`): the view (180-fix-menus) draws Restart/Quit and answers through
    /// `Drones::fail_choose_restart` / `fail_choose_quit` (crate::death). With
    /// `Drones.auto_restart` set the port answers Restart immediately (the pre-menu behaviour).
    MenuOpen,
}

/// The 0x40000042 fail/death menu's options (`C_GCPAUSE_Handler` 0x204408: control 6's value
/// 1 = Restart (`state_quit_166 = 0`), 2 = Quit (`state_quit_166 = 1`) [M]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MenuChoice { Restart, Quit }

/// The mission layer's globals: `InternalState` 0x30cb98, `MissionState` 0x30cb9c, `BaseMap`
/// 0x30cba0, `ThisOrderNum` 0x30cba4, `FailLabel` 0x30cba8, `LevelToEndTo` 0x30cbac, `TimeOut`
/// (0x182c60's local static) — plus the per-objective runtime states [M symtab].
pub struct Mission {
    pub base_map: u32,
    pub order: u32,
    pub internal: u32,
    pub state: u32,
    pub fail_label: u32,
    pub to_end: u32,
    pub timeout: f32,
    /// `record+0x14` per LEVELS entry, per objective.
    pub states: Vec<Vec<u32>>,
    /// The 0x40000042 fail/death menu is up (`MenuManager_Create` [M 0x182c60:78]); the mission
    /// waits in InternalState 5 for `Drones::fail_choose_*` (crate::death). Not saved — the game's
    /// menu dies with the level change.
    pub menu_open: bool,
    /// The player's answer (`C_GCPAUSE_Handler` 0x204408 controls 6/0xc/0xd).
    pub menu_choice: Option<MenuChoice>,
}

impl Default for Mission { fn default() -> Self { Self::new() } }

impl Mission {
    /// The BSS globals: everything 0 [L ee_start_05.bin 0x30cb98..0x30cbb0 all zero].
    pub fn new() -> Mission {
        Mission {
            base_map: 0, order: 0, internal: 0, state: 0, fail_label: 0, to_end: 0, timeout: 0.0,
            states: LEVELS.iter().map(|l| vec![0; l.objectives.len()]).collect(),
            menu_open: false,
            menu_choice: None,
        }
    }

    /// `Mission_Init(level, IsWarmReset)` (0x182270), called from `ResetMap_GameInit`
    /// (0x1d5080:53 `Mission_Init(GameState._12_4_, IsWarmReset)`).
    pub fn init(&mut self, level: u32, warm: bool, ch: &mut crate::triggers::Channels, frame: u32) {
        self.to_end = 0x0700_0048; // [M 0x182270:20]
        if !warm { self.base_map = u32::MAX; } // [M 0x182270:22-24 param_2 == 0]
        self.internal = 1;
        self.order = u32::MAX;
        self.fail_label = 0; // the game's non-label sentinel [M 0x182270:27]
        self.set_state(1);   // Mission_SetStatus(1) [M 0x182270:28]
        match LEVELS.iter().find(|l| l.level == level) {
            None if !warm => {} // BaseMap stays -1 → state 5 (no mission) below
            None => {}           // warm: the previous BaseMap survives [M 0x182270:34-49]
            Some(l) => { self.order = l.part; self.base_map = l.mission; }
        }
        if self.base_map == u32::MAX {
            self.internal = 5;
            self.set_state(5);
            return;
        }
        // `PlrStat_ResetForMission` when the level is the mission's first [M 0x182270:59-66].
        // (fix-progress: the port's PlrStat counters live in `Drones::progress`; the reset runs in
        // the tick's `Mission_Init` latch, next to this call.)
        // Per record of this mission [M 0x182270:68-129]: previous parts keep their state,
        // current + future parts reset (state 0/1 by the enable channel, the condition channels
        // back to their init values).
        for (i, l) in LEVELS.iter().enumerate() {
            if l.mission != self.base_map { continue; }
            // The game's modes: part < current → 1 (cold) / 4 (warm), part == current → 2, part >
            // current → 0 [M 0x182270:73-84]. Modes 1 and 4 both skip the reset (previous parts keep
            // their state); only modes 0 and 2 run the body below.
            if l.part < self.order {
                // fix-objective-messages: the game's record states live in `.data` and NO load path
                // reinitialises them (`ResetMap_GameInit` 0x1d5080 only calls `Mission_Init`, which
                // touches current + future parts [M 0x182270:68-129]) — at any real arrival the prior
                // parts sit ANNOUNCED: [L work/ee_start_0700000a.bin @ 0x2a3f78 — mission 07000009's
                // rec1 (condition 0x46, still unfired) is state 3 at 0700000a's start]. The port's
                // fresh `Mission::new()` fabricates 0 there, and case 0 then announces every prior
                // record whose `level <= current` at the new level (07000007 stacked 07000005/06's
                // banners; the restart re-announced them). Mark those fabricated 0s announced — the
                // monitor's own 0→2/3 transition [M 0x1827d8:47-51], without queueing a message.
                for (j, o) in l.objectives.iter().enumerate() {
                    if self.states[i][j] == 0 { self.states[i][j] = if o.flags & 2 != 0 { 2 } else { 3 }; }
                }
                continue;
            }
            for (j, o) in l.objectives.iter().enumerate() {
                self.states[i][j] = if o.enable != 0 { 1 } else { 0 }; // [M 0x182270:98-105]
                if o.channel != 0 {
                    ch.set(o.channel as u32, o.init, frame); // [M 0x182270:106-107]
                    if o.init != 0 { ch.time[o.channel as usize] = frame; } // [M 0x182270:112]
                }
            }
        }
    }

    /// `Mission_SetStatus` (0x1830d8): the global `MissionState`.
    pub fn set_state(&mut self, s: u32) { self.state = s; }

    /// `Mission_Status` (0x1830d0): the global for the level-end/progress readers.
    pub fn status(&self) -> u32 { self.state }

    /// `Mission_SetFailLabel` (0x183098).
    pub fn set_fail_label(&mut self, label: u32) { self.fail_label = label; }

    /// `Mission_Update` (0x182c60), called once per tick from `Game_Run`. `dead` = Bond's health has
    /// run out (the game reads ch 0x62, written by `Player_HandleDeath` 0x190bb0:32).
    pub fn update(&mut self, ch: &mut crate::triggers::Channels, frame: u32, level: u32, dead: bool, out: &mut Vec<Event>) {
        if level == 0x0700_0048 { return; } // [M 0x182c60:13-15]
        match self.internal {
            1 => {
                if !ch.on(0x62) {
                    self.monitor(ch, frame, level, out);
                    if ch.on(99) {
                        self.internal = 2; // [M 0x182c60:30-33 Music_Event(7,1)]
                    } else if ch.on(100) {
                        self.internal = 6; // [M 0x182c60:23-28 Music_Event(8,1), FadeClr 0xff]
                    }
                } else {
                    // [M 0x182c60:35-42: Music_Event(6,1), state 3, FadeClr 0xff0000ff, health 0]
                    self.internal = 3;
                    out.push(Event::PlayerDead);
                }
            }
            2 => {
                // [M 0x182c60:45-57: MissionState 2, LevelToEndTo 0x7000048, the type-3
                // "Mission Fail" msg, TimeOut = VIDEO_FRAME_RATE·4 + VIDEO_FRAME_RATE = 300]
                self.state = 2;
                self.to_end = 0x0700_0048;
                self.internal = 7;
                out.push(Event::MissionFailed);
                self.timeout = 300.0;
            }
            3 => {
                // [M 0x182c60:58-64: TimeOut 1.0, MissionState 3, state 7, to_end 0x7000048]
                self.timeout = 1.0;
                self.state = 3;
                self.internal = 7;
                self.to_end = 0x0700_0048;
            }
            6 => {
                // [M 0x182c60:85-122: MissionState 6, the per-level LevelToEndTo, the type-3
                // "Mission Complete" msg, TimeOut = VIDEO_FRAME_RATE << 2 = 240]
                self.state = 6;
                self.internal = 7;
                self.to_end = match level {
                    0x0700_0004 => 0x0730_0004,
                    0x0700_0008 => 0x0730_0008,
                    0x0700_000b => 0x0730_000b,
                    0x0700_000d => 0x0730_000d,
                    0x0700_000f => 0x0730_000f,
                    0x0700_0013 => 0x0730_0013,
                    0x0700_0016 => 0x0730_0017,
                    0x0700_001b => 0x0730_001b,
                    _ => 0x0700_0048,
                };
                out.push(Event::MissionComplete { next: self.to_end });
                self.timeout = 240.0;
            }
            7 => {
                // [M 0x182c60:123-130: GS_PausePlayer(1, 0); TimeOut -= FRAME_RATE_MUL; pause]
                self.timeout -= 1.0;
                if self.timeout <= 0.0 { self.internal = 4; }
            }
            4 => {
                if self.state == 6 {
                    // [M 0x182c60:65-76: the level-progress bookkeeping, ResetMap_LevelToLoad,
                    // GameFlow_PushState fade]
                    out.push(Event::LevelEnd { next: self.to_end });
                    self.internal = 5;
                } else {
                    // [M 0x182c60:77-81: MenuManager_Create(-0x80000004, 0x40000042, 0, 0, 0x28, 0, 4)
                    // + GS_PauseGame(1); InternalState = 5] — the fail/death menu opens and the
                    // mission waits for the player's Restart / Quit (crate::death's fail_choose_*).
                    self.menu_open = true;
                    out.push(Event::MenuOpen);
                    self.internal = 5;
                }
            }
            5 => {
                // The 0x40000042 menu is up and the mission waits for the view's Restart/Quit —
                // the game's case 5 is a bare `break` [M 0x182c60:83-84]. The port used to fall to
                // the catch-all and re-arm internal 1, re-running the whole death/fail sequence
                // (and queueing another restart) every tick after the answer — the user's "replay
                // loop where it kept replaying at the start of the mission over and over again".
            }
            _ => self.internal = 1, // [M 0x182c60:131-133]
        }
    }

    /// `Mission_MonitorObjectives` (0x1827d8) — runs from state 1 while the player is alive.
    fn monitor(&mut self, ch: &mut crate::triggers::Channels, frame: u32, level: u32, out: &mut Vec<Event>) {
        let mut all = true;
        for (i, l) in LEVELS.iter().enumerate() {
            if l.mission != self.base_map { continue; }
            let is_new = self.order <= l.part; // [M 0x1827d8:26 — the Text_AddMsg isNew arg]
            for (j, o) in l.objectives.iter().enumerate() {
                // The condition [M 0x1827d8:32-38]: flags&2 → the channel being OFF satisfies it;
                // otherwise the channel being ON. The raw table byte (the game has no ch-0 guard).
                let raw = ch.v.get(o.channel as usize).copied().unwrap_or(0);
                let cond = if o.flags & 2 != 0 { raw == 0 } else { raw != 0 };
                all &= cond;
                match self.states[i][j] {
                    0 => {
                        // [M 0x1827d8:41-54: announce once the record's level is reached]
                        if l.level <= level {
                            let failed = o.flags & 2 != 0;
                            out.push(Event::Objective { label: o.text, is_new, ticks: 300, failed });
                            self.states[i][j] = if failed { 2 } else { 3 };
                        }
                    }
                    1 => {
                        // [M 0x1827d8:55-67: the enable channel fires → announce]
                        if o.enable != 0 && o.enable != 0xff && ch.on(o.enable as u32) {
                            let failed = o.flags & 2 != 0;
                            out.push(Event::Objective { label: o.text, is_new, ticks: 300, failed });
                            self.states[i][j] = if failed { 2 } else { 3 };
                        }
                    }
                    2 => {
                        // [M 0x1827d8:68-77: the fail-objective's channel turned ON → the mission fails;
                        // the notice is "%s: %s" = "Fail: <text>"]
                        if !cond {
                            ch.set(99, 1, frame);
                            out.push(Event::Objective { label: o.text, is_new, ticks: 180, failed: true });
                            self.states[i][j] = 5;
                        }
                    }
                    3 => {
                        // [M 0x1827d8:78-85: the channel turned ON → "OBJECTIVE COMPLETE"]
                        if cond {
                            out.push(Event::Objective { label: 0x0200_0053, is_new, ticks: 180, failed: false });
                            self.states[i][j] = 4;
                        }
                    }
                    5 => {
                        // [M 0x1827d8:86-89: a failed objective with its own fail label]
                        if o.fail != 0xffff_ffff && self.fail_label != o.fail {
                            self.fail_label = o.fail;
                            out.push(Event::FailLabel(o.fail));
                        }
                    }
                    _ => {}
                }
            }
        }
        // The standing fail hooks [M 0x1827d8:98-117].
        if ch.on(0x61) {
            self.fail_label = 0x0400_0032;
            out.push(Event::FailLabel(0x0400_0032));
            ch.set(99, 1, frame);
        }
        if ch.on(0x60) {
            self.fail_label = 0x0400_0034;
            out.push(Event::FailLabel(0x0400_0034));
            ch.set(99, 1, frame);
        }
        if level == 0x0700_0016 && ch.on(0x7b) {
            self.fail_label = 0x0200_0059;
            out.push(Event::FailLabel(0x0200_0059));
            ch.set(99, 1, frame);
        } else if level == 0x0700_001b && ch.on(199) {
            self.fail_label = 0x0200_005a;
            out.push(Event::FailLabel(0x0200_005a));
            ch.set(99, 1, frame);
        }
        // All conditions AND-ed → success [M 0x1827d8:119-123 + Music_Event(8,1)].
        if all {
            ch.set(100, 1, frame);
        }
    }

    /// The state snapshot for a level carry / save (the game's `LS_MakeMission` 0x1cd720 stores
    /// BaseMap/ThisOrderNum/FailLabel + the states; `LS_LoadMission` 0x1cd880 restores them).
    pub fn snapshot(&self) -> MissionSave {
        MissionSave {
            base_map: self.base_map, order: self.order, internal: self.internal, state: self.state,
            fail_label: self.fail_label, to_end: self.to_end,
            states: self.states.iter().flatten().copied().collect(),
        }
    }

    /// The warm-transition carry: restore the mission layer's `.data` objective records.
    ///
    /// The game's level load never restores the layer's globals: `ResetMap_GameInit` (0x1d5080:53)
    /// runs `Mission_Init(GameState._12_4_, IsWarmReset)`, which re-arms the layer (InternalState 1,
    /// MissionState 1, FailLabel 0, LevelToEndTo 0x07000048 [M 0x182270:20-28]) and resets the
    /// current + future parts' records in place — the past parts' records are the live `.data` the
    /// load finds, and `LS_LoadMission` (0x1cd880) is the SAVE-FILE path, not a level load [M
    /// docs/systems/mission.md §3]. Restoring the carried internal/state here froze the next
    /// level's monitor whenever the previous level ended inside a post-banner arm (InternalState 5
    /// — case 4 sets it after both the complete and the fail/menu arms): the objective monitor
    /// (`Mission_Update` case 5 [M 0x182c60:83-84] is a bare break) never ran again, no objective
    /// could fold and the level could never end (campaign-flow: 07000007/08/0a arrived frozen).
    /// The save-file path (210-fix-progress' LS_LoadMission) will need the globals variant.
    pub fn restore(&mut self, s: &MissionSave) {
        let mut it = s.states.iter();
        for row in self.states.iter_mut() { for v in row.iter_mut() { *v = it.next().copied().unwrap_or(0); } }
    }
}

/// The save payload (`LS_MakeMission`/`LS_LoadMission` 0x1cd720/0x1cd880 carry the mission globals
/// and the per-objective states across levels/saves; the port flattens the states in LEVELS order).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MissionSave {
    pub base_map: u32, pub order: u32, pub internal: u32, pub state: u32,
    pub fail_label: u32, pub to_end: u32,
    pub states: Vec<u32>,
}
