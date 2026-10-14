//! fix-handcannon — the Remote Mine's detonator record 0x38 fires with no observable event.
//!
//! What 0x38 is [L ee_start_05.bin + USATxt.dat via the game's fixup table]: the Remote Mine family
//! 0x37/0x38/0x39 — 0x37 selectable ("Grenade" mode label 0x0500000b, the SP/MP name 0x0500003e/67
//! "Remote Mine"), 0x38 the DETONATOR (mode label 0x0500004d "Detonate", +4 selectable 0, base 0x37,
//! ammo type 22, clip-add +0x91 = 0, fire script +0xB0 == the idle script 0x060004ea) — and 0x39 an
//! inert sibling (no scripts). Holding the mine shows the game's own hint [M Player_WeaponFiring
//! 0x1a5460:385-390]: label 0x10002dc "Use alternate fire to switch to detonator".
//!
//! The game's fire path for the detonator [M Player_WeaponInitBullet 0x1a4bf0 disasm 0x1a4c30-0x1a4cd4]:
//! the FIRST branch (before the bullet loop) reads weapon_data+0x68 & 0x20000 — the detonator's flag
//! (0x38's +0x68 = 0x00020010 [L]; the mine 0x37's 0x00014090 lacks it [L]) — and:
//!   1. walks every object; each type-5 object whose weapon record's base (+0x02) equals the fired
//!      record's base (0x37) and whose BD+0x30 owner is the firing player gets **BD+0xFC = 10.0**
//!      (`swc1 f0,252(a1)` — the mine bullet's detonate fuse; `Bullet_DoTrails` 0x12d890:268-281
//!      decrements it per tick and blasts at < 0: `Explode_Create(radius = record+0x08, damage =
//!      record+0x0C, script 0x6000052)` [M 0x12d890:388-393] — 8.0/100.0 [L record 0x37]);
//!   2. `Player_WeaponHasAmmo(BL, 0x37)` arms **BL+0x63 = 0x37** (`sb v0,99(v1)` — switch back to the
//!      mine); dry: the mine's owned byte (+0x1BA lane [M bldata.md]) is cleared and
//!      `Player_GetBestWeapon` 0x1a9680 writes the switch request instead [M 0x1a4ccc-0x1a4cd0];
//!   3. returns — NO bullet, NO muzzle flash (the +0x95b latch is never set in this arm), NO casing.
//! The mine itself, player-thrown, NEVER self-detonates: `Bullet_init` 0x12c5a0:196-223 sets
//! BU+0xFC = 7777.0 when the owner is type 3 (the player; guards keep 420), and `Bullet_DoTrails`
//! 0x12d890:268-273 skips the decrement while the fuse is 7777.0.
//!
//! The port's gaps (pre-fix, all three tests fail): `GunParams::ammo_per_shot` forced the raw
//! clip-add 0 up to 1 (`(w[0x91]).max(1)`), so `update_with`'s `clip >= ammo_per_shot` refused the
//! detonator's fire outright (the F family's "sounds but no shot tick": the idle==fire script's own
//! loop sounds recorded, nothing else); the +0x20000 arm did not exist (a generic hitscan shot would
//! have been wrong too); and the player's mine carried the guard's 420-tick self-destruct fuse.

use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn q(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
}

/// Load with both family records pre-merged (the detonator's fp skin 0x0500004b and its scripts are
/// not in the level archive), give the MINE 0x37 (the game's own pickup; the panel fills the shared
/// clip slot [M Player_EquipWeapon 0x1a8a58 via Inventory::clip_slot]), and wait for the draw.
fn load_mine(iso: &str) -> nf_view::scene::Scene {
    let mut scene = nf_view::load_weapons(iso, "07000005.bin", false, &[0x37, 0x38]).expect("load");
    let (msg, switch_to) = nf_view::panel::apply(&mut scene, &q(&[("do", "weapon"), ("id", "0x37"), ("rounds", "5")]));
    eprintln!("[give] 0x37: {msg} -> {switch_to:?}");
    assert_eq!(switch_to, Some(0x37), "the panel must switch to the mine 0x37");
    for _ in 0..240 {
        scene.sim.as_mut().unwrap().step_bond(Input { switch_to: Some(0x37), ..Default::default() }, false, false, false);
        let sim = scene.sim.as_ref().unwrap();
        if sim.gun.weapon == 0x37 && sim.gun.switch_state == 0 && !sim.gun.fire_state { break; }
    }
    assert_eq!(scene.sim.as_ref().unwrap().gun.weapon, 0x37, "the mine 0x37 is in hand");
    for _ in 0..4 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    scene
}

/// Throw one mine: hold the trigger until the grenade list gains it, then let go and let it settle.
fn throw_mine(scene: &mut nf_view::scene::Scene) {
    scene.sim.as_mut().unwrap().bond.pitch = -0.55; // a few metres ahead, not at the feet
    let mut thrown = false;
    for _ in 0..300 {
        scene.sim.as_mut().unwrap().step_bond(Input::default(), !thrown, false, false);
        if !scene.sim.as_ref().unwrap().drones.grenades.is_empty() { thrown = true; }
        if thrown { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
        if thrown && scene.sim.as_ref().unwrap().drones.grenades.iter().all(|g| g.resting || g.state >= 1) { break; }
    }
    assert!(thrown, "the mine 0x37 threw");
    // Let the mine land and settle (the throw's fire script also ends inside this wait).
    for _ in 0..240 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
}

/// Reach the detonator the game's way: the mode press from the mine [M Player_Weapon 0x1a24c8:246-260].
fn mode_press_to_detonator(scene: &mut nf_view::scene::Scene) {
    for _ in 0..300 {
        scene.sim.as_mut().unwrap().step_bond(Input { mode_switch: true, ..Default::default() }, false, false, false);
        if scene.sim.as_ref().unwrap().gun.weapon == 0x38 { break; }
    }
    assert_eq!(scene.sim.as_ref().unwrap().gun.weapon, 0x38, "the mode press reaches the detonator 0x38");
    for _ in 0..8 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
}

/// ── The detonator's fire detonates the placed mine and hands the mine back ───────────────────────
/// Firing 0x38 sets the player's 0x37 mines' fuse to 10.0 [M 0x1a4bf0 disasm 0x1a4c84-0x1a4c88] —
/// `Explode_Create(radius 8.0, damage 100.0)` lands 10-11 ticks later [M 0x12d890:268-281,388-393] —
/// and requests the switch back to the base mine [M 0x1a4cac-0x1a4cb8 `sb v0,99(v1)`].
#[test]
fn the_detonator_detonates_the_placed_mine_and_switches_back_to_it() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load_mine(&iso);
    throw_mine(&mut scene);
    mode_press_to_detonator(&mut scene);
    // Fire the detonator once.
    scene.sim.as_mut().unwrap().step_bond(Input::default(), true, false, false);
    let switch_back_tick = scene.sim.as_ref().unwrap().drones.frame;
    scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
    // The game's arm ran on the press: the switch request is the mine [M 0x1a4cb8].
    assert_eq!(scene.sim.as_ref().unwrap().gun.switch_request, 0x37,
        "the detonator's fire requests the switch back to the mine [M 0x1a4bf0 disasm 0x1a4cb8]");
    // The marked mine blasts within the 10-tick fuse (+ the tick the mark lands on).
    let mut blast = None;
    for t in 0..30u32 {
        scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
        let ex = &scene.sim.as_ref().unwrap().drones.explosions;
        if !ex.is_empty() { blast = Some((switch_back_tick + t, ex.clone())); break; }
    }
    let (at, ex) = match blast { Some(v) => v, None => panic!("the detonator's fire detonates the placed mine within ~11 ticks") };
    eprintln!("[detonate] explosion {} ticks after the press: {ex:?}", at - switch_back_tick);
    assert!(at - switch_back_tick <= 14, "the 10.0 fuse lands within the 11-tick window, took {}", at - switch_back_tick);
    assert!((ex[0].1 - 8.0).abs() < 1e-3, "the blast radius is weapon_data[0x37]+0x08 = 8.0 [L], got {}", ex[0].1);
    assert!((ex[0].2 - 100.0).abs() < 1e-3, "the blast damage is weapon_data[0x37]+0x0C = 100.0 [L], got {}", ex[0].2);
    assert_eq!(ex[0].4, 0x37, "the blast is the mine's record");
    // The mine is gone (the game's bullet object is destroyed by its own blast).
    assert!(scene.sim.as_ref().unwrap().drones.grenades.iter().all(|g| g.state >= 2), "the mine consumed itself");
}

/// ── The armed mine never self-detonates (the sister assertion) ───────────────────────────────────
/// `Bullet_init` gives a PLAYER-thrown mine BU+0xFC = 7777.0 [M 0x12c5a0:196-223] and `Bullet_DoTrails`
/// 0x12d890:268-273 skips the decrement at 7777.0 — the old port kept the guard's 420-tick fuse, so
/// the mine blew up on its own 7 seconds after the throw with no detonator press.
#[test]
fn the_armed_mine_never_self_detonates() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load_mine(&iso);
    throw_mine(&mut scene);
    // 600 idle ticks: well past the old 420-tick self-destruct, far short of anything else.
    for _ in 0..600 {
        scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
        assert!(scene.sim.as_ref().unwrap().drones.explosions.is_empty(),
            "the player's armed mine must not self-detonate [M 0x12c5a0:214-223 + 0x12d890:268-273]");
    }
    let g = &scene.sim.as_ref().unwrap().drones.grenades;
    assert!(!g.is_empty() && g.iter().any(|g| g.state < 2), "the mine is still armed");
}

/// ── The detonator's fire spawns no bullet (the sister assertion) ──────────────────────────────────
/// The +0x20000 arm returns before the bullet loop [M 0x1a4bf0 disasm 0x1a4c38 → 0x1a4cdc is the
/// non-0x20000 path]: no shot tick, no muzzle light (+0x95b never latches), even with no mines out —
/// but the has-ammo tail still requests the switch back to the mine [M 0x1a4cac-0x1a4cb8].
#[test]
fn the_detonator_fire_spawns_no_bullet_and_still_switches_back() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load_mine(&iso);
    // No throw: straight to the detonator with a full magazine and no mines placed.
    mode_press_to_detonator(&mut scene);
    scene.sim.as_mut().unwrap().step_bond(Input::default(), true, false, false);
    let sim = scene.sim.as_ref().unwrap();
    assert!(!sim.shot_tick, "the detonator's fire spawns no shot [M 0x1a4bf0's +0x20000 arm returns before the bullet loop]");
    assert_eq!(sim.gun.switch_request, 0x37, "with ammo left the arm still requests the mine [M 0x1a4cb8]");
}
