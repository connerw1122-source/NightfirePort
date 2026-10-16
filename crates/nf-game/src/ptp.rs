//! The PTP handover block — the ACTION↔DRIVING transfer (drive-ptp).
//!
//! ACTION fills a 0xa50-byte block (`PTPDATA` at EE 0x31c090, `Boot_GetPTPData` 0x1c8198, 968 B),
//! `psiLaunchDriving` 0x120af0 copies it to EE 0x100000 and execs DRIVING.ELF; DRIVING reads its
//! fields (level key at PTP+0x4f4, mission index at +0x974, status at +0x984) and, on exit,
//! writes the result flag +0x97c, the next index +0x974 and the next key +0x4f4 before BASE.ELF
//! re-dispatches. ACTION's next boot mirrors the read (`Boot_LoadPTPData` 0x1c8560).
//!
//! Every offset below is measured from the decompile [M], with the DRIVING-side strings from the
//! ELF [D]. The struct models the fields the decompile writes AND reads on both sides; the gaps
//! (e.g. +0x1b4..+0x4b4, never touched by either side's handover code) are modelled as raw bytes.

/// The block's size [M 0x1c8198:98 `*param_2 = 0xa50`].
pub const PTP_SIZE: u32 = 0xa50;
/// Where ACTION builds it [M 0x1c8198 — `&PTPDATA` = 0x31c090].
pub const PTPDATA_ADDR: u32 = 0x31c090;
/// Where the launch copies it (DRIVING's view) [M 0x120af0 `amemcpy(0x100000, data, size)`].
pub const DRIVING_ADDR: u32 = 0x100000;
/// The level the return resumes ACTION at — `Boot_LoadPTPData`'s unconditional
/// `ResetMap_LevelToLoad(0x7000048, 0, 0)` [M 0x1c8560 tail].
pub const RESUME_LEVEL: u32 = 0x0700_0048;

/// The 8 driving archives' keys in the game's fixed order [D DRIVING.ELF 0x25c948 table,
/// 16-byte stride; the same order as DRIVING main 0x174bf0's switch]. `index - 1` = the row.
pub static DRIVING_KEYS: [&str; 8] = [
    "paris_mis01",    // 1 [D 0x374da8; switch case 1]
    "uw_mis11",       // 2 [D 0x374db8; case 2]
    "junglea_mis13a", // 3 [D 0x374dc8; case 3]
    "jungleb_mis13b", // 4 [D 0x374dd8; case 4]
    "snow1a_mis3",    // 5 [D 0x374de8; case 5]
    "snow2a_mis4",    // 6 [D 0x374df8; case 6]
    "junglec_mis13c", // 7 [D 0x374e08; case 7]
    "snow2a_race",    // 8 [D 0x374e18; case 8]
];

/// The switch's default [D 0x374e28 "paris_map"] — the map-view fallback DRIVING main writes
/// when the index byte is not 1..8 [M 0x174bf0:180-190].
pub const DEFAULT_KEY: &str = "paris_map";

/// The level-key buffer's size, 15 chars + NUL [M 0x174bf0:244 `FUN_002a9a50(0x1004f4, arg, 0xf)`].
pub const KEY_MAX: usize = 0xf;

/// `Mission_Update` 0x182c60 case 6 — the on-foot level that ends an arc hands to its driving
/// interlude code (`LevelToEndTo`) [M]. `0x7000016`'s interlude is `0x7300017` (as measured).
pub fn drive_interlude_for(level: u32) -> Option<u32> {
    let next = match level {
        0x0700_0004 => 0x0730_0004,
        0x0700_0008 => 0x0730_0008,
        0x0700_000b => 0x0730_000b,
        0x0700_000d => 0x0730_000d,
        0x0700_000f => 0x0730_000f,
        0x0700_0013 => 0x0730_0013,
        0x0700_0016 => 0x0730_0017,
        0x0700_001b => 0x0730_001b,
        _ => return None,
    };
    Some(next)
}

/// True for the `0x0730xxxx` interlude codes (the drive-intro movie ids ACTION loads through
/// `ResetMap_LevelToLoad` → `ResetMap_Load` phase 2's movie branch [M 0x1d6360:66-114]).
pub fn is_driving_interlude(code: u32) -> bool {
    code & 0xffff_0000 == 0x0730_0000
}

/// The drive index an interlude hands to — the port's stand-in for the hub menu's mission list
/// (`sp_level`, read by `Menu_GetLevelIndex` 0x2023e8), which the retail flow consults before
/// `ResetMap_LevelToLoad(0x900000X)` pushes GameFlow state 9 [M 0x1d3800:106]. The menu table is
/// not readable in this session's window, so the pairing is [S]: the 8 interludes in
/// `Mission_Update`'s measured order paired with the 8 drives in DRIVING's table order [D
/// 0x25c948] — the castle's 0x7300004 → 1 (paris_mis01), the task's own example. A live check in
/// the report replaces this table with the real pairing.
pub fn interlude_drive_index(interlude: u32) -> u8 {
    match interlude {
        0x0730_0004 => 1,
        0x0730_0008 => 2,
        0x0730_000b => 3,
        0x0730_000d => 4,
        0x0730_000f => 5,
        0x0730_0013 => 6,
        0x0730_0017 => 7,
        0x0730_001b => 8,
        _ => 1, // the switch's default lands on "paris_map" [M 0x174bf0:180]; the port re-arms 1
    }
}

/// `Menu_IsDrivingLevel` 0x202750: true for exactly the driving hashes {0x09000001, 0x09000002,
/// 0x09000003, 0x09000005, 0x09000006} [M the band `> 0x9000000 && (< 0x9000004 || (0x9000004 < &&
/// < 0x9000007))` — 0x09000004 and 0x0900007+ are NOT driving levels]. `ResetMap_LevelToLoad`
/// 0x1d3800 branches on this: a driving hash pushes GameFlow state 9 (the DRIVING.ELF handover)
/// instead of the on-foot loader.
pub fn is_driving_level(hash: u32) -> bool {
    hash > 0x0900_0000 && (hash < 0x0900_0004 || (hash > 0x0900_0004 && hash < 0x0900_0007))
}

/// The drive index `PTP+0x974` carries: `GameState._8_4_ - 0x9000000` at the state-9 handover
/// [M 0x1c8198:62 `DAT_0031ca04 = GameState._8_4_ + -0x9000000`], where `GameState._8_4_` is a
/// `0x900000X` driving id (`Menu_IsDrivingLevel` 0x202750 accepts 0x9000001/2/3/5/6; the state-9
/// push itself is [M 0x1d3800:106]). DRIVING masks it to a byte [M 0x174bf0:69].
pub fn driving_index(map_id: u32) -> u8 {
    map_id.wrapping_sub(0x0900_0000) as u8
}

/// The level key DRIVING main writes from the index [M 0x174bf0:82-190].
pub fn key_for_index(index: u8) -> &'static str {
    match index {
        1..=8 => DRIVING_KEYS[(index - 1) as usize],
        _ => DEFAULT_KEY,
    }
}

/// The archive suffix DRIVING opens: everything after the last `_` [M 0x175a28 `FUN_00175a28`].
pub fn key_archive_suffix(key: &str) -> &str {
    match key.rfind('_') {
        Some(i) => &key[i + 1..],
        None => key,
    }
}

/// The 12 `PlrStats` driving slots. `PlrStats_GetDrivingData` 0x199ac8 / `Set` 0x199b18 walk a
/// 12-entry table rooted at the pointer at 0x2bc658 (the table itself 0x2bbda0, stride 0x3c:
/// +0x00 record pointer, +0x18 id) — Get copies `entry+0x18` into the ids array (PTP+0x154) and
/// `record+8` into the values array (PTP+0x184); Set writes them back [M]. The records' +0/+4
/// are a kind and a threshold in .data [L ACTION.ELF 0x2bbda0..]; +8 (the carried value) is 0 in
/// the boot snapshot [L ee_start_05.bin @ 0x2bbda0+8 — all 12 zero].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DrivingStats {
    /// PTP+0x154..+0x184: the ids [M 0x199ac8 `*param_1 = (int)ppuVar2[6]`].
    pub ids: [u32; 12],
    /// PTP+0x184..+0x1b4: the values (record+8) [M 0x199ac8 `*param_2 = *(record + 8)`].
    pub values: [u32; 12],
}

impl Default for DrivingStats {
    fn default() -> Self {
        // [L ee_start_05.bin @ 0x2bbda0 — every record+8 is 0 at boot; the id slots read 0 in the
        // snapshot too (one transient word excepted), so the port carries 0 ids like the game's
        // data does.]
        DrivingStats { ids: [0; 12], values: [0; 12] }
    }
}

impl DrivingStats {
    /// `PlrStats_GetDrivingData(ids, values)` [M 0x199ac8]: read the 12 (id, value) pairs out.
    pub fn get_into(&self, ids: &mut [u32; 12], values: &mut [u32; 12]) {
        *ids = self.ids;
        *values = self.values;
    }

    /// `PlrStats_SetDrivingData(ids, values)` [M 0x199b18]: write the pairs back in.
    pub fn set_from(&mut self, ids: &[u32; 12], values: &[u32; 12]) {
        self.ids = *ids;
        self.values = *values;
    }
}

/// The dev stand-in's chosen outcome for a skipped drive (the real DRIVING.ELF decides on the
/// track; the port cannot run it, so the choice is a knob the report lists).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum DriveResult {
    /// The drive's success path: DRIVING main's exit writes `PTP+0x974 = DAT_0032e6ac` (the next
    /// index), the next key into +0x4f4, and calls `FUN_00300058(0x100000, 1)` [M 0x174bf0:315-323].
    #[default]
    Success,
    /// The abort path: `FUN_00300058(0x100000, 0)` with no next key [M 0x174bf0:316].
    Fail,
}

/// The DRIVING→ACTION return, filled by the dev stand-in for `FUN_00300058` + DRIVING main's
/// exit tail, consumed by [`DriveReturn::apply`] (the `Boot_LoadPTPData` port).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DriveReturn {
    /// PTP+0x97c: the result flag ACTION's `psiLaunchDriving` pre-set to 1 [M 0x120af0], which
    /// `FUN_00300058` overwrites with its `flag` arg [M 0x300058:11].
    pub result: u32,
    /// PTP+0x978: the status byte `Boot_LoadPTPData` maps into `Mission_SetStatus` [M 0x1c8560:
    /// 12-20 — 0 → 6 (complete), 4 → 3, else → 2]. The DRIVING-side writer is not identified
    /// (driving.md §2 lists it open); the dev stand-in writes 0 on Success [S].
    pub status_byte: u32,
    /// PTP+0x974: the next drive index DRIVING main's exit tail writes (`uRam00100974 =
    /// DAT_0032e6ac` [M 0x174bf0:319]).
    pub next_index: u8,
    /// PTP+0x4f4: the next level key [M 0x174bf0:320 `FUN_002a9a50(0x1004f4, 0x32e6b0, 0xf)`].
    pub next_key: &'static str,
    /// PTP+0x154/+0x184: the stats the drive session returns (the port carries them unchanged
    /// through the skipped drive — the dev stand-in draws nothing) [S].
    pub stats: DrivingStats,
}

impl DriveReturn {
    /// The `MissionState` the return maps to [M 0x1c8560:12-20].
    pub fn mission_state(&self) -> u32 {
        match self.status_byte {
            0 => 6,
            4 => 3,
            _ => 2,
        }
    }

    /// The `GameState+0x28` menu code [M 0x1c8560 tail: 0x9000001-with-flag → 0x40000043;
    /// +0x978 == 0 → 0x40000036; 1/2/4 → 0x4000001c; 3/5+ → none]. Returns `None` for the none.
    pub fn game_state_28(&self, base_map: u32, returned_to_base: bool) -> Option<u32> {
        if base_map == 0x0900_0001 && !returned_to_base {
            return Some(0x4000_0043);
        }
        match self.status_byte {
            0 => Some(0x4000_0036),
            1 | 2 | 4 => Some(0x4000_001c),
            _ => None,
        }
    }

    /// The `Boot_LoadPTPData` mirror [M 0x1c8560]: set the mission status, hand the 12 slots to
    /// the new session's stats (`PlrStats_SetDrivingData`). The resume level itself is always
    /// [`RESUME_LEVEL`] [M 0x1c8560 tail] — the caller loads it.
    pub fn apply(&self, mission: &mut crate::mission::Mission, stats: &mut DrivingStats) {
        mission.set_state(self.mission_state()); // [M 0x1c8560:12-20]
        stats.set_from(&self.stats.ids, &self.stats.values); // [M 0x1c8560 `PlrStats_SetDrivingData(0x31c1e4, 0x31c214)`]
    }
}

/// The PTP block: every field the two sides' handover code writes or reads. Offsets are comments
/// on each field [M 0x1c8198 fills / 0x1c8560 mirrors / 0x174bf0 + 0x175a28 + 0x300058 consume].
#[derive(Clone, Debug)]
pub struct PtpBlock {
    /// +0x000..+0x020: 32-byte text copied from 0x2e1ae8 [M 0x1c8198:23 `amemcpy(0x31c090,
    /// 0x2e1ae8, 0x20)`] — the mini-mission name buffer `Menu_RunMiniMission` strcpy's into
    /// [M 0x1ffb90:17-18]; mirrored back by 0x1c8560's `amemcpy(0x2e1ae8, 0x31c090, 0x20)`.
    pub mission_name: [u8; 0x20],
    /// +0x080 ← 0x2e1ae0 [M 0x1c8198:24; mirrored 0x1c8560].
    pub mini_mission_kind: u32,
    /// +0x084 ← 0x2e1b0d (byte) [M 0x1c8198:33; mirrored].
    pub flag_2e1b0d: u8,
    /// +0x088..+0x0a8 ← the DAT_002a38xx cheat/settings bytes, one per slot [M 0x1c8198:25-32;
    /// mirrored 0x1c8560]. Order as written: cc, cb, c9, d2, cd, cf, ce, d4.
    pub cheat_bytes: [u8; 8],
    /// +0x0a4 ← `SFXGetLanguage()` [M 0x1c8198:34; `SFXSetLanguage` on return].
    pub sfx_language: u32,
    /// +0x0ac ← DAT_002a38d6 [M 0x1c8198:43; mirrored].
    pub flag_2a38d6: u32,
    /// +0x0b0/+0x0b4 ← `MPSettings._40_4_`/`_44_4_` [M 0x1c8198:39-40; mirrored].
    pub mp_40: u32,
    pub mp_44: u32,
    /// +0x0b8 ← DAT_002a38ca [M 0x1c8198:41; mirrored].
    pub flag_2a38ca: u32,
    /// +0x0bc ← `DrawInfo` [M 0x1c8198:42; mirrored].
    pub draw_info: u32,
    /// +0x0c0..+0x0e4 ← the MP settings words, written in the order 416/412/408/436/444/456/452/
    /// 448/440/468 [M 0x1c8198:47-58; mirrored].
    pub mp_416: u32,
    pub mp_412: u32,
    pub mp_408: u32,
    pub mp_436: u32,
    pub mp_444: u32,
    pub mp_456: u32,
    pub mp_452: u32,
    pub mp_448: u32,
    pub mp_440: u32,
    pub mp_468: u32,
    /// +0x0e8 ← `SFXMusicGetVolume()` [M 0x1c8198:77; `SFXMusicSetVolume` on return].
    pub music_volume: u32,
    /// +0x0ec ← `SFXGetVolume()` [M 0x1c8198:74; `SFXSetVolume` on return].
    pub sfx_volume: u32,
    /// +0x0f0..+0x128: 56 bytes copied from 0x2e1b50 [M 0x1c8198:60; mirrored].
    pub settings_block: [u8; 0x38],
    /// +0x128 ← `menu_unlock_everything` [M 0x1c8198:35; mirrored].
    pub menu_unlock_everything: u32,
    /// +0x12c ← `CheatInfo` [M 0x1c8198:36; mirrored].
    pub cheat_info: u32,
    /// +0x130 ← DAT_002a3740 [M 0x1c8198:37; mirrored].
    pub flag_2a3740: u32,
    /// +0x134 ← DAT_002a373c [M 0x1c8198:38; mirrored].
    pub flag_2a373c: u32,
    /// +0x138 ← `Menu_GetNightfireStatus()` [M 0x1c8198:44; `Menu_SetNightfireStatus` on return].
    pub nightfire_status: u32,
    /// +0x13c/+0x140 ← `Menu_GetBonus(0)` (hi/lo of the u64) [M 0x1c8198:45-46,55;
    /// `Menu_SetBonus` on return].
    pub bonus_hi: u32,
    pub bonus_lo: u32,
    /// +0x144 ← `DAT_0031ca14` (the handover's mission-state byte: 1 when the base map is
    /// 0x9000001 with DAT_002e1b10 set, else `GameState._40_4_`) [M 0x1c8198:14-21; +0x984 mirrors
    /// it back to `GameState._40_4_` on return via `DAT_0031c1d4`].
    pub mission_state_1: u32,
    /// +0x148 ← DAT_002e1b11 (byte) [M 0x1c8198:59; mirrored].
    pub flag_2e1b11: u8,
    /// +0x14c ← `GameState._8_4_` — the map id [M 0x1c8198:63; `Mission_SetMapHCode` on return].
    pub map_id: u32,
    /// +0x150 ← `GameState[0x52]` (word 82) [M 0x1c8198:57; mirrored].
    pub game_state_52: u32,
    /// +0x154..+0x184: the 12 slot ids [M 0x1c8198:61 `PlrStats_GetDrivingData(0x31c1e4, 0x31c214)`].
    pub driving_ids: [u32; 12],
    /// +0x184..+0x1b4: the 12 slot values [M 0x1c8198:61].
    pub driving_values: [u32; 12],
    /// +0x4b4: the "game ended" flag DRIVING main's exit gates on (`cRam001004b4`, checked by
    /// `FUN_00300058` [M 0x300058:7-10]) — DRIVING-owned, never written by ACTION [M-negative:
    /// no +0x4b4 write in 0x1c8198/0x1c8560].
    pub game_ended: u8,
    /// +0x4f4..+0x503: the level key (15 chars + NUL) [M 0x175a28 reads it for the archive
    /// suffix; DRIVING main writes it from the index before reading it back (0x174bf0:82-190);
    /// the exit writes the NEXT key there (0x174bf0:320)]. ACTION-side 0x31c584 is never touched
    /// by 0x1c8198/0x1c8560 [M-negative — no 0x31c5xx reference in either decompile].
    pub level_key: [u8; KEY_MAX + 1],
    /// +0x540/+0x548: the language file DRIVING main writes ("IDL_ENG.LOC") [D 0x374e38;
    /// M 0x174bf0:191-202].
    pub lang_file: [u8; 0x10],
    /// +0x960: the block-valid magic, written 1 by `Boot_GetPTPData` [M 0x1c8198:65]; DRIVING
    /// main gates the whole retail path on it [M 0x174bf0:68].
    pub valid: u32,
    /// +0x964 ← `SFXGetVolume()/2` — the driving mixer's SFX volume [M 0x1c8198:72-73;
    /// the `Music Volume : Uber(%d) --> Driving(%f)` string [D 0x26fc30] reads it].
    pub driving_sfx_volume: u32,
    /// +0x968 ← `SFXMusicGetVolume()/2` [M 0x1c8198:75-76].
    pub driving_music_volume: u32,
    /// +0x970 ← 1 [M 0x1c8198:65]; DRIVING main maps it to `uRam00100614` (0/2) [M 0x174bf0:203-208].
    pub flag_970: u32,
    /// +0x974: the driving mission index (`GameState._8_4_ - 0x9000000`) [M 0x1c8198:62] —
    /// DRIVING main's switch source [M 0x174bf0:69,82]; the exit overwrites it with the next
    /// index [M 0x174bf0:319].
    pub drive_index: u32,
    /// +0x978: the status byte the return maps into `Mission_SetStatus` [M 0x1c8560:12-20].
    /// No ACTION-side writer [M-negative]; the DRIVING-side writer is unidentified (open).
    pub return_status: u32,
    /// +0x97c: the "came from ACTION" flag `psiLaunchDriving` sets to 1 after the copy
    /// [M 0x120af0]; `FUN_00300058` overwrites it with the exit flag [M 0x300058:11].
    pub result_flag: u32,
    /// +0x984: the status carried in (`DAT_0031ca14`, the same value as +0x144) [M 0x1c8198:15,20];
    /// DRIVING main maps 1→0, 2→1, 3/4→2 into its local and restores it at exit [M 0x174bf0:70-81,
    /// 304-306]; the return mirrors it to `GameState._40_4_` [M 0x1c8560].
    pub mission_state_2: u32,
    /// +0x988 ← `Txt_GetLanguage()` [M 0x1c8198:64; `Txt_SetLanguage` + `LanguageToLoad` on
    /// return].
    pub txt_language: u32,
    /// +0x98c ← DAT_002a38d6 [M 0x1c8198:79; mirrored].
    pub flag_2a38d6_ret: u32,
    /// +0x990 ← DAT_002a3a1e [M 0x1c8198:80; mirrored].
    pub flag_2a3a1e: u32,
    /// +0x994 ← DAT_002a38d1 [M 0x1c8198:82; mirrored].
    pub flag_2a38d1: u32,
    /// +0x998 ← `DAT_002a38d3 == 0` [M 0x1c8198:86; the return writes `DAT_002a38d3 = (+0x998)==0`].
    pub not_2a38d3: u32,
    /// +0x99c ← `DAT_002a38d0 == 0` [M 0x1c8198:78; mirrored likewise].
    pub not_2a38d0: u32,
    /// +0x9a0: the medal bits — `Menu_HasMedal(0x9000001, 3, 0)` sets bit 3 (0x8) [M 0x1c8198:90-96].
    pub medal_bits: u32,
    /// +0xa14 ← DAT_002a38d8 [M 0x1c8198:81; mirrored].
    pub flag_2a38d8: u32,
    /// +0xa18 ← 1, cleared to 0 when the base map is 0x9000001 with DAT_002e1b10 set [M
    /// 0x1c8198:14-21]; the return's `GameState+0x28` = 0x40000043 branch reads it [M 0x1c8560].
    pub base_flag: u32,
    /// +0xa1c ← `psiGetTVType()` [M 0x1c8198:66].
    pub tv_type: u32,
    /// +0xa20 ← 2 [M 0x1c8198:67].
    pub const_2: u32,
    /// +0xa24 ← `PlayerSetting` [M 0x1c8198:69; `PlayerSetting =` on return].
    pub player_setting: u32,
    /// +0xa28 ← `SFXGetMode()` [M 0x1c8198:68; `SFXSetMode` on return].
    pub sfx_mode: u32,
    /// +0xa2c ← DAT_002a37c8 [M 0x1c8198:70; mirrored].
    pub flag_2a37c8: u32,
    /// +0xa30/+0xa34 ← `psiGetScreenPos(&+0xa30, &+0xa34)` [M 0x1c8198:71; `psiSetScreenPos` on
    /// return].
    pub screen_x: i32,
    pub screen_y: i32,
    /// +0xa38 ← 0 [M 0x1c8198:88].
    pub zero_a38: u32,
    /// +0xa3c ← `Menu_GetBestMedal(GameState._8_4_, 0)` [M 0x1c8198:89].
    pub best_medal: u32,
    /// +0xa40..+0xa48 ← DAT_002a38c9/cb/d2 again [M 0x1c8198:83-85; mirrored].
    pub ret_2a38c9: u32,
    pub ret_2a38cb: u32,
    pub ret_2a38d2: u32,
}

impl Default for PtpBlock {
    fn default() -> Self {
        Self::empty()
    }
}

impl PtpBlock {
    /// A zeroed block — `bootup_bootup` 0x1c7e78 memsets `PTPDATA` (0xa50) at boot [M].
    pub fn empty() -> PtpBlock {
        PtpBlock {
            mission_name: [0; 0x20],
            mini_mission_kind: 0,
            flag_2e1b0d: 0,
            cheat_bytes: [0; 8],
            sfx_language: 0,
            flag_2a38d6: 0,
            mp_40: 0,
            mp_44: 0,
            flag_2a38ca: 0,
            draw_info: 0,
            mp_416: 0,
            mp_412: 0,
            mp_408: 0,
            mp_436: 0,
            mp_444: 0,
            mp_456: 0,
            mp_452: 0,
            mp_448: 0,
            mp_440: 0,
            mp_468: 0,
            music_volume: 0,
            sfx_volume: 0,
            settings_block: [0; 0x38],
            menu_unlock_everything: 0,
            cheat_info: 0,
            flag_2a3740: 0,
            flag_2a373c: 0,
            nightfire_status: 0,
            bonus_hi: 0,
            bonus_lo: 0,
            mission_state_1: 0,
            flag_2e1b11: 0,
            map_id: 0,
            game_state_52: 0,
            driving_ids: [0; 12],
            driving_values: [0; 12],
            game_ended: 0,
            level_key: [0; KEY_MAX + 1],
            lang_file: [0; 0x10],
            valid: 0,
            driving_sfx_volume: 0,
            driving_music_volume: 0,
            flag_970: 0,
            drive_index: 0,
            return_status: 0,
            result_flag: 0,
            mission_state_2: 0,
            txt_language: 0,
            flag_2a38d6_ret: 0,
            flag_2a3a1e: 0,
            flag_2a38d1: 0,
            not_2a38d3: 0,
            not_2a38d0: 0,
            medal_bits: 0,
            flag_2a38d8: 0,
            base_flag: 0,
            tv_type: 0,
            const_2: 0,
            player_setting: 0,
            sfx_mode: 0,
            flag_2a37c8: 0,
            screen_x: 0,
            screen_y: 0,
            zero_a38: 0,
            best_medal: 0,
            ret_2a38c9: 0,
            ret_2a38cb: 0,
            ret_2a38d2: 0,
        }
    }

    /// The `Boot_GetPTPData` port [M 0x1c8198]: fill the block from the handover state.
    pub fn build(input: &BuildInputs) -> PtpBlock {
        let mut b = PtpBlock::empty();
        // The base-map special case [M 0x1c8198:14-21]: base 0x9000001 with DAT_002e1b10 set →
        // +0x984 = 1 and +0xa18 = 0; otherwise +0x984 = GameState._40_4_ and +0xa18 = 1.
        let base_special = input.map_id == 0x0900_0001;
        b.mission_state_1 = if base_special { 1 } else { input.game_state_40 };
        b.base_flag = if base_special { 0 } else { 1 };
        b.map_id = input.map_id; // [M 0x1c8198:62-63]
        b.game_state_52 = input.game_state_52; // [M 0x1c8198:57]
        b.mission_state_2 = b.mission_state_1; // [M 0x1c8198:15/20 — +0x984 mirrors +0x144's source]
        // The index [M 0x1c8198:62].
        b.drive_index = driving_index(input.map_id) as u32;
        // The block-valid magic [M 0x1c8198:65].
        b.valid = 1;
        b.flag_970 = 1; // [M 0x1c8198:65]
        b.result_flag = 1; // psiLaunchDriving's post-copy store [M 0x120af0]
        b.const_2 = 2; // [M 0x1c8198:67]
        b.zero_a38 = 0; // [M 0x1c8198:88]
        // The 12 slots [M 0x1c8198:61].
        b.driving_ids = input.stats.ids;
        b.driving_values = input.stats.values;
        // The dev stand-in for the drive itself: DRIVING main's key switch + the exit tail
        // [M 0x174bf0:82-190, 315-323; S — the port does not run DRIVING.ELF].
        b.game_ended = 1; // FUN_00300058's gate [M 0x300058:7-10]
        b.level_key = key_bytes(key_for_index(b.drive_index as u8));
        let ret = drive_exit(input.result, b.drive_index as u8, input.stats);
        b.return_status = ret.status_byte;
        b.result_flag = ret.result;
        b.drive_index = ret.next_index as u32;
        b.level_key = key_bytes(ret.next_key);
        b
    }

    /// The 12 slots as a `DrivingStats` (the `PlrStats_GetDrivingData` view) [M 0x199ac8].
    pub fn driving_stats(&self) -> DrivingStats {
        DrivingStats { ids: self.driving_ids, values: self.driving_values }
    }
}

/// The inputs `PtpBlock::build` needs from the running Sim — the fields the port actually
/// has. Everything else stays 0 (`bootup_bootup`'s memset baseline) with the measured
/// constants set [S for the zeros: the port's profile/menu systems are not wired here].
#[derive(Clone, Copy, Debug, Default)]
pub struct BuildInputs {
    /// `GameState._8_4_` at the handover — the 0x900000X driving id [M 0x1c8198:62-63].
    pub map_id: u32,
    /// `GameState._40_4_` — the handover's mission-state byte (1 on the mini-mission path,
    /// else the flow's value) [M 0x1c8198:14-21].
    pub game_state_40: u32,
    /// `GameState[0x52]` [M 0x1c8198:57].
    pub game_state_52: u32,
    /// The 12 slots to carry (the running session's stats) [M 0x1c8198:61].
    pub stats: DrivingStats,
    /// The dev-chosen drive outcome [S — the port skips the drive].
    pub result: DriveResult,
}

/// Pack a key into the 16-byte PTP+0x4f4 buffer [M 0x174bf0's byte-writes; 0xf-length copies].
fn key_bytes(key: &str) -> [u8; KEY_MAX + 1] {
    let mut out = [0u8; KEY_MAX + 1];
    let bytes = key.as_bytes();
    let n = bytes.len().min(KEY_MAX);
    out[..n].copy_from_slice(&bytes[..n]);
    out
}

/// The DRIVING-side exit stand-in [M 0x174bf0:310-323 + 0x300058]: pick the result flag and the
/// next handover the drive leaves behind. `next_index` = `DAT_0032e6ac` (the dev stand-in keeps
/// the current drive's index — the hub menu re-arms whatever comes next) [S].
pub fn drive_exit(result: DriveResult, index: u8, stats: DrivingStats) -> DriveReturn {
    match result {
        DriveResult::Success => DriveReturn {
            result: 1, // FUN_00300058(0x100000, 1) [M 0x174bf0:322]
            status_byte: 0, // → Mission_SetStatus(6) [S — see the field doc]
            next_index: index,
            next_key: key_for_index(index),
            stats,
        },
        DriveResult::Fail => DriveReturn {
            result: 0, // FUN_00300058(0x100000, 0) [M 0x174bf0:316]
            status_byte: 4, // → Mission_SetStatus(3) [S]
            next_index: index,
            next_key: key_for_index(index),
            stats,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The 8 keys are the ELF's strings in table order [D 0x25c948 + 0x374da8..0x374e28].
    #[test]
    fn key_table_matches_the_game_strings() {
        assert_eq!(DRIVING_KEYS[0], "paris_mis01");
        assert_eq!(DRIVING_KEYS[1], "uw_mis11");
        assert_eq!(DRIVING_KEYS[2], "junglea_mis13a");
        assert_eq!(DRIVING_KEYS[3], "jungleb_mis13b");
        assert_eq!(DRIVING_KEYS[4], "snow1a_mis3");
        assert_eq!(DRIVING_KEYS[5], "snow2a_mis4");
        assert_eq!(DRIVING_KEYS[6], "junglec_mis13c");
        assert_eq!(DRIVING_KEYS[7], "snow2a_race");
        assert_eq!(key_for_index(0), "paris_map"); // the switch's default [M 0x174bf0:180]
        assert_eq!(key_for_index(9), "paris_map");
    }

    /// The archive suffix rule [M 0x175a28 — everything after the last `_`].
    #[test]
    fn key_suffix_is_what_driving_opens() {
        assert_eq!(key_archive_suffix("paris_mis01"), "mis01");
        assert_eq!(key_archive_suffix("snow2a_mis4"), "mis4");
        assert_eq!(key_archive_suffix("snow2a_race"), "race");
        assert_eq!(key_archive_suffix("paris_map"), "map");
    }

    /// The index math [M 0x1c8198:62 — `GameState._8_4_ - 0x9000000`], byte-masked DRIVING-side.
    #[test]
    fn index_is_map_id_minus_base() {
        assert_eq!(driving_index(0x0900_0001), 1);
        assert_eq!(driving_index(0x0900_0008), 8);
        // A non-driving id wraps; DRIVING masks to a byte [M 0x174bf0:69].
        assert_eq!(driving_index(0x0700_0004), 0x04);
    }

    /// `Mission_Update` 0x182c60 case 6's measured pairs [M].
    #[test]
    fn interlude_map_matches_mission_update() {
        assert_eq!(drive_interlude_for(0x0700_0004), Some(0x0730_0004));
        assert_eq!(drive_interlude_for(0x0700_0008), Some(0x0730_0008));
        assert_eq!(drive_interlude_for(0x0700_000b), Some(0x0730_000b));
        assert_eq!(drive_interlude_for(0x0700_000d), Some(0x0730_000d));
        assert_eq!(drive_interlude_for(0x0700_000f), Some(0x0730_000f));
        assert_eq!(drive_interlude_for(0x0700_0013), Some(0x0730_0013));
        assert_eq!(drive_interlude_for(0x0700_0016), Some(0x0730_0017));
        assert_eq!(drive_interlude_for(0x0700_001b), Some(0x0730_001b));
        assert_eq!(drive_interlude_for(0x0700_0002), None);
        assert!(is_driving_interlude(0x0730_0004));
        assert!(!is_driving_interlude(0x0700_0004));
    }

    /// The block built from a known state: the measured field values [M 0x1c8198].
    #[test]
    fn build_fills_the_measured_fields() {
        let stats = DrivingStats { ids: [0; 12], values: [7, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0] };
        let input = BuildInputs {
            map_id: 0x0900_0001,
            game_state_40: 0x55,
            game_state_52: 0x1234,
            stats,
            result: DriveResult::Success,
        };
        let b = PtpBlock::build(&input);
        // The magic + the flags [M 0x1c8198:65,67,88].
        assert_eq!(b.valid, 1);
        assert_eq!(b.flag_970, 1);
        assert_eq!(b.const_2, 2);
        assert_eq!(b.zero_a38, 0);
        // The map id and the index [M 0x1c8198:62-63].
        assert_eq!(b.map_id, 0x0900_0001);
        assert_eq!(b.drive_index, 1);
        // The base-map special case: +0x984 = 1, +0xa18 = 0 [M 0x1c8198:14-17].
        assert_eq!(b.mission_state_1, 1);
        assert_eq!(b.mission_state_2, 1);
        assert_eq!(b.base_flag, 0);
        // psiLaunchDriving's flag [M 0x120af0] — later overwritten by the exit stand-in.
        assert_eq!(b.result_flag, 1);
        // The slots [M 0x1c8198:61].
        assert_eq!(b.driving_values[0], 7);
        assert_eq!(b.driving_stats(), stats);
        // The key DRIVING main writes from index 1 [M 0x174bf0:82-93].
        assert_eq!(b.level_key, key_bytes("paris_mis01"));
        // The exit stand-in: result 1, the next key still this drive's [M 0x174bf0:315-323].
        assert_eq!(b.return_status, 0);
        assert_eq!(b.game_ended, 1);
    }

    /// The non-base handover: +0x984 = `GameState._40_4_`, +0xa18 = 1 [M 0x1c8198:18-21].
    #[test]
    fn build_non_base_handover_carries_game_state_40() {
        let input = BuildInputs {
            map_id: 0x0900_0005,
            game_state_40: 0x42,
            game_state_52: 0,
            stats: DrivingStats::default(),
            result: DriveResult::Success,
        };
        let b = PtpBlock::build(&input);
        assert_eq!(b.mission_state_1, 0x42);
        assert_eq!(b.base_flag, 1);
        assert_eq!(b.drive_index, 5);
        assert_eq!(b.level_key, key_bytes("snow1a_mis3"));
    }

    /// The return mapping [M 0x1c8560:12-20 + tail].
    #[test]
    fn return_maps_the_status_byte() {
        let mut ret = drive_exit(DriveResult::Success, 1, DrivingStats::default());
        assert_eq!(ret.result, 1);
        assert_eq!(ret.mission_state(), 6); // 0 → 6
        assert_eq!(ret.game_state_28(0x0900_0001, true), Some(0x4000_0036)); // 0 → 0x40000036
        ret.status_byte = 4;
        assert_eq!(ret.mission_state(), 3); // 4 → 3
        assert_eq!(ret.game_state_28(0x0700_0048, false), Some(0x4000_001c)); // 4 → 0x4000001c
        ret.status_byte = 2;
        assert_eq!(ret.mission_state(), 2); // else → 2
        assert_eq!(ret.game_state_28(0x0700_0048, false), Some(0x4000_001c)); // 2 → 0x4000001c
        ret.status_byte = 3;
        assert_eq!(ret.mission_state(), 2);
        assert_eq!(ret.game_state_28(0x0700_0048, false), None); // 3 → none
        // The 0x9000001 base-with-flag branch [M 0x1c8560's first if].
        assert_eq!(ret.game_state_28(0x0900_0001, false), Some(0x4000_0043));
        // The abort path [M 0x174bf0:316].
        let fail = drive_exit(DriveResult::Fail, 6, DrivingStats::default());
        assert_eq!(fail.result, 0);
        assert_eq!(fail.mission_state(), 3);
        assert_eq!(fail.next_key, "snow2a_mis4");
    }

    /// The stats round-trip: Get fills the PTP arrays, Set restores them [M 0x199ac8/0x199b18].
    #[test]
    fn driving_stats_round_trip() {
        let mut s = DrivingStats::default();
        assert_eq!(s, DrivingStats::default()); // [L ee_start_05.bin — all zero at boot]
        let mut ids = [0u32; 12];
        let mut values = [0u32; 12];
        s.values[3] = 0x2a;
        s.get_into(&mut ids, &mut values);
        assert_eq!(values[3], 0x2a);
        let mut restored = DrivingStats::default();
        restored.set_from(&ids, &values);
        assert_eq!(restored, s);
        // Through the block: build → apply (the Boot_GetPTPData → Boot_LoadPTPData round trip).
        let input = BuildInputs {
            map_id: 0x0900_0001,
            game_state_40: 1,
            game_state_52: 0,
            stats: s,
            result: DriveResult::Success,
        };
        let b = PtpBlock::build(&input);
        let ret = drive_exit(DriveResult::Success, b.drive_index as u8, b.driving_stats());
        let mut mission = crate::mission::Mission::new();
        let mut carried = DrivingStats::default();
        ret.apply(&mut mission, &mut carried);
        assert_eq!(mission.status(), 6); // [M 0x1c8560:12-20]
        assert_eq!(carried.values[3], 0x2a); // [M 0x1c8560's PlrStats_SetDrivingData]
    }

    /// The resume target is the front end for every drive [M 0x1c8560 tail].
    #[test]
    fn resume_is_always_the_front_end() {
        for i in 1u8..=8 {
            let ret = drive_exit(DriveResult::Success, i, DrivingStats::default());
            let mut mission = crate::mission::Mission::new();
            let mut stats = DrivingStats::default();
            ret.apply(&mut mission, &mut stats);
            // The caller's load target [M 0x1c8560 `ResetMap_LevelToLoad(0x7000048, 0, 0)`].
            assert_eq!(RESUME_LEVEL, 0x0700_0048);
            assert_eq!(mission.status(), 6);
            assert_eq!(ret.next_key, DRIVING_KEYS[(i - 1) as usize]);
        }
    }
}
