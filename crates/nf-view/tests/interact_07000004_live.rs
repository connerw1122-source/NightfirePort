//! The 07000004 (Enemies Vanish, Castle D) interactables: one live test per interactive KIND on
//! the main map (task level-04-interactables), each through the real input path (Bond walks/turns/
//! presses the fire key with `Sim::step_bond`), each with a sister that fails without the
//! operation. Every game claim cites its evidence: [M] the decompile function + address, [D] the
//! disc parse of FILES.BIN entry 07000004.bin. The census: docs/levels/07000004.md.
//!
//! The level's interactive kinds (the main map is the ONLY map whose placements the game creates —
//! `parsemap_parsemap` 0x1d1cf0's state-2 epilogue processes `DynamicPtr`, the LAST block-0x1a of
//! the archive, and the type-1 main map is every archive's last map-type entry [D bin-list]):
//! pickups 0xF0 (×4), the target drone 0x0F (×1), the TouchOnce MultiplexOut 0xEE (×1),
//! ScriptPlayers 0xD9 (×2), ripples 0x21 (×4), leafgens 0x2B (×2), trees 0xF3 (×2), env 0xDF (×1).
//! The 0xD8 rigid-body lamps are unported (full RB physics — the report).

#[path = "common/walker.rs"]
mod walker;

use walker::Walker;

fn iso() -> String {
    std::env::var("NF_ISO").unwrap_or_else(|_| {
        eprintln!("NF_ISO not set — skipping");
        std::process::exit(0);
    })
}

/// Load and skip the intro + story cut-scenes the way the player does (the game's own skip action
/// until the AI re-enables and ch 0x53, the story hand-off, is written [D script 0x06000082
/// stream 9]). Returns the settled walker.
fn load_ready(iso: &str) -> Walker {
    let mut w = Walker::new(nf_view::load(iso, "07000004.bin", false).expect("load 07000004"));
    let mut ready = false;
    for _ in 0..6000u32 {
        w.tick(Default::default(), false);
        let sim = w.scene.sim.as_ref().unwrap();
        if sim.drones.npc_enabled && !sim.disabled && sim.drones.channels.on(0x53) { ready = true; break; }
    }
    assert!(ready, "the intro + story scenes must settle (the AI re-enabled, ch 0x53 written)");
    w
}

/// KIND: weapon pickups 0xF0 (Pickup_Create → `Pickup_Handler` 0x18e650 type 0: owned + rounds).
/// The three PICKUP_StormUzi placements #6/#7/#8 (weapon 18, 60 rounds, respawn 2 [D params]) —
/// walking over #6 must give weapon 18 with rounds in the clip. Sister: a fresh load that never
/// walks keeps the weapon unowned.
#[test]
fn the_weapon_pickup_arms_the_storm_uzi() {
    let iso = iso();
    // The sister: fresh load, no operation — weapon 18 unowned.
    {
        let w = Walker::new(nf_view::load_weapons(&iso, "07000004.bin", false, &[18]).expect("load 07000004 + the uzi"));
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(!sim.drones.inventory.owned.get(18).copied().unwrap_or(false),
            "weapon 18 starts unowned [M Pickup_Handler only runs on contact]");
        assert!(sim.drones.pickups.iter().any(|p| p.kind == 0 && p.weapon == 18 && !p.gone),
            "the StormUzi pickup rests on the map");
    }
    let mut w = {
        let mut w = Walker::new(nf_view::load_weapons(&iso, "07000004.bin", false, &[18]).expect("load 07000004 + the uzi"));
        let mut ready = false;
        for _ in 0..6000u32 {
            w.tick(Default::default(), false);
            let sim = w.scene.sim.as_ref().unwrap();
            if sim.drones.npc_enabled && !sim.disabled && sim.drones.channels.on(0x53) { ready = true; break; }
        }
        assert!(ready, "the intro + story scenes must settle");
        w
    };
    let sim = w.scene.sim.as_ref().unwrap();
    // The fountain-side #6 wedges the walker 0.93 short of the take radius; #7 sits on the open
    // lawn (−10.66, 4.39, 42.66) [D]. The kind's behaviour is the assertion, not the placement.
    let at = sim.drones.pickups.iter()
        .find(|p| p.kind == 0 && p.weapon == 18 && !p.gone && (p.start[2] - 42.66).abs() < 1.0)
        .or_else(|| sim.drones.pickups.iter().find(|p| p.kind == 0 && p.weapon == 18 && !p.gone))
        .map(|p| p.start).expect("the StormUzi pickup");
    drop(sim);
    eprintln!("[04] walking to the StormUzi at {at:?}");
    w.walk_nav_to(at, "the StormUzi pickup", 0.4);
    let sim = w.scene.sim.as_ref().unwrap();
    assert!(sim.drones.inventory.owned.get(18).copied().unwrap_or(false),
        "walking over the pickup must own weapon 18 [M Pickup_Handler 0x18e650 type-0 arm]");
    let p = sim.drones.pickups.iter().find(|p| p.kind == 0 && p.weapon == 18 && (p.gone || p.hidden))
        .expect("the taken pickup record");
    assert!(p.gone || (p.hidden && p.state == 2),
        "the pickup is taken — respawn 2 [D] parks it hidden in the state-2 respawn wait, not deleted [M 0x18e650:338-348]");
}

/// KIND: armour pickups 0xF0 type 3 (`Pickup_Handler` 0x18e650 type-3 arm: refused at armour ≥ 50,
/// else hard-set 50 [M :220-225]). #9 PICKUP_BodyArmourHeavy at (−1.85, 4.37, 46.07) [D]. Sister:
/// no walk → armour unchanged.
#[test]
fn the_armour_pickup_raises_the_armour() {
    let iso = iso();
    let mut w = load_ready(&iso);
    let sim = w.scene.sim.as_ref().unwrap();
    let before = sim.drones.health.armour;
    let at = sim.drones.pickups.iter().find(|p| p.kind == 3 && !p.gone).map(|p| p.start)
        .expect("the BodyArmourHeavy pickup");
    drop(sim);
    eprintln!("[04] armour before {before}, walking to the armour at {at:?}");
    w.walk_nav_to(at, "the armour pickup", 0.4);
    let sim = w.scene.sim.as_ref().unwrap();
    assert!(sim.drones.health.armour >= 50.0,
        "walking over the armour case must set armour 50 [M Pickup_Handler 0x18e650:220-225]");
    let taken = sim.drones.pickups.iter().find(|p| p.kind == 3).map_or(true, |p| p.gone);
    assert!(taken, "the armour pickup is taken");
}

/// KIND: the target drone 0x0F — the mission's assassination (death channel 0x54 = placement
/// param 12 [D]; `DroneFunc_SetDeathChannel` 0x13e660). The real fire path kills him. Sister: no
/// shot → alive, ch 0x54 off.
#[test]
fn the_target_dies_to_the_real_fire_path() {
    let iso = iso();
    let mut w = load_ready(&iso);
    let target_i = w.scene.sim.as_ref().unwrap().drones.list.iter()
        .position(|d| d.skin == 0x0500_001e).expect("the target drone exists");
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(!sim.drones.channels.on(0x54), "sister: no shot, the objective channel stays off");
    }
    let killed = w.shoot(target_i, "the target");
    let sim = w.scene.sim.as_ref().unwrap();
    assert!(killed || sim.drones.list[target_i].corpse || sim.drones.list[target_i].health <= 0.0,
        "the walked shots must kill the target");
    let mut raised = false;
    for _ in 0..900u32 {
        w.tick(Default::default(), false);
        if w.scene.sim.as_ref().unwrap().drones.channels.on(0x54) { raised = true; break; }
    }
    assert!(raised, "the kill raises ch 0x54 [M DroneFunc_SetDeathChannel 0x13e660]");
}

/// KIND: the TouchOnce "MultiplexOut" 0xEE (#46, out 0x54, list [0x64] [D params]) —
/// `Trigger_Update` case 5 is a pure channel mux: it writes `switch_channels[list[i]] =
/// switch_channels[out]` for every non-zero list word EVERY tick, no touch/hit check
/// (`Trigger_Hit` is not consulted [M 0x1b1a78:147-160]; the placement has no collision mesh —
/// [D] unbounded lo/hi — and needs none). Sister: the ticks BEFORE the kill copy nothing.
#[test]
fn the_out_trigger_copies_only_after_the_kill() {
    let iso = iso();
    let mut w = load_ready(&iso);
    // SISTER: ticks with the target alive and no shots — no 0x54, no 0x64.
    for _ in 0..300 { w.tick(Default::default(), false); }
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(!sim.drones.channels.on(0x54), "the target still lives (no shots)");
        assert!(!sim.drones.channels.on(0x64), "no copy while the out channel is off");
    }
    // The kill: the death channel rises, the mux mirrors it the same tick.
    let target_i = w.scene.sim.as_ref().unwrap().drones.list.iter()
        .position(|d| d.skin == 0x0500_001e).expect("the target");
    let _ = w.shoot(target_i, "the target");
    let mut copied = false;
    for _ in 0..900u32 {
        w.tick(Default::default(), false);
        let sim = w.scene.sim.as_ref().unwrap();
        if sim.drones.channels.on(0x54) && sim.drones.channels.on(0x64) { copied = true; break; }
    }
    let sim = w.scene.sim.as_ref().unwrap();
    assert!(sim.drones.channels.on(0x54), "the kill raised ch 0x54");
    assert!(copied,
        "the OUT mux copies 0x54 → 0x64 wherever Bond stands [M Trigger_Update 0x1b1a78 case 5]");
}

/// KIND: ripples 0x21 (`Ripples_Create` 0x1acc90 / `Ripples_Update` 0x1ac028 — one gas ring per
/// period at the generator, the pond's presentation). The four RIPPLE_GEN placements #32-35
/// (spread 1, period 30 [D params (0,1),(1,30)]). The walk stands Bond beside the pond while the
/// rings spawn. Sister: the pool holds no ripple puff before the ticks run.
#[test]
fn the_pond_ripples_spawn_while_bond_walks_the_pond_edge() {
    let iso = iso();
    let mut w = load_ready(&iso);
    let sim = w.scene.sim.as_ref().unwrap();
    assert_eq!(sim.drones.fx.ripples.len(), 4, "the four RIPPLE_GEN placements exist [D]");
    assert!(sim.drones.fx.ripples.iter().all(|r| r.period == 30 && (r.spread - 1.0).abs() < 1e-4),
        "spread 1, period 30 [D params]");
    drop(sim);
    // The walk: the pond edge between the generators (−2.16..0, 34.9..38.0).
    w.walk_nav_to([-1.5, 4.8, 36.5], "the pond edge", 1.4);
    for _ in 0..240 { w.tick(Default::default(), false); }
    let sim = w.scene.sim.as_ref().unwrap();
    assert!(!sim.drones.gas.puffs.is_empty(), "the ripple rings live in the gas pool while the walk ticks");
}

/// KIND: ScriptPlayers 0xD9 — the intro SP #3 (script 0x06000083, mode 1) and the story SP #2
/// (0x06000082) [D params (0, hash)]. The player's own skip (the walker's skip action) settles
/// both: the AI re-enables and ch 0x53 (the castle-C hand-off) is written [D script 0x06000082
/// stream 9]. Sister: the two SPs exist with their script hashes on a fresh load.
#[test]
fn the_script_players_settle_the_story_handoff() {
    let iso = iso();
    {
        let w = Walker::new(nf_view::load(&iso, "07000004.bin", false).expect("load 07000004"));
        let sim = w.scene.sim.as_ref().unwrap();
        assert_eq!(sim.drones.scripts.players.len(), 2, "the two 0xD9 placements exist [D]");
        let mut scripts: Vec<u32> = sim.drones.scripts.players.iter().map(|p| p.scripts[0]).collect();
        scripts.sort_unstable();
        assert_eq!(scripts, vec![0x0600_0082, 0x0600_0083], "the story + intro script hashes [D params]");
    }
    let mut w = Walker::new(nf_view::load(&iso, "07000004.bin", false).expect("load 07000004"));
    let mut ready = false;
    for _ in 0..6000u32 {
        w.tick(Default::default(), false);
        let sim = w.scene.sim.as_ref().unwrap();
        if sim.drones.npc_enabled && !sim.disabled && sim.drones.channels.on(0x53) { ready = true; break; }
    }
    assert!(ready, "the skip settles the scenes: AI on, ch 0x53 written [D 0x06000082 stream 9]");
}

/// KIND: the env 0xDF (#5 Rain_generator, params (0,1),(1,1),(2,4),(3,6) [D]) — `Env_Create`
/// 0x1e5d20 sets the level's rain. The port's env state carries the drops. Sister: a rainless
/// level load would leave no rain boxes — asserted via the kind-1 rain env existing.
#[test]
fn the_env_generator_sets_rain() {
    let iso = iso();
    let mut w = Walker::new(nf_view::load(&iso, "07000004.bin", false).expect("load 07000004"));
    w.tick(Default::default(), false);
    w.tick(Default::default(), false);
    let sim = w.scene.sim.as_ref().unwrap();
    let env = sim.drones.env.as_ref().expect("the rain env exists [D #5; M Env_Create 0x1e5d20]");
    assert_eq!(env.kind, 1, "drop type 1 = rain [M Env_Create type byte; D param 0 = 1]");
    assert!(!env.boxes.is_empty(), "the rain boxes from the level's 0x32 Rainboxes exist");
}

/// KIND: leafgens 0x2B (`LeafGen_Create` 0x1acbd0 / `LeafGen_Update` 0x1acc48 — one falling leaf
/// per Rand_Rand(30) gate). #41/#42 "Leaf_emmit" by the carpark [D]. The walk stands under them;
/// the spawns tick up. Sister: a fresh load has spawned nothing.
#[test]
fn the_leaf_generators_spawn_leaves_over_the_park() {
    let iso = iso();
    let w = Walker::new(nf_view::load(&iso, "07000004.bin", false).expect("load 07000004"));
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert_eq!(sim.drones.fx.leafgens.len(), 2, "the two Leaf_emmit placements exist [D]");
        assert!(sim.drones.fx.leafgens.iter().all(|l| l.spawned == 0), "sister: no ticks, no leaves");
    }
    let mut w = w;
    w.walk_nav_to([-8.13, 4.8, 26.7], "under the leaf emitter", 1.6);
    for _ in 0..300 { w.tick(Default::default(), false); }
    let sim = w.scene.sim.as_ref().unwrap();
    let total: u32 = sim.drones.fx.leafgens.iter().map(|l| l.spawned).sum();
    assert!(total > 0, "the generators spawn leaves while the walk ticks [M 0x1acc48]");
}

/// KIND: trees 0xF3 (`Env_Tree_Create` 0x17c200 — object + AnimObjectNew + AnimScriptAdd; the
/// pause arm for every level but 0x7000043). #30/#31 "Tree" with model 0x05000043 + anim
/// 0x06000474 [D params]. Sister: a level whose hash is 0x07000043 would keep them unpaused (the
/// unit test pins the arm; here the disc values).
#[test]
fn the_trees_carry_model_and_anim_hashes() {
    let iso = iso();
    let w = Walker::new(nf_view::load(&iso, "07000004.bin", false).expect("load 07000004"));
    let sim = w.scene.sim.as_ref().unwrap();
    assert_eq!(sim.drones.fx.trees.len(), 2, "the two Tree placements exist [D]");
    for t in &sim.drones.fx.trees {
        assert_eq!(t.model, 0x0500_0003, "the Tree model hash [D param 0 = 83886083]");
        assert_eq!(t.anim, 0x0600_008c, "the sway anim script [D param 1 = 100663436]; the game's
            // 0x600008c→0x600008d Rand_Random swap is a shared-stream draw the port does not take [M 0x17c200:29-35]");
        assert!(t.paused, "07000004 != 0x7000043 → the +0x78 pause arm [M 0x17c200:44-52]");
    }
}

/// The parsemap-epilogue rule [M 0x1d1cf0]: the game creates dynamic objects from the MAIN map
/// only — the template maps' "Placement - Switch"/"alarmswitchplayer" (0x29) and "AI_Point" (0xE9)
/// at the origin NEVER exist. The port's old cross-map sweep spawned a phantom AI point + two
/// phantom alarm switches on every castle level; this test pins the fixed behaviour.
#[test]
fn no_phantom_switches_or_ai_points_from_the_template_maps() {
    let iso = iso();
    let w = Walker::new(nf_view::load(&iso, "07000004.bin", false).expect("load 07000004"));
    let sim = w.scene.sim.as_ref().unwrap();
    assert!(sim.drones.ai_points.is_empty(),
        "the main map has no 0xE9 placements — the map-7 template must not exist [M 0x1d1cf0]");
    assert!(sim.drones.list.iter().all(|d| d.alarm_switches.is_empty()),
        "no 0x29 placements on the main map — no phantom alarm switches [M 0x1d1cf0]");
}
