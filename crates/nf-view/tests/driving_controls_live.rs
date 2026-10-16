//! The driving controls table's labels, read from the real disc.
//!
//! `PC_DRIVING_CONTROLS` (crates/nf-view/src/menu.rs, the driving counterpart of `PC_CONTROLS`)
//! names each row with the game's own action label id from USATxt.dat. This test binds every id
//! from the disc (`$NF_ISO`-gated, the `Lang::parse`/`Lang::bind` path the menus use
//! [M Txt_BindLabel 0x1cad60 = fixups[id>>24] + (id & 0xffffff)]) and asserts the exact words of
//! the driving controls block (bank 0, ids 0x30a-0x315) — with sister assertions (wrong-word
//! negatives) that can fail if the block ever moves or a row is mis-assigned.

use nf_view::menu::PC_DRIVING_CONTROLS;

/// Reads USATxt.dat out of FILES.BIN [M Txt_LoadLanguage 0x1caad0 loads FNames[SelLang]].
fn iso_lang(iso_path: &str) -> Option<nf_formats::hud::Lang> {
    let iso = nf_formats::iso9660::IsoImage::open(iso_path).ok()?;
    let (entries, bin) = nf_formats::disc::files_bin_entries(&iso).ok()?;
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case("USATxt.dat"))?;
    let bytes = iso.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).ok()?;
    nf_formats::hud::Lang::parse(&bytes).ok()
}

/// The rows' expected game words: (label id, exact text). The two id-0 rows are the port-only
/// fallbacks (no driving label exists for a secondary fire or a reload) and are asserted
/// separately.
const EXPECTED: [(u32, &str); 11] = [
    (0x30c, "Steer Left-Right"),
    (0x30a, "Gas"),
    (0x100_032a, "Brake/Reverse"),
    (0x314, "E-Brake"),
    (0x30d, "Fire"),
    (0x310, "Next Weapon"),
    (0x30f, "Previous Weapon"),
    (0x311, "Use Gadget"),
    (0x313, "Camera View"),
    (0x312, "Look back"),
    (0x315, "Pause"),
];

#[test]
fn the_driving_controls_rows_bind_the_game_s_own_words() {
    let Some(iso) = std::env::var("NF_ISO").ok() else { return };
    let Some(lang) = iso_lang(&iso) else { panic!("USATxt.dat must parse — the menus are drawn from it") };
    for (id, word) in EXPECTED {
        let bound = lang.bind(id).unwrap_or_else(|| panic!("label {id:#x} did not resolve from USATxt.dat"));
        assert_eq!(bound, word, "label {id:#x} must be the driving block's own word");
    }
    // Sisters that can fail: the neighbours must NOT bleed into the rows — 0x30a is "Gas", not
    // "Brake" (0x30b is), and 0x30d is "Fire", not "Use Gadget" (0x311 is).
    assert_ne!(lang.bind(0x30a), Some("Brake"));
    assert_eq!(lang.bind(0x30b), Some("Brake"), "the block's alignment check: 0x30b is Brake");
    assert_ne!(lang.bind(0x30d), Some("Use Gadget"));
    // The unused "Cycle Weapon " row keeps its trailing space [M the table's bytes].
    assert_eq!(lang.bind(0x30e), Some("Cycle Weapon "));
    // The port-only rows: id 0 + their fallback text, exactly like the on-foot table's id-0 rows.
    let port_only: Vec<&str> = PC_DRIVING_CONTROLS.iter().filter(|(_, id, _)| *id == 0).map(|(_, _, fb)| *fb).collect();
    assert_eq!(port_only, vec!["Alt. Fire", "Reload"]);
    // Every non-zero row's id must be one of the asserted driving-block ids (no stray ids).
    for (_, id, _) in PC_DRIVING_CONTROLS.iter().filter(|(_, id, _)| *id != 0) {
        assert!(EXPECTED.iter().any(|(e, _)| e == id), "label {id:#x} is not part of the driving block");
    }
}
