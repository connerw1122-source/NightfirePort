//! map-castle walkthrough: 07000004 "Enemies Vanish" (Castle D) — the mission-complete route from the
//! level's own data. Run with: `NF_ISO=… cargo test -p nf-view --test walk_07000004_live -- --nocapture`
//!
//! The chain [D placements + scripts; M decompile addresses cited inline]:
//! 1. The level has NO 0xE8 exit trigger at all [D placement probe] — it ends through the MISSION system:
//!    the single objective (the mission table 0x2a3f60, label 0x0400000d) watches **ch 0x54**; when every
//!    objective's watch channel is on, `Mission_MonitorObjectives` (0x1827d8:165-172) sets
//!    **switch_channels[100 (0x64)]** and `Mission_Update` (0x182c60 case 1 → 6) loads
//!    `LevelToEndTo = 0x7300004` — the next arc's DRIVING level [M 0x182c60:88-90 case 0x7000004].
//! 2. Ch 0x54 is the TARGET's death channel: the level's one live drone (skin 0x0500001e, placement
//!    param 12 = 0x54 [D placement probe]) raises it when he dies [M DroneFunc_SetDeathChannel 0x13e660;
//!    the port's `death_channel` → `set_death_channel` on the death-anim exit].
//! 3. The level-start SP #3 (script 0x06000083, mode 1) writes ch 0x38 at its stream 6 [D script decode];
//!    a second script 0x06000082 (SP on the map) writes ch 0x53 — the castle-C objective's completion
//!    channel, the story hand-off from the previous level.
//!
//! The port has no Mission_Update layer yet (031-fix-mission): ch 0x64 and the 0x7300004 load are not
/// consumed. The walkthrough asserts the part the level's own data drives: the target's death raises
/// ch 0x54; not killing him leaves it off (the sister).

use nf_formats::iso9660::IsoImage;

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

/// Killing the target (skin 0x0500001e) raises the objective channel ch 0x54 via its death channel.
#[test]
fn killing_the_target_raises_the_objective_channel() {
    let iso_v = iso();
    let mut scene = nf_view::load(&iso_v, "07000004.bin", false).expect("load 07000004");
    let sim = scene.sim.as_mut().unwrap();
    assert_eq!(sim.level, 0x0700_0004);
    assert!(sim.drones.triggers.iter().all(|t| t.kind != nf_game::triggers::kind::LOAD_LEVEL),
        "07000004 has no 0xE8 exit [D placement probe]");
    // The start: the intro SP #3 (0x06000083) freezes the AI (event 5 [M Script_EventHandler 0x1d94c0
    // case 5 → Drone_EnableAll(0,·)]) and disables Bond (event 4); ch 0x38 then starts the story scene
    // SP #2 (0x06000082) whose stream-8 event 7 sets the Script_Free 0x80 flag — its end re-enables the
    // AI [M Script_Free 0x1da880:41-43] and writes ch 0x53 (the castle-C objective's completion, the
    // story hand-off). Everything settles by ~tick 1600 [D probe].
    let mut ready = false;
    for _ in 0..3000u32 {
        sim.step_bond(Default::default(), false, false, false);
        if sim.drones.npc_enabled && !sim.disabled && sim.drones.channels.on(0x53) { ready = true; break; }
    }
    assert!(ready, "the intro + story scenes must settle (the AI re-enabled, ch 0x53 written)");
    assert!(!sim.drones.channels.on(0x54), "the objective channel starts off");
    let target_i = sim.drones.list.iter().position(|d| d.skin == 0x0500_001e).expect("the target drone exists");
    assert_eq!(sim.drones.list[target_i].death_channel, 0x54, "the target's death channel is the objective's watch channel");
    // The grenade at his feet (the game's own Explode_Propagate → Drone_ExplosiveHit path [M 0x17c518]).
    let pos = sim.drones.list[target_i].pos;
    sim.drones.grenades.push(nf_game::grenade::Grenade {
        pos: [pos[0], pos[1] + 0.5, pos[2]], velocity: [0.0, 0.0, 0.0], speed: 0.0,
        travelled: 0.0, range: 60.0, damage: 100.0, radius: 3.0, weapon_type: 0x34, mesh: 0x0200_0726, owner: usize::MAX, state: 0, bounces: 0, smoke_popped: false, resting: false,
    });
    let mut raised = false;
    let mut completed = false;
    for _ in 0..900u32 {
        scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
        let s = scene.sim.as_ref().unwrap();
        if s.drones.channels.on(0x54) { raised = true; }
        // The MultiplexOut #46 (0xEE out 0x54, list [0x64]) copies the death channel to the
        // mission-complete channel [M Trigger_Update's OUT case; the mission layer consumes 0x64 —
        // Mission_Update 0x182c60 case 1 → 6, unported (031-fix-mission)].
        if s.drones.channels.on(0x64) { completed = true; break; }
    }
    assert!(raised, "the target's death must raise ch 0x54 (the objective's watch channel)");
    assert!(completed, "the OUT trigger #46 must copy ch 0x54 to the mission-complete ch 0x64");
}

/// Sister: leave the target alive and neither the objective channel nor the mission-complete channel
/// turns on — the mission cannot complete.
#[test]
fn leaving_the_target_alive_keeps_the_objective_open() {
    let iso_v = iso();
    let mut scene = nf_view::load(&iso_v, "07000004.bin", false).expect("load");
    let sim = scene.sim.as_mut().unwrap();
    for _ in 0..3000u32 {
        sim.step_bond(Default::default(), false, false, false);
        if sim.drones.npc_enabled && !sim.disabled && sim.drones.channels.on(0x53) { break; }
    }
    for _ in 0..600u32 { sim.step_bond(Default::default(), false, false, false); }
    assert!(!sim.drones.channels.on(0x54), "no kill, no objective channel");
    assert!(!sim.drones.channels.on(0x64), "no kill, no mission-complete channel");
    assert!(sim.drones.list.iter().any(|d| d.skin == 0x0500_001e && !d.corpse), "the target is alive");
}
