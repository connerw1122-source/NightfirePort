//! fix-spawner-p7 NF_ISO tests + fix-script-start-tick: the spawner's param 7 is the KILL channel,
//! measured live on 07000005 (docs/fields/hitdata-spawner-ally.md "Live result"): ch 0x22 spawns the
//! two gate guards; ch 0x43 (the wall-hug plate) runs SP 298's script (0x06000994 — event 5 freezes
//! the NPCs on the TOUCH TICK, the game's one-walk order), and when the script's end re-enables the
//! NPCs the kill latch removes both guards and the spawner — the placed pair behind the gate swaps
//! in at the same moment. Plus 07000011's stairwell (the fix-actors playtest case): the keep-level
//! split keeps a drawn guard. Every step goes through the real input path: `step_bond` with Bond
//! teleported into the disc's touch volumes. Opt-in: NF_ISO.

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn touch_centre(sim: &nf_view::scene::Sim, out: u8) -> [f32; 3] {
    let t = sim.drones.triggers.iter().find(|t| t.out == out)
        .unwrap_or_else(|| panic!("the ch {out:#x} touch exists [D placement 305/301]"));
    [(t.lo[0] + t.hi[0]) * 0.5, (t.lo[1] + t.hi[1]) * 0.5, (t.lo[2] + t.hi[2]) * 0.5]
}

fn group2(sim: &nf_view::scene::Sim) -> Vec<usize> {
    sim.drones.list.iter().enumerate().filter(|(_, d)| d.group == 2).map(|(i, _)| i).collect()
}

fn present(sim: &nf_view::scene::Sim, tpls: &[usize]) -> usize {
    tpls.iter().filter(|&&i| sim.drones.list[i].present).count()
}

/// The live measurement end to end, in the game's visible order [L docs/fields/hitdata-spawner-ally.md
/// "Live result"]: ch 0x22 → the two gate guards spawn; the wall-hug plate runs SP 298's script
/// (0x06000994 — the disc's placement data for placement 298; the live run's loadable-pack variant is
/// 0x06000AD4), whose event 5 freezes the NPCs IN THE TOUCH TICK and the whole ~1,400-tick scene —
/// the gate pair stays present the whole time, the two placed guards behind the gate stay disabled —
/// and when the script's end re-enables the NPCs the kill latch fires: both guards removed, the
/// spawner gone, and the placed pair switch on. The player never sees an empty gate.
#[test]
fn the_wall_hug_scene_holds_the_gate_pair_and_swaps_the_pairs_at_its_end() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005");
    // The placed pair behind the gate: drones at (32.4, 37.0, 49.9) / (27.6, 37.0, 49.7) — the live
    // read's (27.6, 37.2, 49.9) / (32.3, 37.2, 50.1), object flags 0x8b0 (disabled) during the scene
    // and 0x880 after [L hitdata-spawner-ally]. The port's `enabled` is the obj+0xF0 gate's stand-in.
    let placed = |sim: &nf_view::scene::Sim| -> Vec<usize> {
        sim.drones.list.iter().enumerate()
            .filter(|(_, d)| (d.pos[0] - 30.0).abs() < 4.0 && (d.pos[2] - 50.0).abs() < 4.0)
            .map(|(i, _)| i).collect()
    };
    for _ in 0..30 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    {
        let sim = scene.sim.as_mut().unwrap();
        let tpls = group2(sim);
        assert!(!tpls.is_empty(), "the gate spawner's group-2 templates exist [D placement 268]");
        assert!(tpls.iter().all(|&i| !sim.drones.list[i].present), "the spawner holds them back [M 0x137aa0]");
        assert_eq!(placed(sim).len(), 2, "the two placed guards behind the gate exist [L hitdata-spawner-ally]");
        // The trigger: ch 0x22 comes from the OR gate (placement 0xEF #25) over the three
        // level-start touches (placements 21/22/23, out 0x23/24/25 [D probe]) — Bond onto the
        // first one's volume, the real input path to the gate spawner's trigger channel.
        sim.bond.pos = touch_centre(sim, 0x23);
    }
    for _ in 0..10 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    {
        let sim = scene.sim.as_mut().unwrap();
        assert!(sim.drones.channels.on(0x22), "the gate touch fires ch 0x22 (via the OR) [D]");
        // Step Bond away from the spawn spots so the stealth checks (p8 = 4.0 [D]) let them through.
        sim.bond.pos = [104.9, 25.7, 108.8];
    }
    let mut spawned = 0;
    for _ in 0..24u32 {
        scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
        spawned = { let sim = scene.sim.as_ref().unwrap(); present(sim, &group2(sim)) };
        if spawned == 2 { break; }
    }
    {
        let sim = scene.sim.as_ref().unwrap();
        assert_eq!(spawned, 2, "the two gate guards spawn on ch 0x22 [L: 15 drones, two at the gate]");
        let sp = sim.drones.spawners.iter().find(|sp| sp.group == 2).expect("the gate spawner");
        assert!(!sp.gone, "the spawner is alive");
    }
    // Sister-in-advance: without ch 0x43 they stay.
    for _ in 0..60 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    {
        let sim = scene.sim.as_ref().unwrap();
        assert_eq!(present(sim, &group2(sim)), 2, "no kill channel: both guards stay");
    }
    // The kill channel: Bond onto the wall-hug plate (placement 301, out 0x43 [D]).
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.bond.pos = touch_centre(sim, 0x43);
    }
    scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
    // THE TOUCH TICK, in the game's walk order [M control_movement_object_handler 0x1336f8:54-68:
    // one newest-first walk runs every control func — the plate #301 writes ch 0x43, then SP #298
    // (placement 298) triggers and `SP_LoadScript`'s trailing `Script_Update` [M 0x1dbc08:56] runs
    // its first commands (event 5 → `Drone_EnableAll(0)` [M 0x1d94c0 case 5]), then the spawner
    // (placement 268) returns at `NPCGlobals == 0` [M 0x137e68:21-23] with its latch still off]:
    // the freeze holds the SAME tick, the gate pair is still present, the spawner alive.
    {
        let sim = scene.sim.as_ref().unwrap();
        assert!(sim.drones.channels.on(0x43), "the wall-hug plate fires ch 0x43 [D]");
        assert!(!sim.drones.npc_enabled, "the freeze lands on the touch tick (event 5 in the same walk) [L]");
        assert_eq!(present(sim, &group2(sim)), 2, "the gate pair survives the touch tick — the spawner was frozen first [L: no empty gate]");
        let sp = sim.drones.spawners.iter().find(|sp| sp.group == 2).expect("the gate spawner");
        assert!(!sp.gone && !sp.kill_latch, "the spawner is alive with its kill latch still off [L: +0x20 stays 0 through the scene]");
        assert!(placed(sim).iter().all(|&i| !sim.drones.list[i].enabled), "the placed pair is disabled by the freeze (obj flags 0x8b0) [L]");
    }
    // MID-SCENE (~300 ticks in, the scene is 1,400 [D script 0x06000994 length]): the same picture.
    for _ in 0..300 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    {
        let sim = scene.sim.as_ref().unwrap();
        assert!(!sim.drones.npc_enabled, "the freeze holds through the scene [L]");
        assert_eq!(present(sim, &group2(sim)), 2, "the gate pair is present mid-scene [L: 15 drones throughout]");
        let sp = sim.drones.spawners.iter().find(|sp| sp.group == 2).expect("the gate spawner");
        assert!(!sp.kill_latch, "the latch waits (Control returns at its first line) [M 0x137e68:21-23]");
        assert!(placed(sim).iter().all(|&i| !sim.drones.list[i].enabled), "the placed pair stays hidden mid-scene [L]");
    }
    // The script's end re-enables the NPCs (stream 8's event 7 at clock 1398 → `Script_Free`'s 0x80
    // flag [M 0x1da880:41-43]); then the latch fires and the kill scan removes both guards [L: 15 → 13].
    let mut freed = None;
    for k in 0..1500u32 {
        scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
        if scene.sim.as_ref().unwrap().drones.npc_enabled { freed = Some(k); break; }
    }
    assert!(freed.is_some(), "the script's end re-enabled the NPCs (event 7 → Script_Free 0x80) [L]");
    let mut gone_ticks = None;
    for k in 0..30u32 {
        scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
        let sim = scene.sim.as_ref().unwrap();
        if present(sim, &group2(sim)) == 0 { gone_ticks = Some(k); break; }
    }
    {
        let sim = scene.sim.as_ref().unwrap();
        assert!(gone_ticks.is_some(), "both gate guards removed once Control runs again [L: 15 → 13]");
        let sp = sim.drones.spawners.iter().find(|sp| sp.group == 2).expect("the gate spawner");
        assert!(sp.gone, "the spawner deletes itself [M 0x137e68: obj+0xFE |= 1]");
        assert!(placed(sim).iter().all(|&i| sim.drones.list[i].enabled), "the placed pair switched on at the re-enable (0x8b0 → 0x880) [L]");
    }
    // And it never spawns again.
    for _ in 0..120 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    let sim = scene.sim.as_ref().unwrap();
    assert_eq!(present(sim, &group2(sim)), 0, "the spawner never spawns again");
}

/// The sister: a script started on a tick whose first command is a WAIT starts its clock on the
/// start tick. SP 298's script stream 12 is `Wait(369)` → `Sound(368)` [D 0x06000994 decoded]: the
/// sound lands exactly 369 ticks after the touch (the start tick), not 370 — the old pre-fix player
/// (first commands one tick after `start`) played it at touch+370.
#[test]
fn the_wait_first_stream_starts_its_clock_on_the_start_tick() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005");
    for _ in 0..30 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.bond.pos = touch_centre(sim, 0x43);
    }
    scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
    {
        let sim = scene.sim.as_ref().unwrap();
        assert!(sim.drones.channels.on(0x43), "the wall-hug plate fires ch 0x43 [D]");
        assert!(!sim.drones.npc_enabled, "the script started and ran its first commands this tick [M 0x1dbc08:56]");
        let at = |sim: &nf_view::scene::Sim| sim.drones.script_sounds.iter().any(|s| s.0 == (0x0600_0994, 12));
        assert!(!at(sim), "stream 12's Wait(369) holds the sound back on the start tick [D 0x06000994 stream 12]");
    }
    // 368 ticks later: still waiting (369 − 1 for the start tick's own update); the 369th tick after
    // the touch plays it.
    for _ in 0..368 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    {
        let sim = scene.sim.as_ref().unwrap();
        assert!(!sim.drones.script_sounds.iter().any(|s| s.0 == (0x0600_0994, 12)), "the wait is not done at touch+368 [D]");
    }
    scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
    let sim = scene.sim.as_ref().unwrap();
    assert!(sim.drones.script_sounds.iter().any(|s| s.0 == (0x0600_0994, 12)),
        "the sound lands at touch+369 — the clock started on the start tick [D; pre-fix: touch+370]");
}

/// The sister: the same spawn sequence without the kill channel — the guards stay.
#[test]
fn without_the_kill_channel_the_gate_guards_stay() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005");
    for _ in 0..30 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.bond.pos = touch_centre(sim, 0x23); // the level-start touch feeding the ch-0x22 OR [D]
    }
    for _ in 0..10 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    scene.sim.as_mut().unwrap().bond.pos = [104.9, 25.7, 108.8];
    for _ in 0..120u32 {
        scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
        let sim = scene.sim.as_ref().unwrap();
        if present(sim, &group2(sim)) == 2 { break; }
    }
    for _ in 0..300 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    let sim = scene.sim.as_ref().unwrap();
    assert_eq!(present(sim, &group2(sim)), 2, "no ch 0x43: both gate guards remain");
    assert!(sim.drones.spawners.iter().find(|sp| sp.group == 2).map(|sp| !sp.gone).unwrap_or(false),
        "the spawner stays");
}

/// A spawner gated while the NPCs are disabled does nothing until they are re-enabled: with
/// `npc_enabled` forced off (what the game's `Drone_EnableAll(0)` does), the kill channel fires and
/// nothing happens; the tick Control runs again (the script's `Script_Free`), the removal lands.
#[test]
fn the_kill_channel_waits_for_the_npcs_to_be_re_enabled() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005");
    for _ in 0..30 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.bond.pos = touch_centre(sim, 0x23); // the level-start touch feeding the ch-0x22 OR [D]
    }
    for _ in 0..10 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    scene.sim.as_mut().unwrap().bond.pos = [104.9, 25.7, 108.8];
    for _ in 0..120u32 {
        scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
        let sim = scene.sim.as_ref().unwrap();
        if present(sim, &group2(sim)) == 2 { break; }
    }
    {
        let sim = scene.sim.as_mut().unwrap();
        assert_eq!(present(sim, &group2(sim)), 2, "both guards spawned");
        sim.drones.npc_enabled = false; // the game's Drone_EnableAll(0) state [M 0x1383e8]
        sim.bond.pos = touch_centre(sim, 0x43);
    }
    for _ in 0..60 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    {
        let sim = scene.sim.as_ref().unwrap();
        assert!(sim.drones.channels.on(0x43), "the kill channel is on");
        assert!(!sim.drones.npc_enabled, "the script froze the NPCs again");
        assert_eq!(present(sim, &group2(sim)), 2, "the spawner's Control returns at its first line: no removal [M 0x137e68:21-23]");
    }
    // The script frees the NPCs (flag 0x80) and the removal lands.
    let mut freed = false;
    for _ in 0..6000u32 {
        let sim = scene.sim.as_mut().unwrap();
        sim.step_bond(Default::default(), false, false, false);
        if sim.drones.npc_enabled { freed = true; break; }
    }
    assert!(freed, "Script_Free re-enabled the NPCs");
    for _ in 0..30u32 {
        scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
        let sim = scene.sim.as_ref().unwrap();
        if present(sim, &group2(sim)) == 0 { break; }
    }
    let sim = scene.sim.as_ref().unwrap();
    assert_eq!(present(sim, &group2(sim)), 0, "the removal lands once Control runs [L]");
}

/// 07000011, map-tower2's stairwell (the fix-actors playtest case): the group-1 spawner (mode 3,
/// waves 2, trigger ch 0x15 ← TouchOnce #182, stop ch 0x16 ← #183, kill ch 0x1D ← Touch #285 [D])
/// brings its guard in on the trigger, and the kill channel behaves as the keep-level split says
/// [M 0x137e68:235-262]: a guard the game is drawing (`obj+0xFC` != 0 — the port's per-tick
/// `drawn` [M `lbu $v0, 0xfc($s0)` 0x13805c]) STAYS; once he is not drawn any more the next scan
/// removes him and the spawner deletes itself. Sister-in-advance: without ch 0x1D he stays.
#[test]
fn tower2_stairwell_the_kill_channel_keeps_the_drawn_guard() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000011.bin", false).expect("load 07000011");
    // The level-start cut-scene holds the NPCs; wait it out (the fix-actors `free` shape).
    for _ in 0..400 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); if !scene.sim.as_ref().unwrap().disabled { break; } }
    let tpls: Vec<usize> = scene.sim.as_ref().unwrap().drones.list.iter().enumerate()
        .filter(|(_, d)| d.group == 1).map(|(i, _)| i).collect();
    let present = |s: &nf_view::scene::Sim| tpls.iter().filter(|&&i| s.drones.list[i].present).count();
    {
        let sim = scene.sim.as_mut().unwrap();
        assert!(!tpls.is_empty(), "the group-1 templates exist [D]");
        assert_eq!(present(&sim), 0, "the spawner holds them back [M 0x137aa0]");
        // The trigger: TouchOnce #182 (out 0x15 [D], centre of lo [6.02, 47.09, -4.65] /
        // hi [10.48, 49.16, 0.10]) — the real input path to the spawner's start channel.
        sim.bond.pos = [8.25, 48.13, -2.28];
    }
    for _ in 0..10 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    {
        let sim = scene.sim.as_ref().unwrap();
        assert!(sim.drones.channels.on(0x15), "touch #182 fires ch 0x15 [D]");
        assert!(sim.drones.spawners.iter().find(|sp| sp.group == 1).map(|sp| sp.active).unwrap_or(false),
            "the stairwell spawner is active on its trigger [M 0x137e68:53-64]");
    }
    // A spot where the stealth checks pass (the templates far and behind: p8 ≈ 0 [D], the
    // visibility half of spot_ok does the work). Facing +z, away from every template spot.
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.bond.pos = [-30.0, 48.0, 40.0];
        sim.bond.yaw = 0.0;
    }
    let mut spawned = 0;
    for _ in 0..24u32 {
        scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
        spawned = present(&scene.sim.as_ref().unwrap());
        if spawned > 0 { break; }
    }
    {
        let sim = scene.sim.as_ref().unwrap();
        assert_eq!(spawned, 1, "the guard arrives (mode 3: only when none alive [M 0x137e68:180-203])");
    }
    let guard = tpls.iter().copied().find(|&i| scene.sim.as_ref().unwrap().drones.list[i].present).expect("the arrived guard");
    // Sister-in-advance: no kill channel yet, the guard stays. (Short — this level's end-level
    // script chain freezes the NPCs a minute in, and the drawn step below needs him enabled.)
    for _ in 0..12 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    assert_eq!(present(&scene.sim.as_ref().unwrap()), 1, "no ch 0x1D: the arrived guard stays");
    // Bond right next to him: `visible_to`'s l < 2 arm → drawn (the game's renderer sets
    // obj+0xFC = 2 for every drawn object [M 0x1e62d8:71]).
    {
        let sim = scene.sim.as_mut().unwrap();
        let p = sim.drones.list[guard].pos;
        sim.bond.pos = [p[0] + 1.0, p[1], p[2]];
    }
    scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
    {
        let sim = scene.sim.as_ref().unwrap();
        assert!(sim.drones.list[guard].drawn, "Bond next to the guard: the game would draw him (obj+0xFC = 2 [M 0x1e62d8:71])");
    }
    // The stairwell kill touch #285 (out 0x1D [D], centre of lo [13.62, 31.29, -2.86] /
    // hi [15.69, 35.75, -0.78]). The latch fires and the kill scan reads LAST tick's drawn
    // (the spawner's Control runs before this tick's drone pass) → the drawn guard is KEPT
    // [M 0x138060 `bnez $v0` → 0x138098: the kept count].
    scene.sim.as_mut().unwrap().bond.pos = [14.65, 33.52, -1.82];
    let mut latched = false;
    for _ in 0..6 {
        scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
        let sim = scene.sim.as_ref().unwrap();
        let sp = sim.drones.spawners.iter().find(|sp| sp.group == 1).unwrap();
        if sim.drones.channels.on(0x1d) && sp.kill_latch { latched = true; break; }
    }
    {
        let sim = scene.sim.as_ref().unwrap();
        let sp = sim.drones.spawners.iter().find(|sp| sp.group == 1).unwrap();
        assert!(latched, "the stairwell touch fires ch 0x1D and latches the kill [M 0x137e68:50-52]");
        assert_eq!(present(&sim), 1, "the keep level keeps the drawn guard [M 0x137e68:235-262 — pre-fix-actors this is where he died]");
        assert!(!sp.gone, "the spawner lingers while a drawn guard remains [M 0x1380bc `bnez $s6`]");
    }
    // The level's end-level script chain freezes the NPCs (event 5; the port's model of this
    // level's chain never frees — the Script_Free stand-in, the same direct-state technique the
    // 07000005 tests use). The freeze clears the guard's enabled (obj+0xF0 |= 0x10|0x20
    // [M 0x149dd0:8-15]), so he is no longer drawn — exactly the game's undrawn arm.
    scene.sim.as_mut().unwrap().drones.npc_enabled = true;
    scene.sim.as_mut().unwrap().bond.yaw = std::f32::consts::PI; // face away from the guard
    let mut removed = false;
    for _ in 0..40u32 {
        scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
        if present(&scene.sim.as_ref().unwrap()) == 0 { removed = true; break; }
    }
    {
        let sim = scene.sim.as_ref().unwrap();
        let sp = sim.drones.spawners.iter().find(|sp| sp.group == 1).unwrap();
        assert!(removed, "the undrawn guard is removed on the next scan [M 0x13805c: obj+0xFC == 0]");
        assert!(sp.gone, "once none remain the spawner deletes itself [M 0x1380b4-0x1380cc: obj+0xFE |= 1]");
    }
    // And it never spawns again (mode 3 or not — the latched path never reaches the spawn
    // machinery [M 0x137e68:86-233 runs only while +0x20 is clear]).
    for _ in 0..120 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    assert_eq!(present(&scene.sim.as_ref().unwrap()), 0, "the latched spawner never spawns again");
}
