//! fix-auto-switch: the pickup auto-switch drain is LIVE — the fix-pickups-remainder `if false`
//! gate in scene.rs is re-opened. Through the REAL path the viewer uses: `nf_view::load` + the dev
//! `sg5-pickup` scenario (a real `Pickup::create_simple` drop pushed into the live pickups list) +
//! `Sim::step_bond` ticks steered with real `Input` — no field pokes on the take/switch path.
//!
//! Evidence base:
//! - [M Player_EquipWeapon 0x1a8a58 tail]: a weapon take writes ONLY `anim+0x63` (the switch
//!   request) when the per-player auto-switch flag (DAT_002a38d2, live 1 [L ee_start_05]) is set
//!   and `Player_IsBetterWeapon` 0x1a6eb0 ranks the pickup better — it fills the inventory
//!   (+0x1ba owned, +0x170 ammo) but never touches the held-weapon byte itself.
//! - [M Player_SetWeaponAnimObj 0x1a2e38 default tail]: on the next idle tick `anim+0x63 !=
//!   anim+0x62` → `anim+0x64 = anim+0x62` → `Player_WeaponSelect` 0x1a2cc0 → the holster/draw
//!   machine (view states 1→2→3→4). The port's scene.rs machine mirrors this; the re-opened drain
//!   is the request write.
//! - [M census 0x1a5460/0x1a24c8/0x191dd0/0x193600]: the switch states gate only the fire
//!   pipeline — movement/walk/capsule are never gated on them, so the escort walk stays green
//!   (walkreal_07000001 re-run green with the drain live this session).

use nf_game::player::Input;
use nf_view::dev;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn dxz(a: [f32; 3], b: [f32; 3]) -> f32 { ((a[0] - b[0]).powi(2) + (a[2] - b[2]).powi(2)).sqrt() }

fn wrap_pi(mut a: f32) -> f32 {
    while a > std::f32::consts::PI { a -= std::f32::consts::TAU; }
    while a < -std::f32::consts::PI { a += std::f32::consts::TAU; }
    a
}

/// 07000005 with an SG5 Commando (0x16) drop at Bond's feet — the dev sg5-pickup scenario, the
/// EXACT functions the viewer calls (`--dev --scenario`). Returns the scene and the drop's XZ.
fn scene_with_sg5_at_bonds_feet(iso: &str) -> (nf_view::scene::Scene, [f32; 2]) {
    let mut scene = nf_view::load(iso, "07000005.bin", false).expect("load 07000005");
    let drop_xz = {
        let sim = scene.sim.as_ref().unwrap();
        let p = sim.bond.pos;
        [p[0] + 1.0, p[2] + 1.5] // the scenario's drop offset (dev.rs sg5_pickup)
    };
    let (_, f) = dev::scenario("sg5-pickup").expect("the sg5-pickup scenario exists");
    f(&mut scene);
    (scene, drop_xz)
}

/// Steer Bond to an XZ point with the real input path (turn + forward, the walker's band logic)
/// while ticking the sim; stops as soon as `until` fires or `max` ticks pass.
fn steer_until(scene: &mut nf_view::scene::Scene, xz: [f32; 2], max: u32, mut until: impl FnMut(&nf_view::scene::Sim) -> bool) -> Option<u32> {
    for t in 0..max {
        let (desired, d, yaw) = {
            let sim = scene.sim.as_ref().unwrap();
            let p = sim.bond.pos;
            ((xz[0] - p[0]).atan2(xz[1] - p[2]), dxz(p, [xz[0], p[1], xz[1]]), sim.bond.yaw)
        };
        let err = wrap_pi(desired - yaw);
        let input = Input { turn: err.clamp(-0.07, 0.07), forward: if d > 0.8 { 1.0 } else { 0.0 }, ..Default::default() };
        let sim = scene.sim.as_mut().unwrap();
        sim.step_bond(input, false, false, false);
        if until(sim) { return Some(t); }
    }
    None
}

/// Main test (would have FAILED before the fix — the gate dropped every auto-switch request, so
/// Bond held the P2K forever): taking a better-ranked dropped weapon fires the auto-switch — the
/// draw machine holsters the held weapon and draws the SG5, then settles idle.
///
/// Ranks [L BestWeapon 0x2bede0, the port's BEST_WEAPON table]: the SG5 (0x16) ranks 6th, the
/// spawn P2K (0x02) 16th — rank 6 < 16, so `Player_IsBetterWeapon` requests the switch.
#[test]
fn taking_a_better_weapon_switches_through_the_draw_machine() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let (mut scene, drop_xz) = scene_with_sg5_at_bonds_feet(&iso);
    let held_before = scene.sim.as_ref().unwrap().gun.weapon;
    eprintln!("[auto-switch] held weapon at load: {:#x}", held_before);
    assert_ne!(held_before, 0x16, "Bond already holds the SG5 — the scenario setup is wrong");

    // Walk the ~1.8 m to the drop and take it; the machine then swaps hands.
    let swapped = steer_until(&mut scene, drop_xz, 900, |sim| {
        sim.drones.inventory.owned.get(0x16).copied().unwrap_or(false)
            && sim.gun.weapon == 0x16
            && sim.gun.switch_state == 0
    });
    let at = swapped.expect("the draw machine never swapped to the SG5 (0x16) — the auto-switch \
        drain is not applying the take request");
    let sim = scene.sim.as_ref().unwrap();
    eprintln!("[auto-switch] swapped to the SG5 (0x16) at tick {at}, weapon {:#x}, switch_state {}",
        sim.gun.weapon, sim.gun.switch_state);

    // Sisters (pin the machine's settle behaviour around the fix):
    assert_eq!(sim.gun.weapon, 0x16, "Bond must hold the SG5 after the auto-switch");
    assert_eq!(sim.gun.switch_state, 0, "the draw machine must settle idle after the draw");
    assert!(sim.drones.inventory.owned.get(0x16).copied().unwrap_or(false),
        "the SG5 must be owned (the take filled the inventory)");
}

/// Sister: a take of a weapon that does NOT rank better never moves the draw machine
/// [M Player_IsBetterWeapon 0x1a6eb0: the verdict is rank(pickup) < rank(current); the P2K 0x06
/// ranks 14th — worse than the held SG5's 6th — so Player_EquipWeapon's tail writes no request
/// and Bond keeps the SG5 in his hands]. The P2K still goes into the inventory (the take is real).
#[test]
fn taking_a_worse_weapon_does_not_switch() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let (mut scene, drop_xz) = scene_with_sg5_at_bonds_feet(&iso);
    // Phase 1: hold the SG5 through the real path (as the main test).
    let swapped = steer_until(&mut scene, drop_xz, 900, |sim| {
        sim.gun.weapon == 0x16 && sim.gun.switch_state == 0
            && sim.drones.inventory.owned.get(0x16).copied().unwrap_or(false)
    });
    assert!(swapped.is_some(), "phase 1: the SG5 swap never settled (see the main test)");

    // Phase 2: a dropped WORSE-ranked weapon at Bond's feet. The P2K (0x06) is already owned at
    // this level's loadout (a 0x06 drop is an ammo take — [M EquipAmmo has no switch tail]), so
    // pick the first worse-ranked id the inventory does NOT own yet: its take is a real Weapon
    // equip whose IsBetterWeapon verdict is false — the sharp boundary the sister pins.
    // Ranks [L BestWeapon 0x2bede0]: SG5 (0x16) = 6; all of these rank worse (8th..15th).
    let (worse_id, p2k_xz) = {
        let sim = scene.sim.as_mut().unwrap();
        let worse_ranked = [0x0au8, 0x0c, 0x0d, 0x0e, 0x08, 0x10, 0x12, 0x04];
        let id = *worse_ranked.iter().find(|&&w| {
            let owned = sim.drones.inventory.owned.get(w as usize).copied().unwrap_or(true);
            let has_record = sim.weapons.records.get(w as usize).is_some();
            has_record && !owned
        }).expect("07000005's loadout owns every worse-ranked weapon — pick another sister id");
        let p = sim.bond.pos;
        let m: [[f32; 4]; 4] = [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [p[0] + 1.0, p[1], p[2] + 1.5, 1.0]];
        let made = if let Some(mut pickup) = nf_game::pickup::Pickup::create_simple(&m, id as u16, 16, nf_game::pickup::kind::WEAPON, &sim.weapons, &sim.models) {
            pickup.state = 1; // already on the floor (the sg5-pickup scenario's pattern)
            sim.drones.pickups.push(pickup);
            true
        } else { false };
        assert!(made, "could not create the {id:#x} drop");
        (id, [p[0] + 1.0, p[2] + 1.5])
    };
    let owned_before: Vec<bool> = scene.sim.as_ref().unwrap().drones.inventory.owned.clone();

    // Phase 3: the take equips it into the inventory (a new owned bit) but the held weapon must
    // stay the SG5 — no request, no draw cycle.
    let taken = steer_until(&mut scene, p2k_xz, 600, |sim| {
        sim.drones.inventory.owned.iter().zip(owned_before.iter()).any(|(now, was)| *now && !*was)
    });
    let at = taken.expect("the worse-ranked drop was never taken — the touch path broke");
    let sim = scene.sim.as_ref().unwrap();
    eprintln!("[auto-switch] {worse_id:#x} taken at tick +{at}; held weapon {:#x}", sim.gun.weapon);
    assert_eq!(sim.gun.weapon, 0x16, "a worse-ranked take must not move the draw machine [M 0x1a6eb0]");
    assert_eq!(sim.gun.switch_state, 0, "no switch cycle may run for a worse-ranked take");
}
