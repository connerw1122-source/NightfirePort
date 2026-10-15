//! fix-switch-after-cold-load: a weapon switch is ignored after a cold load (task 110n).
//!
//! Evidence base:
//! - The number-key/quick-select request is the game's `anim+0x63` byte. The cycle write is
//!   REFUSED in the view states {1, 3, 5, 6, 7, 8, 9, 0xb, 0xd, 0xe, 0xf, 0x10} and ALLOWED in
//!   {0, 2, 4, 0xa, 0xc, ≥0x11} [M `Player_WeaponChange` 0x1a6f80:24-40 — the state switch maps
//!   (state−1) to the return cases] — state 4 (the draw anim playing) is ALLOWED.
//! - The machine honours a request written mid-draw: `Player_SetWeaponAnimObj` 0x1a2e38 case 4
//!   re-checks `+0x63 != +0x62` EVERY tick while the draw anim plays and retargets (`+0x64 =
//!   +0x62`, re-run `Player_WeaponSelect` 0x1a2cc0 → the holst-then-draw sequence restarts to the
//!   new weapon) [M 0x1a2e38:309-315]. So a press during the level-start draw queues and fires.
//! - The level start itself: `Player_InitWeapon` 0x1a7ef8:62 zeroes the view state and writes
//!   `+0x63` = the kit weapon; the state-0 arm applies it (`Player_WeaponSelect`), so the cold
//!   load opens with a draw. The port: `nf_view::load` → `init_level_loadout` →
//!   `sim.gun.switch_request = req` (lib.rs) with `gun.weapon` still the default — the same
//!   start switch, states 1→3.
//! - The port's bug: `Sim::step_bond`'s switch_to arm sat inside the idle-only gate
//!   (`switch_state == 0 && reload_state == 0 && !fire_busy`, scene.rs) — a press during the
//!   start switch (state 1/3) was dropped entirely, and the switch_to input is per-tick, so the
//!   press never came back: the player's number key did nothing.
//! - The fix keeps the game's exact refused set: the request write is allowed while the port is
//!   idle (state 0) or drawing (state 3 ≈ the game's states 3+4 — the draw-anim window the game's
//!   case 4 polls), still refused while holstering (1), swapping variants (8), reloading or
//!   fire-busy [M 0x1a6f80's set]. The state-3 arm gains the game's case-4 mid-draw retarget:
//!   a new request while the draw plays saves `saved_weapon` and drops to the holst-then-draw
//!   sequence for the new weapon [M 0x1a2e38:309-315].
//!
//! Opt-in via `NF_ISO`.

use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn load(iso: &str, level: &str) -> nf_view::scene::Scene {
    nf_view::load(iso, level, false).expect("load")
}

fn tick(sim: &mut nf_view::scene::Sim, n: u32) {
    for _ in 0..n { sim.step_bond(Input::default(), false, false, false); }
}

fn press(sim: &mut nf_view::scene::Sim, id: u8) {
    sim.step_bond(Input { switch_to: Some(id), ..Default::default() }, false, false, false);
}

/// The cold load's start switch (the kit draw) must not eat the number keys: pressing 2 while the
/// start draw runs ends with the P2K in hand, as the game's mid-draw retarget does [M 0x1a2e38
/// case 4]. Sister: a single press on the FIRST possible tick (the draw window's first tick) is
/// enough; and after the start switch settles the press works as before.
#[test]
fn cold_load_number_key_press_switches_once_the_start_draw_settles() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso, "07000016.bin");
    let sim = scene.sim.as_mut().unwrap();
    // The kit: crossbow 0x11 in request (the loadout line), the default weapon 2 in hand.
    assert_eq!(sim.gun.switch_request, 0x11, "the cold kit's switch request [M 0x1a7ef8:159-207]");
    tick(sim, 1);
    assert!(sim.gun.switch_state != 0, "the start switch is running (holst/draw) [M 0x1a2cc0]");
    // The dev give path: the P2K, loaded.
    assert!(sim.drones.inventory.equip_weapon(&sim.weapons, 2, 7, true), "the give equipped the P2K");
    assert!(sim.drones.inventory.owned.get(2).copied().unwrap_or(false), "owned[2]");
    // Press 2 every tick through the start switch (a player mashing 2 during the load) — at least
    // one press lands in the draw window the game accepts.
    for i in 0..90 {
        press(sim, 2);
        if i == 0 { eprintln!("tick 1: state {} request {:#x} weapon {:#x}", sim.gun.switch_state, sim.gun.switch_request, sim.gun.weapon); }
    }
    assert_eq!(sim.gun.weapon, 2, "the P2K is in hand after the start draw settles [M 0x1a2e38 case 4]");
    assert_eq!(sim.gun.switch_request, 2, "the request settled on the pressed weapon");
}

/// The single-press sisters: one press on the FIRST tick of the draw window (the game's state-4
/// allowance [M 0x1a6f80's allowed set]), and one press after the start switch settles. Both end
/// with the pressed weapon. Also on 07000012, whose cold kit (0x10) differs from the P2K.
#[test]
fn first_possible_tick_and_settled_presses_switch_on_both_levels() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    for (level, kit) in [("07000016.bin", 0x11u8), ("07000012.bin", 0x10)] {
        let mut scene = load(&iso, level);
        let sim = scene.sim.as_mut().unwrap();
        assert_eq!(sim.gun.switch_request, kit, "the cold kit request on {level}");
        // A: the FIRST possible tick — the first tick of the draw window (state 3, the game's
        // state-4 poll window [M 0x1a2e38:309-315]).
        assert!(sim.drones.inventory.equip_weapon(&sim.weapons, 2, 7, true), "the give on {level}");
        let mut pressed = false;
        for _ in 0..200 {
            if sim.gun.switch_state == 3 && !pressed {
                press(sim, 2); // the single press
                pressed = true;
            } else {
                tick(sim, 1);
            }
            if pressed && sim.gun.switch_state == 0 && sim.gun.weapon == 2 { break; }
        }
        assert!(pressed, "the draw window ran on {level}");
        assert_eq!(sim.gun.weapon, 2, "the first-possible-tick press switched on {level} [M 0x1a2e38 case 4]");
        // B: settled — after everything is idle, a press still switches (the existing path).
        let other = if kit == 0x11 { 0x10 } else { 0x11 };
        if sim.drones.inventory.owned.get(other as usize).copied().unwrap_or(false)
            && sim.weapons.first_person(other as usize) != 0 {
            press(sim, other);
            for _ in 0..120 { if sim.gun.switch_state == 0 && sim.gun.weapon == other { break; } tick(sim, 1); press(sim, other); }
            assert_eq!(sim.gun.weapon, other, "a settled press switches on {level}");
        }
    }
}
