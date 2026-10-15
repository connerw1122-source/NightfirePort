//! fix-ammo-pane: the HUD ammo pane's numbers (`HUD_UpdateAmmoPane` 0x19a7f0 and its create 0x1a0ed8):
//! the "clip-spare" count text (sprite 0), the per-ammo-type bullet-strip icon (sprites 5/6 from
//! `BulletImg.189` 0x2BEA38), the fire-mode label (sprite 1, `weapon_data[w]+0x30 + mode·4`) vs the
//! weapon name (sprite 2, `weapon_data[w]+0x38`), and the `BL+0x95E` switch-timer rule: while a switch
//! request differs from the current weapon (drone+0x63 vs +0x62) and the two records' +2 words differ,
//! the timer is RE-SET to FRAME_RATE (60) every pending frame, then decays; name shown, count text and
//! strip cleared (the `sh $zero,0x20(s6)` delay-slot store at 0x19aa9c the decompiler drops).
//!
//! fix-ammo-pane2: every game number here comes from the DISC through the port's own loaders — the
//! weapon table via `WeaponData::from_action_elf` (it runs the game's own boot static initialiser
//! `_GLOBAL_$I$weapon_data` 0x1b9900 over the disc's ACTION.ELF, the same fill that put these bytes
//! at 0x2BF150 in boot memory; docs/formats/weapon-data.md), and the `BulletImg.189` rows raw-sliced
//! from the same ELF read off the ISO. `work/ee_snapshot.bin` (a local PCSX2 dump, not in the repo —
//! the merge checkouts don't have it) is no longer read by the assertions: the snapshot's pane values
//! for the four weapons are the [L] literals asserted below, and the parse-equals-boot-memory check
//! is the `#[ignore]`d `probe_weapon_table_matches_the_boot_snapshot` (docs/formats/weapon-data.md's
//! PINE cross-check measured our table equal to PCSX2 memory in 30,819/30,820 bytes). Opt-in via `NF_ISO`.

use nf_formats::iso9660::IsoImage;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn repo(p: &str) -> String { format!("{}/../../{}", env!("CARGO_MANIFEST_DIR"), p) }

/// The disc's ACTION.ELF, through the port's own ISO loader [D /ACTION.ELF on the image].
fn action_elf() -> Vec<u8> {
    let path = iso().expect("NF_ISO");
    let img = IsoImage::open(&path).expect("open iso");
    img.read_file(img.find("/ACTION.ELF").expect("no ACTION.ELF on the disc")).expect("read ACTION.ELF")
}

/// The game's weapon table from the DISC, through the port's own loader: `WeaponData::from_action_elf`
/// runs the game's boot static initialiser over the user's ELF (docs/formats/weapon-data.md "How we
/// get it") — the same boot fill that produced the [L] ee_snapshot values the assertions expect.
fn weapon_data() -> nf_formats::weapon::WeaponData {
    let elf = action_elf();
    nf_formats::weapon::WeaponData::from_action_elf(&elf).expect("weapon_data parses")
}

/// One dword of a weapon record (`weapon_data` 0x2bf150 + w·0x10C + at).
fn field(wd: &nf_formats::weapon::WeaponData, w: u8, at: usize) -> u32 {
    let r = &wd.records[w as usize];
    u32::from_le_bytes(r[at..at + 4].try_into().unwrap())
}

/// The game's `BulletImg.189` row for an ammo kind [D ACTION.ELF 0x2BEA38 + kind·16] (f32 u, v, width, px/round),
/// raw bytes sliced from the DISC's ACTION.ELF (vaddr → file offset delta 0x106100).
fn game_bullet(elf: &[u8], kind: usize) -> [f32; 4] {
    let at = 0x2BEA38 - 0x106100 + kind * 16;
    [
        f32::from_le_bytes(elf[at..at + 4].try_into().unwrap()),
        f32::from_le_bytes(elf[at + 4..at + 8].try_into().unwrap()),
        f32::from_le_bytes(elf[at + 8..at + 12].try_into().unwrap()),
        f32::from_le_bytes(elf[at + 12..at + 16].try_into().unwrap()),
    ]
}

/// The pane's count text for a state, exactly what 0x19a7f0 sprintfs at rest (0x19aad4/0x19ab18):
/// kinds 0x14/0x1a → `clip·100/max`; kind 0x1f → the depleted `min(max−clip, max−12)` of `max−12`
/// [M 0x19a7f0:129-136]; every other kind → "%d-%d" clip-spare (the spare = `BL+0x170+kind·2`).
fn pane_text(kind: u8, clip: u32, spare: u32, clip_cap: u32) -> String {
    match kind {
        0x14 | 0x1a => format!("{}", clip * 100 / clip_cap.max(1)),
        0x1f => {
            let mx = clip_cap.saturating_sub(12);
            format!("{}", mx.min(clip_cap.saturating_sub(clip)) * 100 / mx.max(1))
        }
        _ => format!("{}-{}", clip, spare),
    }
}

#[test]
fn the_four_weapons_pane_text_labels_and_icons_match_the_game() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let scene = nf_view::load(&iso, "0700000c.bin", false).expect("load 0700000c");
    let s = scene.sim.as_ref().expect("sim");
    let lang = s.lang.as_ref().expect("USATxt.dat parsed");
    // (weapon, kind, clip cap, the pane text at a full clip with an empty reserve) — the kind/cap
    // pairs are the game's numbers [L ee_snapshot 0x2BF150 + w·0x10C].
    for (w, kind, cap, count) in [(0x06u8, 0x02u8, 16u32, "16-0"), (0x1e, 0x07, 5, "5-0"), (0x2a, 0x0d, 6, "6-0"), (0x4a, 0x14, 100, "100")] {
        let wd = weapon_data();
        assert_eq!(field(&wd, w, 0x90) as u8, kind, "w {w:#04x}: the disc's ammo kind (+0x90) [L ee_snapshot]");
        assert_eq!(field(&wd, w, 0x92) & 0xffff, cap, "w {w:#04x}: the disc's clip capacity (+0x92) [L ee_snapshot]");
        let txt = pane_text(kind, cap, 0, cap);
        assert_eq!(txt, count, "w {w:#04x}: the pane's count text (full clip, empty reserve)");
        // Sister: percent kinds show one number, never "clip-spare" (the Stunner's charge counts down).
        if matches!(kind, 0x14 | 0x1a | 0x1f) { assert!(!txt.contains('-')); } else { assert!(txt.contains('-')); }
        // The label ids equal the game's bytes: mode 0 = +0x30, the name = +0x38 — the sim's own
        // table vs a fresh parse of the disc's ACTION.ELF (both off NF_ISO).
        assert_eq!(s.weapons.fire_mode_label(w as usize, 0), field(&wd, w, 0x30), "w {w:#04x} mode-0 label id");
        assert_eq!(s.weapons.name_label(w as usize), field(&wd, w, 0x38), "w {w:#04x} name label id");
        if w == 0x1e {
            assert_eq!(lang.bind(s.weapons.name_label(w as usize)), Some("Winter Tactical Sniper"), "the task's compare name");
            assert_eq!(lang.bind(s.weapons.fire_mode_label(w as usize, 0)), Some("Single"), "the live pass's sniper mode label");
        }
        if w == 0x06 {
            assert_eq!(lang.bind(s.weapons.fire_mode_label(w as usize, 0)), Some("Semi"), "the live pass's P2K mode label");
        }
    }
    // The per-ammo-type icon rows the strips draw: BulletImg.189[kind] [D 0x2BEA38 + kind·16],
    // the port's parse vs the raw bytes off the disc.
    let tbl = &scene.hud.as_ref().expect("hud tables").bullets;
    let elf = action_elf();
    for kind in [0x01usize, 0x07, 0x0d, 0x14] {
        assert_eq!(tbl.get(kind).copied().unwrap_or([0.0; 4]), game_bullet(&elf, kind), "strip icon row kind {kind:#04x}");
    }
    // Sister: kind 0 (unarmed/weapons without ammo) is an all-zero row — no strip draws.
    assert_eq!(tbl.get(0).copied().unwrap_or([0.0; 4]), [0.0; 4], "kind 0 row is empty");
}

#[test]
fn the_level_start_switch_pins_the_timer_then_it_decays_to_the_rest_pane() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "0700000c.bin", false).expect("load 0700000c");
    // The level's own loadout IS the repro: 0700000c starts Bond on the Winter Tactical Sniper
    // through a real switch request (the load line "switch request 30"), the same drone+0x63 the
    // game's HUD re-arms on [M 0x19a7f0:213-217].
    {
        let sim = scene.sim.as_ref().unwrap();
        assert_eq!(sim.gun.switch_request, 0x1e, "the level's start switch request is the sniper");
    }
    // While the request is pending (weapon != request) the game re-sets the timer to 60 and then
    // decrements it in the SAME update (0x19abf4 arm → 0x19ac04 decay), so pending frames read 59.
    let mut pending = 0;
    for i in 0..120 {
        let sim = scene.sim.as_mut().unwrap();
        sim.step_bond(Default::default(), false, false, false);
        if sim.gun.weapon == 0x1e {
            assert_eq!(i32::from(sim.hud.weapon_label_timer), 58, "tick {i}: completion decays the last pending 59 to 58 (the game: no re-arm when +0x63 == +0x62, the 0x19ac04 decay still runs)");
            break;
        }
        pending += 1;
        assert_eq!(i32::from(sim.hud.weapon_label_timer), 59, "tick {i}: a pending differing request re-arms every frame (pre-fix it decays during the draw)");
    }
    assert!(pending > 0, "the start switch had a pending window at all");
    // Then it decays 1/frame from 58 and the pane rests (fire mode back, name hidden).
    for i in 1..=60u32 {
        let sim = scene.sim.as_mut().unwrap();
        sim.step_bond(Default::default(), false, false, false);
        assert_eq!(i32::from(sim.hud.weapon_label_timer), (58 - i as i32).max(0), "decay tick {i}");
    }
    // Sister: at rest the pane's inputs are the sniper's — the level's own "5-30" [D the task's
    // PCSX2 compare: clip 5 of cap 5, spare 30].
    let sim = scene.sim.as_ref().unwrap();
    assert_eq!(sim.gun.weapon, 0x1e);
    assert_eq!(sim.gun.clip, 5, "the sniper's full clip [L +0x92 = 5]");
    assert_eq!(sim.gun.spare, 30, "the level's sniper reserve");
    assert_eq!(pane_text(0x07, sim.gun.clip, sim.gun.spare, 5), "5-30", "the compare capture's count text");
}

#[test]
fn the_kind_1f_display_is_the_depleted_clip_of_max_minus_12() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    // [M 0x19a7f0:129-136]: kind 0x1f → clip' = min(max − clip, max − 12), max' = max − 12.
    // The Samurai pair (0x32/0x33) are the kind-0x1f weapons [L ee_snapshot +0x90 = 0x1f, +0x92 = 100].
    let wd = weapon_data();
    assert_eq!(field(&wd, 0x32, 0x90) as u8, 0x1f, "the Samurai is a kind-0x1f weapon [L ee_snapshot]");
    assert_eq!(field(&wd, 0x32, 0x92) & 0xffff, 100);
    assert_eq!(pane_text(0x1f, 100, 0, 100), "0", "full clip shows the depleted 0 of 88");
    assert_eq!(pane_text(0x1f, 56, 0, 100), "50", "min(44, 88) = 44 → 44·100/88");
    // Sister: the percent never exceeds 100 even with max − clip > max − 12.
    assert_eq!(pane_text(0x1f, 12, 0, 100), "100");
}

/// The boot-memory cross-check the assertions used to read inline: the port's parse of the disc's
/// ACTION.ELF vs the PCSX2 EE snapshot's `weapon_data` at 0x2BF150 (boot-filled). The snapshot is a
/// local dump, NOT in the repo (CLAUDE.md: no game bytes; merge checkouts don't have it), so the
/// live tests above assert the [L] literals instead and this runs only where a snapshot exists.
#[test]
#[ignore = "needs the local PCSX2 EE snapshot work/ee_snapshot.bin (not in the repo); run with --ignored on a sandbox that has it"]
fn probe_weapon_table_matches_the_boot_snapshot() {
    let Some(_iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let Ok(snap) = std::fs::read(repo("work/ee_snapshot.bin")) else { eprintln!("no work/ee_snapshot.bin — nothing to cross-check"); return };
    let wd = weapon_data();
    assert_eq!(wd.records.len(), 115, "the table's record count [D symbol size 30,820 / 0x10C]");
    for w in 0..115u8 {
        let base = 0x2BF150 + w as usize * 0x10C;
        // The pane's fields (+0x30..+0x3c labels, +0x90 kind, +0x92 cap) plus the +0x00 id — NOT
        // +0xDC, the skin hashcode the game rewrites at run time (docs/formats/weapon-data.md).
        for at in [0x00usize, 0x30, 0x34, 0x38, 0x3c, 0x90, 0x92] {
            let snap_v = u32::from_le_bytes(snap[base + at..base + at + 4].try_into().unwrap());
            assert_eq!(field(&wd, w, at), snap_v, "w {w:#04x} +{at:#x}: the disc parse vs boot memory [L ee_snapshot]");
        }
    }
}
