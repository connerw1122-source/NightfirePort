//! fix-restart-health: the restart/checkpoint health — die and restart through the real flow, the
//! game's health/armour (the user: "if I die and it takes you to the beginning of the level, I
//! should be at full health. This is ridiculous." and (07): "I'm not really sure I believe you
//! when you say you start with half health at a checkpoint.").
//!
//! The game's model, end to end:
//! - EVERY level load ends in an unconditional `Player_RamSave` [M `Player_InitWeapon` 0x1a7ef8:325
//!   — SP; the MP arm jumps past it to `LAB_001a8a18`]: the RamSave buffer 0x2b9440 holds the
//!   POST-load state. On a COLD load nothing restores first (`Player_RamLoad`'s whole body is
//!   gated on `IsWarmReset` [M 0x1957a8:9]), so `Player_Init`'s health 100 / armour 0 [M
//!   0x1910f8:128-138] and the level arm's loadout ARE the buffer.
//! - The fail/death menu's Restart is `ResetMap_LevelToLoad(current, 1, 1)` + `Player_RamLoad(Bond,
//!   1)` [M `P_ENDMISSION_Handler` 0x216618:19-21]: the boost arg 1 floors the health at the
//!   difficulty's `ContinueHealthBoost*` — 50.0 on all three [L ACTION.ELF 0x30cc34-3c] — and the
//!   disasm writes the max BACK INTO the buffer (`lwc1 $f12, 0x5fc($s1)` … `swc1 $f12, 0x5fc($v1)`
//!   at 0x1958a0-0x1958e4) before `Player_SetHealth` (jal 0x1960b8 at 0x1958e8, $f12 = the floored
//!   value). The reload's `Player_InitWeapon` re-runs the level's entry arm over that buffer.
//! - So the restart health = **max(health at the last RamSave before the menu, 50.0)**: FULL
//!   (100.0) whenever the level was entered at full health — the common case — and the 50.0 floor
//!   ("half health") only when the last save was under 50 (a hurt entry or a hurt checkpoint).
//! - The port's bug: the viewer kept the save the level was LOADED with as the entry checkpoint —
//!   `None` after a cold load — and the restart then served a zeroed buffer: health
//!   max(0, 50) = **50.0 always**. The fix captures the POST-load state after every load
//!   (`nf_view::entry_checkpoint_save` — the 0x1a7ef8:325 mirror) and serves it with the floor
//!   (`nf_view::fail_restart_save`).
//!
//! Real input path: `nf_view::load` / `load_with_carry` + `Sim::step_bond` (the kills ride the
//! real health path, ✕ is action 25, the menu auto-restart is the sim's own flow).

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

fn pain(sim: &mut nf_view::scene::Sim, amount: f32, level: u32) {
    let (h, r) = (&mut sim.drones.health, &mut sim.drones.rng); h.pain_generic(r, amount, 1, level, 0);
}

/// Die → ✕ → the mission states → the 0x40000042 menu → the auto-Restart, all in the sim.
fn die_to_restart(sim: &mut nf_view::scene::Sim, level: u32) {
    pain(sim, 500.0, level);
    tick(sim, 1);
    assert!(sim.drones.death.active, "the death sequence started [M 0x190810]");
    tick_fire(sim, 2); // ✕ raises ch 0x62 [M 0x190bb0:32]
    tick(sim, 4);      // MissionState 3 → 7 → 4, the menu opens and auto-restarts [M 0x182c60]
}

/// ── 1. The user's report: die on a cold-loaded level → Restart → FULL health ────────────────────
/// The level was loaded COLD (the mission's first entry, or after the pause menu's cold Restart):
/// the game's buffer holds the POST-load re-save [M 0x1a7ef8:325] — health 100, the fresh loadout,
/// the clean channels — and the floored serve gives max(100, 50) = 100. The port's old model kept
/// the save the level was LOADED with (`None` here) and served a ZEROED buffer: 50.0, the
/// playtest's "half health".
#[test]
fn restart_after_death_on_a_cold_level_gives_full_health() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    // The cold load, and the entry checkpoint the App now captures after EVERY load
    // (`entry_checkpoint_save` — the 0x1a7ef8:325 mirror).
    let mut scene = load(&iso, "07000005.bin");
    let entry = {
        let sim = scene.sim.as_ref().unwrap();
        assert_eq!(sim.drones.health.health, 100.0, "a cold load spawns at Player_Init's health 100 [M 0x1910f8:138]");
        Some(nf_view::entry_checkpoint_save(sim))
    };
    assert_eq!(entry.as_ref().unwrap().health, 100.0,
        "the cold load's buffer holds health 100 [M 0x1a7ef8:325 + 0x1910f8:128-138]");
    // The dirty run: damage + an objective channel — none of it may ride the restart.
    let (lv, cold) = {
        let sim = scene.sim.as_mut().unwrap();
        tick(sim, 10);
        sim.drones.set_channel(0x45);
        pain(sim, 37.0, 0x0700_0005);
        tick(sim, 2);
        assert!(sim.drones.health.health < 100.0 && sim.drones.channels.on(0x45), "the run is dirty");
        die_to_restart(sim, 0x0700_0005);
        (sim.drones.level_exit, sim.drones.level_exit_cold)
    };
    assert!(lv == Some(0x0700_0005) && !cold && scene.sim.as_ref().unwrap().drones.level_exit_restart,
        "the fail menu auto-restarted: the CURRENT level, warm, on the entry checkpoint [M 0x216618:19-21]");
    // The viewer's serve (`fail_restart_save` — the reload loop's `level_exit_restart` branch).
    let serve = {
        let sim = scene.sim.as_ref().unwrap();
        let s = nf_view::fail_restart_save(entry.as_ref(), sim);
        assert_eq!(s.health, 100.0, "max(entry 100, the 50.0 floor) = FULL health [M 0x1957a8 + disasm 0x1958a0-0x1958e8]");
        assert_eq!(s.armour, 0.0, "the entry checkpoint's armour 0 [M 0x1910f8:130]");
        assert_eq!(s.channels, entry.as_ref().unwrap().channels, "the serve rides the entry checkpoint's channels — the dirty run did not [M 0x195670:13, 0x1957a8:17]");
        s
    };
    // The warm reload the viewer performs:
    let mut r = load_carry(&iso, "07000005.bin", Some(&serve));
    tick(r.sim.as_mut().unwrap(), 5);
    let mut clean = load(&iso, "07000005.bin");
    tick(clean.sim.as_mut().unwrap(), 5);
    let (rr, c) = (r.sim.as_ref().unwrap(), clean.sim.as_ref().unwrap());
    assert_eq!(rr.drones.health.health, 100.0, "RESTARTED AT FULL HEALTH — the user's report [M 0x1a7ef8:325]");
    assert_eq!(rr.drones.health.armour, 0.0, "armour 0 [M 0x1910f8:130]");
    assert_eq!(rr.drones.channels.v, c.drones.channels.v, "the channels match a clean load's [M 0x1957a8:17]");
    assert_eq!(rr.drones.inventory.owned, c.drones.inventory.owned, "the loadout is the fresh one [M 0x1a7ef8 case 0x7000005]");
    assert_eq!(rr.drones.mission.states, c.drones.mission.states, "Mission_Init's states match a clean load [M 0x182270:68-129]");
    assert_eq!(rr.drones.mission.status(), 1, "the mission is playing [M 0x182270:28]");
    assert!(!rr.drones.death.active && !rr.drones.channels.on(0x62), "no death rides the reload");
    // No re-fail over 360 ticks (the loop check fix-restart-target established).
    tick(rr_sim(&mut r), 360);
    assert_eq!(r.sim.as_ref().unwrap().drones.mission.status(), 1, "still playing — no replay loop");
    assert!(r.sim.as_ref().unwrap().drones.level_exit.is_none(), "no second restart queued");
}

fn rr_sim(r: &mut nf_view::scene::Scene) -> &mut nf_view::scene::Sim { r.sim.as_mut().unwrap() }

/// ── 2. (07)'s doubt: the checkpoint floor is real, but it is a FLOOR, not the rule ──────────────
/// `max(entry health, 50.0)`: entering 07000007 at FULL health and dying restarts at FULL 100; the
/// 50.0 ("half health") only bites when the last save was under 50 — a hurt entry (the warm carry
/// of a 37-health exit). The user was right to disbelieve an unconditional half health: the game
/// floors, it does not set [M 0x1957a8:20-31, the max idiom + disasm].
#[test]
fn restart_health_is_the_entry_health_floored_at_the_50_boost() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    // The exit saves, the way `Trigger_Activate` makes them [M 0x1b2318:115] — captured from the
    // live 07000005 sim at full health and at 37.
    let exit_full = {
        let mut s = load(&iso, "07000005.bin");
        tick(s.sim.as_mut().unwrap(), 10);
        let sim = s.sim.as_ref().unwrap();
        assert_eq!(sim.drones.health.health, 100.0);
        RamSave::capture(&sim.drones.health, sim.gun.switch_request, &sim.drones.inventory, &sim.drones.channels)
            .with_mission(&sim.drones.mission)
    };
    let exit_hurt = {
        let mut s = load(&iso, "07000005.bin");
        let sim = s.sim.as_mut().unwrap();
        tick(sim, 10);
        // The difficulty-scaled damage factor shrinks single hits (the pain-generic factor) —
        // apply until the health is genuinely under the 50.0 floor.
        for _ in 0..8 {
            if sim.drones.health.health < 50.0 { break; }
            pain(sim, 63.0, 0x0700_0005);
            tick(sim, 1);
        }
        tick(sim, 2);
        let sim = s.sim.as_ref().unwrap();
        assert!(sim.drones.health.health < 50.0, "the exit save's health is under the 50.0 floor, got {}", sim.drones.health.health);
        RamSave::capture(&sim.drones.health, sim.gun.switch_request, &sim.drones.inventory, &sim.drones.channels)
            .with_mission(&sim.drones.mission)
    };
    // (i) FULL entry → die → Restart → FULL 100.
    {
        let mut scene = load_carry(&iso, "07000007.bin", Some(&exit_full));
        let entry = {
            let sim = scene.sim.as_ref().unwrap();
            assert_eq!(sim.drones.health.health, 100.0, "the warm carry restores the exit health [M 0x1957a8]");
            Some(nf_view::entry_checkpoint_save(sim))
        };
        let serve = {
            let sim = scene.sim.as_mut().unwrap();
            tick(sim, 5);
            die_to_restart(sim, 0x0700_0007);
            nf_view::fail_restart_save(entry.as_ref(), sim)
        };
        assert_eq!(serve.health, 100.0, "max(100, 50) = 100 — full health at a full-health checkpoint");
        let mut r = load_carry(&iso, "07000007.bin", Some(&serve));
        tick(r.sim.as_mut().unwrap(), 5);
        let rr = r.sim.as_ref().unwrap();
        assert_eq!(rr.drones.health.health, 100.0, "restarted at FULL health [M 0x1a7ef8:325]");
        assert_eq!(rr.drones.health.armour, 0.0, "the 07000007 arm sets armour 0 [M 0x1a7ef8:180]");
        assert_eq!(rr.drones.mission.status(), 1, "the mission is playing");
        tick(rr_sim(&mut r), 360);
        assert_eq!(r.sim.as_ref().unwrap().drones.mission.status(), 1, "no re-fail over 360 ticks");
        assert!(r.sim.as_ref().unwrap().drones.level_exit.is_none());
    }
    // (ii) A hurt entry (37) → die → Restart → the 50.0 floor: "half health" IS the game's answer
    // when the last save was under 50 — the user's doubt resolved: a floor, not the rule.
    {
        let mut scene = load_carry(&iso, "07000007.bin", Some(&exit_hurt));
        let entry = {
            let sim = scene.sim.as_ref().unwrap();
            assert!(sim.drones.health.health < 50.0, "the hurt carry restores the sub-50 health");
            Some(nf_view::entry_checkpoint_save(sim))
        };
        let serve = {
            let sim = scene.sim.as_mut().unwrap();
            tick(sim, 5);
            die_to_restart(sim, 0x0700_0007);
            nf_view::fail_restart_save(entry.as_ref(), sim)
        };
        assert_eq!(serve.health, 50.0, "max(37, the 50.0 boost) = 50.0 [L ACTION.ELF 0x30cc34-3c]");
        let mut r = load_carry(&iso, "07000007.bin", Some(&serve));
        tick(r.sim.as_mut().unwrap(), 5);
        let rr = r.sim.as_ref().unwrap();
        assert_eq!(rr.drones.health.health, 50.0, "the floored restart health [M 0x1957a8:20-31]");
        assert_eq!(rr.drones.health.armour, 0.0, "armour 0 [M 0x1a7ef8:180]");
        assert_eq!(rr.drones.mission.status(), 1, "the mission is playing");
        tick(rr_sim(&mut r), 360);
        assert_eq!(r.sim.as_ref().unwrap().drones.mission.status(), 1, "no re-fail over 360 ticks");
        assert!(r.sim.as_ref().unwrap().drones.level_exit.is_none());
    }
}
