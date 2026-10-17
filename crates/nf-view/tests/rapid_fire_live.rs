//! The PC-port "Rapid fire" option (`--rapid-fire`, the dev Toggles page): the user (2026-10-16) "pc
//! port should have a rapid fire option just to show the possibilities of performance". [S port-only:
//! the game has no such mode.] Held fire shoots one round per tick from a full clip; off, the weapon
//! keeps the game's own refire rules. Drives the real `step_bond` path. Opt-in: `NF_ISO`.

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// Hold the trigger for `ticks` ticks with the unguided rocket launcher on 07000005; returns the
/// rounds fired, the most live player bullets at once and the clip at the end.
fn hold(rapid: bool, ticks: u32) -> (u32, usize, u32) {
    let iso = iso().unwrap();
    let mut scene = nf_view::load_weapons(&iso, "07000005.bin", false, &[0x2d]).expect("load 05");
    let sim = scene.sim.as_mut().unwrap();
    let wd = sim.weapons.clone();
    let rounds = wd.clip_size(0x2d).max(1);
    assert!(sim.drones.inventory.equip_weapon(&wd, 0x2d, rounds, true), "equip the unguided launcher");
    sim.drones.inventory.equip_ammo(&wd, 0x2d, rounds * 3);
    sim.gun.switch_request = 0x2d;
    for _ in 0..150 {
        if sim.disabled { sim.drones.scripts.skip_request = true; }
        sim.step_bond(Default::default(), false, false, false);
    }
    assert_eq!(sim.gun.weapon, 0x2d, "the launcher is drawn");
    sim.rapid_fire = rapid;
    let (mut fired, mut most) = (0u32, 0usize);
    for _ in 0..ticks {
        if sim.disabled { sim.drones.scripts.skip_request = true; }
        sim.step_bond(Default::default(), true, false, false);
        if sim.shot_tick { fired += 1; }
        most = most.max(sim.drones.player_bullets.len());
    }
    (fired, most, sim.gun.clip)
}

#[test]
fn rapid_fire_shoots_every_tick_and_off_keeps_the_game_rate() {
    unsafe { std::env::set_var("NF_SILENT", "1"); }
    if iso().is_none() { return; }
    let (fired, most, clip) = hold(true, 120);
    eprintln!("rapid fire: {fired} rockets in 120 ticks, {most} in flight at once, clip {clip}");
    assert!(fired >= 115, "rapid fire shot {fired} of 120 ticks");
    assert!(most >= 60, "only {most} rockets in flight at once");
    assert!(clip >= 1, "the clip ran dry");
    // Sister: off, the same hold fires at the launcher's own refire rate (measured 4 in the 2 s).
    let (fired_off, _, _) = hold(false, 120);
    eprintln!("game rate: {fired_off} rockets in 120 ticks");
    assert!(fired_off >= 1 && fired_off * 10 < fired, "the game's rate fired {fired_off} rockets in 2 s");
}
