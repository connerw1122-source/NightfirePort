//! fix-searchlights-guards: 07000006's searchlight alarm through the REAL walked path, and the
//! guards' first-shot gates, driven through `nf_view::load` + `Sim::step_bond`.
//!
//! The user's playtest (results 1013-1825): "Alarm still isn't going off" when walking into a
//! courtyard searchlight beam; "a spotlight will cover a piece of ground and it will flip on its
//! side"; "his gun's firing before it lowers at me". The walk here is what fix-searchlight-alarm's
//! tests stood in for with a teleport (`sim.bond.pos` writes) — this file walks Bond from the truck
//! start with `Input` only.
//!
//! Opt-in via NF_ISO.

#[path = "common/walker.rs"]
mod walker;

use nf_game::player::{Input, RamSave};
use walker::{Act, Walker, Wp};

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn carry(on: &[u32]) -> RamSave {
    let mut ch = nf_game::triggers::Channels::default();
    for &c in on { ch.set(c, 1, 0); }
    // A fresh Bond with only the carried gates (the truck route's own, [D placements]: the 0x1D gate
    // selects the truck-ride start #496; 0x45/0x70 ride the save like walk_07000006's).
    RamSave::capture(&Default::default(), 0, &nf_game::pickup::Inventory::level_start(0x73, 34), &ch)
}

/// The 06-searchlight.trace's opening beam spot (the game's own alarmed frame 140866, Bond at
/// (4.80, 42.09, −16.00)) [L tests/traces/06-searchlight.trace].
const BEAM_SPOT: [f32; 3] = [4.8, 42.09, -16.0];

/// The truck yard's anchors: the spawn pocket opens EAST only (the 16-dir ray fan at the spawn hits
/// within 3 units everywhere but +x), so the walk leaves the pocket east, rounds the parked truck
/// (north of it), crosses the cellar→crate gap (the D/C nav paths' nearest nodes are 6.8 apart —
/// no nav route exists, the ground is open) and enters the crate & well area. From there the direct
/// leg reaches the beam spot (the same leg walkreal's other routes use).
const ANCHORS: [[f32; 3]; 5] = [
    [61.0, 39.0, 22.6],  // east out of the spawn pocket
    [63.0, 38.7, 27.5],  // north-east around the truck
    [57.0, 38.6, 28.6],  // behind the truck (nav D33's node)
    [50.5, 38.8, 27.5],  // through the D→C gap
    [46.0, 38.5, 16.5],  // into the crate & well area (nav C56's node)
];

/// The walk the user plays: truck start → the courtyard beam, REAL INPUT ONLY (no `bond.pos`
/// writes after the spawn). The alarm channel 55 — the searchlights' write while tracking
/// [M Searchlight_Update 0x1cb900:142-144], the guards' second switch (+0x135 = 55) — must rise
/// from the walk itself; fix-searchlight-alarm's test teleported to the spot and pre-set ch 12.
#[test]
fn the_walked_beam_entry_raises_the_alarm_channel() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut w = Walker::new(nf_view::load_with_carry(&iso, "07000006.bin", false, Some(&carry(&[0x1d, 0x45, 0x70]))).expect("load 07000006"));
    w.wait_enabled(3000, "the truck-arrival intro");
    {
        let p = w.scene.sim.as_ref().unwrap().bond.pos;
        assert!((p[0] - 57.18).abs() < 2.0 && (p[2] - 22.61).abs() < 2.0, "the truck-ride spawn #496 [D], got {p:?}");
    }
    // The heads sweep from the light boot (ch 64/63/61/62 at the script's waits) [D 0x06000066].
    // By the time the walk reaches the courtyard (minutes of sim) all four sweep.
    for (k, a) in ANCHORS.iter().enumerate() {
        w.walk_to(Wp::new(a[0], a[1], a[2]).r(1.4), &format!("anchor {k}"));
    }
    w.walk_to(Wp::new(BEAM_SPOT[0], BEAM_SPOT[1], BEAM_SPOT[2]).r(1.3), "the courtyard beam spot");
    {
        let p = w.scene.sim.as_ref().unwrap().bond.pos;
        assert!(walker::dxz(p, BEAM_SPOT) < 2.0, "the walk reaches the beam spot [L 06-searchlight.trace], got {p:?}");
    }
    // Standing in the sweeping beam: the spot ray lands on Bond's pick sphere and the 2.0 arm test
    // flips a head to tracking [M 0x1cb900:251-263]; tracking writes ch 55 every tick [M :142-144].
    // The game's alarm was up within ~1 s of standing there [L trace frame 140866+].
    let mut armed = None;
    let mut alarm = None;
    for t in 0..720u32 {
        w.tick(Input::default(), false);
        let sim = w.scene.sim.as_ref().unwrap();
        if armed.is_none() && sim.drones.searchlights.parts.iter().any(|p| p.kind == 1 && p.state == 1) { armed = Some(t); }
        if alarm.is_none() && sim.drones.channels.on(55) { alarm = Some(t); break; }
    }
    let alarm = alarm.expect("the WALKED beam entry raises alarm channel 55 (the head arms off the \
        spot ray on Bond's sphere [M 0x1cb900:251-263]; fix-searchlight-alarm's teleport already \
        proved the channel from the spot — the walk is the user's path)");
    let armed = armed.expect("a head tracked (state 1) before the alarm rose");
    assert!(alarm >= armed, "the alarm channel only writes from the tracking state [M 0x1cb900:142-144]");
    // The guards consume the channel through their second switch [M PreDroneControl 0x148db0:51-61].
    for _ in 0..180u32 { w.tick(Input::default(), false); }
    let sim = w.scene.sim.as_ref().unwrap();
    let attacks = sim.drones.list.iter().filter(|d| d.present && d.enabled
        && matches!(d.state, 0x56 | 0x58 | 0x59 | 0x5a | 0x5b | 0x5c | 0x67 | 0x68 | 0x6f | 0x70 | 0x73 | 0x74 | 0x8c | 0x93 | 0x94 | 0x95 | 0x96)).count();
    assert!(attacks >= 3, "the courtyard guards react to the walked alarm (got {attacks} in attack states)");
}

/// The game's AimStandFire entry holds the first bullet while the guard is still playing the hit
/// reaction [M 0x166328:48-52: `+0xda == 0x13` (danim 19, Drone_AnimInfo[0x13] +8 = anim state
/// 0x4c) → `+0x3b = 0`, timer C armed for FRAME_RATE_INT ticks; the bullet rides the TIMER_C fire].
/// The port's entry fired unconditionally when Bond was seen — a guard shot mid-approach fired
/// straight out of the stagger. Sister: without the danim the entry fire keeps its game timing.
#[test]
fn a_guard_shot_mid_approach_holds_his_first_shot_for_the_impact_danim() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000006.bin", false).expect("load 07000006");
    let sim = scene.sim.as_mut().unwrap();
    for _ in 0..300 { sim.step_bond(Default::default(), false, false, false); if !sim.disabled { break; } }
    sim.drones.channels.set(12, 1, 0);
    // The 06-firstsight guard, Bond parked inside the crouch band (< 6 [M 0x173280]) but outside
    // the backoff one (> 4): the standing AIM_STAND_FIRE entry is forced on the sighting.
    sim.bond.pos = [90.0, 35.13, 24.0];
    sim.bond.yaw = 1.2934;
    let slot = sim.drones.list.iter().enumerate()
        .find(|(_, d)| d.present && (d.pos[0] - 92.0).abs() < 0.5 && (d.pos[2] - 20.0).abs() < 2.0)
        .map(|(i, _)| i).expect("the courtyard guard d9 is placed");
    // Stage the stagger the tick before the sighting resolves: the guard's machine into the fresh
    // sighting (via next_state, so the machine's own ENTER runs) and his danim to the hit reaction —
    // a bullet arriving on the same beat. The walker's top-up keeps Bond alive while the guard
    // reacts.
    let mut staged = false;
    for _ in 0..400u32 {
        {
            let dr = &mut sim.drones.list[slot];
            if dr.alert >= 1.0 && !staged {
                dr.next_state = Some(0x5b); // COMBAT_NEW_SIGHTING
                dr.anim.danim = 0x13;
                dr.unseen_ticks = 0;
                staged = true;
            }
        }
        sim.step_bond(Default::default(), false, false, false);
        if staged { break; }
    }
    if !staged { eprintln!("no sighting in 400 ticks (RNG roll) — the deferral assert is vacuous"); return; }
    // The entry tick: NO shot (the game's deferred path). The first shot rides timer C — at least
    // 60 ticks after the entry [M 0x166328:50 +0xcc4 = now + FRAME_RATE_INT].
    let mut first = None;
    for t in 0..300u32 {
        sim.step_bond(Default::default(), false, false, false);
        if !sim.drones.list[slot].shots.is_empty() { first = Some(t); break; }
    }
    let first = first.expect("the deferred guard still fires (timer C)");
    assert!(first >= 59, "the first shot waits out the impact-danim timer (got +{first} ticks; \
        [M 0x166328:48-52] arms timer C for FRAME_RATE_INT ticks) — the pre-fix entry fired on the \
        same tick the stagger ended");
}
