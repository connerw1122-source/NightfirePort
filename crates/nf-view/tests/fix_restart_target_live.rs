//! fix-restart-target: the fail/death menu's Restart reloads the CURRENT level, warm, on the entry
//! checkpoint (playtest 2026-10-13: "rather than go to the start of the level it goes to the very
//! beginning map, 05").
//!
//! Evidence base (all [M] work/decomp/ACTION.ELF unless noted):
//! - The 0x40000042 fail/death menu's handler is `P_ENDMISSION_Handler` 0x216618 —
//!   `Handler_HandleMessage` 0x2029e0 routes menu id 0x40000042 to it (:498-500). The prior task's
//!   claim that `C_GCPAUSE_Handler` 0x204408 serves it was wrong: 0x204408 is bound to the PAUSE
//!   menu 0x10000028 (:217-219) whose Restart is `ResetMap_LevelToLoad(Mission_BaseMapHCode(), 0, 1)`
//!   — BaseMap, cold (:139).
//! - `P_ENDMISSION_Handler`'s event 0x4b arms (`__Menu_Send(0x100001a4, 0x35)` returns the row):
//!   row 0 → `ResetMap_LevelToLoad(GameState._12_4_, 1, 1)` + `Player_RamLoad(Bond, 1)`
//!   (0x216618:19-21) — the CURRENT level (GameState+0xC), `IsWarmReset = 1`, no 0x70→0x71 remap;
//!   row 1 → `ResetMap_LevelToLoad(Mission_BaseMapHCode(), 0, 1)` (BaseMap, cold, :15-17);
//!   row 2 → `GameState+0x1c = 0x4000001c` + `ResetMap_LevelToLoad(0x7000048, 0, 0)` (the front end).
//! - The warm reload keeps the switch channels (`Init_SwitchChannels` only runs `IsWarmReset == 0`,
//!   0x1d5080:48-50) and `Player_RamLoad` restores them from the RamSave (0x1957a8:17-20). The
//!   RamSave the restart restores is the ENTRY checkpoint — `Player_RamSave` ran at the previous
//!   level's exit (0x1b2318:115, Script_EventHandler case 0x12 0x1d94c0:196) — so its ch 99/100 are
//!   clean even though the level just failed: no replay loop. The dying state is NOT captured.
//! - The restart's health floor: `Player_RamLoad`'s second arg 1 applies the difficulty
//!   `ContinueHealthBoost*` — 50.0 on all three difficulties [L ee_start_05.bin 0x30cc34-3c].
//! - `Mission_Init(level, 1)` re-derives BaseMap/ThisOrderNum from the level and resets the current
//!   and future parts' objective states + condition channels (0x182270:68-129); previous parts keep
//!   theirs.
//! - On level entry `Player_InitWeapon` (0x1a7ef8) re-runs per level; 0x07000007's case RamLoads
//!   again, runs `Player_InitAmmoWeapons`, zeroes armour and equips the fixed party loadout
//!   (:176-186) — the port's `party()` (nf-game::player).
//!
//! Opt-in via `NF_ISO`.

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

/// One tick with the fire button held (action 25 — the ✕ skip [M 0x190bb0:30]).
fn tick_fire(sim: &mut nf_view::scene::Sim, n: u32) {
    for _ in 0..n { sim.step_bond(Default::default(), true, false, false); }
}

/// The restart's exit the way the viewer's reload loop sees it: (target, cold, restart).
fn exit(sim: &nf_view::scene::Sim) -> (Option<u32>, bool, bool) {
    (sim.drones.level_exit, sim.drones.level_exit_cold, sim.drones.level_exit_restart)
}

/// ── 1. The restart target (the user's 07000007 report) ─────────────────────────────────────────
/// A punched guest → "Killed Civilians" ch 0x60 (`DroneFunc_SetMissionFailReason` reason [M
/// 0x13e560]) → the 0x60 hook raises ch 99 [M 0x1827d8:103-107] → Mission Fail (MissionState 2 [M
/// 0x182c60:47]) → the 0x40000042 menu [M 0x182c60:77-81] → Restart. The game's Restart is
/// `P_ENDMISSION_Handler` row 0: the CURRENT level, warm, + `Player_RamLoad(Bond, 1)` — 07000007,
/// NOT the mission's base map 07000005 (the port's old behaviour — the user's report).
/// Sister: the pause-menu Restart stays BaseMap-cold (`C_GCPAUSE_Handler` 0x204408:139).
#[test]
fn fail_restart_reloads_the_current_level_warm() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso, "07000007.bin");
    let sim = scene.sim.as_mut().unwrap();
    tick(sim, 10);
    // The civilian-kill channel, exactly what the punched-out guest's OnInitDeath writes.
    sim.drones.set_channel(0x60);
    // The fail banner runs its 300 ticks before the menu opens [M 0x182c60:52-57].
    tick(sim, 6);
    assert!(sim.drones.channels.on(99), "the 0x60 hook raised the fail channel [M 0x1827d8:104-107]");
    assert_eq!(sim.drones.mission.status(), 2, "MissionState 2 (failed) [M 0x182c60:47]");
    tick(sim, 310);
    let (lv, cold, restart) = exit(sim);
    assert_eq!(lv, Some(0x0700_0007),
        "Restart → ResetMap_LevelToLoad(GameState+0xC) = the CURRENT level [M 0x216618:20]");
    assert!(!cold, "IsWarmReset = 1 — the warm arm [M 0x216618:20]");
    assert!(restart, "the viewer re-serves the ENTRY checkpoint, not a fresh capture [M 0x216618:21]");
    assert!(!sim.drones.mission.menu_open && sim.drones.mission.menu_choice == Some(nf_game::mission::MenuChoice::Restart),
        "the menu resolved with Restart");
}

/// ── 2. What the restart restores (07000007) ────────────────────────────────────────────────────
/// The full viewer reload the game's restart produces: the ENTRY checkpoint (the RamSave as of
/// level entry — captured here right after the load, before the run dirties it) restored WARM with
/// the 50.0 health floor [L 0x30cc34-3c], then `Player_InitWeapon` 0x07000007's fixed party loadout
/// [M 0x1a7ef8:176-186]. Sister: the died-with state (a capture taken at the fail, ch 99 up) is NOT
/// what the restart restores — carrying THAT is the replay loop the entry checkpoint avoids.
#[test]
fn restart_restores_the_entry_checkpoint_with_the_health_floor() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    // The entry checkpoint: what a warm 05→06→07 arrival would have carried into 07 (the viewer
    // keeps the save the level was loaded with; here the direct load's clean state stands in).
    let entry = {
        let mut scene = load(&iso, "07000007.bin");
        let sim = scene.sim.as_mut().unwrap();
        tick(sim, 10);
        RamSave::capture(&sim.drones.health, sim.gun.switch_request, &sim.drones.inventory, &sim.drones.channels)
            .with_mission(&sim.drones.mission)
    };
    // The run dirties the state and dies (a real pain path): damage, a channel, then the kill.
    let (lv, cold, restart) = {
        let mut scene = load(&iso, "07000007.bin");
        let sim = scene.sim.as_mut().unwrap();
        tick(sim, 10);
        sim.drones.set_channel(0x45);
        let (h, r) = (&mut sim.drones.health, &mut sim.drones.rng); h.pain_generic(r, 37.0, 1, 0x0700_0007, 0);
        tick(sim, 2);
        let (h, r) = (&mut sim.drones.health, &mut sim.drones.rng); h.pain_generic(r, 500.0, 1, 0x0700_0007, 0);
        tick_fire(sim, 2);
        tick(sim, 4);
        exit(sim)
    };
    assert_eq!(lv, Some(0x0700_0007) , "the restart targets the current level");
    assert!(!cold && restart, "warm, on the entry checkpoint");
    // The viewer's reload: the entry checkpoint, health floored at 50.0 (the game's boost [L]).
    let mut entry_boosted = entry.clone();
    entry_boosted.health = entry_boosted.health.max(50.0);
    let mut restarted = load_carry(&iso, "07000007.bin", Some(&entry_boosted));
    let clean = load(&iso, "07000007.bin");
    // A few ticks so the port's Mission_Init latch runs (the game runs it inside the load,
    // ResetMap_GameInit 0x1d5080:53; the port latches on the level's first tick).
    tick(restarted.sim.as_mut().unwrap(), 5);
    let (r, c) = (restarted.sim.as_ref().unwrap(), clean.sim.as_ref().unwrap());
    assert_eq!(r.drones.health.health, entry_boosted.health,
        "health = the entry checkpoint's, floored at 50.0 (here 100 ≥ 50) [M 0x1957a8:21-35]");
    assert_eq!(r.drones.health.armour, 0.0, "0x07000007's InitWeapon zeroes armour [M 0x1a7ef8:180]");
    assert!(r.drones.inventory.owned.iter().any(|&o| o), "the fixed party loadout is equipped [M 0x1a7ef8:181-184]");
    assert!(!r.drones.channels.on(99), "the entry checkpoint's channels are clean — ch 99 did not come back");
    assert!(!r.drones.channels.on(0x62), "Player_Init cleared the death channel [M 0x1910f8:26,30]");
    assert!(!r.drones.death.active, "the death state is fresh");
    assert_eq!(r.drones.mission.status(), 1, "the mission is playing again [M 0x182270:28]");
    // The loop check: 6 seconds of play on the restarted level must NOT re-fail (the user's loop
    // was: reload → the carried ch 99 re-fails → the menu → restart → repeat).
    let sim = restarted.sim.as_mut().unwrap();
    tick(sim, 360);
    assert!(!sim.drones.channels.on(99), "no re-fail without input — the loop is gone");
    let (lv2, _, _) = exit(sim);
    assert!(lv2.is_none(), "no restart queued — the loop is gone");
}
