//! fix-level-chain: the no-obvious-exit levels chain the way the game does. The mechanisms, found in
//! the data and the decompile (docs/reports/fix-level-chain.md):
//! - 07000006: the party-entrance TOUCH (main map, out ch 0x83) triggers ScriptPlayer #220 (script
//!   0x06000071); its stream 9 fires script event 0x12 = LoadLevel 0x07000007 [D probe; M
//!   Script_EventHandler 0x1d94c0 case 0x12 → Player_RamSave + ResetMap_LevelToLoad(param,1,0)].
//!   No 0xE8 exists in the archive and the mission AND cannot fold mid-mission [M 0x1827d8:22-38].
//! - 07000009: the lift-zone TouchOnce (out ch 0x32) is the gate of the 0xE8 (main map, next
//!   0x0700000a) — Trigger_Update case 10 auto-fires on the gate [M 0x1b1a78:162-166].
//! - 0700000a: the balcony 0xE8 (next 0x0700000b) is key-gated on ch 0x4a; the E-path loads with the
//!   key on and refuses without it [M 0x1b2318:76-111].
//! - 07000011: two 0xE8s → 0x07000012 (gate ch 0x1f / key ch 0x1e); the gate's in-level writer is
//!   not in any placement param or script (live check filed) — the auto-fire arm itself is pinned.
//! - 0700004a: the lobby TouchOnce (out ch 0x84) triggers ScriptPlayer #276 (pack 0790004a script
//!   0x060006d3); stream 9 fires 0x12 with the STANDALONE MOVIE code 0x0720004a, which the load
//!   pipeline's phase 2 maps through GetLevelToLoadFromMovieCode to 0x07000013 [M 0x1d3560].
use nf_game::triggers::{self, kind};
use nf_view::scene::Scene;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn load(iso: &str, name: &str) -> Scene {
    match nf_view::load_with_carry(iso, name, false, None) {
        Ok(s) => s,
        Err(e) => panic!("{name}: {e}"),
    }
}

/// Bond standing inside a trigger's bounding box (the runner's aim: box centre, a tenth above the floor).
fn stand_in_box(sim: &mut nf_view::scene::Sim, lo: [f32; 3], hi: [f32; 3]) {
    let c = [(lo[0] + hi[0]) * 0.5, lo[1] + 0.1, (lo[2] + hi[2]) * 0.5];
    sim.bond = nf_game::player::Bond::new(c, 0.0);
}

fn find(sim: &nf_view::scene::Sim, pred: impl Fn(&triggers::Trigger) -> bool) -> triggers::Trigger {
    sim.drones.triggers.iter().find(|t| pred(t)).cloned()
        .unwrap_or_else(|| panic!("the level data must hold the trigger the chain needs"))
}

/// 07000006 → 07000007 through the party-entrance touch and the end-of-level cut-scene's event 0x12.
/// This is the level the campaign runner had to force-complete: its chain is a script, not a trigger.
#[test]
fn the_07000006_party_touch_chains_to_07000007() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut sc = load(&iso, "07000006.bin");
    let sim = sc.sim.as_mut().unwrap();
    // The data pin: no LOAD_LEVEL trigger anywhere (the old blocker), and the touch that fires the
    // cut-scene script player instead [D probe: main-map 0xEB placement #189, out ch 0x83].
    assert!(!sim.drones.triggers.iter().any(|t| t.kind == kind::LOAD_LEVEL && t.next_level != 0),
        "07000006 has no 0xE8 exit — its chain is the script event");
    let touch = find(sim, |t| t.kind == kind::TOUCH && t.out == 0x83);
    assert!(touch.lo.iter().all(|v| v.is_finite()), "the party touch zone has a volume");
    stand_in_box(sim, touch.lo, touch.hi);
    let mut seen = None;
    for _ in 0..4000 {
        sim.step_bond(Default::default(), false, false, false);
        if sim.drones.level_exit.is_some() { seen = sim.drones.level_exit; break; }
    }
    assert_eq!(seen, Some(0x0700_0007),
        "the party cut-scene's script event 0x12 must request 07000007 [D script 06000071 stream9; M 0x1d94c0 case 0x12]");
    // Sister: the mission layer never completed — the AND spans the mission's later parts
    // (07/08's condition channels stay off during the cut-scene) [M 0x1827d8:22-38].
    assert_ne!(sim.drones.mission.state, 6, "the mission must not complete mid-mission on 06");
}

/// 07000009 → 0700000a: the lift-zone TouchOnce raises ch 0x32, the 0xE8's gate, and the exit
/// auto-fires. Sister: Bond left at the level start fires nothing (the old self-loop reading was a
/// stale parse — the placement's next_level is 0x0700000a [D probe]).
#[test]
fn the_07000009_lift_touch_chains_to_0700000a() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    // Sister first: the fresh level alone does not exit.
    let mut sc = load(&iso, "07000009.bin");
    let sim = sc.sim.as_mut().unwrap();
    let exit = find(sim, |t| t.kind == kind::LOAD_LEVEL && t.next_level == 0x0700_000a);
    assert_eq!((exit.gate, exit.key_gate), (0x32, 0), "the 0xE8's gate is ch 0x32, no key [D placement #60]");
    for _ in 0..300 { sim.step_bond(Default::default(), false, false, false); }
    assert_eq!(sim.drones.level_exit, None, "an idle level must not exit");
    // The real chain: the lift zone raises the gate channel, the exit auto-fires.
    let touch = find(sim, |t| t.kind == kind::TOUCH_ONCE && t.out == 0x32);
    stand_in_box(sim, touch.lo, touch.hi);
    let mut seen = None;
    for _ in 0..600 {
        sim.step_bond(Default::default(), false, false, false);
        if sim.drones.level_exit.is_some() { seen = sim.drones.level_exit; break; }
    }
    assert_eq!(seen, Some(0x0700_000a), "the gate touch chains 09 → 0a [M Trigger_Update case 10]");
}

/// Bond standing at a trigger's box centre, swept through the four facings. A 0xE8's mesh is the
/// door slab it sits on: the collision push-out lands Bond beside it (a metre off a 0.05-thick
/// plane), so the use sphere reaches it only when he FACES it — the game's own activation is a
/// radius-1.0 sphere at head + camera dir vs the mesh [M 0x1a5170:100-137].
fn sweep_use(sim: &mut nf_view::scene::Sim, lo: [f32; 3], hi: [f32; 3], ticks: u32) -> Option<u32> {
    let c = [(lo[0] + hi[0]) * 0.5, lo[1] + 0.1, (lo[2] + hi[2]) * 0.5];
    for k in 0..4u32 {
        sim.bond = nf_game::player::Bond::new(c, k as f32 * std::f32::consts::FRAC_PI_2);
        for _ in 0..ticks {
            sim.step_bond(Default::default(), false, false, true);
            if sim.drones.level_exit.is_some() { return sim.drones.level_exit; }
        }
    }
    sim.drones.level_exit
}

/// 0700000a → 0700000b: the balcony 0xE8 is E-usable only with ch 0x4a on — and 0x4a is not a key
/// pickup's channel but the AND mux #568's output over 0a's first two objective channels (0x48 Q-Worm,
/// 0x49 security centre) [D probe: main-map 0xEC placement #568, out 0x4a, inputs 48 49] — the mux
/// recomputes it every tick, so the test raises the INPUTS the way the objectives do, then works the
/// E-path [M 0x1b2318:76-111].
#[test]
fn the_0700000a_balcony_exit_needs_the_key_and_chains_to_0700000b() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut sc = load(&iso, "0700000a.bin");
    let sim = sc.sim.as_mut().unwrap();
    let exit = find(sim, |t| t.kind == kind::LOAD_LEVEL && t.next_level == 0x0700_000b);
    assert_eq!((exit.gate, exit.key_gate), (0, 0x4a), "key-gated on ch 0x4a, no auto gate [D placement #2]");
    assert!(exit.lo.iter().all(|v| v.is_finite()), "the balcony exit has a mesh to aim at");
    // Sister: objectives unmet → the mux holds 0x4a low → the E-path refuses from every facing
    // (locked, no load) [M 0x1b2318:76-90].
    assert_eq!(sweep_use(sim, exit.lo, exit.hi, 60), None, "without the unlock the exit stays locked");
    // The objectives' channels on → the AND mux #54 raises 0x4a ("unlocked exterior door") → the
    // load. 0x48/0x49's own writers are the Q-Worm script player (SP out 72 [D levels.md §5.2]) and
    // the security-centre zone — per-level walkthrough material (campaign-flow blocker 6) — and
    // whatever reclaims them per tick, so the test holds them across the sweep the way their live
    // writers would.
    for k in 0..4u32 {
        let c = [(exit.lo[0] + exit.hi[0]) * 0.5, exit.lo[1] + 0.1, (exit.lo[2] + exit.hi[2]) * 0.5];
        sim.bond = nf_game::player::Bond::new(c, k as f32 * std::f32::consts::FRAC_PI_2);
        let mut seen = None;
        for _ in 0..120 {
            sim.drones.set_channel(0x48);
            sim.drones.set_channel(0x49);
            sim.step_bond(Default::default(), false, false, true);
            if sim.drones.level_exit.is_some() { seen = sim.drones.level_exit; break; }
        }
        if seen.is_some() { assert_eq!(seen, Some(0x0700_000b), "the keyed E-path loads 0700000b [M 0x1b2318:111-116]"); return; }
    }
    panic!("the keyed E-path never loaded 0700000b");
}

/// 07000011 → 07000012: the two 0xE8s (gate ch 0x1f / key ch 0x1e) load 07000012. The gate's
/// in-level writer is gameplay the port cannot drive headless (no placement param or script event
/// carries 0x1f/0x1e — live check filed), so the test raises the gate the way that gameplay would
/// and pins the auto-fire and the keyed E-path arms.
#[test]
fn the_07000011_exit_gate_auto_fires_to_07000012() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut sc = load(&iso, "07000011.bin");
    let sim = sc.sim.as_mut().unwrap();
    let gated = find(sim, |t| t.kind == kind::LOAD_LEVEL && t.gate == 0x1f);
    let keyed = find(sim, |t| t.kind == kind::LOAD_LEVEL && t.key_gate == 0x1e);
    assert_eq!((gated.next_level, keyed.next_level), (0x0700_0012, 0x0700_0012),
        "both exits load 07000012 [D placements #124/#118]");
    // Sister: nothing fires before the gate rises.
    for _ in 0..300 { sim.step_bond(Default::default(), false, false, false); }
    assert_eq!(sim.drones.level_exit, None);
    // The gate on → the auto-fire arm (no touch needed) [M Trigger_Update case 10 0x1b1a78:162-166].
    sim.drones.set_channel(0x1f);
    let mut seen = None;
    for _ in 0..300 {
        sim.step_bond(Default::default(), false, false, false);
        if sim.drones.level_exit.is_some() { seen = sim.drones.level_exit; break; }
    }
    assert_eq!(seen, Some(0x0700_0012), "the gate arm chains 11 → 12");
}

/// 0700004a → 07000013: the lobby touch (out ch 0x84) starts the pack cut-scene (0790004a's script
/// 0x060006d3), whose stream 9 fires event 0x12 with the standalone MOVIE code 0x0720004a; the load
/// pipeline's phase 2 maps it to 07000013 [M 0x1d3560]. The port used to hand the raw movie code to
/// the archive loader (no 0720004a.bin exists on the disc).
#[test]
fn the_0700004a_cutscene_requests_the_movie_code_that_lands_on_13() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut sc = load(&iso, "0700004a.bin");
    let sim = sc.sim.as_mut().unwrap();
    assert!(!sim.drones.triggers.iter().any(|t| t.kind == kind::LOAD_LEVEL && t.next_level != 0),
        "0700004a has no 0xE8 exit [D probe: all 42 maps]");
    let touch = find(sim, |t| t.kind == kind::TOUCH_ONCE && t.out == 0x84);
    stand_in_box(sim, touch.lo, touch.hi);
    let mut seen = None;
    for _ in 0..4000 {
        sim.step_bond(Default::default(), false, false, false);
        if sim.drones.level_exit.is_some() { seen = sim.drones.level_exit; break; }
    }
    assert_eq!(seen, Some(0x0720_004a),
        "the cut-scene script's 0x12 carries the movie code [D pack 0790004a script 060006d3 stream9]");
    // And the resolve the view now applies lands on 13 (the unit test pins the whole table).
    assert_eq!(triggers::resolve_exit_code(0x0720_004a), 0x0700_0013,
        "the movie code resolves to 07000013 [M 0x1d3560]");
}
