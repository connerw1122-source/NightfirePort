//! frontend-music headless tests: which audio the front end plays, and the transitions — numbers
//! and ids only (the main agent draws every page). The state machine under test is
//! `nf_view::menu_music::MenuMusic`, the port of the game's menu music calls.
//!
//! Evidence: the six restart handlers (P_START 0x20a408:17-22, P_MAIN 0x20a7b0:42-47, P_NFMAP
//! 0x20b608:36-41, P_NFRESULTS 0x208e78:60-65, P_MPDEBRIEFING 0x212198:85-90 — plus
//! Menu_PlayMovie 0x215990:16-18 and the movie-finish arms 0x209e88:23-28 / 0x20a1f0:45-50 /
//! 0x215100:30-35), the credits swap (P_CREDITS 0x2152b0:72-94 enter, :20-38 leave), the bank
//! alternation (Sound_Ready 0x1dceb8 case 0x7000048, `DAT_002e1b11`), the skips
//! (`UpdateMusicalEvents` 0x114e70:17), the pause pair (MenuManager_Create 0x21b908:121-124 +
//! Delete 0x21c2b0:11-22), the quit fade (C_GCPAUSE 0x204408:150, P_ENDMISSION 0x216618:28,
//! P_MPCONFIRM 0x20ec00:155), the AV slider (P_CNAVOPTIONS 0x214818) and the cheats toggle
//! (C_CHCHMUSIC 0x215bf0). The disc facts live in the NF_ISO sisters at the bottom.

use nf_view::menu::{Action, FrontEnd, Page};
use nf_view::menu_music::{MenuMusic, MusicCmd};

/// The restart pair's exact command list [M `if menu_movie == 0 { SFXRemove; SFXRemove; SFXStart;
/// SFXStart; menu_movie = 1 }` — the port issues the loaded bank's id, see menu_music.rs].
fn expected_restart(id: u32) -> Vec<MusicCmd> { vec![MusicCmd::StopJingles, MusicCmd::StartJingle(id)] }

#[test]
fn the_fresh_front_end_state_matches_the_game() {
    let m = MenuMusic::new();
    // `menu_movie` starts 0 (MenuManager_Create resets it [M 0x21b908:125]) and the boot's first
    // front end loads bank 0xc [M 0x1dceb8: DAT_002e1b11 starts 0 (.bss)].
    assert!(!m.jingles_up);
    assert!(!m.bank_flip);
    assert_eq!(m.jingle_id(), 0x1d7, "bank 0xc → SB_0 carries 0x1d7 [D SBINFO.SBI + SB_0.SFX]");
    // `MusicMasterVolume` boots at 70 [L ACTION.ELF .data 0x30c958 = 70; SfxMasterVolume 0x30c95c = 100].
    assert_eq!(m.music_master, 70);
    assert_ne!(m.music_master, 100, "the music default is NOT the sfx default (a mix would prove the mirror wrong)");
}

#[test]
fn the_boot_page_restarts_the_pair_and_the_list_pages_keep_it() {
    let mut fe = FrontEnd::new();
    assert_eq!(fe.page, Page::Start);
    fe.music.page_enter(Page::Start);
    assert_eq!(fe.music.take_cmds(), expected_restart(0x1d7), "P_START's 0x4c [M 0x20a408:17-22]");
    assert!(fe.music.jingles_up);
    // The list pages send NO music messages — the pair continues across them [M the handlers'
    // decomp has no SFXStart/SFXRemove on Codename/Difficulty/Dossier/CnMenu/MP setup].
    for page in [Page::Codename, Page::Difficulty, Page::Dossier, Page::Records, Page::Options,
                 Page::CnSelect, Page::CnMenu, Page::ScreenAdjust, Page::MpJoin, Page::MpScenario,
                 Page::MpMap, Page::MpSetup, Page::MpOptions, Page::MpRules, Page::MpBots, Page::MpBotSetup] {
        fe.music.page_enter(page);
        assert!(fe.music.take_cmds().is_empty(), "{page:?} must not touch the pair");
    }
    assert!(fe.music.jingles_up, "the pair survived the walk");
    // The mission list DOES restart when the pair is down (a movie swapped in meanwhile) [M 0x20b608:36-41].
    fe.music.movie_started();
    assert_eq!(fe.music.take_cmds(), vec![MusicCmd::StopJingles]);
    fe.music.page_enter(Page::MissionSelect);
    assert_eq!(fe.music.take_cmds(), expected_restart(fe.music.jingle_id()), "P_NFMAP's 0x4c");
    // But an enter while the pair is up sends nothing (the `if menu_movie == 0` gate) [M].
    fe.music.page_enter(Page::MissionSelect);
    assert!(fe.music.take_cmds().is_empty());
}

#[test]
fn every_restart_page_and_the_results_page_map_to_the_pair() {
    // The five restart pages of the decomp [M the five 0x4c handlers].
    for page in [Page::Start, Page::Main, Page::MissionSelect, Page::Results, Page::MpDebriefing] {
        let mut m = MenuMusic::new();
        m.page_enter(page);
        assert_eq!(m.take_cmds(), expected_restart(m.jingle_id()), "{page:?} enter must restart the pair");
    }
    // The pause page sends nothing itself: its hold is the manager create's SFXPause +
    // SFXPauseAllStreams (the host's set_paused), not a page command [M 0x21b908:121-124].
    let mut m = MenuMusic::new();
    m.page_enter(Page::Pause);
    assert!(m.take_cmds().is_empty());
}

#[test]
fn the_bank_alternation_flips_the_jingle_id_every_front_end_load() {
    let mut m = MenuMusic::new();
    // Sound_Ready 0x1dceb8 case 0x7000048: bank 0xc while DAT_002e1b11 == 0, else 0x38; DAT
    // toggles every load. SBI slot 0 = hash 0xc = SB_0 (0x1d7), slot 1 = hash 0x38 = SB_1 (0x470).
    assert_eq!(m.front_end_sound_ready(), 0xc);
    assert_eq!(m.jingle_id(), 0x1d7);
    assert_eq!(m.front_end_sound_ready(), 0x38);
    assert_eq!(m.jingle_id(), 0x470);
    assert_eq!(m.front_end_sound_ready(), 0xc, "third load flips back");
    assert_eq!(m.jingle_id(), 0x1d7);
}

#[test]
fn the_intro_movie_swap_stops_and_restarts_the_pair() {
    // The BOOT P_INTRO pair (the EA then the MGM logo), which ends on the title 0x40000009
    // [M P_INTRO_Handler 0x209e88:16-62].
    let mut m = MenuMusic::new();
    m.page_enter(Page::Start); // the pair is up
    assert!(m.take_cmds().len() == 2);
    // The intro's start arm [M 0x209e88:42-44]: remove both, menu_movie = 0 — the movie's own
    // audio (started with psiStartBackgroundMovie(…, SFXGetVolume())) replaces it.
    m.movie_started();
    assert_eq!(m.take_cmds(), vec![MusicCmd::StopJingles]);
    assert!(!m.jingles_up);
    // Between the two intro movies the finish arm restarts the pair [M 0x209e88:23-28].
    m.movie_finished();
    assert_eq!(m.take_cmds(), expected_restart(m.jingle_id()));
    // The second movie's start removes it again [M 0x209e88:56-58]; after the last movie it comes
    // back and the TITLE's enter (pair already up) sends nothing [M P_START 0x20a408:17-23's
    // `if menu_movie == 0`; P_INTRO opens 0x40000009, 0x209e88:50].
    m.movie_started();
    assert_eq!(m.take_cmds(), vec![MusicCmd::StopJingles]);
    m.movie_finished();
    assert_eq!(m.take_cmds().len(), 2);
    m.page_enter(Page::Start);
    assert!(m.take_cmds().is_empty(), "no double start when menu_movie is already 1");
}

#[test]
fn the_credits_swap_saves_volume_plays_track_0x27_and_restores() {
    let mut m = MenuMusic::new();
    // Enter [M 0x2152b0:72-94]: music_vol_524 = SFXMusicGetVolume(); the pair removed;
    // SFXStartMusic(0x27, 0) — the credits' own music system track.
    m.page_enter(Page::Credits);
    assert_eq!(m.take_cmds(), vec![MusicCmd::StopJingles, MusicCmd::StartMusic(0x27, 0)]);
    assert!(m.credits_music);
    assert_eq!(m.credits_saved_vol, Some(70), "the saved volume is the entry MusicMasterVolume");
    // The AV options' music slider mid-visit: value × 5 [M 0x214818's 0x50 arm].
    m.music_slider(13);
    assert_eq!(m.take_cmds(), vec![MusicCmd::MusicVolume(65)]);
    assert_eq!(m.music_master, 65);
    // Leave [M 0x2152b0:54-65]: SFXStopMusic, SFXMusicSetVolume(music_vol_524) — the ENTRY value,
    // not the slider's last one — and the pair restarts.
    m.credits_leave();
    assert_eq!(m.take_cmds(), vec![MusicCmd::StopMusic, MusicCmd::MusicVolume(70), MusicCmd::StopJingles, MusicCmd::StartJingle(0x1d7)]);
    assert!(!m.credits_music);
    assert_eq!(m.credits_saved_vol, None);
    // A leave without an enter is a no-op (the game's 0x4d only runs from the credits page).
    m.credits_leave();
    assert!(m.take_cmds().is_empty());
}

#[test]
fn the_quit_fade_is_fademusic_only() {
    // C_GCPAUSE 0x204408:150 (restart AND quit arms), P_ENDMISSION 0x216618:28, P_MPCONFIRM
    // 0x20ec00:155: SFXFadeDown(1) — the music fades over 2 s, nothing else sounds.
    let mut fe = FrontEnd::new();
    // menu-audio-state: the constructor's boot pair now rides the music channel too — drain it
    // and assert the fade adds nothing but the fade.
    let boot: Vec<MusicCmd> = fe.music.take_cmds();
    assert!(boot.iter().all(|c| matches!(c, MusicCmd::StopJingles | MusicCmd::StartJingle(_))),
        "the boot open queues only the pair");
    fe.music.quit_fade();
    assert_eq!(fe.music.take_cmds(), vec![MusicCmd::FadeDownMusic]);
}

#[test]
fn the_volume_knobs_land_on_the_game_numbers() {
    let mut m = MenuMusic::new();
    // The AV slider's range: 0..20 steps at 5 [M 0x214818: the msg 0x27 slider carries 0x14 = 20,
    // the 0x50 arm multiplies by 5].
    m.music_slider(0);
    assert_eq!(m.music_master, 0);
    m.music_slider(20);
    assert_eq!(m.music_master, 100);
    assert_eq!(m.take_cmds(), vec![MusicCmd::MusicVolume(0), MusicCmd::MusicVolume(100)]);
    // The cheats page's MUSIC toggle [M 0x215bf0: SoundInfo = 0/100; SFXMusicSetVolume].
    m.cheat_music_toggle(false);
    assert_eq!(m.music_master, 0);
    m.cheat_music_toggle(true);
    assert_eq!(m.music_master, 100);
}

#[test]
fn a_full_walk_from_boot_to_mission_and_back_hands_over() {
    // Boot → the pair on Start → Main → the mission pick (no music messages: the load's
    // SFXReset stops the pair [M Sound_Ready 0x1dceb8 starts with SFXReset]) → the results page
    // over the mission (the enter queues a restart but the mission bank resolves neither id —
    // the host's gate) → the front end again (manager reset + bank flip) → the pair on the
    // mission list with the OTHER jingle.
    let mut fe = FrontEnd::new();
    // The boot front end's own load consumed the flip: DAT (.bss) = 0 → bank 0xc, DAT → 1
    // [M 0x1dceb8:94-101]; P_START's once arm pins DAT = 1 right after [M 0x20a408:31].
    assert_eq!(fe.music.front_end_sound_ready(), 0xc);
    fe.music.page_enter(Page::Start);
    assert_eq!(fe.music.take_cmds(), expected_restart(0x1d7));
    fe.music.page_enter(Page::Main);
    assert!(fe.music.take_cmds().is_empty());
    // The pick: the game's page change to the loading flow stops nothing by itself — the scene
    // reload (SFXReset) kills the items. The port's host resets `jingles_up` at the front-end
    // load (MenuManager_Create's menu_movie = 0 [M 0x21b908:125]).
    fe.music.jingles_up = false;
    // The results page over the mission: the state machine queues the restart (the game does —
    // P_NFRESULTS 0x4c); the host's front-end gate mutes it because the mission bank lacks the id.
    fe.music.page_enter(Page::Results);
    assert_eq!(fe.music.take_cmds(), expected_restart(0x1d7), "the command mirrors the game's call");
    // The front end loads: the bank flips and the manager reset drops the pair flag.
    assert_eq!(fe.music.front_end_sound_ready(), 0x38);
    fe.music.jingles_up = false;
    fe.music.page_enter(Page::MissionSelect);
    assert_eq!(fe.music.take_cmds(), expected_restart(0x470), "the second front end plays the OTHER jingle");
}

#[test]
fn the_front_end_action_walk_sound_tracks_the_page_changes() {
    // Through the port's own input path: START → P_PARISENUM → P_ESTHERO (one movie, 0x073B0048)
    // → Main 0x40000002, the music following the pages [M 0x20a648:43, 0x20a060:16-45].
    let mut fe = FrontEnd::new();
    fe.music.page_enter(fe.page);
    assert_eq!(fe.music.take_cmds().as_slice(), &[MusicCmd::StopJingles, MusicCmd::StartJingle(0x1d7)]);
    // START, then P_ESTHERO's 2nd 0x50 starts the movie: the pair is removed [M 0x20a060:20-22].
    assert!(fe.action(Action::Confirm));
    for _ in 0..3 { fe.tick(); }
    assert!(fe.intro_movie);
    assert!(fe.music.take_cmds().contains(&MusicCmd::StopJingles));
    assert!(!fe.music.jingles_up);
    // The one movie done → its finish arm restarts the pair once [M :34-40] and opens Main.
    fe.intro_movie_finished();
    fe.tick();
    assert_eq!(fe.page, Page::Main);
    assert_eq!(fe.music.take_cmds().len(), 2, "one restart: remove + start");
    // Sister: the page enter on the already-up pair adds nothing [M P_MAIN 0x20a7b0:36-47].
    fe.music.page_enter(Page::Main);
    assert!(fe.music.take_cmds().is_empty());
}

// --- The NF_ISO sisters: the real disc bytes (each can fail) ---

fn iso() -> Option<nf_formats::iso9660::IsoImage<std::fs::File>> {
    let path = std::env::var("NF_ISO").ok()?;
    nf_formats::iso9660::IsoImage::open(&path).ok()
}

/// A bank slot's three files, through the repo's `bank_path` (PS2/<lang>/SB_<slot>>3>/SB_<slot>.<ext>)
/// [M nf_formats::soundbank::bank_path; the same paths the host's spare-bank loader reads].
fn read_bank(get: &dyn Fn(&str) -> Option<Vec<u8>>, slot: usize) -> (nf_formats::soundbank::Bank, Vec<u8>) {
    use nf_formats::soundbank::bank_path;
    let shf = get(&bank_path("ENGLISH", slot, "SHF")).expect("SB SHF on the disc");
    let sfx = get(&bank_path("ENGLISH", slot, "SFX")).expect("SB SFX on the disc");
    let sbf = get(&bank_path("ENGLISH", slot, "SBF")).expect("SB SBF on the disc");
    (nf_formats::soundbank::Bank::parse(&shf, &sfx).expect("a parseable bank"), sbf)
}

#[test]
fn sbinfo_slots_zero_and_one_are_the_two_front_end_banks() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let get = |p: &str| iso.find(p).and_then(|e| iso.read_file(e).ok());
    let sbi = get("PS2/SBINFO.SBI").expect("SBINFO.SBI on the disc");
    let h = |k: usize| u32::from_le_bytes(sbi[k * 4..k * 4 + 4].try_into().unwrap());
    // [D SBINFO.SBI] slot 0 = hash 0xc (the boot's front-end bank), slot 1 = 0x38 (the flip's).
    assert_eq!(h(0), 0xc);
    assert_eq!(h(1), 0x38);
    assert_ne!(h(0), h(1), "the two front-end banks must be distinct hashes");
}

#[test]
fn sb0_carries_jingle_0x1d7_and_sb1_carries_0x470_and_neither_carries_both() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let get = |p: &str| iso.find(p).and_then(|e| iso.read_file(e).ok());
    let (b0, _sbf0) = read_bank(&get, 0);
    let (b1, _sbf1) = read_bank(&get, 1);
    // [D SB_0.SFX]: 0x1d7 = polyphonic 2-subtrack effect; its two samples are loop-flagged.
    let e0 = b0.effect(0x1d7).map(nf_formats::soundbank::Effect::parse).expect("SB_0 has 0x1d7").expect("a parseable effect");
    assert_eq!(e0.count, 2);
    // The menu blips ride BOTH banks (Menu_PlaySound's ids 0x1d8-0x1da) [D].
    assert!(b0.effect(0x1d8).is_some() && b0.effect(0x1d9).is_some() && b0.effect(0x1da).is_some());
    assert!(b1.effect(0x1d8).is_some() && b1.effect(0x1da).is_some());
    // [D] 0x470 lives in SB_1, NOT in SB_0 — the bank gate that makes the pair alternate.
    assert!(b0.effect(0x470).is_none(), "SB_0 must not carry the second jingle");
    let e1 = b1.effect(0x470).map(nf_formats::soundbank::Effect::parse).expect("SB_1 has 0x470").expect("a parseable effect");
    assert_eq!(e1.count, 2);
    assert!(b1.effect(0x1d7).is_none(), "SB_1 must not carry the first jingle");
}

#[test]
fn a_mission_bank_lacks_both_jingles() {
    // The results page restarts the pair over a MISSION — and the mission banks resolve neither
    // id, so nothing sounds until the front end reloads [D the bank probe: SB_12 (bank hash 0x16's
    // slot? the probe's finding) carries 0x1d8-0x1da but neither jingle]. Slot 2 is a level bank
    // on this disc; assert both jingles are absent there.
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let get = |p: &str| iso.find(p).and_then(|e| iso.read_file(e).ok());
    let (bank, _sbf) = read_bank(&get, 12);
    assert!(bank.effect(0x1d7).is_none(), "a mission bank must not carry the first jingle");
    assert!(bank.effect(0x470).is_none(), "a mission bank must not carry the second jingle");
    assert!(bank.effect(0x1da).is_some(), "…but it still carries the menu blip 0x1da");
}

#[test]
fn the_credits_track_files_exist_on_the_disc() {
    // SFXStartMusic(0x27, 0) [M 0x2152b0:94] — MFXINFO.MXI maps track 0x27 to an MFX_<n> file
    // pair, the same loader the level music uses [M docs/systems/music.md].
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let get = |p: &str| iso.find(p).and_then(|e| iso.read_file(e).ok());
    let mxi = get("PS2/MFXINFO.MXI").expect("MFXINFO.MXI on the disc");
    let n = nf_formats::music::track_file(&mxi, 0x27).ok().flatten().expect("track 0x27 in MFXINFO.MXI");
    let smf = get(&nf_formats::music::music_path(n, "SMF")).expect("the track's SMF");
    let ssd = get(&nf_formats::music::music_path(n, "SSD")).expect("the track's SSD");
    let smf = nf_formats::music::Smf::parse(&smf).expect("a parseable SMF");
    assert!(!ssd.is_empty());
    assert!(!smf.markers.is_empty() || !smf.starts.is_empty(), "the SMF carries sections");
}
