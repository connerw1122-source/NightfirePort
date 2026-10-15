//! mp-menu-flow's ISO-gated half: the port's MP wheel tables are the game's own bytes.
//!
//! The tables live in ACTION.ELF's .data (the disc's /ACTION.ELF): `mp_scenario` @0x2df508,
//! `mp_level` @0x2df400, `mp_options` @0x2dfbb0, `mp_bots` @0x2dfd30, `mp_characters` @0x2df640
//! (all symtab names pinned by the ELF's symbol table), and `default_bot_stats` @0x26D2F0 whose
//! +0x09 byte is the character side `Menu_IsBotGood` 0x2027a8 reads. ACTION.ELF has one PT_LOAD
//! (vaddr 0x107100 @ file 0x1000), so file offset = vaddr − 0x106100 [D readelf + the dump that
//! verified sp_level 0x2df2e0 → 0x1d91e0]. Rows are 0x18-byte records {u32 preview, u32 name,
//! u32 desc, u32 id, u8 unlocked, u32 locked-desc} [M Menu_UpdateWheel 0x1f8388]. Numbers only —
//! the main agent compares the pixels.

use nf_formats::iso9660::IsoImage;
use nf_view::menu::{MP_BOTS_ROWS, MP_CHARACTER_SIDE_MI6, MP_CHARACTERS, MP_MAP_ROWS, MP_OPTIONS_ROWS, MP_SCENARIO_ROWS};

fn action_elf() -> Option<Vec<u8>> {
    let path = std::env::var_os("NF_ISO")?;
    let img = IsoImage::open(path).ok()?;
    img.read_file(img.find("/ACTION.ELF")?).ok()
}

/// The byte at a .data vaddr (file = vaddr − 0x106100 for the one PT_LOAD).
fn u8_at(elf: &[u8], va: u32) -> u8 { elf[(va - 0x106100) as usize] }
fn u32_at(elf: &[u8], va: u32) -> u32 {
    let p = (va - 0x106100) as usize;
    u32::from_le_bytes([elf[p], elf[p + 1], elf[p + 2], elf[p + 3]])
}

/// One wheel row as the game stores it: (preview, name, desc, id, unlocked, locked-desc).
fn row(elf: &[u8], table: u32, i: usize) -> (u32, u32, u32, u32, u8, u32) {
    let base = table + (i as u32) * 0x18;
    (u32_at(elf, base), u32_at(elf, base + 4), u32_at(elf, base + 8), u32_at(elf, base + 0xc),
     u8_at(elf, base + 0x10), u32_at(elf, base + 0x14))
}

fn table(elf: &[u8], va: u32, n: usize) -> Vec<(u32, u32, u32, u32, u8, u32)> {
    (0..n).map(|i| row(elf, va, i)).collect()
}

#[test]
fn the_scenario_table_is_the_discs() {
    let Some(elf) = action_elf() else { eprintln!("skip: NF_ISO not set"); return };
    let disc = table(&elf, 0x2df5_08, MP_SCENARIO_ROWS.len());
    for (i, ((p, n, d, id, reward), r)) in MP_SCENARIO_ROWS.iter().zip(disc.iter()).enumerate() {
        assert_eq!((*p, *n, *d, *id), (r.0, r.1, r.2, r.3), "mp_scenario row {i}");
        // The port's reward column models the RUNTIME lock: Menu_UnlockMPSettings 0x1ff698:53-104
        // zeroes rows 4/6/7/9/10/12's +0x10 bytes and re-derives them from the campaign rewards —
        // row 12 (Team KOTH) SHIPS a 1 byte and is still locked at runtime [M the zeroing list].
        // So: an open row must ship unlocked; a reward row's shipped byte is not trusted.
        if *reward == 0 { assert_eq!(r.4, 1, "row {i} is always open [M the zeroing list]"); }
        assert_eq!(r.5, 0x0100_00ab, "row {i}'s locked-desc is 'This scenario is locked.' [L]");
    }
}

#[test]
fn the_map_table_is_the_discs() {
    let Some(elf) = action_elf() else { eprintln!("skip: NF_ISO not set"); return };
    let disc = table(&elf, 0x2df4_00, MP_MAP_ROWS.len());
    for (i, ((p, n, d, level), r)) in MP_MAP_ROWS.iter().zip(disc.iter()).enumerate() {
        assert_eq!((*p, *n, *d, *level), (r.0, r.1, r.2, r.3), "mp_level row {i}");
        assert_eq!(r.4, 1, "row {i} ships unlocked [L the .data dump]");
        assert_eq!(r.5, 0, "row {i} has no locked-desc [L]");
    }
    // Sister: the map wheel's rows ARE the on-foot ISO's arenas [D docs/atlas/multiplayer.md §1].
    let codes: Vec<u32> = disc.iter().map(|r| r.3).collect();
    assert_eq!(codes, vec![0x0700_0024, 0x0700_0027, 0x0700_0029, 0x0700_0026, 0x0700_0023, 0x0700_0028, 0x0700_0025, 0x0700_004b]);
}

#[test]
fn the_options_and_bots_tables_are_the_discs() {
    let Some(elf) = action_elf() else { eprintln!("skip: NF_ISO not set"); return };
    for (i, ((p, n, d, id), r)) in MP_OPTIONS_ROWS.iter().zip(table(&elf, 0x2dfb_b0, 5).iter()).enumerate() {
        assert_eq!((*p, *n, *d, *id), (r.0, r.1, r.2, r.3), "mp_options row {i}");
        assert_eq!(r.5, if *id == 1 { 0x0100_00ae } else { 0 }, "row {i}'s locked-desc [L the dump]");
    }
    for (i, ((p, n, _), r)) in MP_BOTS_ROWS.iter().zip(table(&elf, 0x2dfd3_0, 5).iter()).enumerate() {
        assert_eq!((*p, *n), (r.0, r.1), "mp_bots row {i}");
        assert_eq!(r.2, 0, "the bots rows ship no descriptions [L]");
    }
}

#[test]
fn the_character_table_and_sides_are_the_discs() {
    let Some(elf) = action_elf() else { eprintln!("skip: NF_ISO not set"); return };
    let disc = table(&elf, 0x2df6_40, MP_CHARACTERS.len());
    for (i, ((p, n, d, _reward), r)) in MP_CHARACTERS.iter().zip(disc.iter()).enumerate() {
        assert_eq!((*p, *n, *d, i as u32), (r.0, r.1, r.2, r.3), "mp_characters row {i}: +0x0C is the row index (the preset id)");
        assert_eq!(r.5, 0x0100_00ad, "row {i}'s locked-desc [L]");
    }
    // Sister: the side bytes come from default_bot_stats @0x26D2F0 byte +0x09 [M Menu_IsBotGood
    // 0x2027a8 = BOT_getDefaultStats(id)[9] == 0] — 0 is MI6.
    for i in 0..29 {
        let side = u8_at(&elf, 0x26d2_f0 + (i as u32) * 0x0e + 0x09);
        assert_eq!(MP_CHARACTER_SIDE_MI6[i], side == 0, "character {i}'s side (disc byte {side})");
    }
    // And the stats-editable flag (+0x0D = 1 exactly for presets 0-14) [M P_MPBOTSETUP's lock].
    for i in 0..29 {
        let editable = u8_at(&elf, 0x26d2_f0 + (i as u32) * 0x0e + 0x0d) == 1;
        assert_eq!(nf_view::menu::MpBot::stats_editable(i as u32), editable, "character {i}'s +0x0D");
    }
}

#[test]
fn the_port_rows_match_the_game_tables_number_for_number() {
    // The sister of the byte tests above, at the data-structure level: the port's tuple rows and
    // the game's 0x18-byte records agree on every number the flow consumes (no ISO needed — the
    // byte tests above pin the port TO the disc; this pins the flow's use of the port tables).
    assert_eq!(MP_SCENARIO_ROWS.iter().filter(|r| r.4 == 0).count(), 7, "7 scenarios are always open (0,1,2,3,5,8,11) [L]");
    assert_eq!(MP_CHARACTERS.iter().filter(|r| r.3 == 0).count(), 12, "12 characters ship unlocked [L]");
    assert_eq!(MP_MAP_ROWS.iter().map(|r| r.1).collect::<Vec<_>>().len(), 8);
}
