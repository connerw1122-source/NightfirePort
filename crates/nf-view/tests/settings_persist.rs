//! settings-persist: the option file (`settings.toml` in the user's config dir) round-trips
//! through the real input path — the option pages' Confirm/Back raise
//! `FrontEnd::settings_save_pending`, the host drains it into `settings::save_opts`, and the
//! boot (`settings::load_default`) applies the file over `default_opts`. Keys are the game's
//! row LABEL ids per page section, so reordering rows can't scramble saved settings.
//!
//! Every test writes under a temp dir (or an explicit path) — never the real user dir; the
//! `NF_CONFIG_DIR` override test proves the env route the host's default path takes.

use std::collections::HashMap;
use std::path::PathBuf;

use nf_view::menu::{default_opts, Action, FrontEnd, Page};
use nf_view::settings;

/// A fresh unique scratch dir for one test (created; removed best-effort on drop).
struct Scratch(PathBuf);
impl Scratch {
    fn new(tag: &str) -> Self {
        let p = std::env::temp_dir().join(format!("nf-settings-persist-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).expect("scratch dir");
        Scratch(p)
    }
    fn path(&self) -> PathBuf { self.0.join("settings.toml") }
}
impl Drop for Scratch {
    fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.0); }
}

/// Walks the wheel the way the game does: the Edit wheel's row 3 opens Advanced Options
/// [M C_SBCNOPTIONS_Handler 0x213528:32-60 — rows 3/4/5 → pages 0x4000002d/2e/31].
fn open_advanced(fe: &mut FrontEnd) {
    fe.page = Page::CnMenu;
    fe.cursor = 3;
    assert!(fe.action(Action::Confirm));
    assert_eq!(fe.page, Page::CnAdvanced);
    assert!(!fe.settings_save_pending, "opening the page must not save");
}

#[test]
fn round_trip_through_the_real_input_path() {
    let mut fe = FrontEnd::new();
    open_advanced(&mut fe);
    // Row 1 is Auto Aim (ctrl 0x1000_011b, default 1): one Left steps the ON_OFF list to 0.
    fe.cursor = 1;
    assert!(fe.action(Action::Left));
    assert_eq!(fe.opts.get(&0x1000_011b), Some(&0));
    // Confirm leaves the page: the edits stand and the save is requested.
    assert!(fe.action(Action::Confirm));
    assert_eq!(fe.page, Page::CnMenu);
    assert!(fe.settings_save_pending, "Confirm must request the save");
    fe.settings_save_pending = false;

    let s = Scratch::new("roundtrip");
    settings::save_opts(&s.path(), &fe.opts).expect("save");
    let text = std::fs::read_to_string(s.path()).unwrap();
    assert!(text.contains("[adv]"), "the file is sectioned by page:\n{text}");
    assert!(text.contains("0x0197 = 0"), "the Auto Aim row is keyed by its LABEL id:\n{text}");

    // A fresh launch: defaults + the file = the edit survives.
    let (opts, warn) = settings::load_opts(&s.path());
    assert!(warn.is_none(), "a good file warns nothing: {warn:?}");
    let mut fe2 = FrontEnd::new();
    fe2.opts = opts;
    assert_eq!(fe2.opts.get(&0x1000_011b), Some(&0), "the saved edit came back");
    assert_eq!(fe2.opts.get(&0x1000_011a), Some(&1), "a row the file didn't edit keeps its default");
    assert_eq!(fe2.opts.get(&0x1000_0135), Some(&80), "the AV music slider keeps its default");
}

#[test]
fn missing_file_falls_back_to_the_defaults_with_a_warning() {
    let s = Scratch::new("missing");
    let (opts, warn) = settings::load_opts(&s.path());
    assert_eq!(opts, default_opts());
    let warn = warn.expect("a missing file warns");
    assert!(warn.contains("no settings file"), "the warning names the cause: {warn}");
}

#[test]
fn corrupt_file_falls_back_to_the_defaults_with_a_warning() {
    let s = Scratch::new("corrupt");
    std::fs::write(s.path(), "this line is not toml at all\n").unwrap();
    let (opts, warn) = settings::load_opts(&s.path());
    assert_eq!(opts, default_opts(), "corrupt → the defaults");
    assert!(warn.expect("corrupt warns").contains("corrupt"));

    // A bad VALUE is corrupt too.
    std::fs::write(s.path(), "[adv]\n0x0196 = loud\n").unwrap();
    let (opts, warn) = settings::load_opts(&s.path());
    assert_eq!(opts, default_opts());
    assert!(warn.expect("a bad value warns").contains("not an integer"));

    // A key before any section is corrupt (it has no page to land on).
    std::fs::write(s.path(), "0x0196 = 1\n[adv]\n").unwrap();
    let (opts, warn) = settings::load_opts(&s.path());
    assert_eq!(opts, default_opts());
    assert!(warn.expect("a key before a section warns").contains("before any [section]"));
}

#[test]
fn unknown_keys_and_sections_are_ignored() {
    let s = Scratch::new("unknown");
    std::fs::write(
        s.path(),
        // 0xffff matches no row label; `radar` is no label id at all; 0x2b6 is the AV page's
        // Screen Adjust ACTION row (no value); [window] is a newer port's section — skipped.
        "[adv]\n0xffff = 3\nradar = 5\n[av]\n0x2b6 = 1\n[window]\nwidth = 1280\n[mp]\n0x0296 = 25\n",
    )
    .unwrap();
    let (opts, warn) = settings::load_opts(&s.path());
    assert!(warn.is_none(), "unknown is not corrupt: {warn:?}");
    // The map differs from the defaults by exactly the one KNOWN key's value — nothing
    // unknown (bad label text, unknown label, an action row, a newer port's section) moved.
    let mut expected = default_opts();
    expected.insert(0x1000_011c, 25);
    assert_eq!(opts, expected, "nothing unknown may touch the map");
}

#[test]
fn reordered_rows_keep_their_values() {
    // Non-default values on three pages.
    let mut fe = FrontEnd::new();
    fe.opts.insert(0x1000_011a, 0);  // Vibration (ADV, label 0x196)
    fe.opts.insert(0x1000_011c, 50); // Health Handicap (MP, label 0x296)
    fe.opts.insert(0x1000_0198, 4);  // Speaker Surround (AV, label 0x195)
    let saved = settings::collect(&fe.opts);

    // The same rows serialized in a scrambled order (every page reversed) must read back
    // onto the SAME controls: the file is keyed by label, never by position.
    let mut scrambled = saved.clone();
    for page in scrambled.0.iter_mut() { page.0.reverse(); }
    assert_ne!(settings::to_toml(&scrambled), settings::to_toml(&saved), "the scramble moved the lines");

    let text = settings::to_toml(&scrambled);
    let (mut loaded, warn) = settings::load_opts(&write_temp(&text));
    assert!(warn.is_none(), "{warn:?}");
    assert_eq!(loaded.get(&0x1000_011a), Some(&0));
    assert_eq!(loaded.get(&0x1000_011c), Some(&50));
    assert_eq!(loaded.get(&0x1000_0198), Some(&4));
    // And the full map matches a straight round trip — nothing scrambled anywhere.
    let mut straight = fe.opts.clone();
    settings::apply(&settings::parse(&text).unwrap(), &mut straight);
    settings::apply(&saved, &mut loaded);
    assert_eq!(straight, loaded);
}

#[test]
fn a_changed_value_really_changes_the_file() {
    // The sister assertion: if the file didn't move with the value, the round trip would
    // pass vacuously. Save two different values and demand two different files.
    let mut fe = FrontEnd::new();
    let s = Scratch::new("sister");
    settings::save_opts(&s.path(), &fe.opts).unwrap();
    let before = std::fs::read_to_string(s.path()).unwrap();
    fe.opts.insert(0x1000_0135, 30); // the music slider, default 80
    settings::save_opts(&s.path(), &fe.opts).unwrap();
    let after = std::fs::read_to_string(s.path()).unwrap();
    assert_ne!(before, after, "the file must change with the value");
    assert!(after.contains("0x0194 = 30"), "the new value is in the file:\n{after}");
    assert!(!before.contains("0x0194 = 30"));
    let (opts, _) = settings::load_opts(&s.path());
    assert_eq!(opts.get(&0x1000_0135), Some(&30));
}

#[test]
fn out_of_range_values_are_clamped_onto_legal_ones() {
    let s = Scratch::new("clamp");
    std::fs::write(
        s.path(),
        "[adv]\n0x0196 = 9\n[mp]\n0x0296 = 9999\n0x01b3 = -7\n[av]\n0x0194 = 250\n0x0195 = 3\n",
    )
    .unwrap();
    let (opts, warn) = settings::load_opts(&s.path());
    assert!(warn.is_none(), "out of range is not corrupt: {warn:?}");
    assert_eq!(opts.get(&0x1000_011a), Some(&1), "9 → the nearest of On/Off is 1");
    assert_eq!(opts.get(&0x1000_011c), Some(&100), "9999 clamps to the handicap's +100 end");
    assert_eq!(opts.get(&0x1000_0128), Some(&0), "-7 → the nearest of On/Off (radar label 0x1b3) is 0");
    assert_eq!(opts.get(&0x1000_0135), Some(&100), "the sliders clamp 0..=100 (opt_step's own clamp)");
    assert_eq!(opts.get(&0x1000_0198), Some(&4), "3 → the nearest of Mono/Stereo/Surround is 4");
}

#[test]
fn backed_out_edits_are_not_saved() {
    let mut fe = FrontEnd::new();
    open_advanced(&mut fe);
    fe.cursor = 0; // Vibration, default 1
    assert!(fe.action(Action::Left));
    assert_eq!(fe.opts.get(&0x1000_011a), Some(&0));
    // Back = the game's cancel: the page's snapshot comes back [M the ~B Back hint],
    // and the save still fires — with the RESTORED values.
    assert!(fe.action(Action::Back));
    assert_eq!(fe.page, Page::CnMenu);
    assert_eq!(fe.opts.get(&0x1000_011a), Some(&1), "Back restored the enter snapshot");
    assert!(fe.settings_save_pending, "Backed out of = saved (the restored values)");

    let s = Scratch::new("backout");
    settings::save_opts(&s.path(), &fe.opts).unwrap();
    let (opts, _) = settings::load_opts(&s.path());
    assert_eq!(opts, default_opts(), "the file holds the pre-edit values");
}

#[test]
fn the_mp_auto_aim_row_does_not_collide_with_the_adv_one() {
    // The two Auto Aim rows share label 0x197 but live in different sections and drive
    // different controls (ADV 0x1000_011b, MP 0x1000_0129) — the section split keeps both.
    let s = Scratch::new("autoaim");
    std::fs::write(s.path(), "[adv]\n0x0197 = 0\n[mp]\n0x0197 = 1\n").unwrap();
    let (opts, warn) = settings::load_opts(&s.path());
    assert!(warn.is_none(), "{warn:?}");
    assert_eq!(opts.get(&0x1000_011b), Some(&0), "the ADV row");
    assert_eq!(opts.get(&0x1000_0129), Some(&1), "the MP row");
}

#[test]
fn save_creates_a_missing_config_dir() {
    let s = Scratch::new("mkdir");
    let deep = s.0.join("one").join("two").join("settings.toml");
    let opts = default_opts();
    settings::save_opts(&deep, &opts).expect("save creates the dirs");
    assert!(deep.is_file());
}

#[test]
fn the_nf_config_dir_override_routes_the_default_path() {
    let s = Scratch::new("env");
    // Tests run with --test-threads=1 (the suite's flag), so the process env is safe here.
    std::env::set_var("NF_CONFIG_DIR", &s.0);
    let p = settings::default_path().expect("the override resolves");
    assert_eq!(p, s.0.join("settings.toml"));
    // The full default route saves and loads through it.
    let opts = default_opts();
    settings::save_opts(&p, &opts).unwrap();
    let (loaded, warn) = settings::load_default();
    assert!(warn.is_none(), "{warn:?}");
    assert_eq!(loaded, opts);
    std::env::remove_var("NF_CONFIG_DIR");
}

/// Writes text to a throwaway file and returns its path (for parse→apply tests).
fn write_temp(text: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("nf-settings-persist-{}-writetemp.toml", std::process::id()));
    std::fs::write(&p, text).unwrap();
    p
}
