//! profile-options (610): the AV page's volume sliders reach the audio driver's masters through
//! `nf_view::sound_tick` — the game's SFXSetVolume 0x116d98 / SFXMusicSetVolume 0x116cf8 write
//! SfxMasterVolume 0x30c95c / MusicMasterVolume 0x30c958 [M symtab]. NF_ISO-gated (a Sim needs a
//! level); the driver runs with `output: None` so the test stays silent headless.

use nf_view::scene;

fn sim_02() -> Option<scene::Sim> {
    let path = std::env::var_os("NF_ISO")?;
    let sc = nf_view::load(path.to_str().unwrap(), "07000002", false).ok()?;
    sc.sim
}

#[test]
fn volume_sliders_reach_the_driver() {
    let Some(mut sim) = sim_02() else { eprintln!("skip: NF_ISO not set"); return };
    let mut sound = nf_audio::sfx::Sound {
        driver: nf_audio::sfx::Driver::new(),
        output: None,
        music_files: None,
        music: None,
        music_pending: Vec::new(),
    };
    // The page's sliders: `Menu_Send(0x40)` returns 0..20, ×5 → the 0..100 the GSET stores
    // [M P_CNAVOPTIONS_Handler 0x214818:71-74]. 20 → music 100, 11 → sfx 55.
    sim.drones.options.music_volume = 100;
    sim.drones.options.sfx_volume = 55;
    nf_view::sound_tick(&mut sound, &mut sim);
    assert_eq!(sound.driver.music_master, 100, "the music slider lands on the driver's master");
    assert_eq!(sound.driver.master, 55, "the effects slider lands on the driver's master");
    // Sister: the driver's own boot defaults are the profile defaults (70 / 100) [M Driver::new —
    // the game's MusicMasterVolume boots at 70, SfxMasterVolume at 100].
    assert_eq!(nf_audio::sfx::Driver::new().music_master, 70);
    assert_eq!(nf_audio::sfx::Driver::new().master, 100);
    // Sister 2: the next tick re-syncs after an option change (the AV page writes the profile, the
    // snapshot follows).
    sim.drones.options.music_volume = 0;
    nf_view::sound_tick(&mut sound, &mut sim);
    assert_eq!(sound.driver.music_master, 0, "a lowered slider re-syncs the next tick");
}
