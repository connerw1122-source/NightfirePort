//! fix-gadgets: the gadget selection through the real input path (the G cycle, `Input::gadget_cycle`).
//!
//! The user's playtest (2026-10-12): "can't equip the camera gadget" (0700000c), "it wouldn't let me
//! switch to it" (07000012's briefcase), "cant find the gadget" (0700000a's Q-Worm). The gadget
//! machinery was headless-complete (the cycle, the toggle-back, the equip) but the viewer's G key
//! was commented out (lib.rs), and the port-map claimed the gadgets were unselectable — the
//! weapon_data says otherwise: +0x04 (selectable) is 1 for 0x4A/0x4C/0x4E/0x4F/0x50/0x51/0x52/0x53/
//! 0x54/0x56/0x57/0x58/0x59 [D ACTION.ELF weapon_data static initialiser].

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

/// 0700000c: walk-free take of the camera pickup (weapon 0x52 at (54.0, 1.3, −173.5) [D pickup
/// dump]), then the gadget cycle equips it. Sister: the weapon cycle never lands on a gadget.
#[test]
fn the_camera_pickup_equips_through_the_gadget_cycle() {
    let iso = iso();
    let mut scene = nf_view::load(&iso, "0700000c.bin", false).expect("load 0700000c");
    let sim = scene.sim.as_mut().unwrap();
    sim.bond.pos = [54.0, 2.4, -173.5];
    for _ in 0..120 { sim.step_bond(Input::default(), false, false, false); }
    assert!(sim.drones.inventory.owned.get(0x52).copied().unwrap_or(false),
        "the camera pickup (weapon 0x52) must enter the inventory");
    assert_ne!(sim.gun.weapon, 0x52, "the camera starts unequipped");
    // The G cycle: walk the gadget wheel until the camera comes up (the game's d-pad Up/Down;
    // the port's PC bind is G) [M Player_WeaponChange 0x1a6f80 mode 1].
    let mut equipped = false;
    for _ in 0..12 {
        sim.step_bond(Input { gadget_cycle: 1, ..Default::default() }, false, false, false);
        for _ in 0..60 { sim.step_bond(Input::default(), false, false, false); }
        if sim.gun.weapon == 0x52 { equipped = true; break; }
    }
    assert!(equipped, "the gadget cycle must equip the camera (0x52); weapon now {:#x}", sim.gun.weapon);
    // The weapon cycle never selects a gadget (bounds 1..0x47 [M 0x1a7120]). The 70-tick settle
    // lets each switch finish (the draw anim ~60 ticks) before the next read.
    let mut saw_gadget = false;
    for _ in 0..0x40 {
        sim.step_bond(Input { weapon_cycle: 1, ..Default::default() }, false, false, false);
        for _ in 0..70 { sim.step_bond(Input::default(), false, false, false); }
        if (0x4a..=0x5e).contains(&sim.gun.weapon) { saw_gadget = true; }
    }
    assert!(!saw_gadget, "the weapon cycle must stay in 1..0x47");
}

/// 07000005: Bond starts owning the Q-Claw (0x4A), the Q-Decryptor (0x4E) and the Q-Laser (0x50)
/// [L ee_start_05.bin: the level-start inventory, Player_InitWeapon's table] — the gadget cycle
/// must reach each of them from the level's first weapon.
#[test]
fn the_level_start_gadgets_are_all_reachable_by_the_cycle() {
    let iso = iso();
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005");
    let sim = scene.sim.as_mut().unwrap();
    for g in [0x4Au8, 0x4e, 0x50, 0x54] {
        assert!(sim.drones.inventory.owned.get(g as usize).copied().unwrap_or(false),
            "gadget {g:#x} must be owned at the 07000005 level start [L]");
    }
    let mut reached = std::collections::HashSet::new();
    for _ in 0..12 {
        sim.step_bond(Input { gadget_cycle: 1, ..Default::default() }, false, false, false);
        for _ in 0..60 { sim.step_bond(Input::default(), false, false, false); }
        if [0x4Au8, 0x4e, 0x50, 0x54].contains(&sim.gun.weapon) { reached.insert(sim.gun.weapon); }
        if reached.len() == 4 { break; }
    }
    assert_eq!(reached.len(), 4, "the cycle must reach the Q-Claw, the Q-Decryptor, the Q-Laser and the watch; got {reached:?}");
}

use nf_game::player::Input;
