//! fix-audio2 live tests: the animation scripts' op-1 sounds (`AnimProcessScriptCmds` 0x1c1b38 case 1) through
//! the real `Script_Interp` case 2 path (`nf_view::load` + `Sim::step_bond`).
//!
//! The user's playtest (2026-10-12): "voices are missing everywhere — cut-scenes on 07000001, 07000005 (the
//! truck ride end: 'it transitioned but there were no voicelines') and 07000007". The voices are op-1 commands
//! in the cut-scene characters' ANIMATION scripts (e.g. 07000005's ride script 0x06000994 starts anim
//! 0x06000996 whose op-1 at frame 769 is sfx 223 → STREAMS.BIN LUT[249], a voice line); the game fires them
//! from `Script_Interp` case 2 (0x1d9a08), which overrides the anim script's time with the stream's frame and
//! runs `AnimObjectUpdate` → `AnimScriptTick` → `AnimProcessScriptCmds`. The port's anim engine only consumed
//! op-0 (sequences), so every cut-scene voice was silent.
//!
//! The assertions watch the EE sound layer (`Drones.audio`, the port of `Sound_Play` / `Sound_Play3D`'s
//! node pool flushed once through `Sounds::update`): a voice line having started = a `Cmd::Start` with its
//! sfx id. Pre-fix none of these commands exist.
//!
//! Opt-in via `NF_ISO`.

use nf_game::sound::Cmd;
use nf_game::player::Input;

fn load(archive: &str) -> Option<nf_view::scene::Scene> {
    let iso = std::env::var("NF_ISO").ok()?;
    Some(nf_view::load(&iso, archive, false).expect("level loads"))
}

/// Flush the sound pool once: every node (2-D or 3-D) becomes a `Cmd::Start` (the table is `None`, so the
/// driver-side distance cull — which needs the bank — is not applied; the EE-side `Sound_Play3D` happened).
fn starts(scene: &mut nf_view::scene::Scene) -> Vec<u32> {
    let sim = scene.sim.as_mut().unwrap();
    let listener = ([0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]);
    sim.drones.audio.update(None, listener, &|_| true);
    sim.drones.audio.out.iter().filter_map(|c| match c {
        Cmd::Start { id, .. } => Some(*id), _ => None,
    }).collect()
}

/// 07000005: ScriptPlayer #298 (gate channel 0x43) plays 0x06000994 — the truck ride's dialogue. Its anims
/// 0x06000995/96/97 carry the voice lines as op-1 commands into the guard streams: 0x06000996 fires sfx 223
/// (LUT[249]) at frame 769 and 224 (LUT[250]) at 934; 0x06000997 fires 227 (LUT[252]) at 838 and 171
/// (LUT[253]) at 986. Before the fix none of these played — "no voicelines" during the ride.
#[test]
fn truck_ride_dialogue_plays_its_voice_lines() {
    let Some(mut scene) = load("07000005.bin") else { eprintln!("NF_ISO not set — skipping"); return };
    {
        let sim = scene.sim.as_mut().unwrap();
        // The gate channel of ScriptPlayer #298 (placement 298, script 0x06000994) — the ride's script
        // trigger opens it in play; the channel write is the same mechanism (Channels::set).
        sim.drones.set_channel(0x43);
        // The first 500 ticks: the dialogue script is running, but its first voice line is at frame 769.
        for _ in 0..500 { sim.step_bond(Input::default(), false, false, false); }
    }
    let early = starts(&mut scene);
    assert!(!early.iter().any(|id| matches!(id, 223 | 224 | 227 | 171)),
        "a ride voice line fired before its animation frame (frames 769/934/838/986): {early:?}");
    {
        let sim = scene.sim.as_mut().unwrap();
        for _ in 0..1100 { sim.step_bond(Input::default(), false, false, false); }
    }
    let all = starts(&mut scene);
    for id in [223, 224, 227, 171] {
        assert!(all.contains(&id), "voice line sfx {id} (the ride dialogue's op-1 stream) never played; starts: {:?}", all);
    }
}

/// 07000007 (the party): the Drake fly-through 0x06000072 (the fix-cutscenes path: Bond walks into TouchOnce
/// placement 356 → ch 0x15 → ScriptPlayer #419) — its guest anims carry the scene's lines: 0x060001FD fires
/// sfx 367 (LUT[16]) at frame 650, 366 (LUT[17]) at 1347, 327 (LUT[18]) at 2110.
#[test]
fn party_flythrough_voices_play() {
    let Some(mut scene) = load("07000007.bin") else { eprintln!("NF_ISO not set — skipping"); return };
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.bond.pos = [95.4, 49.6, -79.3]; // inside trigger 356 (fix_cutscenes_live's volume)
        for _ in 0..90 { sim.step_bond(Input::default(), false, false, false); }
    }
    assert!(scene.sim.as_ref().unwrap().drones.scripts.players.iter().any(|p| p.scripts.contains(&0x0600_0072) && p.state == 2),
        "the fly-through did not start from the trigger");
    {
        let sim = scene.sim.as_mut().unwrap();
        for _ in 0..2300 { sim.step_bond(Input::default(), false, false, false); }
    }
    let all = starts(&mut scene);
    for id in [367, 366, 327] {
        assert!(all.contains(&id), "the fly-through's voice line sfx {id} never played; starts: {:?}", all);
    }
}

/// 07000001: ScriptPlayer placement 277 (mode 1) plays 0x06000087 from the first update — the level's
/// dialogue scene. Its anims fire the voice streams: 0x0600055D sfx 489 (LUT[321]) at frame 615 and 491
/// (LUT[323]) at 1877; 0x0600055C sfx 488 (LUT[320]) at 1108; 0x0600055F also fires bank sample 598 at 1328.
#[test]
fn level_01_intro_dialogue_plays() {
    let Some(mut scene) = load("07000001.bin") else { eprintln!("NF_ISO not set — skipping"); return };
    {
        let sim = scene.sim.as_mut().unwrap();
        for _ in 0..400 { sim.step_bond(Input::default(), false, false, false); }
    }
    let early = starts(&mut scene);
    assert!(!early.iter().any(|id| matches!(id, 488 | 489 | 490 | 491)),
        "an intro voice line fired before its frame (615/1108/1328/1448/1877): {early:?}");
    {
        let sim = scene.sim.as_mut().unwrap();
        for _ in 0..1900 { sim.step_bond(Input::default(), false, false, false); }
    }
    let all = starts(&mut scene);
    for id in [489, 488, 490] {
        assert!(all.contains(&id), "intro voice line sfx {id} never played; starts: {:?}", all);
    }
    // The bank sample in the same anim set rides the same path.
    assert!(all.contains(&598), "the intro's op-1 bank sample 598 never played");
}
