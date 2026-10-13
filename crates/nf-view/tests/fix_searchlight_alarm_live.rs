//! fix-searchlight-alarm: 07000006's courtyard searchlights and the guards' reaction, driven through
//! the real path (`nf_view::load` + `Sim::step_bond`).
//!
//! The user's playtest (2026-10-12): "searchlights don't raise the alarm" and "guards shoot way too
//! soon upon spotting you". The game's measured anchors [tests/traces/06-firstsight.trace]:
//! - a guard's alertness (`Drone+0x50C`) jumps 0.1 → 1.0 on the sighting tick [L trace d9 @148595,
//!   d1 @148537];
//! - the first hit on Bond lands 70 frames (d9, 15.7 units) and 128 frames (d1, 9.6 units) after the
//!   jump — the AIM_STAND/STEP_AIM choreography (31/116 frames) plus the burst timers, never a
//!   same-tick shot [L trace];
//! - the courtyard lights: the disc's NIS 0x06000066 stream 0 stages the heads' on-channels (ch 64 = 1
//!   at wait 1, ch 63 at 59, ch 61 at 116, ch 62 at 167) [D 07000006 kind-7 NIS 0x06000066]; the port
//!   had nothing running it, so the heads stayed state 3 (dark) forever and alarm channel 55 — the
//!   searchlights' write [M Searchlight_Update 0x1cb900:142-144], the guards' second switch
//!   (`Drone+0x135` = 55 on every courtyard guard [L 07000006 placements 436-450, second_ch 55]) —
//!   could never rise.
//!
//! Opt-in via NF_ISO. State ids are written raw so this file compiles against the pre-fix master — and
//! fails there (the heads never leave state 3).

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// The beam spot of the 06-searchlight.trace's opening (Bond inside head #2's sweep at the courtyard
/// entrance). The game's alarm fired within ~1 s of standing there [L trace frame 140866+: the guards
/// run the alarmed cycle from the first recorded frame].
const BEAM_SPOT: [f32; 3] = [4.8, 42.09, -16.0];

#[test]
fn a_courtyard_searchlight_arms_and_raises_the_alarm_channel() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000006.bin", false).expect("load 07000006");
    let sim = scene.sim.as_mut().unwrap();
    // Wait out the level-start NIS if any (the truck-ride arrival is a cold load here).
    for _ in 0..300 { sim.step_bond(Default::default(), false, false, false); if !sim.disabled { break; } }
    assert!(!sim.disabled, "Bond is free after the intro");

    // The boot must stage the light channels [D 0x06000066 stream 0: 64 @ 1, 63 @ 59, 61 @ 116, 62 @ 167]
    // — all four heads sweep within ~2.8 s of the boot (head #2's on-channel 64 goes first).
    let mut lit = None;
    for t in 0..240u32 {
        sim.step_bond(Default::default(), false, false, false);
        if lit.is_none() && sim.drones.searchlights.parts.iter().any(|p| p.kind == 1 && p.state == 0) { lit = Some(t); }
    }
    let lit = lit.expect("a courtyard searchlight head leaves state 3 (the 0x06000066 boot raises its \
        on-channel; pre-fix nothing ran the script and all four heads stayed dark forever)");

    // Bond steps into the beam: the spot ray lands on his pick sphere, the 2.0 arm test flips the head
    // to state 1 [M Searchlight_Update 0x1cb900:251-263], and state 1 writes alarm channel 55 every
    // tick [M 0x1cb900:142-144]. The game's alarm was up within ~1 s of standing in the beam [L trace].
    sim.bond.pos = BEAM_SPOT;
    sim.bond.yaw = -2.2091;
    let mut armed = None;
    let mut alarm = None;
    // A full sweep period (the phase fmod 480) plus margin: the spot must pass over Bond.
    for t in 0..720u32 {
        sim.step_bond(Default::default(), false, false, false);
        if armed.is_none() && sim.drones.searchlights.parts.iter().any(|p| p.kind == 1 && p.state == 1) { armed = Some(t); }
        if alarm.is_none() && sim.drones.channels.on(55) { alarm = Some(t); break; }
    }
    let alarm = alarm.expect("standing in the beam raises alarm channel 55 (pre-fix: the head could \
        never arm, so the channel stayed low and no guard reacted — the user's 'no alarm')");
    let armed = armed.expect("the head tracked (state 1) before the alarm channel rose");
    assert!(alarm >= armed, "the alarm channel only writes from the tracking state [M 0x1cb900:142-144]");
    assert!(alarm <= lit + 720, "the arm beat follows the light-up within a sweep period (the game: within ~1 s of \
        standing in the already-sweeping beam [L 06-searchlight.trace])");

    // The alarm channel is the guards' second switch (`+0x135` = 55, tag 100): PreDroneControl consumes
    // it and sends them to their attack state [M PreDroneControl 0x148db0:51-61] — give the walk a few
    // seconds for the consumption and the combat ramp.
    for _ in 0..180u32 { sim.step_bond(Default::default(), false, false, false); }
    let attacks = sim.drones.list.iter().filter(|d| d.present && d.enabled
        && matches!(d.state, 0x56 | 0x58 | 0x59 | 0x5a | 0x5b | 0x5c | 0x67 | 0x68 | 0x6f | 0x70 | 0x73 | 0x74 | 0x8c | 0x93 | 0x94 | 0x95 | 0x96)).count();
    assert!(attacks >= 3, "the courtyard guards react to alarm channel 55 (got {attacks} in attack \
        states; the game's alarmed cycle runs across ~20 guards [L 06-searchlight.trace])");
}

/// The first-shot floor. The game's chains from the sighting tick to the first hit:
/// reaction ack (10 ticks on 07000006 [M DroneFunc_ReactionTime 0x14ac60 default arm]) + the
/// combat choreography (AIM_STAND 31 frames [L d9] or STEP_AIM 116 [L d1]) + the burst timing
/// (`DroneWeap_NextBulletTime` 0x179d40: `+0xBC8 += BurstDelay` [M 0x179ba8: min 15]) + travel.
/// Even the fastest legal game chain (no combat move rolled, BurstDelay's 15 floor) never shoots
/// inside ~26 ticks of the alertness jump. The port's machinery (the same reaction, the same
/// BurstDelay) must keep that floor.
#[test]
fn the_first_shot_never_lands_inside_the_game_s_reaction_floor() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000006.bin", false).expect("load 07000006");
    let sim = scene.sim.as_mut().unwrap();
    for _ in 0..300 { sim.step_bond(Default::default(), false, false, false); if !sim.disabled { break; } }
    // The guards' switch channel: the touch triggers #0/#41/#55 (out 12) at (73.3, 37.8, 28.1) raise it
    // on the walk in [L placements]; the teleporting test stands in for that corridor walk.
    sim.drones.channels.set(12, 1, 0);
    // The 06-firstsight.trace sighting spot: d9 (92.0, 20.0) at 15.7 units, d1 (91.7, 36.3) at 8.7.
    sim.bond.pos = [83.58, 35.13, 33.21];
    sim.bond.yaw = 1.2934;
    let slots: Vec<usize> = sim.drones.list.iter().enumerate()
        .filter(|(_, d)| d.present && ((d.pos[0] - 92.0).abs() < 0.5 && (d.pos[2] - 20.0).abs() < 2.0
            || (d.pos[0] - 91.7).abs() < 0.5 && (d.pos[2] - 36.3).abs() < 2.0))
        .map(|(i, _)| i).collect();
    assert!(slots.len() >= 1, "the courtyard guards d9/d1 are placed");

    let mut jump: Option<u32> = None;
    let mut first_shot: Option<u32> = None;
    'run: for t in 0..300u32 {
        sim.step_bond(Default::default(), false, false, false);
        for &s in &slots {
            let dr = &sim.drones.list[s];
            if jump.is_none() && dr.alert >= 1.0 { jump = Some(t); }
            if first_shot.is_none() && !dr.shots.is_empty() { first_shot = Some(t); break 'run; }
        }
    }
    let (jump, first_shot) = match (jump, first_shot) {
        (Some(j), Some(f)) => (j, f),
        _ => { eprintln!("no sighting-then-shot in 300 ticks (RNG roll) — the floor assert is vacuous"); return; }
    };
    let delta = first_shot - jump;
    assert!(delta >= 26, "the first shot lands {delta} ticks after the alertness jump — inside the game's \
        legal floor (reaction 10 [M 0x14ac60] + fire-state entry + BurstDelay's 15 [M 0x179ba8] + \
        interval [M 0x179d40]); the game's measured chains were 70 and 128 [L 06-firstsight.trace]");
    assert!(delta <= 180, "the first shot lands {delta} ticks after the jump — the game's slowest \
        measured chain was 128 [L 06-firstsight.trace] and the choreography (AIM_STAND 31 / STEP_AIM 116 \
        + burst) keeps the port in that band");
}
