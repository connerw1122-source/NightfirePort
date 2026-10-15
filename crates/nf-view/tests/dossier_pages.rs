//! dossier-pages-data tests: the Dossier's Rewards / Gadgets / Weapons pages — the lists, the
//! unlock gates, the flow and the upgrade levels.
//!
//! Evidence (docs/systems/dossier-pages-data.md): the handlers P_DSREWARDS 0x2172c8 +
//! C_RBDSREWARDS 0x20c248, P_DSGADGETS 0x217190 + C_SBDSGTSCROLL 0x20bcc0, P_DSWEAPONS 0x217038 +
//! C_SBDSWPSCROLL 0x20bab0; the tables ds_weapons 0x2dfec8 / ds_gadgets 0x2e0150 (24 B rows:
//! preview +0, wheel label +4, description +8, upgrade object +0xc) and RewardsTable 0x2bc920
//! (64 B entries: descriptors +4..+0x14, bonus captions +0x1c..+0x28, stamp textures +0x30..+0x3c);
//! the scroll rules of Scroll_Update 0x2221b20 / Scroll_SendMessage 0x222810; the upgrade walk of
//! Menu_GetObjectUpgradeLevel 0x2025f8. Disc reads are ISO-gated (NF_ISO), each with a sister
//! assertion that runs headless.

use nf_formats::iso9660::IsoImage;
use nf_view::menu::{self, Action, FrontEnd, Page};

fn iso() -> Option<IsoImage> {
    let path = std::env::var("NF_ISO").ok()?;
    Some(IsoImage::open(path).unwrap())
}

// --- the flow: the Dossier wheel's rows 1-3 open the pages [M C_SBDOSSIER 0x216eb0:12-31] ---

#[test]
fn the_dossier_wheels_rows_open_the_three_pages() {
    for (row, want, id) in [(1usize, Page::Rewards, 0x4000_003bu32), (2, Page::Gadgets, 0x4000_003c), (3, Page::Weapons, 0x4000_003d)] {
        let mut fe = FrontEnd::new();
        fe.page = Page::Dossier;
        fe.cursor = row;
        assert!(fe.action(Action::Confirm), "row {row} confirm opens its page");
        assert_eq!(fe.page, want, "the wheel's row {row} opens its page [M 0x216eb0:12-31]");
        assert_eq!(fe.page.id(), id);
        assert_eq!(fe.cursor, 0, "the page's init msg 0x2e (0,0) resets the scroll [D page data]");
        // Back returns to the Dossier (the Records page's own mapping).
        assert!(fe.action(Action::Back));
        assert_eq!(fe.page, Page::Dossier);
    }
}

// --- the scroll: clamp, no wrap [M Scroll_Update 0x2221b20:186-210 — +0x144 never set] ---

#[test]
fn the_three_pages_scroll_with_clamp_and_their_own_row_counts() {
    let mut fe = FrontEnd::new();
    fe.page = Page::Rewards;
    assert_eq!(fe.rows(), 12, "the Rewards list is the 12 sp_level rows [M P_DSREWARDS 0x2172c8:11-13]");
    assert!(!fe.action(Action::Up), "Up at row 0 is a no-op [M 0x2221b20:202-210]");
    assert!(fe.action(Action::Down) && fe.cursor == 1);
    fe.cursor = 11;
    assert!(!fe.action(Action::Down), "Down at the last row clamps [M 0x2221b20:186-194]");
    assert!(fe.action(Action::Up) && fe.cursor == 10);

    fe.page = Page::Gadgets;
    fe.cursor = 0; // the page's own init msg 0x2e (0,0) [D page data]
    assert_eq!(fe.rows(), 14, "the gadgets wheel's range is [0, 0xd] [M C_SBDSGTSCROLL 0x20bcc0:64-67]");
    assert!(!fe.action(Action::Up));
    fe.cursor = 13;
    assert!(!fe.action(Action::Down), "no wrap at the gadgets' last row");
    assert!(fe.action(Action::Back) && fe.page == Page::Dossier);

    fe.page = Page::Weapons;
    fe.cursor = 0;
    assert_eq!(fe.rows(), 27, "the weapons wheel's range is [0, 0x1a] [M C_SBDSWPSCROLL 0x20bab0:64-68]");
    assert!(!fe.action(Action::Confirm), "Confirm does nothing — no handler arm [M 0x20bab0]");
    fe.cursor = 26;
    assert!(!fe.action(Action::Down), "no wrap at the weapons' last row");
    // Sister (headless): the wheel rows bind the tables' +0x04 labels.
    assert_eq!(menu::GADGETS_TABLE.len(), 14);
    assert_eq!(menu::WEAPONS_TABLE.len(), 27);
}

// --- the Rewards rows follow the medals with the cumulative gates [M C_RBDSREWARDS 0x20c248:38-74] ---

#[test]
fn the_rewards_rows_follow_the_medal_flags() {
    let mut fe = FrontEnd::new();
    fe.page = Page::Rewards;
    // Row 1 of the campaign (0x07000005): [L RewardsTable entry 1 — captions 0x2d2/0x2e9/0x3a5/0x2f1,
    // stamps 0x03000134/0x03000116/0x030000f3/0x03000108].
    for (shown, _, _) in fe.reward_rows(1) {
        assert!(!shown, "a fresh profile shows no bonus rows [M the handler's 0x2b/1 first]");
    }
    let mut p = nf_game::profile::Profile::default_codename("BOND"); // [M the template's fresh progress]
    p.progress.set_level_bonus(0x0700_0005, 1, 0); // bronze only
    fe.profile = Some(p);
    let rows = fe.reward_rows(1);
    assert!(rows[0].0 && rows[0].1 == 0x2d2 && rows[0].2 == 0x0300_0134, "bronze shows row 1 [M :31-37]");
    assert!(!rows[1].0 && !rows[2].0 && !rows[3].0, "silver+ need their own medals [M the flag chain]");
    // Sister: the gate comes from the port's own medal bits (bit 26 = 07000005's bronze [L
    // bonus[0] = 0x4004000000 in the medal-bits test's evidence]).
    assert!(fe.profile.as_ref().unwrap().progress.has_medal(0x0700_0005, 1, 0));
    fe.profile.as_mut().unwrap().progress.set_level_bonus(0x0700_0005, 2, 0);
    let rows = fe.reward_rows(1);
    assert!(rows[1].0 && rows[1].1 == 0x2e9 && rows[1].2 == 0x0300_0116, "bronze+silver show row 2");
    assert!(!rows[2].0, "gold still hidden");
    // Another level's rows stay hidden: level 0's table row has its own captions [L entry 0].
    assert!(!fe.reward_rows(0)[0].0, "0x09000001 has no medals in this profile");
}

// --- the wheel rows carry the game's tables and the upgrade captions ---

#[test]
fn the_wheel_rows_match_the_game_tables_and_upgrade_captions() {
    let mut fe = FrontEnd::new();
    // Gadget row 0 [L ds_gadgets[0] = 0x030000e6/0x323/0x331/0x12]: no medals → no caption.
    let (preview, name, desc, cap, _) = fe.gadget_row(0);
    assert_eq!((preview, name, desc), (0x0300_00e6, 0x323, 0x331));
    assert_eq!(cap, None, "upgrade level 0 → no caption [M 0x20bcc0:80 `if (lVar3 == 0) goto`]");
    // Weapon row 0 is the overview [M P_DSWEAPONS 0x217038:18-23 level 0 → 0x030000f3/0x33d/0x359].
    assert_eq!(fe.weapon_row(0), (0x0300_00f3, 0x33d, 0x359, None, true));
    // The watch rows carry object 9 [L ds_weapons[9]/[10] +0x0c = 9]; level 0 → no caption.
    assert_eq!(fe.weapon_row(9).3, None);
    assert_eq!(menu::gadget_suffix(0x12), Some(0x3b7), "the gadget caption map [M 0x20bcc0:97-99]");
    assert_eq!(menu::gadget_suffix(0x3f), None, "0x3f rows have no upgrade [M the switch's default]");
    assert_eq!(menu::weapon_suffix(9, 1), Some(0x3b1), "the watch captions [M 0x20bab0:81-88]");
    assert_eq!(menu::weapon_suffix(9, 3), None, "level 3 adds NO caption [M the if/else-if chain]");
}

// --- the upgrade level is the gold group's set-bit count [M Menu_GetObjectUpgradeLevel 0x2025f8] ---

#[test]
fn the_upgrade_level_counts_the_kind4_group() {
    let mut p = nf_game::progress::Progress::default();
    assert_eq!(p.object_upgrade_level(0, 0), 0, "fresh profile: the watch is un-upgraded");
    assert_eq!(p.object_upgrade_level(0x3f, 0), 0, "object ≥ 0x3f returns 0 [M 0x2025f8:57]");
    // The watch = the 3-member gold group {0x0700005, 0x0700009, 0x0700011} on bits 0..2 [M
    // 0x199210:39-62 — the group scan; the test comment "spreads bits 0..2" in medal_bits].
    p.set_level_bonus(0x0700_0005, 4, 0);
    assert_eq!(p.object_upgrade_level(0, 0), 1, "one group member earned");
    assert!(p.has_medal(0x0700_0005, 3, 0), "the gold bit is the group's bit 0");
    p.set_level_bonus(0x0700_0009, 4, 0);
    p.set_level_bonus(0x0700_0011, 4, 0);
    assert_eq!(p.object_upgrade_level(0, 0), 3, "all three members → watch level 3 [M 0x217038's 0..3 arms]");
    // A singleton gold group: gadget object 0x12 = 0x0900003's kind-4 bit 18 [L REWARDS_TABLE entry 9].
    p.set_level_bonus(0x0900_0003, 3, 0);
    assert_eq!(p.object_upgrade_level(0x12, 0), 1, "the laser's own gold medal is upgrade level 1");
    // The weapons' watch-pair group {0x0700001, 0x070001b} on bits 9/10 [L entries 4/11 desc[3]].
    p.set_level_bonus(0x0700_0001, 3, 0);
    assert_eq!(p.object_upgrade_level(9, 0), 1);
    p.set_level_bonus(0x0700_001b, 3, 0);
    assert_eq!(p.object_upgrade_level(9, 0), 2, "two members → weapon_suffix level 2 [M 0x20bab0:85-88]");
    assert_eq!(menu::weapon_suffix(9, p.object_upgrade_level(9, 0)), Some(0x3b2));
}

// --- the disc check: the port's statics ARE the disc's .data bytes [D ACTION.ELF] ---

/// ACTION.ELF's loaded image from the disc, with a vaddr reader (the same symbol walk
/// `filelist::from_action_elf` uses).
fn disc_reader(iso: &IsoImage) -> impl Fn(u32) -> u32 + '_ {
    let elf_bytes = iso.read_file(iso.find("/ACTION.ELF").expect("ACTION.ELF on the disc")).expect("read");
    let elf = nf_formats::elf::Elf::parse(&elf_bytes).expect("parse ACTION.ELF");
    let segs: Vec<(u32, Vec<u8>)> = elf.load_segments(&elf_bytes).expect("load segments")
        .into_iter().map(|(va, d, _)| (va, d.to_vec())).collect();
    move |addr: u32| -> u32 {
        for (vaddr, data) in &segs {
            if addr >= *vaddr && (addr - vaddr) as usize + 4 <= data.len() {
                let off = (addr - vaddr) as usize;
                return u32::from_le_bytes(data[off..off + 4].try_into().unwrap());
            }
        }
        panic!("address {addr:#x} outside the loaded segments");
    }
}

#[test]
fn the_port_tables_match_the_discs_action_elf_data() {
    let Some(iso) = iso() else { eprintln!("skip: NF_ISO not set"); return };
    let read = disc_reader(&iso);
    // ds_weapons 0x2dfec8, ds_gadgets 0x2e0150: 0x18-byte rows {preview, label, desc, object}.
    for (i, &(preview, label, desc, object)) in menu::WEAPONS_TABLE.iter().enumerate() {
        let a = 0x2dfe_c8 + (i as u32) * 0x18;
        assert_eq!(read(a), preview, "ds_weapons[{i}] +0x00 preview");
        assert_eq!(read(a + 4), label, "ds_weapons[{i}] +0x04 label");
        assert_eq!(read(a + 8), desc, "ds_weapons[{i}] +0x08 desc");
        assert_eq!(read(a + 0xc), object, "ds_weapons[{i}] +0x0c object");
    }
    for (i, &(preview, label, desc, object)) in menu::GADGETS_TABLE.iter().enumerate() {
        let a = 0x2e0_150 + (i as u32) * 0x18;
        assert_eq!((read(a), read(a + 4), read(a + 8), read(a + 0xc)), (preview, label, desc, object), "ds_gadgets[{i}]");
    }
    // RewardsTable 0x2bc920: +0x1c..+0x28 captions, +0x30..+0x3c stamps [M 0x1ffe98's loads].
    for (i, row) in menu::REWARD_BONUS.iter().enumerate() {
        let a = 0x2bc_920 + (i as u32) * 0x40;
        for (k, &(label, tex)) in row.iter().enumerate() {
            assert_eq!(read(a + 0x1c + 4 * k as u32), label, "RewardsTable[{i}] caption {k}");
            assert_eq!(read(a + 0x30 + 4 * k as u32), tex, "RewardsTable[{i}] stamp {k}");
        }
    }
}

// --- the language bank check: the pages' own labels bind [D USATxt.dat] ---

#[test]
fn the_pages_labels_bind_from_the_language_bank() {
    let Some(path) = std::env::var("NF_ISO").ok() else { eprintln!("skip: NF_ISO not set"); return };
    let bin = {
        let iso = IsoImage::open(&path).unwrap();
        let (entries, b) = nf_formats::disc::files_bin_entries(&iso).unwrap();
        let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case("USATxt.dat")).expect("USATxt.dat");
        iso.read_bytes(b.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).unwrap()
    };
    let lang = nf_formats::hud::Lang::parse(&bin).expect("parse USATxt.dat");
    // Headers 0x281/0x282/0x19c, hints 0x49e/0x49f [D page data]; the tables' own labels.
    for id in [0x281u32, 0x282, 0x19c, 0x49e, 0x49f] {
        assert!(lang.bind(id).map_or("", |s| s).len() > 0, "label {id:#x} binds");
    }
    for &(id, _) in menu::REWARD_BONUS[1].iter() {
        assert!(lang.bind(id).is_some(), "RewardsTable entry 1's caption {id:#x} binds");
    }
    for &(_, label, _, _) in menu::GADGETS_TABLE.iter() {
        assert!(lang.bind(label).is_some(), "gadget label {label:#x} binds");
    }
    for &(_, label, desc, _) in menu::WEAPONS_TABLE.iter() {
        assert!(lang.bind(label).is_some() && lang.bind(desc).is_some(), "weapon {label:#x}/{desc:#x} bind");
    }
    // Sister: the upgrade captions exist too [M 0x3b1..0x3b8 the sprintf labels].
    for id in [0x3b1u32, 0x3b2, 0x3b3, 0x3b4, 0x3b5, 0x3b6, 0x3b7, 0x3b8] {
        assert!(lang.bind(id).is_some(), "upgrade caption {id:#x} binds");
    }
}
