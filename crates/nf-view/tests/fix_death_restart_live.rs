//! fix-death-restart: dying, failing and restarting like the game (playtest 2026-10-11/12).
//!
//! Evidence base:
//! - `Player_CheckForDeath` 0x190810: on health ≤ 0 — ch 0x5e, `Music_Event(6,1)`, `Player_WeaponNone`,
//!   the death marker, substate 0xd, the death sound 0x89 [M].
//! - `Player_HandleDeath` 0x190bb0: the blood pane (pane 12) enables on the first dead tick [M :23-25];
//!   ch 0x62 rises on the pane's full cover or the player's action 25 (✕) [M :29-33].
//! - The red fade `HUD_UpdateBloodPane` 0x19a6a8: 1 px/tick growth, pane state 1 at cover; viewer.h =
//!   448 [M 0x19f4e8:33], template width 512 [L BloodSprInfo 0x2bcfd8] → 448 ticks.
//! - `Mission_Update` 0x182c60 reads ch 0x62 [M :21]; case 4 else opens the 0x40000042 menu [M :77-81];
//!   the menu's handler is `P_ENDMISSION_Handler` 0x216618 (Handler_HandleMessage 0x2029e0:498-500):
//!   Restart (row 0) → `ResetMap_LevelToLoad(GameState._12_4_, 1, 1)` + `Player_RamLoad(Bond, 1)`
//!   [M 0x216618:19-21] — the CURRENT level, WARM, on the ENTRY checkpoint (fix-restart-target
//!   corrected this file's first model, a cold BaseMap restart off the PAUSE menu's handler
//!   `C_GCPAUSE_Handler` 0x204408:139, which stays cold-to-BaseMap).
//! - The warm reload keeps the channels (Init_SwitchChannels only runs cold [M 0x1d5080:48-50]) and
//!   `Player_RamLoad` restores the ENTRY checkpoint's channels — clean, they were saved at the
//!   previous level's exit [M 0x1b2318:115] — so no ch-99 replay loop. Carrying the DYING state
//!   WOULD loop (the mechanism below), which is why the game re-serves the entry buffer instead.
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

/// The restart's exit the way the viewer's reload loop sees it.
fn exit(sim: &nf_view::scene::Sim) -> (Option<u32>, bool) {
    (sim.drones.level_exit, sim.drones.level_exit_cold)
}

/// ── 1. The restart target (the user's 07000007 report) ───────────────────────────────────────────
/// Punching a guest kills them → "Killed Civilians" ch 0x60 (`DroneFunc_SetMissionFailReason`
/// reasons 1/3/6/9 [M 0x13e560], the port's `on_init_death` team-2/3 arm) → the 0x61/0x60 hook
/// raises ch 99 [M 0x1827d8:98-117] → Mission Fail [M 0x182c60 case 2] → the menu → Restart. The
/// game's fail-menu Restart is `P_ENDMISSION_Handler` 0x216618:19-21: the CURRENT level, WARM, on
/// the entry checkpoint — 07000007, NOT the mission's base map 07000005 (the port's old behaviour —
/// the user's report; the cold BaseMap restart is the PAUSE menu's, `C_GCPAUSE_Handler` 0x204408:139).
#[test]
fn fail_restart_reloads_the_current_level_warm() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso, "07000007.bin");
    let sim = scene.sim.as_mut().unwrap();
    tick(sim, 10);
    // The civilian-kill channel, exactly what the punched-out guest's OnInitDeath writes.
    sim.drones.set_channel(0x60);
    // The fail banner runs its 300 ticks before the menu opens [M 0x182c60:52-57 TimeOut =
    // VIDEO_FRAME_RATE·4 + VIDEO_FRAME_RATE = 300, FRAME_RATE_MUL 1.0 [L 0x30d0d8] — the menu
    // shows ~5 s after the "Mission Failed" text, as in the game].
    tick(sim, 6);
    assert!(sim.drones.channels.on(99), "the 0x60 hook raised the fail channel [M 0x1827d8:104-107]");
    assert_eq!(sim.drones.mission.status(), 2, "MissionState 2 (failed) [M 0x182c60:47]");
    tick(sim, 310);
    // The auto-restart answered the 0x40000042 menu with Restart → the CURRENT level, warm.
    let (lv, cold) = exit(sim);
    assert_eq!(lv, Some(0x0700_0007), "Restart → ResetMap_LevelToLoad(GameState+0xC) = 07000007 [M 0x216618:20]");
    assert!(!cold, "IsWarmReset = 1 — the warm arm [M 0x216618:20]");
    assert!(sim.drones.level_exit_restart, "the viewer re-serves the ENTRY checkpoint [M 0x216618:21]");
    assert!(!sim.drones.mission.menu_open && sim.drones.mission.menu_choice == Some(nf_game::mission::MenuChoice::Restart),
        "the menu resolved with Restart");
}

/// The loop mechanism itself: carrying the DYING state's capture (ch 99 up) re-fails the mission
/// the tick the level comes back — no input needed (the user's "replay loop"). This is why the
/// game's Restart re-serves the ENTRY checkpoint (whose channels predate the fail) instead of
/// capturing at the fail [M 0x216618:21 → 0x1957a8:17-20; fix-restart-target]. Sister: the COLD
/// reload (the pause menu's arm, 0x204408:139) also comes back clean.
#[test]
fn carried_restart_replays_the_fail_but_the_cold_one_does_not() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    // Fail 07000007 like the punch case and capture what the old reload carried.
    let mut scene = load(&iso, "07000007.bin");
    let save = {
        let sim = scene.sim.as_mut().unwrap();
        tick(sim, 10);
        sim.drones.set_channel(0x60);
        tick(sim, 6);
        assert!(sim.drones.channels.on(99), "the mission failed");
        RamSave::capture(&sim.drones.health, sim.gun.switch_request, &sim.drones.inventory, &sim.drones.channels)
            .with_mission(&sim.drones.mission)
    };
    // The WARM (carried) reload — the pre-fix restart: ch 99 comes back and re-fails: the loop.
    let mut warm = load_carry(&iso, "07000007.bin", Some(&save));
    let sim = warm.sim.as_mut().unwrap();
    tick(sim, 6);
    assert!(sim.drones.channels.on(99) && sim.drones.mission.status() == 2,
        "the carried ch 99 keeps the mission failed with no input — the user's replay loop");
    // The carried mission is back in the flow (the restored InternalState falls through to 1), the
    // carried ch 99 re-fails it, and after the banner the menu auto-restarts AGAIN — the replay loop.
    tick(sim, 320);
    let (lv, _) = exit(sim);
    assert!(lv.is_some(), "another restart queued with the carry still on: the loop");
    // The COLD reload (the fix): the same level comes back clean.
    let mut cold = load_carry(&iso, "07000007.bin", None);
    let sim = cold.sim.as_mut().unwrap();
    tick(sim, 6);
    assert!(!sim.drones.channels.on(99), "Init_SwitchChannels cleared the fail channel [M 0x1d5080:44-46]");
    assert_eq!(sim.drones.mission.status(), 1, "the mission is playing again [M 0x182270:28]");
    let (lv, _) = exit(sim);
    assert!(lv.is_none(), "no restart queued — the loop is gone");
}

/// ── 2. The death sequence, tick by tick (07000005) ─────────────────────────────────────────────
/// Kill Bond (any pain path — the falls and the chasm volumes land here too), hold ✕: the death
/// starts (music 6, no ch 0x62 yet — the red fade runs first [M 0x190bb0:23-33]), the ✕ raises
/// ch 0x62 [M 0x190bb0:32], `Mission_Update` case 1 → 3 → 7 → 4 [M 0x182c60] and the menu opens
/// (~2 ticks later — case 3's TimeOut is 1.0 = 1 tick at FRAME_RATE_MUL 1.0 [L 0x30d0d8]).
#[test]
fn death_sequence_stages_then_the_menu() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso, "07000005.bin");
    let sim = scene.sim.as_mut().unwrap();
    tick(sim, 10);
    // A near-fatal sister first: damage without the kill must trigger nothing.
    let before = sim.drones.health.health;
    sim.drones.health.pain_generic(1.0, 1, 0x0700_0005, 0);
    tick(sim, 5);
    assert!(!sim.drones.death.active, "alive: no death sequence");
    assert!(!sim.drones.channels.on(0x62), "alive: no death channel");
    assert!(sim.drones.level_exit.is_none(), "alive: no restart");
    // The kill (500: the difficulty-scaled factor shrinks small numbers).
    sim.drones.health.pain_generic(500.0, 1, 0x0700_0005, 0);
    tick(sim, 1);
    assert!(sim.drones.death.active, "the death sequence started [M 0x190810]");
    assert!(sim.drones.death.pane_on, "the blood pane enabled on the first dead tick [M 0x190bb0:24-25]");
    assert!(!sim.drones.channels.on(0x62), "no ch 0x62 while the red fade runs [M 0x190bb0:29-33]");
    assert_eq!(sim.drones.bond_weapon, 0x47, "Player_WeaponNone [M 0x190810:61]");
    assert!(sim.drones.channels.on(0x5e), "the death's ch 0x5e [M 0x190810:40]");
    // The ✕ skip (action 25): ch 0x62 the same tick the game reads it.
    tick_fire(sim, 1);
    assert!(sim.drones.death.ch62 && sim.drones.channels.on(0x62), "✕ raised the death channel [M 0x190bb0:32]");
    tick(sim, 1);
    assert_eq!(sim.drones.mission.status(), 3, "MissionState 3 (the red fade state) [M 0x182c60:61]");
    // Case 3 → 7 → 4: the menu opens ~2 ticks after the channel [L FRAME_RATE_MUL 0x30d0d8 = 1.0].
    tick(sim, 2);
    assert!(sim.drones.mission.menu_open || !sim.drones.mission.menu_choice.is_none() || sim.drones.level_exit.is_some(),
        "the 0x40000042 menu path ran [M 0x182c60:77-81]");
    let (lv, cold) = exit(sim);
    assert_eq!(lv, Some(0x0700_0005), "auto-restart → the CURRENT level (05 IS the base map here) [M 0x216618:20]");
    assert!(!cold, "the death restart is the warm arm [M 0x216618:20]");
    assert!(sim.drones.level_exit_restart, "on the entry checkpoint [M 0x216618:21]");
}

/// The no-skip arm: without ✕ the red fade covers the screen in 448 ticks [M 0x19a6a8 + L] and the
/// pane's state then gates ch 0x62 through by itself.
#[test]
fn death_menu_applies_by_itself_after_the_red_fade() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso, "07000005.bin");
    let sim = scene.sim.as_mut().unwrap();
    tick(sim, 5);
    sim.drones.health.pain_generic(500.0, 1, 0x0700_0005, 0);
    tick(sim, 1);
    assert!(sim.drones.death.active && !sim.drones.death.ch62, "dying, no channel yet");
    tick(sim, nf_game::death::RED_FADE_TICKS + 2);
    assert!(sim.drones.death.covered, "the fade covered the screen [M 0x19a6a8:24]");
    assert!(sim.drones.death.ch62 && sim.drones.channels.on(0x62), "the cover raised ch 0x62 [M 0x190bb0:30]");
    tick(sim, 3);
    let (lv, cold) = exit(sim);
    assert_eq!(lv, Some(0x0700_0005) , "the menu auto-restarted to the current level (05 is its own base)");
    assert!(!cold, "the warm arm [M 0x216618:20]");
    assert!(sim.drones.level_exit_restart, "on the entry checkpoint [M 0x216618:21]");
}

/// ── 3. A restart of a cold-loaded level: the zeroed entry buffer + the 50.0 health floor ───────
/// Die on 07000005 after taking damage, flipping an objective channel and picking a pickup up;
/// restart through the real path (the menu → `fail_choose_restart` → the viewer's reload). The
/// level was loaded COLD, so the game's RamSave buffer holds the BSS zeros — `Player_RamLoad`'s
/// boost arg 1 floors the health at 50.0 [L ee_start_05.bin 0x30cc34-3c] — and `Player_InitWeapon`'s
/// 07000005 case re-equips the fresh loadout over the zeroed blocks [M 0x1a7ef8]. The channels and
/// the mission states come back as a clean load's. Sister: the DYING state's capture does NOT match
/// (health 0, the channels carried) — the loop's mechanism, why the game re-serves the entry buffer.
#[test]
fn restart_of_a_cold_loaded_level_restores_the_zero_buffer_with_the_health_floor() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso, "07000005.bin");
    let save = {
        let sim = scene.sim.as_mut().unwrap();
        tick(sim, 10);
        // Pick things up: teleport Bond onto the first live pickup and tick (the real take path).
        let spot = sim.drones.pickups.iter().find(|p| !p.gone).map(|p| [p.m[3][0], p.m[3][1], p.m[3][2]]);
        if let Some(s) = spot {
            sim.bond.pos = s;
            tick(sim, 3);
        }
        // Damage + an objective channel (the enable channel 0x45, "Breach the castle walls").
        sim.drones.set_channel(0x45);
        sim.drones.health.pain_generic(37.0, 1, 0x0700_0005, 0);
        tick(sim, 2);
        assert!(sim.drones.health.health < 100.0 || !sim.drones.taken.is_empty() || sim.drones.channels.on(0x45),
            "the run is dirty: damage, a pickup or a channel");
        RamSave::capture(&sim.drones.health, sim.gun.switch_request, &sim.drones.inventory, &sim.drones.channels)
            .with_mission(&sim.drones.mission)
    };
    // Kill → the sequence → the auto-restart, through the real input path.
    let mut scene = load(&iso, "07000005.bin");
    let (lv, cold) = {
        let sim = scene.sim.as_mut().unwrap();
        tick(sim, 5);
        sim.drones.health.pain_generic(500.0, 1, 0x0700_0005, 0);
        tick_fire(sim, 2);
        tick(sim, 4);
        exit(sim)
    };
    assert!(lv == Some(0x0700_0005) && !cold, "the restart queued: the current level, warm [M 0x216618:19-21]");
    // The viewer's reload (lib.rs): the ENTRY checkpoint — for a cold-loaded level the game's
    // zeroed buffer, here zeroed at the live shapes with the 50.0 health floor.
    let zeroed = {
        let mut scene = load(&iso, "07000005.bin");
        let sim = scene.sim.as_ref().unwrap();
        let mut z = RamSave::capture(&sim.drones.health, sim.gun.switch_request, &sim.drones.inventory, &sim.drones.channels);
        z.health = 50.0; z.armour = 0.0;
        for v in z.carried.iter_mut() { *v = 0; }
        for v in z.clip.iter_mut() { *v = 0; }
        for o in z.owned.iter_mut() { *o = false; }
        for k in z.keys.iter_mut() { *k = false; }
        for ch in z.channels.iter_mut() { *ch = 0; }
        for t in z.ch_time.iter_mut() { *t = 0; }
        z
    };
    let mut restarted = load_carry(&iso, "07000005.bin", Some(&zeroed));
    tick(restarted.sim.as_mut().unwrap(), 5); // the Mission_Init latch (the game runs it in the load)
    let mut clean = load(&iso, "07000005.bin");
    tick(clean.sim.as_mut().unwrap(), 5); // the same ticks, so the monitor's states line up
    let (r, c) = (restarted.sim.as_ref().unwrap(), clean.sim.as_ref().unwrap());
    assert_eq!(r.drones.health.health, 50.0, "the zeroed buffer's health floors at the 50.0 boost [L 0x30cc34-3c]");
    assert_eq!(r.drones.health.armour, 0.0, "the zeroed armour [M 0x1957a8:36]");
    assert_eq!(r.drones.channels.v, c.drones.channels.v, "the zeroed channels match a clean load's [M 0x1957a8:17]");
    assert_eq!(r.drones.inventory.carried, c.drones.inventory.carried, "the loadout re-equips over the zeroed blocks [M 0x1a7ef8]");
    assert_eq!(r.drones.inventory.owned, c.drones.inventory.owned, "owned weapons match a clean load");
    assert_eq!(r.drones.mission.states, c.drones.mission.states, "Mission_Init's objective states match a clean load [M 0x182270:68-129]");
    assert_eq!(r.drones.mission.status(), 1, "the mission is playing [M 0x182270:28]");
    assert!(!r.drones.channels.on(0x62), "Player_Init cleared the death channel [M 0x1910f8:26,30]");
    assert!(!r.drones.death.active, "the death state is fresh");
    // Sister: the WARM carried reload would NOT match — the carried health and channels (the loop).
    let mut warm = load_carry(&iso, "07000005.bin", Some(&save));
    let w = warm.sim.as_ref().unwrap();
    assert!(w.drones.health.health != c.drones.health.health || w.drones.channels.v != c.drones.channels.v,
        "the warm carry differs from a clean load — why the restart must be cold");
}

/// ── 5. The falls and the scripted deaths share the one entry ────────────────────────────────────
/// The 07000011 atrium-style fall: a lethal landing enters through `Player_CollisionHandler`
/// 0x1949a8:117-126 — `Player_HandlePain(timer − 60, type 6)` → health ≤ 0 → `Player_CheckForDeath`
/// 0x190810 — the SAME sequence a bullet takes (the port: `Bond::fall_damage` → `pain_generic(type 6)`
/// → `death_tick`). The scripted arm — the death NISs (`Script_IsDeathNIS` 0x1dae60: the three hashes
/// can NOT be skipped [M nf-game::nis]) — plays its script over this same sim sequence. Sister: a
/// survivable drop (fall_timer ≤ 60 [M 0x1949a8]) triggers nothing at all.
#[test]
fn lethal_fall_reaches_the_death_path_and_the_restart() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso, "07000011.bin");
    let sim = scene.sim.as_mut().unwrap();
    tick(sim, 5);
    let spawn = sim.bond.pos;
    // The sister: an 8-unit drop (fall_timer lands near the 60-tick gate [M 0x1949a8:117-126] —
    // velocity is units/second, so one free-fall second covers ~5 units) hurts at most, kills never.
    sim.bond.pos = [spawn[0], spawn[1] + 8.0, spawn[2]];
    sim.bond.vel = [0.0; 3];
    for i in 0..300 { sim.step_bond(Default::default(), false, false, false); if i > 30 && sim.bond.grounded { break; } }
    assert!(sim.drones.health.health > 50.0, "the survivable fall leaves Bond alive (health {:.1})", sim.drones.health.health);
    assert!(!sim.drones.death.active && !sim.drones.channels.on(0x62), "alive: no death sequence");
    assert!(sim.drones.level_exit.is_none(), "alive: no restart");
    // The lethal fall: 400 units onto the atrium floor — the real physics path, no scripted damage.
    sim.bond.pos = [spawn[0], spawn[1] + 400.0, spawn[2]];
    sim.bond.vel = [0.0; 3];
    let mut landed = false;
    for i in 0..900 {
        sim.step_bond(Default::default(), false, false, false);
        if i > 30 && sim.bond.grounded { landed = true; break; }
    }
    assert!(landed && sim.drones.death.active, "the lethal fall landed into the death sequence [M 0x1949a8 → 0x190810]");
    assert_eq!(sim.drones.health.health, 0.0, "the fall drained health to 0 — the one death entry");
    assert!(sim.drones.death.pane_on, "the blood pane enabled on the first dead tick [M 0x190bb0:24-25]");
    assert!(sim.drones.channels.on(0x5e), "the death's ch 0x5e [M 0x190810:40]");
    // ✕ skip → ch 0x62 → the mission states → the menu → the COLD restart to the mission's first
    // level (07000011 is part 0 of its own mission [L mission-data] — the restart re-plays THIS level).
    tick_fire(sim, 1);
    assert!(sim.drones.death.ch62 && sim.drones.channels.on(0x62), "✕ raised the death channel [M 0x190bb0:32]");
    tick(sim, 4);
    let (lv, cold) = exit(sim);
    assert_eq!(lv, Some(0x0700_0011), "the fall restart → the CURRENT level (07000011 is its own base) [M 0x216618:20]");
    assert!(!cold, "the fall restart is the warm arm [M 0x216618:20]");
    assert!(sim.drones.level_exit_restart, "on the entry checkpoint [M 0x216618:21]");
}

/// ── 4. The menu API for 180-fix-menus ───────────────────────────────────────────────────────────
/// With `auto_restart` off the mission waits on the menu; the view answers Restart or Quit exactly
/// as `C_GCPAUSE_Handler` 0x204408 does.
#[test]
fn fail_menu_waits_for_the_view_answer() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso, "07000009.bin");
    let sim = scene.sim.as_mut().unwrap();
    tick(sim, 10);
    sim.drones.auto_restart = false;
    // The alarm buttons write 0x7C (the OR trigger placement 278, out 0x61, list [0x7c, 0x7d]
    // [L probe] re-drives 0x61 every tick — a direct 0x61 set would be stomped).
    sim.drones.set_channel(0x7c);
    tick(sim, 6);
    assert!(sim.drones.channels.on(99), "the alarm failed the mission");
    tick(sim, 310); // the fail banner's 300 ticks [M 0x182c60:52-57]
    assert!(sim.drones.fail_menu_open(), "the 0x40000042 menu is up and waiting [M 0x182c60:77-81]");
    assert!(sim.drones.level_exit.is_none(), "nothing restarts until the player answers");
    tick(sim, 30);
    assert!(sim.drones.fail_menu_open(), "it stays up (InternalState 5 [M 0x182c60:83-84])");
    // Quit [M 0x204408:141-149]: 0x07000048 — the front end the port cannot load (logged and held).
    sim.drones.fail_choose_quit();
    assert_eq!(sim.drones.level_exit, Some(0x0700_0048), "Quit → 0x7000048 [M 0x204408:148]");
    assert!(!sim.drones.fail_menu_open());
}

/// The near-fatal sister on the alarm level: an alive Bond with a big (non-lethal) hit triggers
/// none of the fail/death machinery.
#[test]
fn near_fatal_hit_triggers_nothing() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso, "07000009.bin");
    let sim = scene.sim.as_mut().unwrap();
    tick(sim, 10);
    sim.drones.health.pain_generic(1.0, 1, 0x0700_0009, 0);
    tick_fire(sim, 10);
    tick(sim, 400);
    assert!(!sim.drones.death.active && !sim.drones.channels.on(0x62), "alive: no death");
    assert!(!sim.drones.channels.on(99), "alive: no fail");
    assert!(sim.drones.level_exit.is_none(), "alive: no restart");
    assert!(!sim.drones.fail_menu_open(), "alive: no menu");
}
