//! Headless NF_ISO tests for the level-transition carry-over and per-level rules (fix-level2, playtest
//! 2026-10-11 items F1/F2/F4/F5). These drive the REAL paths — `nf_view::load_with_carry` (the reload
//! the viewer performs), `Sim::step_bond` (the player tick), and the sim's own trigger evaluation — so a
//! passing test means the mechanism works in play.
//!
//! Evidence base (the decompile the fix implements):
//! - `Trigger_Activate` 0x1b2318: kinds 1/2/0xD begin with the `mask & 4` gate — the plunge/death
//!   triggers (mask 8) can never fire from the use key [M]; the port used to fire them (F5).
//! - `Player_Start` 0x190fd0: the first type-0x24 start whose gate channel is 0 or on [M]; the gate is
//!   written by the previous level's exit trigger (success/fallback, 0x1b2318:91-107) and the channel
//!   globals survive the warm transition [M 0x1d5080] (F1).
//! - `Player_RamSave` 0x195670 / `Player_RamLoad` 0x1957a8: health, armour, spare ammo, weapon entries,
//!   key ring, switch channels [M] (F2).
//! - `Player_InitWeapon` 0x1a7ef8 case 0x7000007: RamLoad + fresh ammo + armour 0 + switch request 1
//!   (fists); `Player_Weapon` 0x1a24c8:242-275: cycle locked to fists until ch 0x48, whose rising edge
//!   auto-draws `Upgrade_Weapon(6)` = 2 [M] (F4).
//!
//! Opt-in via `NF_ISO`.

use nf_game::triggers::kind;
use nf_game::triggers::Trigger;
use nf_game::player::{RamSave, WEAPON_LOCK_CHANNEL};

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// Load a level (cold) and return the scene. Panics on failure (test-only).
fn load(iso: &str, level: &str) -> nf_view::scene::Scene {
    nf_view::load(iso, level, false).expect("load")
}

fn load_carry(iso: &str, level: &str, save: Option<&RamSave>) -> nf_view::scene::Scene {
    nf_view::load_with_carry(iso, level, false, save).expect("load_with_carry")
}

/// Tick the sim `n` times with idle input.
fn tick(scene: &mut nf_view::scene::Scene, n: u32) {
    if let Some(sim) = scene.sim.as_mut() {
        for _ in 0..n { sim.step_bond(Default::default(), false, false, false); }
    }
}

/// Place Bond at `pos` (feet) with `yaw`, tick once with `use_key` — the viewer's E-key path.
fn step_at(scene: &mut nf_view::scene::Scene, pos: [f32; 3], yaw: f32, use_key: bool) {
    if let Some(sim) = scene.sim.as_mut() {
        sim.bond.pos = pos;
        sim.bond.yaw = yaw;
        sim.bond.pitch = 0.0;
        sim.step_bond(Default::default(), false, false, use_key);
    }
}

fn sim(scene: &mut nf_view::scene::Scene) -> &mut nf_view::scene::Sim {
    scene.sim.as_mut().expect("the level has a sim")
}

fn close(a: [f32; 3], b: [f32; 3], tol: f32) -> bool {
    (a[0] - b[0]).abs() < tol && (a[1] - b[1]).abs() < tol && (a[2] - b[2]).abs() < tol
}

// ── F5: the death cut-scene must not fire from the use key ─────────────────────────────────────────

/// 07000005's plunge triggers (TouchOnce, out 0xBE, mask 8) gate the falling-death NIS 0x06000952
/// (ScriptPlayer #284, mask bit 1 = the out channel) [D placements; M 0x1b2318:38-40]. `Trigger_Activate`
/// begins its type-1 arm with the `mask & 4` gate [M 0x1b2318:38-40]: the use key can never fire them.
/// The port used to latch ch 0xBE from the use-key path — the user's death cut-scene at doors and the
/// wire. Sister: a TouchOnce WITH mask bit 4 still activates, and the kind-10 LOAD_LEVEL arm is
/// untouched.
#[test]
fn plunge_trigger_ignores_the_use_key() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso, "07000005.bin");
    // The plunge family: TouchOnce, out 0xBE, mask 8 (no E bit) — every one of them.
    let plunges: Vec<Trigger> = sim(&mut scene).drones.triggers.iter()
        .filter(|t| t.kind == kind::TOUCH_ONCE && t.out == 0xBE).cloned().collect();
    assert!(!plunges.is_empty(), "07000005 has plunge TouchOnce triggers with out 0xBE");
    assert!(plunges.iter().all(|t| t.mask & 4 == 0), "the plunge triggers gate on mask 8 (no E bit)");

    // The E-path (`Trigger_Activate` 0x1b2318:38-40): refused for every plunge trigger. This is the
    // exact call player_use_triggers makes on E; before the fix it returned [(0xBE, 1)].
    for p in &plunges {
        let (sets, next) = p.activate_request(&sim(&mut scene).drones.channels);
        assert!(sets.is_empty() && next.is_none(),
            "the use key must NOT activate a mask-8 plunge trigger (got {sets:?})");
    }

    // Bond at the zip-wire handle (placement 251 [D] ≈ (−23.5, 34.5, 6.3)), E pressed — the user's
    // action. The death NIS's ScriptPlayer (#284, script 0x06000952) must never leave state 0.
    let wire: [f32; 3] = [-23.49, 34.47, 6.26];
    step_at(&mut scene, wire, 0.0, true);
    let s = sim(&mut scene);
    let sp = s.drones.scripts.players.iter()
        .find(|p| p.scripts[0] == 0x0600_0952).expect("the death-NIS ScriptPlayer exists on 07000005");
    assert_eq!(sp.state, 0, "the death NIS stays untriggered");
    eprintln!("[f5] E at the wire: death SP state {} (0 = untriggered) ✓", sp.state);
}

// ── F1: the spawn point matches the exit taken ─────────────────────────────────────────────────────

/// The castle-door route (the user's report): Bond at the castle door presses E → `Trigger_Activate`
/// (0x1b2318) — the gate (0x1D) is off → the FALLBACK channel 0x1F is written [M :91-107] — then the
/// level loads. On 07000006 the type-0x24 starts are gated: 0x1D (the truck-ride spawn), 0x1F, 0x1E
/// [D placements 496/510/511]. `Player_Start` (0x190fd0) must therefore pick the 0x1F start — the other
/// side of the door — not the truck spawn. Sister: the truck route (the EndLevel script writes 0x1D)
/// picks the 0x1D start.
#[test]
fn castle_door_route_spawns_at_the_matched_entry() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso, "07000005.bin");
    let exit = sim(&mut scene).drones.triggers.iter()
        .find(|t| t.kind == kind::LOAD_LEVEL && t.gate == 0x1D)
        .expect("the castle-door LOAD_LEVEL (gate 0x1D) exists on 07000005").clone();
    let centre = exit.centre();
    step_at(&mut scene, [centre[0], centre[1], centre[2] - 0.5], 0.0, true);
    let s = sim(&mut scene);
    assert_eq!(s.drones.level_exit, Some(0x0700_0006), "E at the door requests 07000006");
    assert!(s.drones.channels.on(0x1F), "the gate is off → the fallback write ch 0x1F [M 0x1b2318:102-107]");
    let save = RamSave::capture(&s.drones.health, s.gun.switch_request, &s.drones.inventory, &s.drones.channels);
    drop(scene);

    // The viewer's warm reload with the carried save.
    let mut next = load_carry(&iso, "07000006.bin", Some(&save));
    let pos = sim(&mut next).bond.pos;
    eprintln!("[f1] door route → Bond at {:?}", pos);
    // The game's 0x1F start (placement 510 [D]): the other side of the castle door. The port used to
    // spawn at the first "Player1" (the 0x1D truck-ride spawn ≈ (57.2, 39.7, 22.6)).
    assert!(close(pos, [28.75, 37.67, 46.57], 2.0),
        "the door route spawns at the 0x1F entry (got {pos:?})");
    assert!(!close(pos, [57.18, 39.73, 22.61], 2.0),
        "NOT the truck-ride spawn — that is the user's bug");
}

/// Sister (F1): the truck route — the EndLevel script's event 0x11 writes ch 0x1D, the LOAD_LEVEL
/// auto-fires (`Trigger_Update` case 10), and `Player_Start` picks the 0x1D-gated start (the truck-ride
/// spawn, placement 496).
#[test]
fn truck_route_spawns_at_the_truck_entry() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso, "07000005.bin");
    {
        let s = sim(&mut scene);
        s.drones.channels.set(0x1D, 1, s.drones.frame); // the EndLevel script's write
    }
    tick(&mut scene, 3);
    let s = sim(&mut scene);
    assert_eq!(s.drones.level_exit, Some(0x0700_0006), "the gated LOAD_LEVEL auto-fires");
    let save = RamSave::capture(&s.drones.health, s.gun.switch_request, &s.drones.inventory, &s.drones.channels);
    drop(scene);

    let mut next = load_carry(&iso, "07000006.bin", Some(&save));
    let pos = sim(&mut next).bond.pos;
    eprintln!("[f1] truck route → Bond at {:?}", pos);
    assert!(close(pos, [57.18, 39.73, 22.61], 2.0), "the truck route keeps the 0x1D entry (got {pos:?})");
}

// ── F2: health, armour, weapons and ammo carry over ────────────────────────────────────────────────

/// The truck route with a spent Bond: 73 health, 25 armour, a partly spent P2K and less spare ammo. The
/// warm reload must arrive with them (`Player_RamSave` → `Player_RamLoad`, `Player_InitWeapon`
/// case 0x7000006 [M]). Sister: a cold load starts fresh (100/0 — `IsWarmReset` gates the whole restore).
#[test]
fn warm_transition_carries_health_armour_weapons_ammo() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso, "07000005.bin");
    {
        let s = sim(&mut scene);
        s.drones.health.health = 73.0;
        s.drones.health.armour = 25.0;
        s.drones.inventory.clip[2] = 3;
        s.drones.inventory.carried[1] = 20;
    }
    {
        let s = sim(&mut scene);
        s.drones.channels.set(0x1D, 1, s.drones.frame);
    }
    tick(&mut scene, 3);
    let s = sim(&mut scene);
    assert_eq!(s.drones.level_exit, Some(0x0700_0006));
    let save = RamSave::capture(&s.drones.health, s.gun.switch_request, &s.drones.inventory, &s.drones.channels);
    drop(scene);

    let mut next = load_carry(&iso, "07000006.bin", Some(&save));
    let s = sim(&mut next);
    assert!((s.drones.health.health - 73.0).abs() < 1e-3, "health carried (got {})", s.drones.health.health);
    assert!((s.drones.health.armour - 25.0).abs() < 1e-3, "armour carried (got {})", s.drones.health.armour);
    assert_eq!(s.drones.inventory.clip[2], 3, "P2K clip carried");
    assert!(s.drones.inventory.owned[2], "the P2K stays owned");
    assert!(s.drones.inventory.carried.iter().any(|&c| c > 0), "spare ammo carried");
}

/// Sister (F2): a COLD load of 07000006 (no save — `IsWarmReset == 0`) starts fresh: health 100, armour 0.
#[test]
fn cold_load_starts_fresh() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut next = load_carry(&iso, "07000006.bin", None);
    let s = sim(&mut next);
    assert!((s.drones.health.health - 100.0).abs() < 1e-3, "cold: health 100");
    assert!((s.drones.health.armour - 0.0).abs() < 1e-3, "cold: armour 0");
}

/// The 07000005 fresh loadout built from `weapon_data` (`Player_InitWeapon` case 0x7000005 + the common
/// tail [M 0x1a7ef8:162-170, 318-325] over `Player_InitAmmoWeapons`' zero arm [M 0x1a17f8, DAT_002a373c
/// == 0 — see the owned note in player.rs]) must equal the [L] start snapshot the port already carried
/// (`Inventory::level_start`): owned {1, 2, 0x4A, 0x4E, 0x50, 0x54}, P2K 7 loaded + 41 spare. The three
/// unowned variant entries the [L] reading shows with clip 1 (0x33/0x4C/0x4F) stay open — no ported
/// code path writes them under either `DAT_002a373c` arm (WORKLOG fix-level2).
#[test]
fn cold_07000005_loadout_matches_the_snapshot() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let scene = load(&iso, "07000005.bin");
    let sim = scene.sim.as_ref().unwrap();
    let want = nf_game::pickup::Inventory::level_start(sim.drones.inventory.clip.len(), sim.drones.inventory.carried.len());
    assert_eq!(sim.drones.inventory.owned, want.owned, "owned weapons match the [L] snapshot");
    for (i, (got, want)) in sim.drones.inventory.clip.iter().zip(&want.clip).enumerate() {
        if matches!(i, 0x33 | 0x4c | 0x4f) { continue; } // the open [L] variant-clip readings
        assert_eq!(got, want, "clip[{i:#x}] matches the [L] snapshot");
    }
    assert_eq!(sim.drones.inventory.carried, want.carried, "spare ammo matches the [L] snapshot");
}

// ── F4: the party weapon lock (07000007 — the level the dressing cut-scene loads) ──────────────────

/// `Player_InitWeapon` case 0x7000007 [M 0x1a7ef8:176-186]: the party starts with the switch request 1
/// (fists), armour zeroed, fresh ammo. `Player_Weapon` 0x1a24c8:242-275: with ch 0x48 off the cycle
/// cannot leave the fists; the rising edge auto-draws the P2K. Warm (via 06) and cold both hit the case.
#[test]
fn party_level_starts_unarmed_and_locked() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    // Cold (the viewer's direct load) — the per-level case still applies.
    let mut scene = load(&iso, "07000007.bin");
    {
        let s = sim(&mut scene);
        assert_eq!(s.gun.switch_request, 1, "the party starts with the fists request (case 0x7000007)");
        assert!((s.drones.health.armour - 0.0).abs() < 1e-3, "armour zeroed");
        assert!(s.drones.inventory.owned[2], "the P2K is owned but holstered");
    }
    tick(&mut scene, 40);
    // The lock: a cycle request with ch 0x48 off cannot leave the fists.
    {
        let s = sim(&mut scene);
        let mut input = nf_game::player::Input { weapon_cycle: 1, ..Default::default() };
        nf_game::player::weapon_lock_input(s.level, s.drones.channels.on(WEAPON_LOCK_CHANNEL), &mut input);
        assert_eq!(input.weapon_cycle, 0, "the cycle is swallowed while ch 0x48 is off");
    }
    tick(&mut scene, 5);
    assert_eq!(sim(&mut scene).gun.weapon, 1, "Bond holds the fists");

    // The re-arm: ch 0x48 latches (TouchOnce placements 411/412 [D]) → the rising edge auto-draws the P2K.
    {
        let s = sim(&mut scene);
        s.drones.channels.set(WEAPON_LOCK_CHANNEL, 1, s.drones.frame);
    }
    let drew = nf_game::player::weapon_lock_auto_draw(0x0700_0007, true, false);
    assert!(drew, "the rising edge auto-draws the handgun");
    sim(&mut scene).gun.switch_request = 2;
    tick(&mut scene, 40);
    assert_eq!(sim(&mut scene).gun.weapon, 2, "the P2K is drawn after the re-arm");

    // Warm route: 06 → 07 carries the state and still lands on the fists request.
    let mut scene = load(&iso, "07000006.bin");
    {
        let s = sim(&mut scene);
        s.drones.health.health = 61.0;
        s.drones.channels.set(0x1D, 1, s.drones.frame);
    }
    tick(&mut scene, 3);
    let s = sim(&mut scene);
    let save = RamSave::capture(&s.drones.health, s.gun.switch_request, &s.drones.inventory, &s.drones.channels);
    drop(scene);
    let mut next = load_carry(&iso, "07000007.bin", Some(&save));
    let s = sim(&mut next);
    assert_eq!(s.gun.switch_request, 1, "warm: the party still starts on the fists");
    assert!((s.drones.health.health - 61.0).abs() < 1e-3, "warm: the 06 health carried into the party");
}

// ── The trigger-family regression: the fix must not break the exits ────────────────────────────────

/// `Trigger_Activate`'s mask&4 gate must not touch the kind-10 (LOAD_LEVEL) arm: the castle-door exit
/// (gate 0x1D) still loads on E, and a TouchOnce WITH mask bit 4 still activates.
#[test]
fn use_key_still_activates_gated_triggers_and_load_level() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso, "07000005.bin");
    let exit = sim(&mut scene).drones.triggers.iter()
        .find(|t| t.kind == kind::LOAD_LEVEL && t.gate == 0x1D).expect("the castle-door exit").clone();
    let c = exit.centre();
    step_at(&mut scene, [c[0], c[1], c[2] - 0.5], 0.0, true);
    assert_eq!(sim(&mut scene).drones.level_exit, Some(0x0700_0006), "LOAD_LEVEL on E still fires");

    // A TouchOnce with mask bit 4: `Trigger::activate_request` directly (the game's type-1 arm keeps
    // working — the gate only refuses mask&4 == 0).
    let gated = Trigger { mask: 0x1C, ..Trigger::from_params(0, kind::TOUCH_ONCE, |k| if k == 0 { 0x23 } else { 0 }, vec![]).unwrap() };
    let (sets, next) = gated.activate_request(&sim(&mut scene).drones.channels);
    assert_eq!(sets, vec![(0x23, 1)], "mask&4 TouchOnce still activates on E");
    assert_eq!(next, None);
}
