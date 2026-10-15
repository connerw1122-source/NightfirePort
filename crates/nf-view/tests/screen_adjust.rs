//! screen-adjust-model headless + ISO-gated tests: the AV Options → Screen Adjust page (0x40000047),
//! the picture offset's steps/clamps/conversions, the Restore Defaults list, and the widescreen
//! view aspect.
//!
//! Evidence: `P_SCREENADJUST_Handler` 0x217a58, `C_SBSCREEN_Handler` 0x217b08, `psiAdjustScreenPos`
//! 0x10b890, `psiSetScreenPos` 0x10b910 / `psiGetScreenPos` 0x10b928 / `psiRestoreScreenPos`
//! 0x10b940, `P_CNAVOPTIONS_Handler` 0x214818 (Restore Defaults :96-110, the commit :127-136, the
//! Back :79-85), `VBlankInt` 0x124088 (the DISPLAY2 write), `sceGsSetDefDispEnv` 0x226370 (the
//! field decode), `LS_MakeGlobalSettings` 0x1cd9d0 / `LS_LoadGlobalSettings` 0x1cdbc8 (the profile
//! blob), `Camera_CalcViewAngles` 0x1baf70 (the 16/9 / 4/3 aspect) — all in
//! docs/systems/screen-adjust.md. The menu-def page data is [L 07000048.bin entry 08000002, walked
//! 2026-10-15]; the label texts are [D USATxt.dat].

use nf_view::menu::{Action, FrontEnd, Page, PAGE_SCREEN_ADJUST};

fn tick_to_wheel(fe: &mut FrontEnd) {
    for _ in 0..8 {
        fe.tick();
        if fe.flow.get_state() == nf_view::menu::ST_RUNNER && fe.flow.current_level == nf_view::menu::FRONTEND_LEVEL { break; }
    }
    fe.action(Action::Confirm); // Start → the intro movie state, page Main (the port's contract)
    fe.action(Action::Confirm); // skips the movie [M GameFlow_Main case 6 pops on action 0x19]
    assert_eq!(fe.page, Page::Main);
    fe.cursor = 2;              // the Codenames row [M C_GOCODENAMES 0x216b38 → page 0x4000001b]
    fe.action(Action::Confirm); // → the Edit Codename screen (cursor lands on "Default")
    assert_eq!(fe.page, Page::CnSelect);
    fe.action(Action::Confirm); // "Default" → the Edit wheel [M C_SBCNSELECT 0x212f58]
    assert_eq!(fe.page, Page::CnMenu);
}

fn to_av(fe: &mut FrontEnd) {
    tick_to_wheel(fe);
    fe.cursor = 5; // the wheel's AV Options row [M C_SBCNOPTIONS 0x213528:32-60 — row 5 → 0x40000031]
    assert!(fe.action(Action::Confirm));
    assert_eq!(fe.page, Page::CnAv);
    assert_eq!(fe.page.id(), 0x4000_0031);
}

/// psiAdjustScreenPos 0x10b890's numbers [M]: one step is 12 X-units / 4 Y-units; the clamps are
/// ±0x1b0 (±432) and ±0x30 (±48). Sister: the clamp bounds themselves — a regression to an
/// unclamped or wrongly-scaled accumulator fails here.
#[test]
fn screen_adjust_steps_and_clamps_match_psiadjustscreenpos() {
    let mut fe = FrontEnd::new();
    assert_eq!(fe.screen_x, 0, "the game boots the offset at 0/0 (.sbss) [M]");
    assert_eq!(fe.screen_y, 0);
    fe.screen_adjust(1, 0);
    assert_eq!(fe.screen_x, 12, "one X step = 12 [M 0x10b890: PS2ScreenXPos += dx*0xc]");
    assert_eq!(fe.screen_y, 0);
    fe.screen_adjust(0, 1);
    assert_eq!(fe.screen_y, 4, "one Y step = 4 [M 0x10b890: PS2ScreenYPos += dy*4]");
    fe.screen_adjust(-2, -1);
    assert_eq!(fe.screen_x, -12, "−2 steps back from 12");
    assert_eq!(fe.screen_y, 0, "1 down from 4");
    // The clamps: 40 × 12 = 480 > 432 → 432; −13 × 4 = −52 < −48 → −48.
    for _ in 0..40 { fe.screen_adjust(1, 0); }
    assert_eq!(fe.screen_x, 0x1b0, "X clamps at +432 [M 0x10b890: if (0x1b0 < X) X = 0x1b0]");
    for _ in 0..20 { fe.screen_adjust(0, -1); }
    assert_eq!(fe.screen_y, -0x30, "Y clamps at −48 [M 0x10b890: if (Y < −0x30) Y = −0x30]");
    fe.screen_adjust(1, 1);
    assert_eq!(fe.screen_x, 0x1b0, "the clamp holds against further +X");
    assert_eq!(fe.screen_y, -0x30 + 4, "and Y still moves inside its range");
    // psiRestoreScreenPos 0x10b940 [M].
    fe.screen_restore_defaults();
    assert_eq!((fe.screen_x, fe.screen_y), (0, 0), "Restore Defaults' offset arm [M 0x214818:107]");
}

/// The 640×480 conversion: X/4 and Y·15/14 [M VBlankInt 0x124088 + sceGsSetDefDispEnv 0x226370 —
/// 2560 X-units per visible line (MAGH 5 × 512), 448 field lines]. Sister: the fractions — the same
/// numbers in picture-relative form, so a wrong unit scale breaks exactly one of the two.
#[test]
fn screen_offset_conversions_640_and_fractions() {
    let mut fe = FrontEnd::new();
    fe.screen_adjust(1, 1); // (12, 4)
    let (x640, y480) = fe.screen_offset_640();
    assert!((x640 - 3.0).abs() < 1e-4, "12 X-units = 3 units of the 640 space (X/4), got {x640}");
    assert!((y480 - 60.0 / 14.0).abs() < 1e-4, "4 lines = 4·15/14 of the 480 space, got {y480}");
    let (fx, fy) = fe.screen_offset_frac();
    assert!((fx - 12.0 / 2560.0).abs() < 1e-9, "X fraction of the 2560-unit line, got {fx}");
    assert!((fy - 4.0 / 448.0).abs() < 1e-9, "Y fraction of the 448-line frame, got {fy}");
    assert!((fx * 640.0 - x640).abs() < 1e-3, "the two getters agree through the 640 space");
    assert!((fy * 480.0 - y480).abs() < 1e-3, "…and through the 480 space");
}

/// The full flow: AV Options row 6 (0x10000137, "Screen Adjust") pushes the page; ✕ keeps the
/// adjustment; the page's Back restores the enter snapshot [M 0x217a58 msg 0x4c/0x4b/0x6b]. Sister:
/// the kept and restored values differ, so a broken snapshot or a wrong pop target fails.
#[test]
fn the_screen_adjust_page_keeps_on_confirm_and_restores_on_back() {
    let mut fe = FrontEnd::new();
    to_av(&mut fe);
    fe.cursor = 6; // the AV page's Screen Adjust row (y=256, the 7th of the ten rows [L page data])
    assert!(fe.action(Action::Confirm), "the row is an action row");
    assert_eq!(fe.page, Page::ScreenAdjust);
    assert_eq!(fe.page.id(), PAGE_SCREEN_ADJUST);
    assert_eq!(PAGE_SCREEN_ADJUST, 0x4000_0047, "the page id is the menu-def's [L]");
    assert_eq!(fe.rows(), 2, "the page's two sliders (d1 0 vertical, d1 1 horizontal [L])");
    assert_eq!(fe.screen_saved_x, 0, "the enter snapshot (psiGetScreenPos → PS2OldX) [M 0x217a58:8-10]");
    // Nudge both bars, then ✕: the values KEEP (Manager 0x5f(1) skips the 0x6b restore [M 0x21c858]).
    assert!(fe.action(Action::Right), "the vertical bar steps");
    assert!(fe.action(Action::Down), "cursor to the horizontal bar");
    assert!(fe.action(Action::Right));
    assert_eq!((fe.screen_x, fe.screen_y), (12, 4));
    assert!(fe.action(Action::Confirm), "✕ Confirm");
    assert_eq!(fe.page, Page::CnAv, "✕ pops back to the AV page");
    assert_eq!(fe.cursor, 6, "the cursor returns to the Screen Adjust row");
    assert_eq!((fe.screen_x, fe.screen_y), (12, 4), "✕ keeps the adjustment");
    // Re-enter and cancel: the enter snapshot is now (12, 4), a further nudge then Back restores it.
    assert!(fe.action(Action::Confirm));
    assert_eq!(fe.page, Page::ScreenAdjust);
    assert!(fe.action(Action::Right));
    assert!(fe.action(Action::Down));
    assert!(fe.action(Action::Right));
    assert_eq!((fe.screen_x, fe.screen_y), (24, 8));
    assert!(fe.action(Action::Back), "the hint's ~B Cancel [L 0x67b]");
    assert_eq!(fe.page, Page::CnAv);
    assert_eq!((fe.screen_x, fe.screen_y), (12, 4), "Back restores the enter snapshot [M 0x217a58:16-18]");
}

/// P_CNAVOPTIONS' snapshot discipline [M 0x214818:29/79-85/135]: entering AV snapshots the offset;
/// Back restores it; committing (✕ on a non-action row) re-snapshots, so the Screen Adjust edits
/// survive the wheel's Back. Sister: the restored value differs from the edited one.
#[test]
fn the_av_page_snapshots_restores_and_re_snapshots_the_offset() {
    let mut fe = FrontEnd::new();
    to_av(&mut fe);
    assert_eq!(fe.screen_av_saved_x, 0, "the page enter snapshot [M 0x214818:29]");
    // Edit the offset through the subscreen and ✕ out.
    fe.cursor = 6;
    fe.action(Action::Confirm);
    fe.action(Action::Right); // Y += 4
    fe.action(Action::Down);
    fe.action(Action::Right); // X += 12
    fe.action(Action::Confirm);
    assert_eq!((fe.screen_x, fe.screen_y), (12, 4));
    assert_eq!((fe.screen_av_saved_x, fe.screen_av_saved_y), (12, 4), "the re-enter re-snapshot [M 0x214818:135]");
    // A value row's ✕ commits and leaves the page; the wheel's Back does NOT undo the offset
    // (the commit re-snapshots at :135, the wheel's Back never touches it).
    fe.cursor = 2; // Subtitles (a list row — the commit path)
    assert!(fe.action(Action::Confirm));
    assert_eq!(fe.page, Page::CnMenu);
    assert_eq!((fe.screen_x, fe.screen_y), (12, 4), "the commit keeps the offset");
    // Fresh entry then Back restores the page-enter snapshot.
    fe.cursor = 5;
    assert!(fe.action(Action::Confirm));
    assert_eq!(fe.screen_av_saved_x, 12);
    fe.screen_adjust(1, 0); // a direct nudge without the subscreen
    assert_eq!(fe.screen_x, 24);
    assert!(fe.action(Action::Back), "the AV page's Back");
    assert_eq!(fe.page, Page::CnMenu);
    assert_eq!(fe.screen_x, 12, "Back restores psiSetScreenPos(screen_adjust_x, …) [M 0x214818:82]");
    assert_eq!(fe.screen_y, 4);
}

/// The Restore Defaults row's full reset list [M 0x214818:97-110]: music 0xe → 70, sfx 0x14 → 100,
/// subtitles per LanguageToLoad (NTSC-U's language 0 → Off), split 0, speaker 1, widescreen 0,
/// psiRestoreScreenPos. Sister: the pre-reset values differ, so a missing arm fails.
#[test]
fn restore_defaults_runs_the_game_reset_list() {
    let mut fe = FrontEnd::new();
    to_av(&mut fe);
    // Push everything away from the reset values first.
    fe.opts.insert(0x1000_0135, 10);
    fe.opts.insert(0x1000_0134, 20);
    fe.opts.insert(0x1000_0136, 1);
    fe.opts.insert(0x1000_019a, 1);
    fe.opts.insert(0x1000_0198, 4);
    fe.opts.insert(0x1000_0223, 1);
    fe.screen_adjust(30, 10);
    assert_eq!((fe.screen_x, fe.screen_y), (360, 40));
    fe.cursor = 7; // the Restore Defaults row (0x100001a3, y=284 [L page data])
    assert!(fe.action(Action::Confirm));
    assert_eq!(fe.opt_value(0x1000_0135), 70, "music slider 0xe = 14 units → volume 70 [M :97]");
    assert_eq!(fe.opt_value(0x1000_0134), 100, "sfx slider 0x14 = 20 units → volume 100 [M :98]");
    assert_eq!(fe.opt_value(0x1000_0136), 0, "subtitles Off — LanguageToLoad 0 is not in the On set [M :99-104]");
    assert_eq!(fe.opt_value(0x1000_019a), 0, "split Horizontal [M :105]");
    assert_eq!(fe.opt_value(0x1000_0198), 1, "speaker Stereo [M :106]");
    assert_eq!(fe.opt_value(0x1000_0223), 0, "widescreen Off [M LAB_00214d88's uVar5 = 0]");
    assert_eq!((fe.screen_x, fe.screen_y), (0, 0), "psiRestoreScreenPos [M :107]");
}

/// The widescreen view aspect [M Camera_CalcViewAngles 0x1baf70:28-32]: 0x3faaaaab = 4/3 clear,
/// 0x3fe38e39 = 16/9 set. Sister: the two values differ, so a broken flag read fails.
#[test]
fn view_aspect_follows_the_widescreen_option() {
    let mut fe = FrontEnd::new();
    assert!((fe.view_aspect() - 4.0 / 3.0).abs() < 1e-6, "the default is 4/3 (0x3faaaaab) [M]");
    assert!((fe.view_aspect() - f32::from_bits(0x3faa_aaab)).abs() < 1e-3);
    fe.opts.insert(0x1000_0223, 1);
    assert!((fe.view_aspect() - 16.0 / 9.0).abs() < 1e-6, "widescreen On writes 16/9 (0x3fe38e39) [M]");
    assert!((fe.view_aspect() - f32::from_bits(0x3fe3_8e39)).abs() < 1e-3);
    fe.opts.insert(0x1000_0223, 0);
    assert!((fe.view_aspect() - 4.0 / 3.0).abs() < 1e-6, "and back");
}

// --- The NF_ISO tests: the disc's own data ---

/// Reads USATxt.dat out of FILES.BIN [M Txt_LoadLanguage 0x1caad0] and binds through Lang.
fn iso_lang(iso_path: &str) -> Option<nf_formats::hud::Lang> {
    let iso = nf_formats::iso9660::IsoImage::open(iso_path).ok()?;
    let (entries, bin) = nf_formats::disc::files_bin_entries(&iso).ok()?;
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case("USATxt.dat"))?;
    let bytes = iso.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).ok()?;
    nf_formats::hud::Lang::parse(&bytes).ok()
}

/// The menu-def's raw bytes [L the same extraction menu_movies.rs walks].
fn menu_def(iso_path: &str) -> Option<Vec<u8>> {
    let iso = nf_formats::iso9660::IsoImage::open(iso_path).ok()?;
    let (entries, bin) = nf_formats::disc::files_bin_entries(&iso).ok()?;
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case("07000048.BIN"))?;
    let data = iso.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).ok()?;
    let lb = nf_formats::levelbin::LevelBin::parse(&data).ok()?;
    let e = lb.entries.iter().find(|e| e.kind == 8 && e.name == "08000002")?;
    Some(lb.data(&data, e).to_vec())
}

/// One control record of the walk below (the fields the assertions read).
struct Ctrl { name: u32, d1: u32, d2: u32, d3: u32, msgs: Vec<(u8, u32, u32)> }

/// Walks the tag stream like `MenuManager_Create` 0x21b908 (the same record sizes menu_movies.rs
/// pins) and returns page 0x40000047's controls.
fn page_40000047_controls(def: &[u8]) -> Vec<Ctrl> {
    let mut p = 4usize; // the prologue dword [M 0x21b908]
    let rd = |p: &mut usize, n: usize| -> u32 { let mut v = 0u32; for k in 0..n { v |= (def[*p + k] as u32) << (8 * k); } *p += n; v };
    let mut in_page = false;
    let mut out = Vec::new();
    while p + 4 <= def.len() {
        let tag = rd(&mut p, 4);
        match tag {
            0xfffffff0 => { let _id = rd(&mut p, 4); let _h = rd(&mut p, 4); let _aux = rd(&mut p, 4); let _k = rd(&mut p, 1); let _size = rd(&mut p, 4); in_page = _id == 0x4000_0047; }
            0xfffffff9 => {
                let name = rd(&mut p, 4); let _b = rd(&mut p, 1); let _t = rd(&mut p, 1);
                p += 2 + 2 + 2 + 2; // x, y, w, h
                let _f1 = rd(&mut p, 2); let d1 = rd(&mut p, 4); let _f2 = rd(&mut p, 2); let d2 = rd(&mut p, 4); let d3 = rd(&mut p, 4);
                if in_page { out.push(Ctrl { name, d1, d2, d3, msgs: Vec::new() }); }
            }
            0xfffffff8 => p += 0x20,
            0xfffffff7 => p += 8,
            0xfffffff6 => { let m = rd(&mut p, 1); let a = rd(&mut p, 4); let b = rd(&mut p, 4); if in_page { if let Some(c) = out.last_mut() { c.msgs.push((m as u8, a, b)); } } }
            0xfffffff5 => p += 14,
            0xfffffff4 => p += 13,
            0xfffffff3 => p += 9,
            0xfffffffb => p += 32,
            0xfffffffc => p += 14,
            0xfffffffd => p += 2,
            0xfffffffe => p += 4,
            0xfffffff2 => break,
            other => panic!("unexpected menu-def tag {other:#x} (the walk must match 0x21b908)"),
        }
    }
    out
}

/// The Screen Adjust page's own data [L page 0x40000047 walk 2026-10-15]: two sliders named
/// 0x100001eb distinguished by d1 (0 vertical, 1 horizontal), d3 0xa; the fullscreen picture control
/// with TEX 0x3000074. Sister: the count and the d1 pair — a missing or extra bar fails.
#[test]
fn the_menu_def_screen_adjust_page_carries_two_sliders_and_the_picture() {
    let Some(iso) = std::env::var_os("NF_ISO").map(|s| s.into_string().unwrap()) else { eprintln!("skip: NF_ISO not set"); return };
    let Some(def) = menu_def(&iso) else { eprintln!("skip: no menu-def"); return };
    let ctrls = page_40000047_controls(&def);
    let sliders: Vec<&Ctrl> = ctrls.iter().filter(|c| c.name == 0x1000_01eb).collect();
    assert_eq!(sliders.len(), 2, "the page has exactly the two slider bars [L]");
    let mut d1s: Vec<u32> = sliders.iter().map(|c| c.d1).collect();
    d1s.sort();
    assert_eq!(d1s, vec![0, 1], "d1 0 = the vertical bar, d1 1 = the horizontal bar [L]");
    assert!(sliders.iter().all(|c| c.d3 == 0xa), "both sliders carry d3 0xa [L]");
    // The spring: C_SBSCREEN's msg-0x51 arm sends 0x27 (−1, 1) — the slider range [M 0x217b08:22-25];
    // the picture control sets TEX 0x3000074 [L the page's msg-0x24 control].
    let picture = ctrls.iter().find(|c| c.msgs.iter().any(|&(m, a, _)| m == 0x24 && a == 0x0300_0074));
    assert!(picture.is_some(), "the reference picture control (TEX 0x3000074) is on the page [L]");
    let aux_none = ctrls.iter().filter(|c| c.name == 0x1000_01eb).all(|c| c.msgs.iter().any(|&(m, a, _)| m == 0x2e));
    assert!(aux_none, "both bars carry an initial msg 0x2e value [L]");
}

/// The labels the flow and the pages bind resolve from the disc [D USATxt.dat]. Sister: the exact
/// strings — a wrong id or a truncated bank fails on the comparison, not on presence.
#[test]
fn the_screen_adjust_labels_resolve_from_the_disc() {
    let Some(iso) = std::env::var_os("NF_ISO").map(|s| s.into_string().unwrap()) else { eprintln!("skip: NF_ISO not set"); return };
    let Some(lang) = iso_lang(&iso) else { eprintln!("skip: no USATxt.dat"); return };
    let b = |id: u32| lang.bind(id).map(str::to_string).unwrap_or_default();
    assert_eq!(b(0x2b6), "Screen Adjust", "the AV row 0x10000137's label [M 0x214818:117-126 opens 0x40000047]");
    assert_eq!(b(0x37e), "Restore Defaults", "the AV row 0x100001a3's label [M :96-110]");
    assert_eq!(b(0x198), "Credits", "the AV row 0x10000138's label — NOT Screen Adjust");
    assert_eq!(b(0x550), "Die Another Day Trailer", "the AV row 0x10000222's label");
    assert_eq!(b(0x395), "Surround", "the speaker list's third item (the port's old 'Pro Logic' literal was wrong)");
    assert_eq!(b(0x67b), "~A Confirm  ~B Cancel", "the Screen Adjust page's hint [L page 0x40000047]");
    assert_eq!(b(0x2b8), "Multiplayer Split Screen", "the split row 0x1000019a's label");
    assert_eq!(b(0x2ba), "Horizontal", "split value 0's label [M 0x214818:105 resets to 0]");
    assert_eq!(b(0x2b9), "Vertical", "split value 1's label");
}

/// The widescreen aspect reaches the modelled viewer: `Sim.view_aspect` defaults to 4/3
/// ([L ee_start_05 viewer+0x114 = 0x3faaaaab]) and driving it to 16/9 turns the gun-tilt term
/// (`gun_matrix`'s gy — the one consumer of viewer+0x114 the port models). Sister: the two
/// matrices differ, so a field that nothing reads fails.
#[test]
fn sim_view_aspect_defaults_to_4_3_and_drives_the_gun_tilt() {
    let Some(iso) = std::env::var_os("NF_ISO").map(|s| s.into_string().unwrap()) else { eprintln!("skip: NF_ISO not set"); return };
    let mut scene = match nf_view::load(&iso, "07000002.bin", false) { Ok(s) => s, Err(e) => { eprintln!("skip: {e}"); return } };
    let mut sim = scene.sim.take().expect("sim");
    assert!((sim.view_aspect - 4.0 / 3.0).abs() < 1e-6, "the default viewer+0x114 is 4/3 [M 0x1baf70:29-31]");
    sim.free_aim = true;     // crosshair() passes the raw offsets through [M Player_Aiming]
    sim.cross = [0.5, 0.25]; // a nonzero aim so the gy term is live
    let m43 = sim.gun_matrix();
    sim.view_aspect = 16.0 / 9.0;
    let m169 = sim.gun_matrix();
    let differs = (0..4).any(|r| (0..4).any(|c| (m43[r][c] - m169[r][c]).abs() > 1e-6));
    assert!(differs, "the 16/9 aspect turns the gun-tilt x term (the gy gy.cos() rotation) [M 0x1baf70:33-41]");
}
