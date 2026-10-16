//! options-persist-roundtrip: 651's config file proven END TO END by the real input path —
//! every Advanced / Multiplayer / AV option row is changed through the pages' Left/Right and
//! the page Confirm (no `fe.page`/`fe.cursor` writes anywhere), the front end is dropped, and
//! a fresh FrontEnd booted on the same `NF_CONFIG_DIR` config dir shows the same values. The
//! AV volume rows reach the sound layer's volumes at boot [M LS_LoadGlobalSettings 0x1cdbc8
//! :69-70 — the loaded settings call SFXMusicSetVolume/SFXSetVolume; Boot_LoadPTPData
//! 0x1c8560:81-82]. Back on a page throws the edits away (the file re-saves the RESTORED
//! values). Unknown keys and out-of-range values in the file are tolerated (clamped).
//!
//! Everything writes under a temp dir through `NF_CONFIG_DIR` — never the user's config.
//! Two documented host-side assists stand in for the windowed host (the card dir is "next to
//! the port's config" [M nf-game/src/profile.rs's profiles-dir doc]; the name types through
//! `cn_type_char` like the grid tests): the OPTION PAGES themselves move by actions only.

use std::path::PathBuf;
use std::sync::Mutex;

use nf_view::menu::{default_opts, Action, FrontEnd, OptKind, Page, ADV_ROWS, AV_ROWS, MP_ROWS};
use nf_view::settings;
use nf_view::apply_av_volumes;

/// The `NF_CONFIG_DIR` process env is shared: the three tests serialize on this so a bare
/// `cargo test -p nf-view` (parallel threads) is as deterministic as the suite's own
/// `--test-threads=1`.
static ENV_LOCK: Mutex<()> = Mutex::new(());

/// One action, then the frames a player would wait (menus_e2e's driver): the page-change
/// frames and any intro / attract movie run out.
fn act(fe: &mut FrontEnd, a: Action) -> bool {
    let r = fe.action(a);
    for _ in 0..60 {
        fe.tick();
    }
    for _ in 0..5000 {
        if !fe.intro_playing && !fe.attract_playing {
            break;
        }
        fe.tick();
    }
    r
}

/// A fresh scratch config dir (created; removed best-effort on drop).
struct Scratch(PathBuf);
impl Scratch {
    fn new(tag: &str) -> Self {
        let p = std::env::temp_dir().join(format!("nf-options-roundtrip-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).expect("scratch dir");
        Scratch(p)
    }
    fn file(&self) -> PathBuf {
        self.0.join("settings.toml")
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        std::env::remove_var("NF_CONFIG_DIR");
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// The windowed host's card dir — "next to the port's config" [M the profile.rs profiles doc].
fn point_card_at_config(fe: &mut FrontEnd) {
    let dir = settings::config_dir().expect("NF_CONFIG_DIR resolves").join("profiles");
    fe.set_profile_dir(dir);
}

/// Boot → title → Main → NightFire → New Codename → the name entry → the Edit wheel, actions
/// and ticks only. The card flow's save runs during the ticks (card_tick is fe.tick's own).
fn to_edit_wheel(fe: &mut FrontEnd) {
    assert_eq!(fe.page, Page::Start, "the boot opens the title");
    assert!(act(fe, Action::Confirm), "the title's Enter");
    assert_eq!(fe.page, Page::Main, "after the intro movies, the main menu");
    assert!(act(fe, Action::Confirm), "NightFire");
    assert_eq!(fe.page, Page::Codename);
    assert!(act(fe, Action::Confirm), "New Codename");
    assert_eq!(fe.page, Page::CnName);
    // Type QWERTY over the BOND proposal (the grid tests' typing route); the ✓ holds the
    // focus on open [M P_CNNAME 0x213790:47], so Confirm submits.
    fe.cn_typed.clear();
    fe.cn_clear_on_key = false;
    for ch in "QWERTY".chars() {
        assert!(fe.cn_type_char(ch), "the grid types {ch}");
    }
    assert!(act(fe, Action::Confirm), "the ✓ submits the name");
    // The card flow's "Saving data…" completes into the Edit wheel (pending_page = the wheel
    // [M C_KEYBOARD 0x213a98:53-65 → the completion routes there]).
    for _ in 0..2000 {
        if fe.page == Page::CnMenu {
            break;
        }
        fe.tick();
    }
    assert_eq!(fe.page, Page::CnMenu, "the card flow lands on the Edit wheel");
    assert_eq!(fe.cursor, 1, "the wheel opens on Controller Setup [L PCSX2 m30]");
}

/// The wheel's row → the option page, actions only [M C_SBCNOPTIONS 0x213528:32-60 — rows
/// 3/4/5 open pages 0x4000002d/2e/31].
fn wheel_to_page(fe: &mut FrontEnd, row: usize, page: Page) {
    while fe.cursor < row {
        assert!(act(fe, Action::Down));
    }
    assert!(act(fe, Action::Confirm), "the wheel's row opens its page");
    assert_eq!(fe.page, page);
    assert!(!fe.settings_save_pending, "opening the page must not save");
}

/// The host's drain — App::menu_after's settings arm, exactly: the pending flag writes the
/// option file through `settings::default_path()` (the NF_CONFIG_DIR route).
fn host_saves(fe: &mut FrontEnd) {
    assert!(fe.settings_save_pending, "the page's close requests the save");
    fe.settings_save_pending = false;
    let p = settings::default_path().expect("NF_CONFIG_DIR set");
    settings::save_opts(&p, &fe.opts).expect("the host's save");
}

/// Sweeps ONE page's rows Left (every value row must move; an Action row steps nothing),
/// then Confirms the page closed from its last value row.
fn sweep_page(mut fe: &mut FrontEnd) {
    let rows = match fe.page {
        Page::CnAdvanced => ADV_ROWS,
        Page::CnMp => MP_ROWS,
        Page::CnAv => AV_ROWS,
        _ => panic!("not an option page: {:?}", fe.page),
    };
    assert_eq!(fe.rows(), rows.len(), "the page shows the table's rows");
    let mut last_value_row = 0;
    for (r, row) in rows.iter().enumerate() {
        while fe.cursor < r {
            assert!(act(fe, Action::Down));
        }
        if row.kind == OptKind::Action {
            let before = fe.opt_value(row.ctrl);
            assert!(act(fe, Action::Left), "an action row still eats the key");
            assert_eq!(fe.opt_value(row.ctrl), before, "an action row has no value to step");
            continue;
        }
        last_value_row = r;
        let before = fe.opt_value(row.ctrl);
        assert!(act(fe, Action::Left));
        if row.kind == OptKind::Slider {
            // Two steps: one Left from the music default (80) would land exactly on the
            // driver's own boot volume (70) — the roundtrip must not pass on that accident.
            assert!(act(fe, Action::Left));
        }
        assert_ne!(
            fe.opt_value(row.ctrl),
            before,
            "row {r} (label {:#x}) must move on Left",
            row.label
        );
    }
    while fe.cursor > last_value_row {
        assert!(act(fe, Action::Up));
    }
    // ✕ on a value row commits: the edits stand, the page closes to the wheel.
    assert!(act(fe, Action::Confirm));
    assert_eq!(fe.page, Page::CnMenu);
    host_saves(&mut fe);
}

#[test]
fn every_row_roundtrips_through_left_right_confirm_and_the_restart() {
    let _env = ENV_LOCK.lock().unwrap();
    let s = Scratch::new("all-rows");
    std::env::set_var("NF_CONFIG_DIR", &s.0);

    let mut fe = FrontEnd::new();
    point_card_at_config(&mut fe);
    to_edit_wheel(&mut fe);

    // Advanced → Multiplayer → AV, each row changed through Left + the page Confirm.
    wheel_to_page(&mut fe, 3, Page::CnAdvanced);
    sweep_page(&mut fe);
    wheel_to_page(&mut fe, 4, Page::CnMp);
    sweep_page(&mut fe);
    wheel_to_page(&mut fe, 5, Page::CnAv);
    sweep_page(&mut fe);

    // Every value row of all three pages actually moved off its default.
    let defaults = default_opts();
    for rows in [ADV_ROWS, MP_ROWS, AV_ROWS] {
        for row in rows {
            if row.kind == OptKind::Action {
                continue;
            }
            assert_ne!(
                fe.opt_value(row.ctrl),
                defaults.get(&row.ctrl).copied().unwrap_or(0),
                "row {:#x} was changed through Left",
                row.label
            );
        }
    }
    let fe1_opts = fe.opts.clone();
    drop(fe); // drop the FrontEnd

    // A fresh FrontEnd boots the same config dir: the file's values stand.
    let mut fe2 = FrontEnd::new();
    let (opts, warn) = settings::load_default();
    assert!(warn.is_none(), "a good file warns nothing: {warn:?}");
    fe2.opts = opts;
    assert_eq!(fe2.opts, fe1_opts, "the fresh FrontEnd shows the same values");

    // The AV volume rows reach the sound layer's volumes at boot (the windowed boot's
    // apply_av_volumes call, lib.rs's --fe arm).
    let mut driver = nf_audio::sfx::Driver::new();
    apply_av_volumes(&mut fe2, Some(&mut driver));
    let music = fe1_opts.get(&0x1000_0135).copied().unwrap();
    let sfx = fe1_opts.get(&0x1000_0134).copied().unwrap();
    assert_eq!(driver.music_master, music, "the music row is the driver's music volume");
    assert_eq!(driver.master, sfx, "the sfx row is the driver's sfx master");
    assert_eq!(fe2.music.music_master, music, "the menu-music model carries it too");
    // Sister: the driver's defaults differ from the edited values, so the assertion can bite.
    let d = nf_audio::sfx::Driver::new();
    assert_ne!(music, d.music_master, "the edit moved off the driver's default");
}

#[test]
fn back_on_a_page_throws_the_edits_away() {
    let _env = ENV_LOCK.lock().unwrap();
    let s = Scratch::new("back");
    std::env::set_var("NF_CONFIG_DIR", &s.0);

    let mut fe = FrontEnd::new();
    point_card_at_config(&mut fe);
    to_edit_wheel(&mut fe);
    wheel_to_page(&mut fe, 3, Page::CnAdvanced);
    let entry = fe.opts.clone();

    // Change the first three rows through Left.
    for r in 0..3 {
        while fe.cursor < r {
            assert!(act(&mut fe, Action::Down));
        }
        let row = &ADV_ROWS[r];
        let before = fe.opt_value(row.ctrl);
        assert!(act(&mut fe, Action::Left));
        assert_ne!(fe.opt_value(row.ctrl), before, "the edit landed");
    }
    // Back = the game's cancel: the enter snapshot comes back [M the ~B Back hint] and the
    // file re-saves the RESTORED values — the edits are gone from the map AND the file.
    assert!(act(&mut fe, Action::Back));
    assert_eq!(fe.page, Page::CnMenu);
    assert_eq!(fe.opts, entry, "Back restored the enter snapshot");
    host_saves(&mut fe);

    let (opts, warn) = settings::load_default();
    assert!(warn.is_none(), "{warn:?}");
    assert_eq!(opts, default_opts(), "the file holds the pre-edit values");
    let mut fe2 = FrontEnd::new();
    fe2.opts = opts;
    for row in ADV_ROWS.iter().take(3) {
        assert_eq!(
            fe2.opt_value(row.ctrl),
            default_opts().get(&row.ctrl).copied().unwrap(),
            "the thrown-away edit is not on the fresh FrontEnd"
        );
    }
}

#[test]
fn boot_applies_clamped_file_volumes_to_the_sound_layer() {
    let _env = ENV_LOCK.lock().unwrap();
    let s = Scratch::new("volumes");
    std::env::set_var("NF_CONFIG_DIR", &s.0);
    // Unknown keys (`0xffff`, `radar`), an out-of-range slider (250 → 100), a legal pair
    // (music 30, sfx 40) and an action row's label (no value): all tolerated.
    std::fs::write(
        s.file(),
        "[adv]\n0xffff = 3\nradar = 5\n[mp]\n0x0296 = 9999\n[av]\n0x0194 = 30\n0x0193 = 40\n0x2b6 = 1\n",
    )
    .unwrap();

    let mut fe2 = FrontEnd::new();
    let (opts, warn) = settings::load_default();
    assert!(warn.is_none(), "out of range / unknown is not corrupt: {warn:?}");
    fe2.opts = opts;
    assert_eq!(fe2.opt_value(0x1000_0135), 30, "the music row");
    assert_eq!(fe2.opt_value(0x1000_0134), 40, "the sfx row");
    assert_eq!(fe2.opt_value(0x1000_011c), 100, "the handicap clamped onto its +100 end");

    let mut driver = nf_audio::sfx::Driver::new();
    apply_av_volumes(&mut fe2, Some(&mut driver));
    assert_eq!(driver.music_master, 30, "the file's music volume is the driver's");
    assert_eq!(driver.master, 40, "the file's sfx volume is the driver's");

    // Sister: a slider past 100 clamps first — the driver never sees an out-of-range volume.
    std::fs::write(s.file(), "[av]\n0x0194 = 250\n0x0193 = -7\n").unwrap();
    let (opts, _) = settings::load_default();
    fe2.opts = opts;
    let mut driver = nf_audio::sfx::Driver::new();
    apply_av_volumes(&mut fe2, Some(&mut driver));
    assert_eq!(driver.music_master, 100, "250 clamps to the slider's 100 end");
    assert_eq!(driver.master, 0, "-7 clamps to the slider's 0 end");
}
