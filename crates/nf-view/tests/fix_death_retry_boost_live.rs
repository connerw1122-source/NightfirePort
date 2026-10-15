//! fix-death-retry-boost: `Player_RamLoad`'s boost argument — the fail-menu Restart's health floor.
//!
//! Evidence base:
//! - `Player_RamLoad` 0x1957a8:21-34 [M]: with the boost argument non-zero, the saved health
//!   (`RamSave._1532_4_`, the float at RamSave+0x5FC) is floored at the difficulty's
//!   `ContinueHealthBoost*` — a MAX, written BACK into the buffer (`swc1 $f12, 0x5fc` at 0x1958e4)
//!   before `Player_SetHealth` (jal 0x1960b8 at 0x1958e8) applies it. Armour is NOT boosted — it is
//!   restored separately, as-is (`BL+0x8B0 = RamSave._1536_4_` [M :36]). The whole body is gated on
//!   `IsWarmReset` [M :9] — a cold load never boosts.
//! - The table pick [M :21-33]: `GameState+0x40` (0x2a37a8) — 1 → `ContinueHealthBoostEasy`
//!   (0x30cc34), 2 → `ContinueHealthBoostMedium` (0x30cc38), anything else (0/3/4) →
//!   `ContinueHealthBoostHard` (0x30cc3c). All three are 50.0 on the retail build
//!   [L ee_start_05.bin 0x30cc34..3c].
//! - Who passes what: the fail/death menu's Restart row 0 passes 1 [M `P_ENDMISSION_Handler`
//!   0x216618:21]; `Player_InitWeapon` 0x1a7ef8 passes 0 on ALL EIGHT of its `Player_RamLoad`
//!   calls [M :158-305] — the trigger-exit warm load never boosts.
//! - Pre-fix probe (zz_boost_probe, scratch): the port's `Player_RamLoad` equivalent
//!   (`RamSave::restore`) gave a 10.0-health buffer back at 10.0 — the game's boost arm gives 50.0.
//!   The fix ports the boost as `RamSave::continue_boost` (nf-game::player) and the restart serve
//!   (`nf_view::fail_restart_save`) applies it, replacing fix-restart-health's hardcoded
//!   `max(50.0)` viewer constant.

use nf_game::player::RamSave;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn load(iso: &str, level: &str) -> nf_view::scene::Scene {
    nf_view::load(iso, level, false).expect("load")
}

fn load_carry(iso: &str, level: &str, save: Option<&RamSave>) -> nf_view::scene::Scene {
    nf_view::load_with_carry(iso, level, false, save).expect("load_with_carry")
}

fn tick(sim: &mut nf_view::scene::Sim, n: u32) {
    for _ in 0..n { sim.step_bond(Default::default(), false, false, false); }
}

/// One tick with the fire button held (action 25 — the ✕ death skip [M 0x190bb0:30]).
fn tick_fire(sim: &mut nf_view::scene::Sim, n: u32) {
    for _ in 0..n { sim.step_bond(Default::default(), true, false, false); }
}

fn exit(sim: &nf_view::scene::Sim) -> (Option<u32>, bool) {
    (sim.drones.level_exit, sim.drones.level_exit_cold)
}

// ── The game rule, at the game-logic layer (no ISO needed) ───────────────────────────────────────

/// `Player_RamLoad`'s boost is a MAX against the difficulty's table entry, not a set
/// [M 0x1957a8:21-34]: a 10.0 buffer rises to the 50.0 floor, a 60.0 buffer stays 60.0
/// (`fVar2 < health` keeps the saved value, else the boost wins). The entry is picked by
/// `GameState+0x40`: 1 → Easy, 2 → Medium, else (0/3/4) → Hard [M :23-33]; all 50.0 on retail
/// [L ee_start_05.bin 0x30cc34..3c]. Armour is untouched [M :36].
#[test]
fn boost_is_a_max_picked_by_the_difficulty_global() {
    // The pick [M 0x1957a8:23-33]. All three entries are 50.0 on retail [L 0x30cc34..3c].
    assert_eq!(nf_game::player::continue_health_boost(1), 50.0, "difficulty 1 → ContinueHealthBoostEasy [M 0x1957a8:23]");
    assert_eq!(nf_game::player::continue_health_boost(2), 50.0, "difficulty 2 → ContinueHealthBoostMedium [M 0x1957a8:23]");
    assert_eq!(nf_game::player::continue_health_boost(3), 50.0, "difficulty 3 → the else arm's ContinueHealthBoostHard [M 0x1957a8:29-32]");
    assert_eq!(nf_game::player::continue_health_boost(0), 50.0, "0 → the else arm too (the boot snapshot's in-play value)");
    assert_eq!(nf_game::player::CONTINUE_HEALTH_BOOST, [50.0, 50.0, 50.0], "[L ee_start_05.bin 0x30cc34..3c]");
    // A low buffer rises to the floor...
    let hp = nf_game::bullet::PlayerHealth { health: 10.0, armour: 25.0, flash: 0.0, dir_bits: 0, flashbang: 0.0, flash_total: 0.0, flash_alpha: 0.0, hit_push: 0.0 };
    let inv = nf_game::pickup::Inventory::empty(0x72, 0x30);
    let ch = nf_game::triggers::Channels { v: vec![0u8; 256], time: vec![0u32; 256] };
    let mut low = RamSave::capture(&hp, 2, &inv, &ch);
    low.continue_boost(2);
    assert_eq!(low.health, 50.0, "max(10, 50) = 50 — the boost arm [M 0x1957a8:21-34]");
    assert_eq!(low.armour, 25.0, "armour is NOT boosted — restored as-is [M 0x1957a8:36]");
    // ...a high buffer keeps the saved value (a max, not a set — the sister).
    let mut high = RamSave::capture(&{ let mut h = hp; h.health = 60.0; h }, 2, &inv, &ch);
    high.continue_boost(2);
    assert_eq!(high.health, 60.0, "max(60, 50) = 60 — the saved value wins [M 0x1957a8:25]");
    // And the floored buffer is what `restore` (the RamLoad body) applies — the game's
    // `Player_SetHealth` reads the written-back buffer [M 0x1958e4-88].
    let mut h2 = hp;
    let mut i2 = inv.clone();
    let mut c2 = ch.clone();
    let _w = low.restore(&mut h2, &mut i2, &mut c2);
    assert_eq!(h2.health, 50.0, "the reload's RamLoad applies the floored buffer");
    assert_eq!(h2.armour, 25.0, "the armour rides the buffer untouched [M 0x1957a8:36]");
}

/// The un-boosted arm: without the boost argument the buffer restores raw — the trigger-exit
/// warm load's shape (`Player_InitWeapon` 0x1a7ef8 passes 0 on all eight `Player_RamLoad` calls
/// [M :158-305]). A 10.0 buffer comes back at 10.0, armour as saved.
#[test]
fn without_the_boost_the_buffer_restores_raw() {
    let hp = nf_game::bullet::PlayerHealth { health: 10.0, armour: 25.0, flash: 0.0, dir_bits: 0, flashbang: 0.0, flash_total: 0.0, flash_alpha: 0.0, hit_push: 0.0 };
    let inv = nf_game::pickup::Inventory::empty(0x72, 0x30);
    let ch = nf_game::triggers::Channels { v: vec![0u8; 256], time: vec![0u32; 256] };
    let save = RamSave::capture(&hp, 2, &inv, &ch);
    let mut h2 = hp;
    let mut i2 = inv.clone();
    let mut c2 = ch.clone();
    let _w = save.restore(&mut h2, &mut i2, &mut c2);
    assert_eq!(h2.health, 10.0, "boost 0: the raw buffer value [M 0x1a7ef8:158-305]");
    assert_eq!(h2.armour, 25.0);
}

// ── The real input path (NF_ISO): die → Restart vs the trigger-exit carry ────────────────────────

/// The task's rule, end to end on 07000005: Bond's health drops to 10, the ENTRY checkpoint is
/// taken the way the level start does (`Player_InitWeapon`'s unconditional re-save [M
/// 0x1a7ef8:325] — the buffer a warm entry would hold, the previous level's exit having saved
/// 10 [M 0x1b2318:115]), Bond dies, the menu answers Restart (`fail_choose_restart`), and the
/// reload comes back at the game's rule: **50.0** — the floor is a max [M 0x1957a8:21-34]. The
/// serve the App's reload loop builds (`fail_restart_save`) already carries the floored buffer —
/// the handler's `Player_RamLoad(Bond, 1)` position in the flow [M 0x216618:21]. Sister inside:
/// the armour rides the buffer untouched, and the restart targets the current level warm.
#[test]
fn restart_after_death_at_low_health_comes_back_at_the_floor() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    // The run that leaves the buffer at 10: cold load, the drop, then the checkpoint capture.
    let mut scene = load(&iso, "07000005.bin");
    let entry = {
        let sim = scene.sim.as_mut().unwrap();
        tick(sim, 10);
        let (h, r) = (&mut sim.drones.health, &mut sim.drones.rng); h.pain_generic(r, 90.0, 1, 0x0700_0005, 0);
        assert!(sim.drones.health.health < 50.0 && sim.drones.health.health > 0.0,
            "the drop landed below the floor (health {:.1})", sim.drones.health.health);
        sim.drones.health.health = 10.0; // the test's pin: the exact buffer value the rule is asserted at
        // The ENTRY checkpoint the way the level start does [M 0x1a7ef8:325] — the App's
        // `entry_save` shape, here holding the dropped 10 (the warm-entry buffer).
        let e = nf_view::entry_checkpoint_save(sim);
        assert_eq!(e.health, 10.0, "the buffer holds the dropped health");
        e
    };
    // Die → the death sequence → the menu → Restart, through the real input path.
    let (lv, cold) = {
        let sim = scene.sim.as_mut().unwrap();
        sim.drones.auto_restart = false; // the menu waits for the pick
        let (h, r) = (&mut sim.drones.health, &mut sim.drones.rng); h.pain_generic(r, 500.0, 1, 0x0700_0005, 0);
        tick(sim, 1);
        assert!(sim.drones.death.active, "the death sequence started [M 0x190810]");
        tick_fire(sim, 1); // ✕ raises ch 0x62 [M 0x190bb0:32]
        tick(sim, 4); // the mission states 3 → 7 → 4 → the menu [M 0x182c60]
        assert!(sim.drones.fail_menu_open(), "the 0x40000042 menu is up [M 0x182c60:77-81]");
        sim.drones.fail_choose_restart(); // row 0 [M 0x216618:19-21]
        exit(sim)
    };
    assert_eq!(lv, Some(0x0700_0005), "Restart → the CURRENT level [M 0x216618:20]");
    assert!(!cold, "the warm arm [M 0x216618:20]");
    assert!(scene.sim.as_ref().unwrap().drones.level_exit_restart, "the restart flag = the boost-1 call [M 0x216618:21]");
    // The handler's `Player_RamLoad(Bond, 1)` — the serve carries the floored buffer
    // [M 0x216618:21 → 0x1957a8:21-34; the write-back 0x1958e4].
    let serve = nf_view::fail_restart_save(Some(&entry), scene.sim.as_ref().unwrap());
    assert_eq!(serve.health, 50.0, "the serve's buffer is floored at the difficulty's boost — max(10, 50) [M 0x1957a8:21-34]");
    assert_eq!(serve.armour, entry.armour, "the serve's armour is the buffer's, not boosted [M 0x1957a8:36]");
    // The reload: the game-logic restore applies the floored buffer — health 50.
    let mut restarted = load_carry(&iso, "07000005.bin", Some(&serve));
    tick(restarted.sim.as_mut().unwrap(), 5); // the Mission_Init latch, as the App's reload ticks
    let r = restarted.sim.as_ref().unwrap();
    assert_eq!(r.drones.health.health, 50.0, "the restart comes back at the floor — the game's rule (the pre-fix port gave 10.0)");
    assert_eq!(r.drones.health.armour, entry.armour, "the armour rides the buffer [M 0x1957a8:36]");
    assert!(!r.drones.death.active, "the reload is fresh");
}

/// The sister: the trigger-exit warm load passes boost 0 (`Player_InitWeapon` 0x1a7ef8 passes 0
/// on all eight `Player_RamLoad` calls [M :158-305]) — the same 10-health buffer carried the way
/// `Trigger_Activate`'s exit capture does [M 0x1b2318:97-116, the App's transition branch] comes
/// back at **10.0**, unfloored. The contrast with the restart arm's 50.0 IS the boost argument.
#[test]
fn trigger_exit_warm_load_keeps_the_low_health() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso, "07000005.bin");
    let exit_save = {
        let sim = scene.sim.as_mut().unwrap();
        tick(sim, 10);
        let (h, r) = (&mut sim.drones.health, &mut sim.drones.rng); h.pain_generic(r, 90.0, 1, 0x0700_0005, 0);
        sim.drones.health.health = 10.0; // the same pin: the buffer value the contrast is asserted at
        // `Trigger_Activate`'s exit capture [M 0x1b2318:97-116] — the App's transition branch shape.
        RamSave::capture(&sim.drones.health, sim.gun.switch_request, &sim.drones.inventory, &sim.drones.channels)
            .with_mission(&sim.drones.mission)
    };
    assert_eq!(exit_save.health, 10.0);
    // No death, no menu: the carry loads raw — `Player_RamLoad(param_1, 0)` [M 0x1a7ef8:158-305].
    let mut next = load_carry(&iso, "07000005.bin", Some(&exit_save));
    tick(next.sim.as_mut().unwrap(), 5);
    let n = next.sim.as_ref().unwrap();
    assert_eq!(n.drones.health.health, 10.0, "the trigger-exit warm load keeps the low health — boost 0, no floor");
    assert_eq!(n.drones.health.armour, exit_save.armour, "the armour as saved [M 0x1957a8:36]");
}
