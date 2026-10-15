//! fix-start-loadouts: the per-level start loadouts — `Player_InitWeapon` (0x1a7ef8)'s whole switch on
//! the level id, the parts the port had as "the 07000005 set everywhere". The user-visible symptoms
//! (playtest 1014-1725 + the 2026-10-14 measurements): 07000014 starts with the Delta Repeater crossbow
//! 0x11 in hand ("3-9 Single", in a crawl vent) — the port handed the 05 kit; the 07000015 gadget cycle
//! begins at the Laser 0x4e — the port's cycle begins at the Stunner 0x4a (the 05 kit's first gadget).
//!
//! Evidence (the fix implements these):
//! - [M 0x1a7ef8 case 0x7000014] `Player_InitAmmoWeapons` + equip(2, 48) + equip(0x11, 12) +
//!   equip(0x34, 5) + equip(0x37, 5) + equip(0x4a, 999) + equip(0x50, 0) + equip(0x54, 0) +
//!   equip(0x56, 0); the request byte `M_CONTROL+0x63` = 0x11 (disasm `001a8990 sb s0,99(v1)` is the
//!   shared tail's held=Upgrade_Weapon(6); case 14 writes 0x11 directly [M :296]).
//! - [M 0x1a7ef8 cases 0x7000015/16/17] `Player_RamLoad` (nothing on a cold load, `IsWarmReset == 0`,
//!   0x1957a8:9); the request = the saved weapon; when the P2K (Upgrade_Weapon(6) = 2) is unowned the
//!   fresh 14 loadout re-runs (`switchD_001a8284_caseD_7000014`) — a cold 15 IS the 14 kit.
//! - [M 0x1a7ef8 case 0x7000001] equip(2, 48) + equip(0x4a, 999) + equip(uVar11, 999) with
//!   uVar11 = 0x5b on the 60 Hz NTSC build (disasm `001a8244 movz s4,v0,a0` picks 0x59 only at
//!   VIDEO_FRAME_RATE == 50); request = 2.
//! - [M 0x1a7ef8 default] (level ids outside the switch — e.g. the non-story archives) the
//!   give-everything list with the request = 2 (`001a897c addiu a1,zero,22` = 0x16, the walk's own
//!   final equip; held = s0 = 2).
//! - The clip/spare arithmetic is `Player_EquipWeapon` 0x1a8a58 (a fresh own fills the clip to
//!   `weapon_data[w]+0x92`, the extra rounds carry, clamped to `ammo_data[type].max` 0x1a8d18:69-78);
//!   the caps are the measured table [L docs/fields/ammo-data.md] — the Stunner/Laser/Decryptor/
//!   Shaver spare lanes cap at 0 (clip-only gadgets).
//! - [L docs/live/2026-10-14-weapon-measurements-pcsx2.md] the crossbow in hand on 14; the [L]
//!   ee_start_05 shape (P2K 7+41) pins the 05 case that already worked.
//!
//! Opt-in via `NF_ISO`.

use nf_game::player::RamSave;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn load(iso: &str, level: &str) -> nf_view::scene::Scene {
    nf_view::load(iso, level, false).expect(level)
}

fn owned_set(inv: &nf_game::pickup::Inventory) -> Vec<usize> {
    (0..inv.owned.len()).filter(|&w| inv.owned[w]).collect()
}

/// The fresh 07000014 kit [M 0x1a7ef8 case 0x7000014 + the tail 0x4e/1], held = the crossbow. The
/// owned set is the equip list MINUS the weapons whose first-person skins the level archive does not
/// hold — `Player_EquipWeapon` refuses those (`Player_InitWeapon`'s hashtable check, 0x1a8a58:20)
/// and `Player_CheckWeaponsLoaded` (0x1a97c8) revokes the rest. On 07000014 the Stunner 0x4a's fp
/// skin is absent [D the port's skin census = the game's hashtable], so the case's `equip(0x4a,999)`
/// refuses — the same mechanism that keeps the real 15's gadget cycle free of the Stunner.
fn assert_crossbow_kit(sim: &nf_view::scene::Sim, ctx: &str, grapple_loaded: bool) {
    let wd = &sim.weapons;
    let inv = &sim.drones.inventory;
    let mut expected = vec![1usize, 2, 0x11, 0x34, 0x37, 0x4e, 0x54, 0x56];
    if grapple_loaded { expected.push(0x50); expected.sort_unstable(); }
    assert_eq!(owned_set(inv), expected,
        "{ctx}: owned set (case 14's equips + the 0x4e/1 tail, minus the refused equips) [M 0x1a7ef8 case 0x7000014; 0x1a8a58:20]",);
    assert!(!inv.owned[0x4a], "{ctx}: the Stunner is NOT owned — its fp skin is not in this level's archive, the equip refuses [M 0x1a8a58:20; D skin census]");
    assert!(!sim.skins.contains_key(&wd.first_person(0x4a)),
        "{ctx}: sister — the Stunner fp skin hash is genuinely absent from the loaded skins [D]");
    // A fresh own fills the clip to the record's size, the extra carries, capped [M 0x1a8a58:44-48,
    // 0x1a8d18:69-78; L ammo-data.md maxes].
    assert_eq!(inv.clip[2], wd.clip_size(2) as i16, "{ctx}: P2K clip 7 [L]");
    assert_eq!(inv.carried[1], (48 - wd.clip_size(2) as i16).min(70), "{ctx}: P2K spare 41 [L ee_start_05]");
    assert_eq!(inv.clip[0x11], wd.clip_size(0x11) as i16, "{ctx}: crossbow clip 3 [L atlas]");
    assert_eq!(inv.carried[15], 12 - wd.clip_size(0x11) as i16, "{ctx}: crossbow spare 9 (12 rounds)");
    assert_eq!(inv.clip[0x34], wd.clip_size(0x34) as i16, "{ctx}: frag clip 1");
    assert_eq!(inv.carried[25], 4.min(9), "{ctx}: frag spare 4 (5 rounds)");
    assert_eq!(inv.clip[0x37], wd.clip_size(0x37) as i16, "{ctx}: mine clip 1");
    assert_eq!(inv.carried[22], 4.min(4), "{ctx}: mine spare capped at 4 (ammo 22 max [L])");
    assert_eq!(inv.clip[0x4a], 0, "{ctx}: the refused Stunner equip leaves its clip slot empty [M 0x1a8a58:20]");
    assert_eq!(inv.clip[0x4e], wd.clip_size(0x4e) as i16, "{ctx}: laser charge 500");
    assert_eq!(inv.carried[26], 0, "{ctx}: the laser spare lane caps at 0 [L]");
    assert_eq!(inv.clip[0x54], wd.clip_size(0x54) as i16, "{ctx}: camera clip 1");
    assert_eq!(inv.clip[1], wd.clip_size(1) as i16, "{ctx}: the fists tail fills its slot");
}

/// 07000014 starts with the Delta Repeater crossbow 0x11 in hand and the case-14 kit — not the 05
/// loadout the port used to grant [L the 2026-10-14 measurement; M 0x1a7ef8 case 0x7000014].
#[test]
fn the_07000014_start_loadout_is_the_crossbow_kit() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let sc = load(&iso, "07000014.bin");
    let sim = sc.sim.as_ref().unwrap();
    assert_eq!(sim.gun.switch_request, 0x11,
        "14's draw request is the crossbow 0x11 — the game starts it in hand [L measurement; M case 0x7000014 writes 0x11]");
    assert_crossbow_kit(sim, "cold 07000014", true);
}

/// A cold 07000015 runs `Player_RamLoad` (a no-op — `IsWarmReset == 0`), finds the P2K unowned and
/// re-runs the fresh 14 loadout [M 0x1a7ef8 case 0x7000015 → switchD_001a8284_caseD_7000014]. The
/// port used to grant the 05 kit here, whose first gadget is the Stunner.
#[test]
fn the_07000015_cold_start_falls_back_to_the_14_kit() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let sc = load(&iso, "07000015.bin");
    let sim = sc.sim.as_ref().unwrap();
    assert_eq!(sim.gun.switch_request, 0x11,
        "the cold 15's draw request is the fresh-14 fallback's 0x11 [M caseD_7000014]");
    let grapple = sim.skins.contains_key(&sim.weapons.first_person(0x50));
    assert_crossbow_kit(sim, "cold 07000015", grapple);
}

/// A warm 07000015 (`Player_RamLoad`'s body runs) carries the 14-exit inventory and re-selects the
/// saved weapon [M 0x1a7ef8 case 0x7000015; 0x1957a8 the RamLoad copy]. Sister of the cold test.
#[test]
fn the_07000015_warm_start_carries_the_14_inventory() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let sc = load(&iso, "07000014.bin");
    let (save, carried_1, clip_11) = {
        let sim = sc.sim.as_ref().unwrap();
        let save = RamSave::capture(&sim.drones.health, sim.gun.switch_request,
            &sim.drones.inventory, &sim.drones.channels);
        (save, sim.drones.inventory.carried[1], sim.drones.inventory.clip[0x11])
    };
    let sc15 = nf_view::load_with_carry(&iso, "07000015.bin", false, Some(&save))
        .expect("load 15 with the 14 carry");
    let sim = sc15.sim.as_ref().unwrap();
    assert_eq!(sim.gun.switch_request, 0x11,
        "the warm 15 re-selects the saved weapon byte [M 0x1a7ef8 case 0x7000015: +0x63 = RamSave weapon]");
    let inv = &sim.drones.inventory;
    // The carry keeps the weapons whose skins THIS level's archive holds; `Player_CheckWeaponsLoaded`
    // (0x1a97c8) revokes the Grapple 0x50 — 15's archive misses its fp skin, and the game's own warm
    // load shows the same loss (the measured 15 cycle has no Grapple either) [L 2026-10-14].
    assert!(inv.owned[2] && inv.owned[0x11] && inv.owned[0x54],
        "the warm carry keeps the P2K, the crossbow and the camera (RamLoad + the sweep) [M 0x1957a8; 0x1a97c8]");
    assert!(!inv.owned[0x50], "the Grapple loses owned on 15 — its fp skin is absent here [M 0x1a97c8; L the measured cycle]");
    assert_eq!(inv.carried[1], carried_1, "the spare rounds ride the RamSave [M 0x195670:6]");
    assert_eq!(inv.clip[0x11], clip_11, "the crossbow's clip rides the RamSave [M 0x195670:7]");
}

/// 07000005 pins the measured set (the case the port already had): P2K 48, Stunner 999, Grapple 0,
/// Micro-Camera 0 + the 0x4e/fists tail; request = 2. [L ee_start_05 owned = {1, 2, 0x4A, 0x4E, 0x50,
/// 0x54}, clip[2] = 7, carried[1] = 41; M 0x1a7ef8 case 0x7000005.]
#[test]
fn the_07000005_start_loadout_pins_the_measured_set() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let sc = load(&iso, "07000005.bin");
    let sim = sc.sim.as_ref().unwrap();
    let wd = &sim.weapons;
    let inv = &sim.drones.inventory;
    assert_eq!(sim.gun.switch_request, 2, "05's draw request is the P2K [L; M case 0x7000005 held = Upgrade_Weapon(6)]");
    assert_eq!(owned_set(inv), vec![1, 2, 0x4a, 0x4e, 0x50, 0x54], "05's owned set [L ee_start_05]");
    assert_eq!(inv.clip[2], wd.clip_size(2) as i16, "P2K clip 7 [L]");
    assert_eq!(inv.carried[1], (48 - wd.clip_size(2) as i16).min(70), "P2K spare 41 [L]");
    assert_eq!(inv.clip[0x4a], wd.clip_size(0x4a) as i16, "stunner charge 100 [L]");
}

/// 07000001 (the castle level the switch opens with) equips the P2K, the Stunner 999 AND 0x5b 999
/// (the NTSC 60 Hz pick — 0x59 is the 50 Hz one) + the tail, holding the P2K [M 0x1a7ef8 case
/// 0x7000001 → LAB_001a8980/84; the disasm's `movz s4` 0x59/0x5b select]. The port's default gave
/// this level the 05 kit (a 0x50/0x54 the case never equips). The owned set is the case's equip list
/// intersected with the level's loaded skins — `Player_EquipWeapon` refuses a weapon whose fp skin
/// the hashtable misses [M 0x1a8a58:20; 0x1a97c8].
#[test]
fn the_07000001_start_is_the_p2k_stunner_kit() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let sc = load(&iso, "07000001.bin");
    let sim = sc.sim.as_ref().unwrap();
    let wd = &sim.weapons;
    let inv = &sim.drones.inventory;
    assert_eq!(sim.gun.switch_request, 2, "01's draw request is the P2K [M case 0x7000001 held = s0]");
    // The case equips {2, 0x4a, 0x5b} + the tail {0x4e, 1}; the archive's skin table decides which
    // equips land. Either way the 05 kit's fingerprint (the Grapple 0x50 / camera 0x54) must be gone.
    let case_list = [2usize, 0x4a, 0x5b, 0x4e, 1];
    for w in owned_set(inv) {
        assert!(case_list.contains(&w), "01 owns {w:#04x} — outside case 0x7000001's equip list [M]");
    }
    assert!(inv.owned[2] && inv.owned[1], "the P2K and the fists tail land [M]");
    assert!(!inv.owned[0x50] && !inv.owned[0x54],
        "the 05 kit's Grapple/camera must NOT be owned on 01 — the old default bug [M case 0x7000001]");
    assert!(inv.owned[0x5b] || !sim.skins.contains_key(&wd.first_person(0x5b)),
        "0x5b owned, or its skin is genuinely absent from 01's archive (the refusal arm) [M 0x1a8a58:20]");
    assert!(!inv.owned[0x4a] || sim.skins.contains_key(&wd.first_person(0x4a)),
        "the Stunner's ownership agrees with its skin's presence [M 0x1a8a58:20]");
}

/// The switch's default arm (level ids outside the jump table — the non-story archives) is the
/// give-everything list ending in the 0x16 equip, holding the P2K [M 0x1a7ef8 default; disasm
/// `001a897c addiu a1,zero,22` + `001a8990 sb s0,99(v1)`]. Pinned through the real entry point on a
/// cleared inventory (the direct call the dev panel's levelkit runs), with a level id the jump table
/// rejects (`sltiu v1,a0,74` — 0x700004b's index 0x4a is out of range [M 001a8268]).
#[test]
fn the_switch_default_gives_everything_and_holds_the_p2k() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut sc = load(&iso, "07000005.bin");
    let (wd, _skins, clip_len, carried_len) = {
        let sim = sc.sim.as_mut().unwrap();
        sim.drones.inventory = nf_game::pickup::Inventory::empty(
            sim.drones.inventory.clip.len(), sim.drones.inventory.carried.len());
        (sim.weapons.clone(), sim.skins.clone(), sim.drones.inventory.clip.len(), sim.drones.inventory.carried.len())
    };
    let loaded = |_: u8| true; // the default-case pin: isolate the LIST from the level's skin table
    let mut health = nf_game::bullet::PlayerHealth::default();
    let mut inv = nf_game::pickup::Inventory::empty(clip_len, carried_len);
    let mut ch = nf_game::triggers::Channels::default();
    let req = nf_game::player::init_level_loadout(0x0700_004b, None, &mut health, &mut inv, &mut ch, &wd, loaded);
    assert_eq!(req, 2, "the default holds the P2K (s0 = Upgrade_Weapon(6)) [M 001a8990]");
    // The equip marks the BASE owned (`Player_EquipWeapon` 0x1a8a58:48-55) — the list's variants
    // 0x18 (base 0x19) and 0x15 (base 0x14) own their bases' slots.
    let mut owned_expected = [6usize, 0x12, 0x18, 0x46, 0x37, 0x36, 0x35, 0x34, 0x3a, 0x52, 0x2c,
        0x2e, 0x30, 0x4a, 0x50, 0x1e, 0x24, 0xe, 0x10, 0x54, 0x1c, 0x15, 10, 0xd, 0x56, 0x58, 0x41,
        0x59, 0x3b, 0x1a, 0x33, 0x11, 2, 0x2a, 0x42, 0x43, 0x45, 0x16, 0x4e, 1]
        .iter().map(|&w| wd.base(w) as usize).collect::<Vec<_>>();
    owned_expected.sort_unstable();
    owned_expected.dedup();
    for &w in &owned_expected {
        assert!(inv.owned[w], "the default owns {w:#04x} (a listed equip or its base) [M 0x1a7ef8 default + 0x1a8a58:48-55]");
    }
    assert_eq!(owned_set(&inv), owned_expected,
        "and nothing else (the default list's bases + the tail) [M]");
    assert_eq!(inv.carried[8], 30, "EquipAmmo(0x25, 999) caps ammo 8 at its max [M 0x1a8d18:69-78; L max 30]");
    assert_eq!(inv.carried[12], 6, "EquipAmmo(0x1b, 999) caps ammo 12 at its max [M; L max 6]");
}
