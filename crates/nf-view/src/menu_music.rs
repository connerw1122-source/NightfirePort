//! The front end's music — which audio the menus play, and when it starts, stops and hands over.
//! Spec: docs/systems/frontend-music.md (addresses + evidence). State and transitions only, no
//! drawing: the host applies the returned [`MusicCmd`]s to `nf_audio::sfx::Sound` (silent under
//! NF_SILENT, which opens no output).
//!
//! The front end has NO MapMusic track: `UpdateMusicalEvents` skips `EventList == 0x7000048`
//! outright [M 0x114e70:17], so `Sound_Ready`'s `Music_Event(0, 0x7000048)` starts nothing
//! [M 0x1dceb8 tail]. The menus' music is a pair of music-class SFX loops — the "jingles":
//! `SFXStart(0x1d7, 0)` + `SFXStart(0x470, 0)` are the ONLY sounds the menu code ever starts
//! [M 12 call sites each across the P_* handlers]. The loaded front-end bank decides which of the
//! pair sounds: bank hash 0xc = SBI slot 0 = SB_0, whose SFX table holds 0x1d7; bank hash 0x38 =
//! SBI slot 1 = SB_1, which holds 0x470 [D probe of SBINFO.SBI + SB_0.SFX/SB_1.SFX; the IOP's
//! `SFXLoadSoundBank` resolves the hash through `SoundBankInfo` = SBINFO.SBI, M 0x2070
//! FindSoundBankSlot]. The game requests both ids every time and the bank gates the result; this
//! port issues the id the loaded bank carries (the audible result is identical, the port keeps
//! every bank resident as spares).
//!
//! The two jingle effects are polyphonic 2-subtrack loops [D SB_0.SFX id 0x1d7 / SB_1.SFX id 0x470:
//! MultiSample 1, Polyphonic 1, SampleCount 2, both subtracks' samples loop-flagged; the big loop
//! ~36.7 s / ~43.8 s of VAG at the bank's pitch 2731, plus a ~0.46 s tick layer]. Their volume is
//! music-class: `ES_CombineVolumes` scales ids 0x1d7/0x470 by `MusicMasterVolume` (init 70 [L
//! ACTION.ELF .data 0x30c958]) instead of the SFX chain [M docs/systems/sound.md §5.5; port:
//! nf-audio sfx.rs].

use crate::menu::Page;

/// The jingle id of a front-end bank hash [D the disc probe: hash 0xc = SBI slot 0 = SB_0,
/// which carries 0x1d7; hash 0x38 = slot 1 = SB_1, which carries 0x470; neither carries both].
fn jingle_of(bank_hash: u32) -> u32 { if bank_hash == 0x38 { 0x470 } else { 0x1d7 } }

/// One front-end music command — the IOP-side effect of the game's own calls, applied by the host.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MusicCmd {
    /// `SFXRemove(0, 0x1d7)` + `SFXRemove(0, 0x470)` — kill both menu-loop sounds [M the six
    /// restart handlers + `Menu_PlayMovie` 0x215990:16-18].
    StopJingles,
    /// `SFXStart(id, 0)` — the loaded bank's jingle (0x1d7 with bank 0xc, 0x470 with 0x38)
    /// [M the restart handlers; D the bank probe above].
    StartJingle(u32),
    /// `SFXStartMusic(track, section)` — IOP RPC op 0x11 [M 0x116858]. The credits' track 0x27.
    StartMusic(i32, i32),
    /// `SFXStopMusic()` — op 0x12 [M 0x1168e8].
    StopMusic,
    /// `SFXMusicSetVolume(v)` — op 0x15, sets `MusicMasterVolume = v` [M 0x116cf8].
    MusicVolume(i32),
    /// `SFXFadeDown(true)` — the 2 s music fade-down (FadeStatus 1, FadeMusic copies FadeVolume)
    /// [M 0x10e0 SFXFadeDown; callers 0x204408:150, 0x216618:28, 0x20ec00:155].
    FadeDownMusic,
}

/// The front end's audio state. One instance lives in [`crate::menu::FrontEnd`].
pub struct MenuMusic {
    /// The game's `menu_movie` @ 0x2e1c48 [M action-symtab]: true while the jingle pair is up.
    /// `MenuManager_Create` resets it to 0 on every manager create [M 0x21b908:127].
    pub jingles_up: bool,
    /// The game's `DAT_002e1b11` [M 0x1dceb8 case 0x7000048]: the front-end bank alternation.
    /// false → the NEXT front-end load takes bank hash 0xc (jingle 0x1d7), true → 0x38 (jingle
    /// 0x470); toggled on every front-end `Sound_Ready`. `P_START_Handler`'s first-boot arm pins
    /// it to 1 after the boot page's enter [M 0x20a408:31 — inside the same 0x4c, AFTER the
    /// pair restart, so on a fresh boot (DAT already flipped to 1 by the boot's own
    /// `Sound_Ready`) it is a no-op that only defeats a PTP-restored 0], keeping the
    /// alternation anchored: boot = 0xc, the second front end = 0x38.
    pub bank_flip: bool,
    /// The bank hash the CURRENT front-end load has resident — the port's stand-in for "which
    /// bank the IOP holds". Fresh = 0xc: a fresh boot's `DAT_002e1b11` is 0 (.bss), so the
    /// boot's own `Sound_Ready` case 0x7000048 loads hash 0xc [M 0x1dceb8:94-101] and its
    /// jingle 0x1d7 is what the boot front end plays.
    pub loaded_bank: u32,
    /// The credits' saved music volume (`music_vol_524 = SFXMusicGetVolume()`) [M 0x2152b0:88],
    /// restored when the credits close.
    pub credits_saved_vol: Option<i32>,
    /// The credits' track is up (started by `SFXStartMusic(0x27, 0)` [M 0x2152b0:94]).
    pub credits_music: bool,
    /// `MusicMasterVolume` @ 0x30c958 — init 70 [L ACTION.ELF .data]. Mirrors the driver's field
    /// once the host applies [`MusicCmd::MusicVolume`].
    pub music_master: i32,
    /// Commands the host has not applied yet.
    queue: Vec<MusicCmd>,
}

impl MenuMusic {
    pub fn new() -> Self {
        MenuMusic { jingles_up: false, bank_flip: false, loaded_bank: 0xc, credits_saved_vol: None, credits_music: false, music_master: 70, queue: Vec::new() }
    }

    /// The jingle id the CURRENTLY resident front-end bank carries [D the disc probe above].
    pub fn jingle_id(&self) -> u32 { jingle_of(self.loaded_bank) }

    /// The bank hash this front-end load takes, then the game's flip [M 0x1dceb8 case
    /// 0x7000048: 0xc when `DAT_002e1b11 == 0`, else 0x38, then `DAT = 1 - DAT`]. The host loads
    /// the SB files for the returned hash (SBI slot 0 = SB_0 for 0xc, slot 1 = SB_1 for 0x38
    /// [D SBINFO.SBI probe]) and it becomes the resident bank the jingle id reads.
    pub fn front_end_sound_ready(&mut self) -> u32 {
        let hash = if self.bank_flip { 0x38 } else { 0xc };
        self.bank_flip = !self.bank_flip;
        self.loaded_bank = hash;
        hash
    }

    /// A page's enter (the manager's msg 0x4c). The game's page handlers fall into three groups:
    ///
    /// - **Restart the pair** when it is down (`if menu_movie == 0 { SFXRemove both; SFXStart
    ///   both; menu_movie = 1 }`) [M P_START 0x20a408:17-22, P_MAIN 0x20a7b0:42-47, P_NFMAP
    ///   0x20b608:36-41, P_NFRESULTS 0x208e78:60-65, P_MPDEBRIEFING 0x212198:85-90]: the boot
    ///   page, the main menu, the mission list, the results page and the MP debriefing — the
    ///   pages with the looping-FMV backdrop. Every other list page sends no music messages, so
    ///   the pair simply continues across them.
    /// - **The credits** [M 0x2152b0:72-94]: save the music volume, remove the pair, and start
    ///   the credits music track 0x27 instead.
    /// - **Everything else** (Codename, Difficulty, Dossier, Records, Options, the Cn* pages,
    ///   MP setup chain, ScreenAdjust): no music calls — the pair keeps playing [M the handlers'
    ///   decomp: no SFXStart/SFXRemove outside the pages listed above].
    ///
    /// The pause page is the in-game pause: its audio hold is `MenuManager_Create(pause_sound=1)`
    /// → `SFXPause` + `SFXPauseAllStreams` [M 0x21b908:121-124] — the port's host applies it as
    /// the sound hold (`Sound::set_paused`), not through this queue.
    pub fn page_enter(&mut self, page: Page) {
        match page {
            Page::Credits => {
                self.credits_saved_vol = Some(self.music_master);
                self.credits_music = true;
                self.jingles_up = false;
                self.queue.push(MusicCmd::StopJingles);
                self.queue.push(MusicCmd::StartMusic(0x27, 0));
            }
            Page::Start | Page::Main | Page::MissionSelect | Page::Results | Page::MpDebriefing => {
                if !self.jingles_up {
                    self.jingles_up = true;
                    self.queue.push(MusicCmd::StopJingles);
                    self.queue.push(MusicCmd::StartJingle(self.jingle_id()));
                }
            }
            _ => {}
        }
    }

    /// The credits page closed [M 0x2152b0:20-38, the 0x4d arm]: `SFXStopMusic`, restore the
    /// saved volume, and restart the pair if it is down (the arm also restarts it — the credits
    /// always leave it down, so the pair always comes back).
    pub fn credits_leave(&mut self) {
        if !self.credits_music {
            return;
        }
        self.credits_music = false;
        self.queue.push(MusicCmd::StopMusic);
        if let Some(v) = self.credits_saved_vol.take() {
            self.music_master = v;
            self.queue.push(MusicCmd::MusicVolume(v));
        }
        if !self.jingles_up {
            self.jingles_up = true;
            self.queue.push(MusicCmd::StopJingles);
            self.queue.push(MusicCmd::StartJingle(self.jingle_id()));
        }
    }

    /// A menu movie started (`Menu_PlayMovie` 0x215990:14-15 with its stop-music flag, the
    /// fullscreen-movie pages' 0x4c arms): remove the pair, `menu_movie = 0` [M P_INTRO
    /// 0x209e88:42-44, P_ATTRACT 0x20a1f0:20-22, P_WINGAME 0x215100:16-18, P_ESTHERO
    /// 0x20a060:20-22, P_FMVPLAYER 0x20ad30:22-24, P_TRAILER 0x214f10:16-18]. The movie's own
    /// audio replaces the jingles — every menu movie starts with `psiStartBackgroundMovie(movie,
    /// …, SFXGetVolume())` [M 0x215990:18-19].
    pub fn movie_started(&mut self) {
        if self.jingles_up {
            self.jingles_up = false;
            self.queue.push(MusicCmd::StopJingles);
        }
    }

    /// A menu movie finished (the pages' 0x50 arms: `if (DAT_002e1c4a != 0 && menu_movie == 0)
    /// { SFXRemove both; SFXStart both; menu_movie = 1 }`) [M 0x209e88:23-28, 0x20a1f0:34-39,
    /// 0x215100:29-34]: the pair restarts while the page waits on or after its movie.
    pub fn movie_finished(&mut self) {
        if !self.jingles_up {
            self.jingles_up = true;
            self.queue.push(MusicCmd::StopJingles);
            self.queue.push(MusicCmd::StartJingle(self.jingle_id()));
        }
    }

    /// The pause page's quit/restart confirm and the MP setup's start confirm [M C_GCPAUSE
    /// 0x204408:150 (both the restart and the quit arm), P_ENDMISSION 0x216618:28, P_MPCONFIRM
    /// 0x20ec00:155]: `SFXFadeDown(1)` — the level's music fades over 2 s while the load
    /// proceeds. `Menu_RunMiniMission` 0x1ffb90 fades the same way (mini missions are not
    /// ported — reported).
    pub fn quit_fade(&mut self) { self.queue.push(MusicCmd::FadeDownMusic); }

    /// The AV options' music slider, 0..20 steps, `SFXMusicSetVolume(value * 5)` [M 0x214818
    /// 0x50 arm: `SFXMusicSetVolume(slider * 5)`; the entry snapshot divides by 5]. A cancel
    /// restores the entry value [M the 0x4d arm].
    pub fn music_slider(&mut self, value: i32) { self.music_master = value * 5; self.queue.push(MusicCmd::MusicVolume(value * 5)); }

    /// The cheats page's MUSIC toggle [M C_CHCHMUSIC 0x215bf0: `SoundInfo = 0/100;
    /// SFXMusicSetVolume(SoundInfo)`].
    pub fn cheat_music_toggle(&mut self, on: bool) { self.music_master = if on { 100 } else { 0 }; self.queue.push(MusicCmd::MusicVolume(if on { 100 } else { 0 })); }

    /// Drains the queue for the host.
    pub fn take_cmds(&mut self) -> Vec<MusicCmd> { std::mem::take(&mut self.queue) }
}

/// The host's per-redraw front-end music step, extracted from `App::menu_music_tick`
/// (crates/nf-view/src/lib.rs) so the flows are drivable headless. One call per redraw beside
/// `movie_tick`: runs the page-change transitions — the credits' leave when the page moved off
/// Credits [M 0x2152b0:20-38], then the new page's enter (the manager's msg-0x44 re-sends 0x4c
/// to the new page [M 0x21c858:345-346]) — and drains the queued commands.
///
/// The caller applies the returned commands to the sound driver exactly as `menu_music_tick`
/// does, with the game's front-end gate: `StartJingle` only sounds while the front end is the
/// loaded level (a mission's bank resolves neither jingle [D SB_12]); `StopJingles`,
/// `StartMusic`, `StopMusic`, `MusicVolume` and `FadeDownMusic` always run.
pub fn fe_music_step(fe: &mut crate::menu::FrontEnd, music_page: &mut Option<Page>) -> Vec<MusicCmd> {
    if *music_page != Some(fe.page) {
        if *music_page == Some(Page::Credits) {
            fe.music.credits_leave();
        }
        fe.music.page_enter(fe.page);
        *music_page = Some(fe.page);
    }
    fe.music.take_cmds()
}

impl Default for MenuMusic {
    fn default() -> Self { Self::new() }
}
