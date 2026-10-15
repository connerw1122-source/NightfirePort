//! The codename profile save (`fix-save-profile`): the game's PLRS / MPSG / GSET / CHET blocks
//! around the progress layer's MSSN / BNUS, one file per codename.
//!
//! Evidence (all in docs/reports/fix-save-profile.md): the block table `SaveIFFBlocks` 0x2c74d8
//! holds 6 records {mask, id, make, load} in chain order PLRS, MSSN, MPSG, GSET, CHET, BNUS
//! [L ee_start_07000001.bin @ 0x2c74d8; NumSaveBlocks 0x30cf10 = 6], and the game's own loaded
//! chain sits in `InBufData` 0x3416e0 in every ee_start snapshot [L]: PLRS 11 B, MSSN 68 B,
//! MPSG 13 B, GSET 905 B, CHET 9 B, BNUS 16 B — 1022 bytes. The bit packer is the progress
//! layer's LSB-first `BIN_PushBits` 0x1e8b88. The MC file name IS the codename string
//! [M Menu_UpdateMessageBox 0x1f7de8:19/28 passes the name buffer 0x2e1ae8 to LS_Load/LS_Save].

use crate::progress::Progress;

/// `'PLRS'` [L SaveIFFBlocks[0]+4 = 0x53524c50].
pub const ID_PLRS: u32 = 0x5352_4C50;
/// `'MSSN'` (the mission-progress block — encoded by `progress.rs`) [L SaveIFFBlocks[1]+4].
pub const ID_MSSN: u32 = 0x4E53_534D;
/// `'MPSG'` [L SaveIFFBlocks[2]+4 = 0x4753504d].
pub const ID_MPSG: u32 = 0x4753_504D;
/// `'GSET'` [L SaveIFFBlocks[3]+4 = 0x54455347].
pub const ID_GSET: u32 = 0x5445_5347;
/// `'CHET'` [L SaveIFFBlocks[4]+4 = 0x54454843].
pub const ID_CHET: u32 = 0x5445_4843;
/// `'BNUS'` (the bonus/medal block — encoded by `progress.rs`) [L SaveIFFBlocks[5]+4 = 0x53554e42].
pub const ID_BNUS: u32 = 0x5355_4E42;

/// The chain order the game concatenates [M LS_GetSaveStuff 0x1cd0d8 walks SaveIFFBlocks in
/// order; the snapshot's own chain reads PLRS, MSSN, MPSG, GSET, CHET, BNUS [L InBufData]].
pub const CHAIN: [u32; 6] = [ID_PLRS, ID_MSSN, ID_MPSG, ID_GSET, ID_CHET, ID_BNUS];

/// The game's LSB-first bit packer (`BIN_PushBits` 0x1e8b88): value bit i → stream bit (pos+i).
/// Mirrors `progress.rs`'s private helper (same algorithm, proven against the game's own MSSN).
fn push_bits(buf: &mut [u8], pos: &mut usize, value: u64, n: u32) {
    for i in 0..n {
        if (value >> i) & 1 == 1 {
            let k = *pos + i as usize;
            buf[k >> 3] |= 1 << (k & 7);
        }
    }
    *pos += n as usize;
}

/// `BIN_PullBits` 0x1e8db0, the mirror.
fn pull_bits(buf: &[u8], pos: &mut usize, n: u32) -> u64 {
    let mut v = 0u64;
    for i in 0..n {
        let k = *pos + i as usize;
        if k >> 3 < buf.len() && buf[k >> 3] & (1 << (k & 7)) != 0 {
            v |= 1 << i;
        }
    }
    *pos += n as usize;
    v
}

/// One save block: `{u32 id; u32 size; payload}` with the size counting the 8-byte header
/// [M LS_MakePlrSettings 0x1cd418 writes the id at 0x33b6e0 and `(bits+7)>>3` at 0x33b6e4;
/// the snapshot's PLRS block is 11 = 8 + 3 payload bytes [L]].
fn make_block(id: u32, payload_bits: usize, fill: impl FnOnce(&mut [u8], &mut usize)) -> Vec<u8> {
    let size = (64 + payload_bits + 7) / 8;
    let mut buf = vec![0u8; size];
    let mut pos = 64usize; // the header is bytes 0..8
    fill(&mut buf, &mut pos);
    buf[0..4].copy_from_slice(&id.to_le_bytes());
    buf[4..8].copy_from_slice(&(size as u32).to_le_bytes());
    buf
}

/// The per-codename player options — `PlayerSetting[slot]` (0x2a38c8, stride 0x158, 4 slots;
/// the symbol spans exactly 4×0x158 = 1376 bytes [M action-symtab]). Only the fields the game's
/// `LS_MakePlrSettings` 0x1cd418 pushes are modelled, in its push order (the decode of the game's
/// own chain matches `Input_Init` 0x17edd8's defaults field for field [L InBufData PLRS
/// 00 f2 01]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlayerOptions {
    /// +0x00 — the slot-exists flag the pause handlers test [M P_PAUSE_Handler 0x203c28:80,
    /// C_GCPAUSE_Handler 0x204408:512]. Save bit 1.
    pub exists: bool,
    /// +0x0e — the controller style [M Input_ChangeControllerStyle 0x17f5f0 writes it,
    /// Input_Update 0x17ee88 reads it as a short (== 2 / == 5 arms)]. Save bits 2-5 (4 bits).
    pub controller_style: u16,
    /// +0x10 — the second style word; only the PTP hand-off reads it in ACTION.ELF
    /// [M Boot_GetPTPData 0x1c8198:81 → DRIVING.ELF]. Save bits 6-9 (4 bits).
    pub ptp_option: u16,
    /// +0x01 — auto aim [M Player_AutoAim 0x1a7c98:41 reads it when MPSettings+0x180 == 0].
    pub auto_aim: bool,
    /// +0x02 — the auto-aim variant [M Player_AutoAim 0x1a7c98:44 reads it otherwise].
    pub auto_aim_alt: bool,
    /// +0x03 — gates the manual weapon action (action 0x13) [M Player_Weapon 0x1a24c8:45;
    /// the reload-vs-manual semantics are [S]].
    pub reload_manual: bool,
    /// +0x0a — equip the better weapon on pickup [M Player_EquipWeapon 0x1a8a58:76].
    pub auto_equip_best: bool,
    /// +0x04 — crouch toggle vs hold [M Player_SSCrouch 0x1924c0:85 picks action mode 1 or 4].
    pub crouch_toggle: bool,
    /// +0x09 — rumble enabled [M Input_RumbleStart 0x17f6b8:9 / Input_RumbleStop 0x17f748:9].
    pub rumble: bool,
    /// +0x08 — crosshair shown [M HUD_UpdateCrossHair 0x19e6d0:96].
    pub crosshair: bool,
    /// +0x0c — a 2-bit enum with no ACTION.ELF reader beyond Input_Init's default 1
    /// [M 0x17edd8:14] and the PTP hand-off [M 0x1c8198:32]. Save bits 16-17.
    pub aux_style: u8,
    /// +0x0b — gates a decaying per-tick timer (1.0 − 0.0020833·MUL) in Player_Update
    /// [M 0x193600:414; the timer's meaning is [S]]. Save bit 18.
    pub decay_gate: bool,
}

impl PlayerOptions {
    /// `Input_Init` 0x17edd8's defaults for one slot (memset 0, then these writes) — also the
    /// decode of the game's own chain [L PLRS payload 00 f2 01].
    pub fn input_init_defaults() -> Self {
        PlayerOptions {
            exists: false,
            controller_style: 0,
            ptp_option: 0,
            auto_aim: true,
            auto_aim_alt: false,
            reload_manual: false,
            auto_equip_best: true,
            crouch_toggle: true,
            rumble: true,
            crosshair: true,
            aux_style: 1,
            decay_gate: false,
        }
    }

    /// `LS_MakePlrSettings` 0x1cd418's push order — 19 bits → 3 payload bytes → an 11-byte block
    /// [L the snapshot's PLRS size].
    fn make(&self) -> Vec<u8> {
        make_block(ID_PLRS, 19, |buf, pos| {
            push_bits(buf, pos, self.exists as u64, 1);
            push_bits(buf, pos, self.controller_style as u64, 4);
            push_bits(buf, pos, self.ptp_option as u64, 4);
            push_bits(buf, pos, self.auto_aim as u64, 1);
            push_bits(buf, pos, self.auto_aim_alt as u64, 1);
            push_bits(buf, pos, self.reload_manual as u64, 1);
            push_bits(buf, pos, self.auto_equip_best as u64, 1);
            push_bits(buf, pos, self.crouch_toggle as u64, 1);
            push_bits(buf, pos, self.rumble as u64, 1);
            push_bits(buf, pos, self.crosshair as u64, 1);
            push_bits(buf, pos, self.aux_style as u64, 2);
            push_bits(buf, pos, self.decay_gate as u64, 1);
        })
    }

    /// `LS_LoadPlrSettings` 0x1cd5b8 (the same field order; the decompile's byte/element scaling
    /// disagrees between the pair — the coherent reading, confirmed by the game's own chain
    /// decode, is one 0x158-byte `PlayerSetting[slot]` record).
    fn load(&mut self, payload: &[u8], pos: &mut usize) {
        self.exists = pull_bits(payload, pos, 1) != 0;
        self.controller_style = pull_bits(payload, pos, 4) as u16;
        self.ptp_option = pull_bits(payload, pos, 4) as u16;
        self.auto_aim = pull_bits(payload, pos, 1) != 0;
        self.auto_aim_alt = pull_bits(payload, pos, 1) != 0;
        self.reload_manual = pull_bits(payload, pos, 1) != 0;
        self.auto_equip_best = pull_bits(payload, pos, 1) != 0;
        self.crouch_toggle = pull_bits(payload, pos, 1) != 0;
        self.rumble = pull_bits(payload, pos, 1) != 0;
        self.crosshair = pull_bits(payload, pos, 1) != 0;
        self.aux_style = pull_bits(payload, pos, 2) as u8;
        self.decay_gate = pull_bits(payload, pos, 1) != 0;
    }
}

/// The multiplayer options — `MPSettings[slot]` (0x2a47a0, stride 0x30) [M symtab; the slot
/// stride from LS_MakeMPSettings 0x1ce508's `param_2 * 0x30`]. Only the two saved fields.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MpOptions {
    /// +0x28 — the MP radar toggle [M MP_GetRadarObjects 0x189840:22 / HUD_RadarUpdate
    /// 0x19f6e8:102 test it]. Save bit 1 [L the snapshot's MPSG payload starts 01].
    pub radar: bool,
    /// +0x2c — extra spawn health [M Player_Init 0x1910f8:129 / MP_ReSpawn 0x185368:72 /
    /// HUD_UpdateMPHealthPane 0x19b630:69 add it to 100.0]. Save bits 2-33 (32 bits).
    pub health_extra: u32,
}

impl MpOptions {
    /// The boot defaults [M bootup 0x1c7e78:50-52 writes +0x28 = 1, +0x2c = 0] = the template's
    /// values [L def-codename template 0 +0x1a = 1, +0x1c = 0].
    pub fn defaults() -> Self {
        MpOptions { radar: true, health_extra: 0 }
    }

    /// `LS_MakeMPSettings` 0x1ce508: 1 + 32 bits → a 13-byte block [L the snapshot's MPSN size].
    fn make(&self) -> Vec<u8> {
        make_block(ID_MPSG, 33, |buf, pos| {
            push_bits(buf, pos, self.radar as u64, 1);
            push_bits(buf, pos, self.health_extra as u64, 32);
        })
    }

    /// `LS_LoadMPSettings` 0x1ce5f0.
    fn load(&mut self, payload: &[u8], pos: &mut usize) {
        self.radar = pull_bits(payload, pos, 1) != 0;
        self.health_extra = pull_bits(payload, pos, 32) as u32;
    }
}

/// The global (per-codename) options — the GSET block [`LS_MakeGlobalSettings` 0x1cd9d0].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GlobalOptions {
    /// Music volume [M SFXMusicGetVolume / the options page]. Save bits 1-7 (7 bits).
    pub music_volume: u32,
    /// SFX volume [M SFXGetVolume]. Save bits 8-14 (7 bits).
    pub sfx_volume: u32,
    /// Language [M Txt_GetLanguage]. Save bits 15-21 (7 bits).
    pub language: u32,
    /// Subtitles [M GameState+0x60 = 0x2a37c8 — Sound_DoSubtitle 0x1dde58:21 gates on it,
    /// Menu_WillRunMiniMission 0x202450 sets it, bootup 0x1c7e78:34 clears it]. Save bit 22.
    pub subtitles: bool,
    /// The audio output mode [M SFXGetMode/SFXSetMode]. Save bits 23-54 (32 bits).
    pub sfx_mode: u32,
    /// Widescreen [M GameState+0x5c = 0x2a37c4 — C_CHCHWS_Handler 0x216168:9 sets it and copies
    /// it to switch_CONFIG_WIDESCREEN; the AV options page writes it 0x214818:134]. Save bit 55.
    pub widescreen: bool,
    /// The draw-info word [M the global `DrawInfo` 0x2a37c0; the template ships 2 [L]].
    /// Save bits 56-87 (32 bits).
    pub draw_info: u32,
    /// Screen position X/Y [M psiGetScreenPos/psiSetScreenPos around the push]. Save bits 88-151.
    pub screen_x: i32,
    pub screen_y: i32,
    /// The trailing version tag: the game pushes the constant 4 as the last 16 bits
    /// [M 0x1cd9d0's final `BIN_PushBits(…, 4, 0x10)`].
    pub version: u16,
}

impl GlobalOptions {
    /// The "New Codename" template's values [L def-codename template 0 (0x2e1b18): music 0x46,
    /// sfx 0x64, subtitles/game-complete bytes 0, draw_info byte 2] with the runtime fields at
    /// the game's own chain values [L GSET payload: music 70, sfx 100, language 6, screen 0,0].
    pub fn defaults() -> Self {
        GlobalOptions {
            music_volume: 70,
            sfx_volume: 100,
            language: 6,
            subtitles: false,
            sfx_mode: 0,
            widescreen: false,
            draw_info: 2,
            screen_x: 0,
            screen_y: 0,
            version: 4,
        }
    }

    /// `LS_MakeGlobalSettings` 0x1cd9d0: 7+7+7+1+32+1+32+32+32 + 219 zero dwords + 16 bits =
    /// 7175 payload bits → a 905-byte block [L the snapshot's GSET size].
    fn make(&self) -> Vec<u8> {
        make_block(ID_GSET, 151 + 219 * 32 + 16, |buf, pos| {
            push_bits(buf, pos, self.music_volume as u64, 7);
            push_bits(buf, pos, self.sfx_volume as u64, 7);
            push_bits(buf, pos, self.language as u64, 7);
            push_bits(buf, pos, self.subtitles as u64, 1);
            push_bits(buf, pos, self.sfx_mode as u64, 32);
            push_bits(buf, pos, self.widescreen as u64, 1);
            push_bits(buf, pos, self.draw_info as u64, 32);
            push_bits(buf, pos, self.screen_x as u32 as u64, 32);
            push_bits(buf, pos, self.screen_y as u32 as u64, 32);
            for _ in 0..219 {
                push_bits(buf, pos, 0, 32); // the game's zero-filled tail [M 0x1cd9d0's do-loop]
            }
            push_bits(buf, pos, self.version as u64, 16);
        })
    }

    /// `LS_LoadGlobalSettings` 0x1cdbc8 (applies SFXSetMode/Music/Volume at the end — the port
    /// keeps the values for the audio owner to read).
    fn load(&mut self, payload: &[u8], pos: &mut usize) {
        self.music_volume = pull_bits(payload, pos, 7) as u32;
        self.sfx_volume = pull_bits(payload, pos, 7) as u32;
        self.language = pull_bits(payload, pos, 7) as u32;
        self.subtitles = pull_bits(payload, pos, 1) != 0;
        self.sfx_mode = pull_bits(payload, pos, 32) as u32;
        self.widescreen = pull_bits(payload, pos, 1) != 0;
        self.draw_info = pull_bits(payload, pos, 32) as u32;
        self.screen_x = pull_bits(payload, pos, 32) as u32 as i32;
        self.screen_y = pull_bits(payload, pos, 32) as u32 as i32;
        for _ in 0..219 {
            pull_bits(payload, pos, 32);
        }
        self.version = pull_bits(payload, pos, 16) as u16;
    }
}

/// The cheats — the CHET block's three one-bit flags at CheatInfo 0x2a3738 (+4, +8)
/// [M symtab: CheatInfo = 0x2a3738, 48 bytes]. `Menu_MapDefaultCodename` 0x1f7fa8 zeroes all
/// three whenever a default codename maps in.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Cheats {
    /// 0x2a3738 — the `CheatInfo` flag itself.
    pub cheat_info: bool,
    /// 0x2a373c — read by Player_InitAmmoWeapons 0x1a17f8:20/35 [S: the ammo cheat].
    pub cheat_ammo: bool,
    /// 0x2a3740 — read by Player_RoundToFire 0x1a18e0:24 [S: the fire-rate cheat].
    pub cheat_fire: bool,
}

impl Cheats {
    /// `LS_MakeCheats` 0x1ce260: 3 bits → a 9-byte block [L the snapshot's CHET size].
    fn make(&self) -> Vec<u8> {
        make_block(ID_CHET, 3, |buf, pos| {
            push_bits(buf, pos, self.cheat_info as u64, 1);
            push_bits(buf, pos, self.cheat_ammo as u64, 1);
            push_bits(buf, pos, self.cheat_fire as u64, 1);
        })
    }

    /// `LS_LoadCheats` 0x1ce340.
    fn load(&mut self, payload: &[u8], pos: &mut usize) {
        self.cheat_info = pull_bits(payload, pos, 1) != 0;
        self.cheat_ammo = pull_bits(payload, pos, 1) != 0;
        self.cheat_fire = pull_bits(payload, pos, 1) != 0;
    }
}

/// Why a codename was rejected — `Menu_IsValidCodename` 0x1f9930's returns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CodenameError {
    /// 0x3bc — the name is empty (or all spaces) [M 0x1f9930's final return].
    Empty,
    /// Longer than the 32-byte copy the validator makes [M acStack_a0[32] / the MC enum's
    /// strlen < 0x21 gate in Menu_EnumSaves 0x1f7b78].
    TooLong,
    /// 0x2c0 — the name contains a default codename's text [M 0x1f9930's strstr loop over
    /// def_codename 0x2e1b18's two labels].
    Reserved,
    /// The file for this codename already exists (the game asks MemCard_ASK_OVERWRITE 0x10ee38
    /// [M]; the port refuses — the host decides).
    Exists,
    /// No profiles dir is set (no memory card in the port).
    NoCard,
    /// The write failed.
    Io,
}

/// A codename profile: the options blocks around the progress layer's data. The MC file name is
/// the codename [M Menu_UpdateMessageBox 0x1f7de8:19/28]; the port stores `<dir>/<codename>.profile`.
pub struct Profile {
    /// The codename (the file stem, uppercased like the game's validator [M 0x1f9930 strupr]).
    pub name: String,
    pub plr: PlayerOptions,
    pub mpsg: MpOptions,
    pub gset: GlobalOptions,
    pub chet: Cheats,
    /// The MSSN / BNUS payload (unlocks, best scores, moments, medal bits, game complete).
    pub progress: Progress,
}

impl Profile {
    /// The "New Codename" defaults: `Menu_MapDefaultCodename` 0x1f7fa8 maps def-codename
    /// template 0 (0x2e1b18 [L: nightfire 0 → Menu_SetNightfireStatus 0x2023a0's 0x3, bonus 0,
    /// music 70, sfx 100, the Input_Init-matching option bytes]) onto the slot, resets the
    /// cheats, and binds def_codename[0] (Txt 0x29b) as the name proposal [L the name buffer
    /// 0x2e1ae8 holds "BOND" at the snapshot].
    pub fn default_codename(name: &str) -> Self {
        let mut progress = Progress::default();
        progress.set_nightfire_status(0); // [M 0x2023a0: a 0 status defaults to 0x3]
        Profile {
            name: name.to_string(),
            plr: {
                let mut plr = PlayerOptions::input_init_defaults();
                // The template's own bytes [L 0x2e1b18+0x1e..+0x2d]: exists 0, style 7, ptp 0,
                // auto_aim 1, alt 0, reload_manual 0, crouch 1, rumble 1, crosshair 1,
                // aux 1, decay 0 — only the style differs from Input_Init's 0.
                plr.controller_style = 7;
                plr
            },
            mpsg: MpOptions::defaults(),
            gset: GlobalOptions::defaults(),
            chet: Cheats::default(), // [M 0x1f7fa8's trailing CheatInfo/373c/3740 = 0]
            progress,
        }
    }

    /// The full chain in `SaveIFFBlocks` order + the CRC-32 trailer (the same file convention as
    /// `Progress::encode_save`; the LS-layer buffer itself is the bare chain [L InBufData]).
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = self.plr.make();
        buf.extend_from_slice(&self.progress.make_mssn());
        buf.extend_from_slice(&self.mpsg.make());
        buf.extend_from_slice(&self.gset.make());
        buf.extend_from_slice(&self.chet.make());
        buf.extend_from_slice(&self.progress.make_buns());
        let crc = nf_formats::png::crc32(0, &buf);
        buf.extend_from_slice(&crc.to_le_bytes());
        buf
    }

    /// Parse a file: verify the CRC trailer, walk the chain, apply every block. All 6 blocks are
    /// required (the game always writes mask 0x3f — all of SaveIFFBlocks [M LS_Save 0x1ce0c0's
    /// callers pass the full mask; the snapshot's chain carries all 6 [L]]).
    pub fn decode(&mut self, file: &[u8]) -> Result<(), &'static str> {
        if file.len() < 4 {
            return Err("short");
        }
        let (body, crc_bytes) = file.split_at(file.len() - 4);
        let crc = u32::from_le_bytes(crc_bytes.try_into().unwrap());
        if nf_formats::png::crc32(0, body) != crc {
            return Err("crc mismatch");
        }
        let mut saw = [false; 6];
        let mut off = 0usize;
        while off + 8 <= body.len() {
            let id = u32::from_le_bytes(body[off..off + 4].try_into().unwrap());
            let size = u32::from_le_bytes(body[off + 4..off + 8].try_into().unwrap()) as usize;
            if id == 0 || size < 8 || off + size > body.len() {
                break;
            }
            let payload = &body[off..off + size];
            let mut pos = 64usize;
            match id {
                ID_PLRS => self.plr.load(payload, &mut pos),
                ID_MSSN => self.progress.load_mssn(payload, &mut pos),
                ID_MPSG => self.mpsg.load(payload, &mut pos),
                ID_GSET => self.gset.load(payload, &mut pos),
                ID_CHET => self.chet.load(payload, &mut pos),
                ID_BNUS => self.progress.load_buns(payload, &mut pos),
                _ => {}
            }
            if let Some(i) = CHAIN.iter().position(|&c| c == id) {
                saw[i] = true;
            }
            off += size;
        }
        for (i, &id) in CHAIN.iter().enumerate() {
            if !saw[i] {
                return Err("missing block");
            }
            let _ = id;
        }
        Ok(())
    }

    /// The file path for a codename: `<dir>/<codename>.profile` — one file per codename, the
    /// game's own naming (the MC file IS the codename [M 0x1f7de8]).
    pub fn path_in(dir: &std::path::Path, name: &str) -> std::path::PathBuf {
        dir.join(format!("{}.profile", name))
    }

    /// Write this profile next to the port's config (the profiles dir), creating the dir.
    pub fn save_to_dir(&self, dir: &std::path::Path) -> std::io::Result<std::path::PathBuf> {
        std::fs::create_dir_all(dir)?;
        let path = Self::path_in(dir, &self.name);
        std::fs::write(&path, self.encode())?;
        Ok(path)
    }

    /// Read one profile file.
    pub fn load_file(path: &std::path::Path) -> Result<Self, String> {
        let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
        let name = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        let mut p = Profile::default_codename(&name);
        p.decode(&bytes).map_err(|e| e.to_string())?;
        Ok(p)
    }

    /// List the saved codenames in a profiles dir (the port's memory-card enumeration —
    /// `Menu_EnumSaves` 0x1f7b78's psiStartSaveEnum/psiGetNextSave walk, name < 0x21 chars).
    pub fn list_dir(dir: &std::path::Path) -> Vec<String> {
        let mut out = Vec::new();
        if let Ok(rd) = std::fs::read_dir(dir) {
            for e in rd.flatten() {
                let p = e.path();
                if p.extension().map_or(true, |x| x != "profile") {
                    continue;
                }
                if let Some(stem) = p.file_stem() {
                    let name = stem.to_string_lossy().to_string();
                    if !name.is_empty() && name.len() < 0x21 {
                        out.push(name);
                    }
                }
            }
        }
        out.sort();
        out
    }

    /// `Menu_IsValidCodename` 0x1f9930: trim spaces (leading and trailing [M the scan loops]),
    /// uppercase the stored name [M the strupr + write-back], reject the empty name (0x3bc),
    /// names over the 32-byte copy (TooLong), and names containing a default codename's text
    /// (0x2c0). `reserved` = the two def_codename labels resolved through the language bank.
    pub fn check_codename(name: &str, reserved: &[&str]) -> Result<String, CodenameError> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(CodenameError::Empty);
        }
        if trimmed.len() >= 32 {
            return Err(CodenameError::TooLong);
        }
        let upper = trimmed.to_uppercase();
        for r in reserved {
            if upper.contains(&r.to_uppercase()) {
                return Err(CodenameError::Reserved);
            }
        }
        Ok(upper)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The game's own chain [L InBufData 0x3416e0 in work/ee_snapshot.bin]: the six blocks'
    /// sizes and the field values of a real profile. Skipped when the snapshot isn't present.
    #[test]
    fn decodes_the_games_own_chain() {
        let Ok(snap) = std::fs::read("../work/ee_snapshot.bin")
            .or_else(|_| std::fs::read("../../work/ee_snapshot.bin"))
        else {
            eprintln!("no ee_snapshot.bin — skipping the [L] cross-check");
            return;
        };
        let chain = &snap[0x3416e0..0x3416e0 + 1022];
        let mut file = chain.to_vec();
        file.extend_from_slice(&nf_formats::png::crc32(0, chain).to_le_bytes());
        let mut p = Profile::default_codename("X");
        p.decode(&file).expect("the game's own chain must decode");
        // PLRS [L payload 00 f2 01] == Input_Init's defaults, style 0:
        assert!(!p.plr.exists);
        assert_eq!(p.plr.controller_style, 0);
        assert_eq!(p.plr.ptp_option, 0);
        assert!(p.plr.auto_aim && !p.plr.auto_aim_alt && !p.plr.reload_manual);
        assert!(p.plr.auto_equip_best && p.plr.crouch_toggle);
        assert!(p.plr.rumble && p.plr.crosshair);
        assert_eq!(p.plr.aux_style, 1);
        assert!(!p.plr.decay_gate);
        // MPSG [L payload 01 00 00 00 00]:
        assert!(p.mpsg.radar);
        assert_eq!(p.mpsg.health_extra, 0);
        // GSET [L]: music 70, sfx 100, language 6, screen 0,0, version tag 4.
        assert_eq!(p.gset.music_volume, 70);
        assert_eq!(p.gset.sfx_volume, 100);
        assert_eq!(p.gset.language, 6);
        assert!(!p.gset.subtitles && !p.gset.widescreen);
        assert_eq!(p.gset.screen_x, 0);
        assert_eq!(p.gset.screen_y, 0);
        assert_eq!(p.gset.version, 4);
        // CHET [L payload 00]: no cheats.
        assert!(!p.chet.cheat_info && !p.chet.cheat_ammo && !p.chet.cheat_fire);
        // BNUS [L payload 40 00 00 00 00 00 00 04]: the bonus word's bits 6 and 58.
        assert_eq!(p.progress.bonus[0], 0x0400_0000_0000_0040);
        // MSSN [L]: all 12 missions unlocked, 12 entries.
        assert_eq!(p.progress.nightfire_status, 0xfff);
    }

    /// The block sizes must match the game's own chain [L]: PLRS 11, MSSN 68, MPSG 13,
    /// GSET 905, CHET 9, BNUS 16 — the whole chain 1022 bytes.
    #[test]
    fn block_sizes_match_the_game() {
        let p = Profile::default_codename("TEST");
        let chain = &p.encode()[..p.encode().len() - 4];
        let mut sizes = Vec::new();
        let mut off = 0;
        while off + 8 <= chain.len() {
            let id = u32::from_le_bytes(chain[off..off + 4].try_into().unwrap());
            let size = u32::from_le_bytes(chain[off + 4..off + 8].try_into().unwrap()) as usize;
            sizes.push((id, size));
            off += size;
        }
        assert_eq!(
            sizes,
            vec![
                (ID_PLRS, 11),
                (ID_MSSN, 68),
                (ID_MPSG, 13),
                (ID_GSET, 905),
                (ID_CHET, 9),
                (ID_BNUS, 16)
            ],
            "the game's own block sizes [L InBufData]"
        );
        assert_eq!(off, 1022, "the game's own chain length [L]");
    }

    /// A fresh codename round-trips every block, and the file convention works.
    #[test]
    fn round_trip_and_files() {
        let mut p = Profile::default_codename("BOND");
        p.plr.controller_style = 2;
        p.plr.crosshair = false;
        p.mpsg.health_extra = 100;
        p.gset.subtitles = true;
        p.gset.music_volume = 33;
        p.chet.cheat_ammo = true;
        p.progress.set_nightfire_status(0xfff);
        let bytes = p.encode();
        let mut q = Profile::default_codename("OTHER");
        q.decode(&bytes).expect("round trip");
        assert_eq!(q.plr, p.plr);
        assert_eq!(q.mpsg, p.mpsg);
        assert_eq!(q.gset, p.gset);
        assert_eq!(q.chet, p.chet);
        assert_eq!(q.progress.nightfire_status, 0xfff);

        let dir = std::env::temp_dir().join(format!("nf-profile-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let path = p.save_to_dir(&dir).expect("save");
        assert_eq!(path, Profile::path_in(&dir, "BOND"));
        assert_eq!(Profile::list_dir(&dir), vec!["BOND".to_string()]);
        let loaded = Profile::load_file(&path).expect("load");
        assert_eq!(loaded.plr, p.plr);
        assert_eq!(loaded.gset, p.gset);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A flipped bit in the chain is rejected (the CRC trailer), and so is a truncated chain.
    #[test]
    fn crc_and_truncation_rejected() {
        let p = Profile::default_codename("CRC");
        let mut bytes = p.encode();
        let n = bytes.len();
        bytes[n / 2] ^= 0x10;
        let mut q = Profile::default_codename("CRC");
        assert!(q.decode(&bytes).is_err(), "the flipped bit must not decode");
        assert!(q.decode(&bytes[..n - 30]).is_err(), "truncation must not decode");
    }

    /// The validator's rules [M Menu_IsValidCodename 0x1f9930]: trim + uppercase, empty 0x3bc,
    /// reserved-substring 0x2c0.
    #[test]
    fn codename_validation() {
        assert_eq!(
            Profile::check_codename("  agent 007 ", &["BOND", "MONEYPENNY"]),
            Ok("AGENT 007".to_string())
        );
        assert_eq!(Profile::check_codename("   ", &[]), Err(CodenameError::Empty));
        assert_eq!(
            Profile::check_codename("MY BOND PLAN", &["BOND"]),
            Err(CodenameError::Reserved)
        );
        assert_eq!(
            Profile::check_codename(&"X".repeat(40), &[]),
            Err(CodenameError::TooLong)
        );
    }
}

// codename-entry: the card's delete [M Menu_UpdateMessageBox 0x1f7de8 case 3 → psiDeleteData(0x2e1ae8),
// reached from the Edit Codename list's ~X (C_SBCNSELECT 0x212f58 case 0x5e) with the card-flow
// confirm; the port models the confirm in nf-view's FrontEnd].
impl Profile {
    /// Deletes the codename's file from the card dir. True when the file existed and went.
    pub fn delete_in(dir: &std::path::Path, name: &str) -> bool {
        std::fs::remove_file(Self::path_in(dir, name)).is_ok()
    }
}

#[cfg(test)]
mod delete_tests {
    use super::*;

    /// The delete removes the file and leaves the rest of the card alone [M psiDeleteData
    /// 0x1f7de8 case 3 deletes the one named datum].
    #[test]
    fn delete_in_removes_only_the_named_file() {
        let dir = std::env::temp_dir().join(format!("nf-cn-delete-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Profile::default_codename("KEEPME").save_to_dir(&dir).unwrap();
        Profile::default_codename("DROPME").save_to_dir(&dir).unwrap();
        assert!(Profile::path_in(&dir, "DROPME").exists(), "sister: the file must exist first");
        assert!(Profile::delete_in(&dir, "DROPME"));
        assert!(!Profile::path_in(&dir, "DROPME").exists());
        assert!(Profile::path_in(&dir, "KEEPME").exists());
        assert!(!Profile::delete_in(&dir, "DROPME"), "a second delete finds nothing");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
