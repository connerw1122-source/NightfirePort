//! walkreal-a: 07000004 "Enemies Vanish" (Castle D) walked START TO FINISH with the player's own
//! movement — no teleports after the spawn. The route (docs/levels/07000004.md):
//!   the start (1.87, 5.84, 41.07) → the intro SP #3 + the story scene SP #2 settle by ~tick 1600
//!   (the AI re-enabled, ch 0x53 written; the story places Bond beside the target) → the armament
//!   walk (the StormUzi pickup #7 at (−10.66, 4.39, 42.66), the BodyArmourHeavy #9 at
//!   (−1.85, 4.37, 46.07) [D]) → back to the target (drone skin 0x0500001e, stationary) → the real
//!   fire path kills him → ch 0x54 [M DroneFunc_SetDeathChannel 0x13e660] → the OUT trigger #46
//!   "MultiplexOut" (out 0x54, list [0x64] [D]) copies it into **ch 0x64 = channel 100 = the
//!   mission-complete channel** while Bond stands inside [M Trigger_MultiplexOut] → `Mission_Update`
//!   (0x182c60) internal 1 sees ch 100 → case 6 posts "Mission Complete" (LevelToEndTo
//!   **0x0730004**, the next arc's driving interlude [M 0x182c60:85-122]) → case 7 holds the 240-
//!   tick banner → case 4 hands `ResetMap_LevelToLoad(0x0730004)` [M 0x182c60:65-76] — the level's
//!   own exit (no 0xE8 trigger exists on this level [D placement probe]).
//! level-04-complete: the walk now carries the mission layer to the banner end and pins the events.

#[path = "common/walker.rs"]
mod walker;

use nf_game::mission;
use walker::Walker;

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

/// The mission-table index of THIS level's record [L ee_start_05.bin 0x2a4350].
fn mission_row(level: u32) -> usize {
    mission::LEVELS.iter().position(|l| l.level == level).expect("the level's mission record")
}

/// Load and skip the intro + story cut-scenes the way the player does (the game's own skip action)
/// until the AI re-enables and ch 0x53 (the castle-C hand-off [D script 0x06000082 stream 9]) is
/// written. Returns the settled walker.
fn load_ready(iso: &str) -> Walker {
    let mut w = Walker::new(nf_view::load(iso, "07000004.bin", false).expect("load 07000004"));
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert_eq!(sim.level, 0x0700_0004);
        assert!(sim.drones.triggers.iter().all(|t| t.kind != walker::trigger_kind::LOAD_LEVEL),
            "07000004 has no 0xE8 exit [D placement probe] — the mission layer IS the exit");
        let p = sim.bond.pos;
        assert!((p[0] - 1.87).abs() < 2.0 && (p[2] - 41.07).abs() < 2.0, "start #0 [D], got {p:?}");
    }
    let mut ready = false;
    for _ in 0..6000u32 {
        w.tick(Default::default(), false);
        let sim = w.scene.sim.as_ref().unwrap();
        if sim.drones.npc_enabled && !sim.disabled && sim.drones.channels.on(0x53) { ready = true; break; }
    }
    assert!(ready, "the intro + story scenes must settle (the AI re-enabled, ch 0x53 written)");
    {
        let sim = w.scene.sim.as_ref().unwrap();
        let p = sim.bond.pos;
        eprintln!("[04] settled at tick {} bond ({:.2},{:.2},{:.2}) ch 0x53 on", w.total_ticks, p[0], p[1], p[2]);
        assert!(!sim.drones.channels.on(0x54), "the objective channel starts off");
        assert_eq!(sim.drones.mission.state, 1, "the mission runs [M Mission_Init 0x182270]");
    }
    w
}

/// The OUT trigger #46's placement box (print helper) [D]. Unbounded (no collision mesh) — the
/// mux needs none [M 0x1b1a78 case 5].
#[allow(dead_code)]
fn out_zone(w: &Walker) -> [f32; 3] {
    let sim = w.scene.sim.as_ref().unwrap();
    let t = sim.drones.triggers.iter()
        .find(|t| t.kind == nf_game::triggers::kind::OUT)
        .expect("the MultiplexOut trigger #46 [D]");
    let c = [(t.lo[0] + t.hi[0]) * 0.5, (t.lo[1] + t.hi[1]) * 0.5, (t.lo[2] + t.hi[2]) * 0.5];
    eprintln!("[04] the MultiplexOut box lo {:?} hi {:?} → centre {:?}", t.lo, t.hi, c);
    c
}

#[test]
fn walk_the_full_mission_start_to_finish() {
    let iso = iso();
    let mut w = load_ready(&iso);
    let row = mission_row(0x0700_0004);
    // The monitor's enable arm: ch 0x53 (the story hand-off) is ON → obj0 announces once [M
    // 0x1827d8:55-67]. Two ticks so the fold's own tick has run.
    w.tick(Default::default(), false);
    w.tick(Default::default(), false);
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert_eq!(sim.drones.mission.states[row][0], 3,
            "obj0 'Defeat the assassin' announced through the real monitor (enable ch 0x53 on) [M 0x1827d8:55-67]");
    }
    // The armament walk (the route's own pickups [D #6/#7/#8 StormUzi + #9 BodyArmourHeavy]):
    // the open-lawn uzi #7 (−10.66, 4.39, 42.66), then the armour #9 (−1.85, 4.37, 46.07).
    let (uzi, armour) = {
        let sim = w.scene.sim.as_ref().unwrap();
        let uzi = sim.drones.pickups.iter()
            .find(|p| p.kind == 0 && p.weapon == 18 && !p.gone && (p.start[2] - 42.66).abs() < 1.0)
            .or_else(|| sim.drones.pickups.iter().find(|p| p.kind == 0 && p.weapon == 18 && !p.gone))
            .map(|p| p.start).expect("the StormUzi pickup");
        let armour = sim.drones.pickups.iter().find(|p| p.kind == 3 && !p.gone).map(|p| p.start)
            .expect("the BodyArmourHeavy pickup");
        (uzi, armour)
    };
    w.walk_nav_to(uzi, "the StormUzi pickup", 0.4);
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(sim.drones.inventory.owned.get(18).copied().unwrap_or(false),
            "walking over the pickup owns weapon 18 [M Pickup_Handler 0x18e650 type-0 arm]");
    }
    w.walk_nav_to(armour, "the armour pickup", 0.4);
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(sim.drones.pickups.iter().all(|p| p.kind != 3 || p.gone),
            "the armour case is consumed (respawn param 5 absent → deleted [D params; M 0x18e650:338-348])");
        assert!(sim.drones.health.armour > 0.0,
            "the take raised armour from 0 to 50 [M 0x18e650:220-225]; the settled assassin's bullet-type pain may chip it on the walk (48.5 measured) — the raise itself is the [M] claim");
    }
    // The assassination: walk back to the target and kill him with the player's own fire path.
    let target_i = w.scene.sim.as_ref().unwrap().drones.list.iter()
        .position(|d| d.skin == 0x0500_001e).expect("the target drone exists");
    let killed = w.shoot(target_i, "the target");
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(killed || sim.drones.list[target_i].corpse || sim.drones.list[target_i].health <= 0.0,
            "the target must die (the walk's own shots)");
    }
    // The death channel settles over the death anim; the objective folds through the REAL monitor.
    let mut raised = false;
    for _ in 0..900u32 {
        w.tick(Default::default(), false);
        if w.scene.sim.as_ref().unwrap().drones.channels.on(0x54) { raised = true; break; }
    }
    assert!(raised, "the target's death must raise ch 0x54 [M DroneFunc_SetDeathChannel 0x13e660]");
    // The OUT mux (trigger #46, out 0x54 → list [0x64] = ch 100): Trigger_Update's case 5 copies
    // the OUT channel into every list channel EVERY tick with NO touch/hit check [M 0x1b1a78
    // case 5:147-160 — Trigger_Hit's result is not consulted; the placement has no collision mesh
    // [D unbounded bbox] and needs none]. The monitor runs at the next tick's top; give both their
    // tick, then pin the fold + the copy.
    for _ in 0..30u32 { w.tick(Default::default(), false); }
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert_eq!(sim.drones.mission.states[row][0], 4,
            "obj0 'Defeat the assassin' folds complete through Mission_MonitorObjectives [M 0x1827d8:78-85]");
        assert!(sim.drones.channels.on(100),
            "the OUT mux mirrors ch 0x54 into ch 100 = the mission-complete channel [D #46 list [0x64]; M 0x1b1a78 case 5]");
    }
    // The game's exit: Mission_Update's next tick sees ch 100 → case 6 (the "Mission Complete"
    // banner, LevelToEndTo 0x0730004) → case 7's 240-tick hold → case 4's ResetMap_LevelToLoad
    // [M 0x182c60:85-122, 65-76]. The walker's ticks drain the layer's events through the scene's
    // real hud path (`Sim::step_bond` ends in `hud_tick` [M scene.rs] — the game's own Game_Run
    // order), so the exit arrives as the viewer's `mission_end` latch: 0x0730004 is a driving
    // interlude code → `ptp_handover` + `mission_end` (scene.rs's LevelEnd arm — the port's end
    // sequence: outro → results). Wait for it and pin the layer's globals.
    let mut ended = false;
    for _ in 0..500u32 {
        w.tick(Default::default(), false);
        if w.scene.sim.as_ref().unwrap().drones.mission_end.is_some() { ended = true; break; }
    }
    assert!(ended, "the mission-complete sequence must land (case 6 banner → case 4 ResetMap)");
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert_eq!(sim.drones.mission_end, Some(0x0730_0004),
            "LevelEnd 0x0730004 → the end sequence with LevelToEndTo 0x0730004 [M 0x182c60:65-76, 89-90]");
        assert_eq!(sim.drones.mission.state, 6, "MissionState 6 = complete [M 0x182c60:88]");
        assert_eq!(sim.drones.mission.to_end, 0x0730_0004, "LevelToEndTo [M 0x182c60:89-90]");
        assert_eq!(sim.drones.mission.states[row][0], 4, "the objective stays complete");
    }
    eprintln!("[04] walked the FULL mission in {} ticks; falls: {:?}", w.total_ticks, w.report.falls);
    for f in &w.report.falls { eprintln!("[04] fall: {f}"); }
    for s in &w.report.stuck_events { eprintln!("[04] stuck: {s}"); }
}

/// Sister: without the assassination the level can never end. Bond walks the route (the pickups)
/// while the target stays ALIVE — ch 0x54 stays off, the OUT mux copies nothing, the mission layer
/// never leaves internal 1/MissionState 1 and no banner events queue. This is the assert that
/// fails on the old code if anything but the player's own kill completes the mission.
#[test]
fn without_the_assassination_the_mission_stays_open() {
    let iso = iso();
    let mut w = load_ready(&iso);
    let row = mission_row(0x0700_0004);
    // Walk the route's pickups (the arming alone must not complete anything).
    let (uzi, armour) = {
        let sim = w.scene.sim.as_ref().unwrap();
        let uzi = sim.drones.pickups.iter()
            .find(|p| p.kind == 0 && p.weapon == 18 && !p.gone && (p.start[2] - 42.66).abs() < 1.0)
            .or_else(|| sim.drones.pickups.iter().find(|p| p.kind == 0 && p.weapon == 18 && !p.gone))
            .map(|p| p.start).expect("the StormUzi pickup");
        let armour = sim.drones.pickups.iter().find(|p| p.kind == 3 && !p.gone).map(|p| p.start)
            .expect("the BodyArmourHeavy pickup");
        (uzi, armour)
    };
    w.walk_nav_to(uzi, "sister: the StormUzi", 0.4);
    w.walk_nav_to(armour, "sister: the armour", 0.4);
    // Long wait with the target alive (the assassin stalks Bond the whole time).
    for _ in 0..600u32 {
        w.tick(Default::default(), false);
        for _ in std::mem::take(&mut w.scene.sim.as_mut().unwrap().drones.mission_events) {}
    }
    let sim = w.scene.sim.as_ref().unwrap();
    assert!(!sim.drones.channels.on(0x54), "no shots → the target lives, ch 0x54 off");
    assert!(!sim.drones.channels.on(100), "the OUT zone copies nothing while 0x54 is off [M Trigger_MultiplexOut]");
    assert_ne!(sim.drones.mission.state, 6, "the mission never completes without the kill");
    assert_eq!(sim.drones.mission.states[row][0], 3,
        "obj0 sits ANNOUNCED, never complete — the condition ch 0x54 only rises on the kill [M 0x1827d8 state 3 arm]");
    // The wait above kept the layer in internal 1 the whole time.
    assert_eq!(sim.drones.mission.internal, 1, "Mission_Update never left the running arm [M 0x182c60 case 1]");
}

/// The original assassination walk (the channel-level pin), kept focused: kill → ch 0x54 → the
/// OUT mux mirrors it into ch 100. The full-mission test above adds the monitor fold + the banner
/// end.
#[test]
fn walk_the_assassination() {
    let iso = iso();
    let mut w = load_ready(&iso);
    let target_i = w.scene.sim.as_ref().unwrap().drones.list.iter()
        .position(|d| d.skin == 0x0500_001e).expect("the target drone exists");
    let killed = w.shoot(target_i, "the target");
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(killed || sim.drones.list[target_i].corpse || sim.drones.list[target_i].health <= 0.0,
            "the target must die (the walk's own shots)");
    }
    let mut raised = false;
    for _ in 0..900u32 {
        w.tick(Default::default(), false);
        if w.scene.sim.as_ref().unwrap().drones.channels.on(0x54) { raised = true; break; }
    }
    assert!(raised, "the target's death must raise ch 0x54 [M DroneFunc_SetDeathChannel 0x13e660]");
    for _ in 0..30u32 { w.tick(Default::default(), false); }
    let copied = w.scene.sim.as_ref().unwrap().drones.channels.on(100);
    assert!(copied, "the OUT mux #46 must copy ch 0x54 to the mission-complete ch 100 [M 0x1b1a78 case 5]");
    eprintln!("[04] walked the assassination in {} ticks; falls: {:?}", w.total_ticks, w.report.falls);
    for f in &w.report.falls { eprintln!("[04] fall: {f}"); }
    for s in &w.report.stuck_events { eprintln!("[04] stuck: {s}"); }
}
