//! cutscene-skip-audio: skipping a cut-scene stops its voice (playtest 1014-1725, 07000002:
//! "Skip the cutscene in the middle of dialogue, and the dialogue still kept going until it
//! finished").
//!
//! The game's mechanism, all [M]:
//! - `Script_FFwd` 0x1dac78 arms the skip: fade ≤ 45 frames and `Music_Event(5, hash)`.
//! - `UpdateMusicalEvents` 0x114e70: event 5 clears `NisPlaying` and arms `NisQuitten = 5`.
//! - the IOP `SFXUpdate` 0x4ee4 step 3: while `NisQuitten` stands, every item with
//!   `StartedWhenNisPlaying` (sampled at `SFXStart3D` 0x3aac init) gets `FadeAndKill`; step 4
//!   spends `FadeAndKillVolume` −5/frame from 100 (≈21 frames), then deletes the item. The
//!   mid-line dialogue — stream or lip-synced anim op-1 — dies inside the skip's fade window.
//! - `Script_SoundStart` 0x1d8820 refuses to start a line once the fade has run past its length
//!   (`+0xAB5 & 0x20`), so the skip's catch-up is silent.
//! - the music handover: event 0xB (`Script_CameraStart`) raises `NisPlaying`; events 0xC (the
//!   camera stream's `Script_KillStream`) and 5 drop it; `NisQuitten` stops the ducker and counts
//!   down over 5 frames [M music.md §4.4].
//!
//! Headless through the real path (`nf_view::load` + `Sim::step_bond` + `nf_view::sound_tick`),
//! silent (NF_SILENT). Opt-in via `NF_ISO`.

use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// A bare output-less sound stack (the fix-pause-audio precedent): the level's bank and streams
/// load into the driver exactly as `build_sound` does, and the assertions watch the items.
fn bare_sound(scene: &mut nf_view::scene::Scene) -> nf_audio::sfx::Sound {
    let mut snd = nf_audio::sfx::Sound {
        driver: Default::default(),
        output: None,
        music_files: None,
        music: None,
        music_pending: Vec::new(),
    };
    if let Some((slot, bank, sbf)) = scene.sound_bank.take() { snd.driver.load_bank(slot, bank, sbf); }
    if let Some((lut, bin)) = scene.streams.take() { snd.driver.load_streams(lut, std::sync::Arc::new(bin)); }
    snd
}

/// One tick through the real path: the sim, then the sound batch (`sound_tick` pushes the music
/// board's `NisPlaying`/`NisQuitten` into the driver and processes the EE pool's commands).
fn tick(scene: &mut nf_view::scene::Scene, snd: &mut nf_audio::sfx::Sound) {
    let sim = scene.sim.as_mut().unwrap();
    sim.step_bond(Input::default(), false, false, false);
    nf_view::sound_tick(snd, sim);
}

/// The live items that started while the NIS played: (id, fade_kill, looping) — the driver's
/// `delete` items are already counted dead.
fn nis_items(snd: &nf_audio::sfx::Sound) -> Vec<(u32, bool, bool)> {
    snd.driver.items_probe().into_iter()
        .filter(|(_, del, _, swn, _)| *swn && !*del)
        .map(|(id, _, fk, _, looped)| (id, fk, looped)).collect()
}

/// 07000002's intro (script 0x0600_0084, the level-start NIS `ResetMap_Load` boots): skip it
/// mid-line and the voice dies inside the game's fade window — no NIS sound item or stream left
/// alive — while the music board quitted (`NisPlaying` down, `NisQuitten` armed then spent).
#[test]
fn the_intro_skip_kills_the_mid_line_voice_and_the_music_quits() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    unsafe { std::env::set_var("NF_SILENT", "1"); }
    let mut scene = nf_view::load(&iso, "07000002.bin", false).expect("load 07000002");
    let mut snd = bare_sound(&mut scene);
    // The intro plays: wait for its first live sound (the camera start arms event 0xB first).
    let mut t0 = None;
    for t in 0..1200u32 {
        tick(&mut scene, &mut snd);
        if !nis_items(&snd).is_empty() { t0 = Some(t); break; }
    }
    let Some(t0) = t0 else {
        panic!("the intro (0x0600_0084) never started a sound: probe {:?}", snd.driver.items_probe());
    };
    assert!(scene.sim.as_ref().unwrap().drones.music.nis_state().0,
        "the intro's camera start never raised NisPlaying (event 0xB)");
    let line_ids: std::collections::HashSet<u32> = nis_items(&snd).iter().map(|(id, ..)| *id).collect();
    eprintln!("[skip] first NIS sound at tick {t0}: {:?}", nis_items(&snd));

    // SKIP, mid-line: one press arms the fade and posts event 5.
    scene.sim.as_mut().unwrap().drones.scripts.skip_request = true;
    tick(&mut scene, &mut snd);
    {
        let (playing, quitten) = scene.sim.as_ref().unwrap().drones.music.nis_state();
        assert!(!playing && quitten, "the skip pulse never reached the music board: playing={playing} quitten={quitten}");
    }
    // Hold the button a few frames (the game's `Input_Action` fires every pressed frame) and run
    // the game's fade window out: fade ≤ 45 frames + `NisQuitten`'s 5 + the ≈21-frame kill — 80
    // ticks clear all of it.
    for t in 0..80u32 {
        if t < 4 { scene.sim.as_mut().unwrap().drones.scripts.skip_request = true; }
        tick(&mut scene, &mut snd);
    }
    assert!(nis_items(&snd).is_empty(),
        "a NIS sound outlived the skip's fade window: {:?}", snd.driver.items_probe());
    let (playing, quitten) = scene.sim.as_ref().unwrap().drones.music.nis_state();
    assert!(!playing, "NisPlaying stuck up after the skip");
    assert!(!quitten, "NisQuitten never spent its 5-frame countdown");
    // The lines really were still alive at the skip (the catch-up didn't outrun the test).
    assert!(!line_ids.is_empty());
}

/// Sister: without a skip the intro's line runs to its own sample end — the item dies while the
/// music board never arms `NisQuitten` and nothing is FadeAndKilled (`Script_SoundEnd` 0x1daf60
/// stops only looping sounds at their marker; a one-shot plays out).
#[test]
fn without_a_skip_the_intro_line_runs_to_its_own_end() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    unsafe { std::env::set_var("NF_SILENT", "1"); }
    let mut scene = nf_view::load(&iso, "07000002.bin", false).expect("load 07000002");
    let mut snd = bare_sound(&mut scene);
    let mut t0 = None;
    for t in 0..1200u32 {
        tick(&mut scene, &mut snd);
        if !nis_items(&snd).is_empty() { t0 = Some(t); break; }
    }
    let Some(t0) = t0 else { panic!("the intro never started a sound"); };
    // The first ONE-SHOT line (the NIS score, if any, loops and never ends on its own).
    let Some(line_id) = nis_items(&snd).into_iter().find(|(_, _, looped)| !*looped).map(|(id, ..)| id) else {
        eprintln!("[skip] only looping sounds in the window — nothing to playout"); return;
    };
    for t in 0..1800u32 {
        tick(&mut scene, &mut snd);
        let (_, quitten) = scene.sim.as_ref().unwrap().drones.music.nis_state();
        assert!(!quitten, "NisQuitten armed without a skip (tick {})", t0 + t);
        let alive = nis_items(&snd).iter().any(|(id, fk, _)| *id == line_id && !*fk);
        if !alive { return; }
    }
    panic!("the line (id {line_id:#x}) never ended on its own within 1800 ticks of tick {t0}");
}

/// 07000007's Kimiko scene (the hall-floor trigger 412 → script 0x0600_0073, the control-taking
/// cut-scene): skip mid-line — no voice item or stream left alive after the fade window, and the
/// level music handover completes (NisPlaying back down, the countdown spent).
#[test]
fn the_kimiko_scene_skip_leaves_nothing_alive_and_the_music_resumes() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    unsafe { std::env::set_var("NF_SILENT", "1"); }
    let mut scene = nf_view::load(&iso, "07000007.bin", false).expect("load 07000007");
    let mut snd = bare_sound(&mut scene);
    {
        let sim = scene.sim.as_mut().unwrap();
        // Inside TouchOnce placement 412's volume (the fix-cutscenes numbers).
        sim.bond.pos = [94.3, 44.6, -45.9];
        for _ in 0..90 { tick(&mut scene, &mut snd); }
    }
    assert!(scene.sim.as_ref().unwrap().drones.scripts.players.iter().any(|p|
        p.scripts.contains(&0x0600_0073) && p.state == 2), "the Kimiko scene (0x06000073) never started");
    let mut t0 = None;
    for t in 0..900u32 {
        tick(&mut scene, &mut snd);
        if nis_items(&snd).iter().any(|(_, _, looped)| !*looped) { t0 = Some(t); break; }
    }
    let Some(t0) = t0 else {
        panic!("the Kimiko scene never started a one-shot sound by tick {t0:?}: {:?}",
            snd.driver.items_probe());
    };
    eprintln!("[skip] Kimiko scene sound at +{t0}: {:?}", nis_items(&snd));

    scene.sim.as_mut().unwrap().drones.scripts.skip_request = true;
    tick(&mut scene, &mut snd);
    {
        let (playing, quitten) = scene.sim.as_ref().unwrap().drones.music.nis_state();
        assert!(!playing && quitten, "the skip pulse never reached the music board: playing={playing} quitten={quitten}");
    }
    for t in 0..80u32 {
        if t < 4 { scene.sim.as_mut().unwrap().drones.scripts.skip_request = true; }
        tick(&mut scene, &mut snd);
    }
    assert!(nis_items(&snd).is_empty(),
        "a Kimiko-scene sound outlived the skip's fade window: {:?}", snd.driver.items_probe());
    let (playing, quitten) = scene.sim.as_ref().unwrap().drones.music.nis_state();
    assert!(!playing && !quitten, "the level music handover never completed: playing={playing} quitten={quitten}");
}
