//! fix-06-alarm: the courtyard's alarm SOUND (the user's playtest 1013-1825: the guards react, "but
//! there's no alarm that goes off").
//!
//! The game's alarm path from the searchlight spot, all evidenced:
//! - the armed head writes alarm channel 55 every tick [M Searchlight_Update 0x1cb900:142-144]
//!   (ported by 045l — the guards' second switch works);
//! - placement 702 (kind 0xF9, object `oSoundTrigger _Cube`, at 28.66, 40.42, −17.30) is the
//!   SIREN EMITTER [D 07000006 placement 702]: params (0,55) ON channel, (1,1) one-shot,
//!   (2,329) sound id, (3,1) 2-D, (4,177) OFF channel — `SoundTrigger_Create` 0x1b2628 reads the
//!   placement's param slots sequentially (p_k at +0x2c+4k; the same layout the searchlight's
//!   known on-channels 64/63/61/62 pin [D 07000006 placements 2/4/6]), and `SoundTrigger_Update`
//!   0x1b2790 plays the id while the ON channel is up, stops on the OFF channel;
//! - sound 0x149 (329) is LOOPED with inner 400 / outer 500 [D SfxTable, `nf-extract sfx 149`] —
//!   the klaxon holds until the OFF channel;
//! - channel 55 also feeds `DroneFunc_CheckAlarmRaised` 0x1453a8 → `Music_Event(2,1)` for the
//!   first ~598 ticks → `UpdateMFX_01CastleB` 0x113060's hysteresis demands the combat music
//!   (ported: drone.rs's CheckAlarmRaised block + music.rs's castle_b).
//!
//! The port never created SoundTrigger objects — the siren half of the alarm was silent. These
//! tests drive the real path (`nf_view::load` + `Sim::step_bond`) and fail on pre-fix master.
//! Opt-in via NF_ISO. NF_SILENT stays respected (assertions read the EE-side node pool and the
//! music command board, never the driver's output).

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// The beam spot of the 06-searchlight.trace's opening (045l's test spot: Bond inside head #2's
/// sweep at the courtyard entrance; the game's alarm fired within ~1 s of standing there [L trace
/// frame 140866+]).
const BEAM_SPOT: [f32; 3] = [4.8, 42.09, -16.0];

/// The alarm siren's sound id [D placement 702 param 2 = 329 = 0x149; SfxTable 0x149 looped].
const SIREN_ID: u32 = 329;

#[test]
fn stepping_into_a_beam_sounds_the_alarm_siren() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000006.bin", false).expect("load 07000006");
    let sim = scene.sim.as_mut().unwrap();
    for _ in 0..300 { sim.step_bond(Default::default(), false, false, false); if !sim.disabled { break; } }
    assert!(!sim.disabled, "Bond is free after the intro");

    // Sister pre-check: outside the beam the siren never sounds (the trigger's ON channel is the
    // searchlights' alarm write [M 0x1cb900:142-144] — before the arm, ch 55 is low).
    for _ in 0..60 { sim.step_bond(Default::default(), false, false, false); }
    assert!(!sim.drones.audio.active_ids().contains(&SIREN_ID),
        "the siren does not sound before the alarm channel rises");
    assert!(!sim.drones.channels.on(55), "ch 55 is low before the beam");

    // Bond steps into the beam: the head arms (state 1), ch 55 rises [M 0x1cb900:142-144], and the
    // courtyard's siren trigger (ON ch 55) starts sound 329 on the same frame's object walk
    // [M SoundTrigger_Update 0x1b2790:14-42].
    sim.bond.pos = BEAM_SPOT;
    sim.bond.yaw = -2.2091;
    let mut alarm = None;
    let mut siren = None;
    for t in 0..720u32 {
        sim.step_bond(Default::default(), false, false, false);
        if alarm.is_none() && sim.drones.channels.on(55) { alarm = Some(t); }
        if siren.is_none() && sim.drones.audio.active_ids().contains(&SIREN_ID) { siren = Some(t); break; }
    }
    let alarm = alarm.expect("standing in the beam raises alarm channel 55 (045l's chain)");
    let siren = siren.expect("the courtyard siren (sound 329, placement 702's oSoundTrigger on ch 55) \
        starts when the alarm channel rises [M SoundTrigger_Update 0x1b2790:29-37; D placement 702] — \
        pre-fix the port created no SoundTrigger objects and the klaxon never played");
    assert!(siren >= alarm, "the siren follows the alarm channel (the trigger reads ch 55 [D placement 702 param 0])");
    assert!(siren - alarm <= 2, "the siren starts within the same-frame object walk + the deferred \
        start [M 0x1b2790 runs each frame; Sound_Play is node-only, the start rides the next \
        Sound_UpdateSounds] — got {} ticks", siren - alarm);

    // The siren is LOOPED [D SfxTable 0x149 looped] and its OFF channel (177 [D placement 702
    // param 4]) never rises here — it keeps sounding while the alarm holds.
    for _ in 0..300 { sim.step_bond(Default::default(), false, false, false); }
    assert!(sim.drones.audio.active_ids().contains(&SIREN_ID),
        "the looped klaxon holds while ch 55 is up and no OFF channel rose");

    // The alarm MUSIC half: ch 55 feeds DroneFunc_CheckAlarmRaised 0x1453a8's Music_Event(2,1)
    // stream (the port's CheckAlarmRaised block), whose CastleB hysteresis (music.rs) picks a
    // combat section ~12 ticks in [M UpdateMFX_01CastleB 0x113060:26-30 + the unit test]. The
    // headless sim drains nothing, so the command board accumulates.
    let combat = sim.drones.music.table.iter().find(|m| m.level == 0x0700_0006)
        .map(|m| m.combat.clone()).unwrap_or_default();
    assert!(!combat.is_empty(), "07000006's MapMusic record carries a combat set");
    let jumped = sim.drones.music.out.iter().any(|c| matches!(c,
        nf_game::music::Cmd::Jump { section, instant: false } if combat.contains(section)));
    assert!(jumped, "the alarm music leaves stealth: a combat-section Jump is requested after the \
        alarm (the CheckAlarmRaised Music_Event(2,1) stream → the CastleB hysteresis) [M 0x1453a8:20-32, 0x113060]");
}

/// Sister: outside the beams nothing sounds — no siren node, no combat jump, ch 55 low.
#[test]
fn outside_the_beam_nothing_sounds() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000006.bin", false).expect("load 07000006");
    let sim = scene.sim.as_mut().unwrap();
    for _ in 0..300 { sim.step_bond(Default::default(), false, false, false); if !sim.disabled { break; } }
    // The 06-firstsight.trace corridor spot (045l's sighting test spot — the guards engage there,
    // and no searchlight beam reaches it).
    sim.bond.pos = [83.58, 35.13, 33.21];
    sim.bond.yaw = 1.2934;
    for _ in 0..500 { sim.step_bond(Default::default(), false, false, false); }
    assert!(!sim.drones.channels.on(55), "ch 55 stays low outside the beams");
    assert!(!sim.drones.audio.active_ids().contains(&SIREN_ID),
        "the siren never sounds outside the beams (sister of the trigger's channel gate [M 0x1b2790:16-17])");
    let combat = sim.drones.music.table.iter().find(|m| m.level == 0x0700_0006)
        .map(|m| m.combat.clone()).unwrap_or_default();
    let jumped = sim.drones.music.out.iter().any(|c| matches!(c,
        nf_game::music::Cmd::Jump { section, instant: false } if combat.contains(section)));
    assert!(!jumped, "no combat music is requested on a quiet run (the CheckAlarmRaised stream \
        needs ch 55 or ch 0x61 [M 0x1453a8:20-32])");
}
