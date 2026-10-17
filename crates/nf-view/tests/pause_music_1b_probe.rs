//! pause-music-1b probe (not a pass/fail test): what plays on 0700001b, and what the pause holds.
//!
//! Playtest 1016-1355: "Music still plays during the pause menu" on 0700001b; earlier fixes stopped
//! it on other levels. This probe watches every audio source the real path starts on 1b — the
//! driver's items (the bank ambients / streamed effects), the MFX music player (the MapMusic track),
//! and the command streams — before and after `Sound::set_paused(true)`.
//! Opt-in via NF_ISO: `NF_ISO=... cargo test --release -p nf-view --test pause_music_1b_probe -- --nocapture --test-threads=1`.

use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn full_sound(scene: &mut nf_view::scene::Scene) -> nf_audio::sfx::Sound {
    let mut snd = nf_audio::sfx::Sound {
        driver: Default::default(),
        output: None,
        music_files: None,
        music: None,
        music_pending: Vec::new(),
    };
    if let Some((slot, bank, sbf)) = scene.sound_bank.take() { snd.driver.load_bank(slot, bank, sbf); }
    if let Some((lut, bin)) = scene.streams.take() { snd.driver.load_streams(lut, std::sync::Arc::new(bin)); }
    if let Some((t, smf, ssd)) = scene.music_files.take() { snd.music_files = Some((t, smf, std::sync::Arc::new(ssd))); }
    snd
}

fn dump(tag: &str, snd: &nf_audio::sfx::Sound) {
    let music = snd.music.as_ref().map(|p| format!("now_playing {} gain(m70,d100,f10000) {:.3}", p.now_playing, p.gain(70, 100, 10000)));
    eprintln!("[{tag}] items {} {:?} pending {:?} music {}", snd.driver.items(), snd.driver.ids(), snd.music_pending, music.unwrap_or("none".into()));
}

#[test]
fn probe_1b_what_plays_and_what_the_pause_holds() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "0700001b.bin", false).expect("load 0700001b");
    let mut snd = full_sound(&mut scene);
    {
        let sim = scene.sim.as_mut().unwrap();
        for t in 0..180 {
            sim.step_bond(Input::default(), false, false, false);
            nf_view::sound_tick(&mut snd, sim);
            if t == 30 || t == 60 || t == 120 || t == 179 { dump(&format!("tick {t}"), &snd); }
        }
    }
    // The pause page opens: the hold.
    snd.set_paused(true);
    dump("paused", &snd);
    // While the pause is up the host stops ticking the sim — but the DRIVER's update still runs in
    // some paths (the movie pump is gated by audio_paused, so no). Watch what a paused update emits.
    let mut out = Vec::new();
    snd.driver.update(&mut out, &[]);
    eprintln!("[paused update] {} cmds: {:?}", out.len(), out.iter().take(12).collect::<Vec<_>>());
    // Unpause: the resume.
    snd.set_paused(false);
    dump("unpaused", &snd);
}

#[test]
fn probe_05_sister_what_plays_and_what_the_pause_holds() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005");
    let mut snd = full_sound(&mut scene);
    {
        let sim = scene.sim.as_mut().unwrap();
        for t in 0..120 {
            sim.step_bond(Input::default(), false, false, false);
            nf_view::sound_tick(&mut snd, sim);
            if t == 30 || t == 60 || t == 119 { dump(&format!("tick {t}"), &snd); }
        }
    }
    snd.set_paused(true);
    dump("paused", &snd);
    snd.set_paused(false);
    dump("unpaused", &snd);
}
