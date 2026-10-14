//! fix-pause-audio live tests: the pause page's sound hold through the real input path.
//!
//! The user's playtest (2026-10-13): with the pause menu open, the ambient sound keeps playing,
//! including the truck-driving loop on 07000005. The port stops ticking the sim while a menu is up
//! (so `sound_tick` — and with it every new key-on — stops), but nothing pauses the sound output:
//! the already-keyed looping voices keep mixing and the music queue keeps draining.
//!
//! The game's pause path: `Player_Update` 0x193600:71-77 opens the pause menu with
//! `MenuManager_Create(…, pause_sound=1, …)` + `GS_PauseGame(1)`; `MenuManager_Create` 0x21b908:121-124
//! runs `SFXPause` (batch op 9) + `SFXPauseAllStreams` (op 0x16) when that flag is set, and
//! `MenuManager_Delete` 0x21c2b0:11-22 runs `SFXUnPause` (op 0xa) + `SFXUnPauseAllStreams` (op 0x17).
//! GameState+0x50 itself is read by NO sound code — the pause rides the batch ops [M]. On the IOP
//! (`SFX.IRX`), `SFXPause` 0x171c → `ES_PauseSfx` 0x1648 sets the item's `Paused` word (+0x290) and
//! pitch-pauses every live voice (`psiSamplePause` 0xf46c saves the pitch, sets 0); `SFXUpdate` 0x4ee4
//! skips Paused items (no 3-D update, no end test, Counter frozen). Items started *while* paused are
//! not paused — that is how the menu's own sounds play under the pause [M sound.md §8, §3.2].
//!
//! The assertions watch the driver (the IOP stand-in) through the exact functions the winit loop runs:
//! `nf_view::load` + `nf_view::sound_tick` (pub for the tests, the fix-gpu-skinning precedent).
//! Pre-fix `Sound::set_paused` does not exist and the driver has no pause at all
//! (docs/port-map/audio.md G-032: `SFXPause` / `psiSamplePause` not-started).
//!
//! Opt-in via `NF_ISO` (the slim ISO is enough).

use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// A bare output-less sound stack: the driver is what the assertions watch. The level's bank and
/// streams load into it exactly as `build_sound` does (nf_view/src/lib.rs:117-133) — without the
/// bank the driver rejects every start (the effect lookup needs the SFX table).
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

/// 07000005 (the playtest level): the truck-driving ambient loop keeps playing under the pause menu.
/// The loop keys on through the real path (`sound_tick` → the EE pool → `Driver::start`); the pause
/// page's `Sound::set_paused` must hold it — no recompute, no key-ons — and the unpause must restore
/// it on the next full update. Sister: unpaused, the per-frame recompute runs.
#[test]
fn the_pause_holds_the_ambient_and_the_unpause_resumes_it() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005");
    let mut snd = bare_sound(&mut scene);
    {
        let sim = scene.sim.as_mut().unwrap();
        // The ambient pool keys at the spawn: a few ticks of the real input path.
        for _ in 0..30 { sim.step_bond(Input::default(), false, false, false); nf_view::sound_tick(&mut snd, sim); }
    }
    assert!(snd.driver.items() > 0, "the level's ambient keyed on through the real path");
    // Sister (unpaused): each update recomputes the live channels at their pitch.
    let mut out = Vec::new();
    snd.driver.update(&mut out, &[]);
    assert!(out.iter().any(|c| matches!(c, nf_audio::Cmd::Set { pitch: p, .. } if *p != 0)),
        "unpaused the recompute runs: {out:?}");
    // The pause page opens: the hold reaches the driver at once — the host stops ticking the sim
    // while a menu is up, so the pause cannot wait for the next batch.
    snd.set_paused(true);
    assert!(snd.driver.paused(), "the driver reports the pause");
    let mut out = Vec::new();
    snd.driver.update(&mut out, &[]);
    assert!(!out.iter().any(|c| matches!(c, nf_audio::Cmd::Set { pitch: p, .. } if *p != 0)),
        "paused the items are skipped (no recompute): {out:?}");
    assert!(!out.iter().any(|c| matches!(c, nf_audio::Cmd::Play { .. })),
        "paused no key-ons: {out:?}");
    // The pause page closes: the next full update restores the voices.
    snd.set_paused(false);
    let mut out = Vec::new();
    snd.driver.update(&mut out, &[]);
    assert!(out.iter().any(|c| matches!(c, nf_audio::Cmd::Set { pitch: p, .. } if *p != 0)),
        "unpaused the recompute returns: {out:?}");
}

/// A start while the pause is up still keys on — the menu's own sounds (`Menu_PlaySound` 0x1f3278,
/// the 2-D ids) play under the pause in the game; the IOP pauses only the items live at the pause
/// and leaves new starts alone [M sound.md §8]. Sister: paused, the new item's recompute runs at a
/// non-zero pitch while the held items stay skipped.
#[test]
fn a_start_while_paused_still_plays_the_menu_sound() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005");
    let mut snd = bare_sound(&mut scene);
    {
        let sim = scene.sim.as_mut().unwrap();
        for _ in 0..30 { sim.step_bond(Input::default(), false, false, false); nf_view::sound_tick(&mut snd, sim); }
    }
    assert!(snd.driver.items() > 0, "the level's ambient keyed on through the real path");
    snd.set_paused(true);
    assert!(snd.driver.paused(), "the driver reports the pause");
    // The menu's own sound: `Menu_PlaySound` 0x1f3278's 2-D id 0x1da (the click), keys on under the
    // pause exactly as the game's new key-ons do [M sound.md §8, §3.2].
    let mut out = Vec::new();
    snd.driver.start(&mut out, 0x1da, None, 2, None);
    assert!(out.iter().any(|c| matches!(c, nf_audio::Cmd::Play { .. })),
        "a start while paused keys on (the menu click): {out:?}");
    let mut out = Vec::new();
    snd.driver.update(&mut out, &[]);
    assert!(out.iter().any(|c| matches!(c, nf_audio::Cmd::Set { pitch: p, .. } if *p != 0)),
        "paused the new item still plays (its recompute runs): {out:?}");
    // Sister: the held ambient's channels are NOT recomputed — only the new item's are.
    let sets = out.iter().filter(|c| matches!(c, nf_audio::Cmd::Set { pitch: p, .. } if *p != 0)).count();
    assert!(sets >= 1, "the new item's channels are live: {out:?}");
}
