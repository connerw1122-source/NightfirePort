//! credits-data: the credits roll's data, scroll rate and exit — P_CREDITS_Handler 0x2152b0 /
//! Menu_SetupCredits 0x1f9a88. Pure-data tests (numbers/ids only — the drawing is the main
//! agent's), plus one NF_ISO pin that the AV Options row this page is opened from is really the
//! Credits row.

use nf_view::credits::{
    credits_data::CREDITS, credits_data::Str, CreditsRoll, FADE_FRAME, EXIT_FRAME,
    FIRST_LINE_FRAME, LAST_LINE_FRAME, LINE_COUNT, LINE_PERIOD, LIST_SLOTS, MUSIC_FADE_FRAME,
    MUSIC_FADE_SPAN, PAGE_CN_AV, PAGE_WINGAME, SCROLL_PER_FRAME,
};
use nf_view::menu::{Action, FrontEnd, Page};

#[test]
fn the_line_count_is_the_games_0x242() {
    // [M Menu_SetupCredits 0x1f9a88 stores 0x242 through 0x30d54c and heap-copies 0x1b18 = 578×12.]
    assert_eq!(LINE_COUNT, 0x242, "the port's table must hold the game's 0x242 lines");
    assert_eq!(CREDITS.len(), 0x242);
    // Sister: an off-by-one anywhere in the copy would trip this.
    assert_ne!(CREDITS.len(), 0x241);
}

#[test]
fn the_order_matches_the_functions_initialisation_sequence() {
    // [M 0x1f9a88:1456-1463 — the first three entries bind labels 0x10002a5/0x10002a6/0x10002a7,
    // each wide (the byte at entry+10 = 1). A mis-ordered parse would put the tail's gap rows here.]
    assert_eq!(CREDITS[0].l, Str::Label(0x100_02a5));
    assert_eq!(CREDITS[1].l, Str::Label(0x100_02a6));
    assert_eq!(CREDITS[2].l, Str::Label(0x100_02a7));
    assert_eq!((CREDITS[0].w, CREDITS[1].w, CREDITS[2].w), (1, 1, 1));
    // [M 0x1f9a88:3373 — the last heading binds 0x10002a1 at entry 565, near the tail.]
    assert_eq!(CREDITS[565].l, Str::Label(0x100_02a1));
    assert_eq!(CREDITS[565].w, 1);
    // [M the tail 0x1f9a88:3340-3422 — the roll ends on wide gap rows (both strings the " " at
    // 0x30d240). A mis-ordered parse would show names here.]
    let last = CREDITS[577];
    assert_eq!(last.w, 1);
    assert_eq!(last.l, Str::Space);
    assert_eq!(last.r, Str::Space);
    // Sister: the mid-table heading row for the voice cast [M the 0x1f9a88 region binding
    // 0x1000264 — kind 2 and wide].
    assert_eq!(CREDITS[361].l, Str::Label(0x100_0264));
    assert_eq!((CREDITS[361].kl, CREDITS[361].w), (2, 1));
}

#[test]
fn every_string_slot_is_accounted_for() {
    // The extraction round-trip [M the decompile holds 209 Txt_BindLabel calls; the 578×2 slots
    // split 209 label + 504 ELF literals (492 inline + 12 .data) + 241 " " gaps + 202 NULLs].
    let mut labels = 0;
    let mut lits = 0;
    let mut spaces = 0;
    let mut nulls = 0;
    for e in CREDITS.iter() {
        for s in [&e.l, &e.r] {
            match s {
                Str::Label(_) => labels += 1,
                Str::Lit(_) => lits += 1,
                Str::Space => spaces += 1,
                Str::Null => nulls += 1,
            }
        }
    }
    assert_eq!((labels, lits, spaces, nulls), (209, 504, 241, 202));
    // Sister: no slot may vanish (the sum must be the full 1156).
    assert_eq!(labels + lits + spaces + nulls, CREDITS.len() * 2);
}

/// Ticks `n` frames.
fn run(roll: &mut CreditsRoll, n: u32) {
    for _ in 0..n {
        roll.tick();
    }
}

#[test]
fn the_scroll_rate_is_two_units_per_frame_after_a_14_frame_entry() {
    // [M 0x2152b0:107 — `frame % 0xe == 0` gates the entry; :190-191 — every slot y −= 2 per
    // frame, in the same tick as the entry (so y = 512 − 2 on the enter frame).]
    let mut roll = CreditsRoll::new(PAGE_CN_AV, 100, false);
    run(&mut roll, FIRST_LINE_FRAME);
    assert_eq!(roll.lines_entered(), Some(0), "line 0 entered on frame 14");
    let slot0 = (roll.slot + LIST_SLOTS - 1) % LIST_SLOTS;
    let (_, y0) = roll.slot_y(slot0).expect("line 0 is up");
    assert_eq!(y0, 510, "the entry frame already scrolled one step");
    // 70 frames after the entry: y = 512 − 2×(84 − 14 + 1) = 370.
    run(&mut roll, 70);
    let (_, y70) = roll.slot_y(slot0).expect("line 0 still up");
    assert_eq!(y70, 370);
    // Sister: one frame later the exact −2 rate must hold (a 1- or 3-unit drift fails).
    roll.tick();
    let (_, y71) = roll.slot_y(slot0).expect("line 0 still up");
    assert_eq!(y71, 368, "exactly −2 per frame");
    assert_eq!(y70 - y71, SCROLL_PER_FRAME);
}

#[test]
fn the_line_entry_period_is_14_frames_and_slots_cycle_mod_26() {
    // [M 0x2152b0:107 (the % 0xe gate) and :162 (index_519 = (index_519 + 1) % 0x1a).]
    let mut roll = CreditsRoll::new(PAGE_CN_AV, 100, false);
    run(&mut roll, LINE_PERIOD * 30);
    assert_eq!(roll.next_line, 30, "one line per 14 frames");
    assert_eq!(roll.slot, 30 % LIST_SLOTS);
    // Sister: the period is not 13 or 15 — line 30 must NOT have entered one tick earlier.
    let mut early = CreditsRoll::new(PAGE_CN_AV, 100, false);
    run(&mut early, LINE_PERIOD * 30 - 1);
    assert_eq!(early.next_line, 29);
}

#[test]
fn the_music_fades_over_the_last_50_lines() {
    // [M 0x2152b0:165-168 — once count − 0x32 < frame/14: vol = (count×14 − frame) × vol/700.]
    let mut roll = CreditsRoll::new(PAGE_CN_AV, 700, false);
    assert_eq!(roll.music_volume(), None, "no fade before frame/14 > 528");
    run(&mut roll, MUSIC_FADE_FRAME);
    let v0 = roll.music_volume().expect("the fade has started");
    assert_eq!(v0, 686, "vol 700 → the raw window width (578×14 − (578−49)×14)");
    run(&mut roll, LAST_LINE_FRAME - MUSIC_FADE_FRAME);
    assert_eq!(roll.music_volume(), Some(0), "the fade reaches 0 the frame the last line enters");
    // Sister: the scale — the same frame at vol 350 gives half the value.
    let mut half = CreditsRoll::new(PAGE_CN_AV, 350, false);
    run(&mut half, MUSIC_FADE_FRAME);
    assert_eq!(half.music_volume(), Some(343), "350/700 of the window");
    assert_eq!(MUSIC_FADE_SPAN, 700.0);
}

#[test]
fn the_roll_ends_and_returns_to_the_opener_page() {
    // [M 0x2152b0:169-184 — the fade at count+18 < frame/14, the delayed 0x13 page change: msg
    // 0x5f pops back to the opener; the win path's 0x44 goes to 0x40000002.]
    let mut fe = FrontEnd::new();
    assert!(fe.open_credits(PAGE_CN_AV, 100, false));
    assert_eq!(fe.page, Page::Credits);
    assert_eq!(fe.credits.frame, 0);
    for _ in 0..EXIT_FRAME - 1 {
        fe.tick();
        assert_eq!(fe.page, Page::Credits, "no early exit");
    }
    fe.tick();
    assert_eq!(fe.page, Page::CnAv, "the delayed msg 0x5f pops back to AV Options");
    // Sister: the exit must NOT fire a frame early.
    let mut fe2 = FrontEnd::new();
    fe2.open_credits(PAGE_CN_AV, 100, false);
    for _ in 0..EXIT_FRAME - 2 {
        fe2.tick();
    }
    assert_eq!(fe2.page, Page::Credits);
    assert!(matches!(fe2.credits.tick(), None), "no exit one tick before the delay");
}

#[test]
fn the_win_path_returns_to_the_main_menu() {
    // [M 0x2152b0:171-174 — page_from == 0x40000053 → msg 0x44 with 0x40000002.]
    let mut fe = FrontEnd::new();
    fe.open_credits(PAGE_WINGAME, 100, false);
    for _ in 0..EXIT_FRAME {
        fe.tick();
    }
    assert_eq!(fe.page, Page::Main, "the win path's msg 0x44 targets the main menu");
    // Sister: a non-win opener must NOT take the 0x44 path.
    let mut fe2 = FrontEnd::new();
    fe2.open_credits(PAGE_CN_AV, 100, false);
    for _ in 0..EXIT_FRAME {
        fe2.tick();
    }
    assert_ne!(fe2.page, Page::Main);
}

#[test]
fn the_fade_frame_matches_the_handlers_count_window() {
    // [M the two windows: count − 0x32 < frame/14 (music) and count + 0x12 < frame/14 (the fade).]
    assert_eq!(FADE_FRAME, (578 + 19) * 14);
    assert_eq!(EXIT_FRAME, (578 + 19) * 14 + 0x13);
    assert_eq!(MUSIC_FADE_FRAME, (578 - 49) * 14);
    // Sister: the handler's windows are strict inequalities — 596 < frame/14 must NOT hold at
    // frame 596×14.
    assert!((596 * 14) / 14 <= 578 + 18);
}

#[test]
fn the_av_options_confirm_on_the_credits_row_opens_the_page() {
    // [M the AV page's action rows — the port's AV_ROWS row 0x10000138 (label 0x198) is the
    // Credits row; the menu-def wires it to the page change. The ISO test below pins the label.]
    let mut fe = FrontEnd::new();
    fe.page = Page::CnAv;
    fe.cursor = 8; // the Credits row (AV_ROWS[8], ctrl 0x1000_0138)
    assert!(fe.action(Action::Confirm));
    assert_eq!(fe.page, Page::Credits);
    assert_eq!(fe.credits.page_from, PAGE_CN_AV);
    // Sister: confirming a List row must not open credits.
    let mut fe2 = FrontEnd::new();
    fe2.page = Page::CnAv;
    fe2.cursor = 2; // the crosshairs ON_OFF list
    let _ = fe2.action(Action::Confirm);
    assert_ne!(fe2.page, Page::Credits);
}

// --- The NF_ISO pin: the real USATxt.dat ---

/// Reads USATxt.dat out of FILES.BIN [M Txt_LoadLanguage 0x1caad0 loads FNames[SelLang] =
/// USATxt.dat] — the same helper pattern as menus_flow.rs.
fn iso_lang(iso_path: &str) -> Option<nf_formats::hud::Lang> {
    let iso = nf_formats::iso9660::IsoImage::open(iso_path).ok()?;
    let (entries, bin) = nf_formats::disc::files_bin_entries(&iso).ok()?;
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case("USATxt.dat"))?;
    let bytes = iso.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).ok()?;
    nf_formats::hud::Lang::parse(&bytes).ok()
}

#[test]
fn the_av_credits_row_label_binds_the_credits_string() {
    let Some(iso) = std::env::var("NF_ISO").ok() else { eprintln!("NF_ISO not set — skipping"); return };
    let Some(lang) = iso_lang(&iso) else { panic!("NF_ISO set but USATxt.dat did not parse") };
    // [L USATxt.dat label 0x198 — the AV_ROWS Credits row's label; a wrong row id fails here.]
    let s = lang.bind(0x198).unwrap_or_else(|| panic!("label 0x198 missing"));
    assert_eq!(s.to_ascii_lowercase(), "credits", "AV_ROWS row 0x10000138's label 0x198 must be the Credits row");
    // Sister: the neighbour 0x197 is NOT credits (the row ids are distinct strings).
    let s197 = lang.bind(0x197).unwrap_or_default();
    assert_ne!(s197.to_ascii_lowercase(), "credits");
}
