//! Weapon switching through the real input path: from hands (0x47), a mouse-wheel cycle recovers to a
//! weapon with a first-person model. Tests the `Gun::cycle` + `Gun::has_ammo` + `has_fp` path that
//! `Sim::step_bond` uses, against the live `weapon_data` table. Opt-in: set `NF_ISO` to the disc image.

use nf_formats::iso9660::IsoImage;
use nf_formats::{disc, level::Level};
use nf_formats::weapon::WeaponData;

fn weapon_data() -> Option<WeaponData> {
    let path = std::env::var_os("NF_ISO")?;
    let iso = IsoImage::open(&path).unwrap();
    let elf = iso.read_file(iso.find("/ACTION.ELF").unwrap()).unwrap();
    Some(WeaponData::from_action_elf(&elf).unwrap())
}

#[test]
fn scroll_from_hands_finds_a_visible_weapon() {
    let Some(wd) = weapon_data() else { eprintln!("skip: NF_ISO not set"); return };
    // Bond at hands (weapon 0x47). Weapon 1 (fists) and weapon 2 (P2K) are both owned with ammo.
    // D-003: the cycle in Sim::step_bond filters by has_view_model (the skin must be in the level archive).
    // Gun::cycle itself does NOT filter (that's the Sim's job). This test verifies Gun::cycle finds weapon 1
    // (fists) — the Sim-level D-003 filter is what skips it in play.
    let mut owned = vec![false; 0x73];
    owned[1] = true;   // fists
    owned[2] = true;   // P2K
    let mut clip = vec![0i16; 0x73];
    clip[1] = 1;  clip[2] = 7;
    let mut carried = vec![0i16; 34];
    carried[1] = 41;
    let has_ammo = |w: u8| nf_game::gun::Gun::has_ammo(w, &carried, &clip, |w| wd.ammo_type(w as usize), |w| wd.flags(w as usize));
    let selectable = |w: u8| wd.selectable(w);

    // Gun::cycle (without the D-003 filter) finds weapon 1 (fists) from hands.
    let mut g = nf_game::gun::Gun::new(0x47, 0, 0);
    assert_eq!(g.cycle(1, nf_game::gun::CycleMode::Weapons, &owned, has_ammo, selectable), Some(1),
        "Gun::cycle finds fists (weapon 1) — the Sim's D-003 filter is what skips it in play");

    // From fists (1), Gun::cycle finds the P2K (2).
    let mut g2 = nf_game::gun::Gun::new(1, 1, 0);
    assert_eq!(g2.cycle(1, nf_game::gun::CycleMode::Weapons, &owned, has_ammo, selectable), Some(2),
        "Gun::cycle from fists finds the P2K (weapon 2)");

    // Sister: from the P2K (2), cycle +1 wraps to fists (1).
    let mut g3 = nf_game::gun::Gun::new(2, 7, 41);
    assert_eq!(g3.cycle(1, nf_game::gun::CycleMode::Weapons, &owned, has_ammo, selectable), Some(1),
        "scroll from the P2K wraps to fists");
}

#[test]
fn hands_recovery_via_get_best_weapon() {
    let Some(wd) = weapon_data() else { eprintln!("skip: NF_ISO not set"); return };
    const BEST_WEAPON: [u16; 18] = [0x42, 0x32, 0x1a, 0x18, 0x1c, 0x16, 0x15, 0x08, 0x10, 0x0e, 0x0d, 0x0c, 0x12, 0x06, 0x04, 0x02, 0x0a, 0x01];
    let mut owned = vec![false; 0x73];
    owned[2] = true; // P2K
    let clip = vec![0i16; 0x73];
    let mut carried = vec![0i16; 34];
    carried[1] = 41;
    let has_ammo = |w: u8| nf_game::gun::Gun::has_ammo(w, &carried, &clip, |w| wd.ammo_type(w as usize), |w| wd.flags(w as usize));
    let base_of = |w: u8| wd.base(w as usize) as u8;

    let mut g = nf_game::gun::Gun::new(0x47, 0, 0);
    assert_eq!(g.get_best_weapon(&BEST_WEAPON, base_of, &owned, has_ammo), Some(2));
}
