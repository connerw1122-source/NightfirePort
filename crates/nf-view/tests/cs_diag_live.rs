#[test]
fn diag_02() {
    let Some(iso) = std::env::var("NF_ISO").ok() else { return };
    unsafe { std::env::set_var("NF_SILENT", "1"); }
    let mut scene = nf_view::load(&iso, "07000002.bin", false).expect("load");
    let mut snd = nf_audio::sfx::Sound { driver: Default::default(), output: None, music_files: None, music: None, music_pending: Vec::new() };
    if let Some((slot, bank, sbf)) = scene.sound_bank.take() { snd.driver.load_bank(slot, bank, sbf); }
    if let Some((lut, bin)) = scene.streams.take() { snd.driver.load_streams(lut, std::sync::Arc::new(bin)); }
    for t in 0..1500u32 {
        scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
        let sim = scene.sim.as_ref().unwrap();
        if t % 200 == 0 {
            let n = sim.drones.scripts.players.len();
            let states: Vec<String> = sim.drones.scripts.players.iter().take(8).enumerate()
                .map(|(i, sp)| format!("sp{i}:st{}", sp.state)).collect();
            eprintln!("[t{t}] disabled {} sps {n} {}", sim.disabled as u8, states.join(" "));
        }
        drop(sim);
        nf_view::sound_tick(&mut snd, scene.sim.as_mut().unwrap());
        let cmds = scene.sim.as_mut().unwrap().drones.audio.take_commands();
        let n_start = cmds.iter().filter(|c| matches!(c, nf_game::sound::Cmd::Start { .. })).count();
        if n_start > 0 { eprintln!("[t{t}] {n_start} EE starts"); }
    }
    eprintln!("[diag] driver items: {:?}", snd.driver.items_probe().len());
}
