//! frag-throw-timing repro/pin tests, driven through the real input path (`nf_view::load` + `Sim::step_bond`).
//!
//! Playtest 1016-1355 (07000014): "frag grenades are flying out way too soon. They don't let the
//! animation finish." The game's release for the grenade family (`weapon_data+0x6C & 0x400` with a
//! `+0xB4` script): the fire script plays at 1.5× in view state 0xb [M `Player_WeaponFiring`
//! 0x1a5460:676-685 — the state store and the `AnimList+0x98 = 1.5` rate], the state machine moves
//! to 0xc only when that script has ENDED [M `Player_SetWeaponAnimObj` 0x1a2e38 case 0xb —
//! `AnimScriptIsStopped` gates the transition; `AnimScriptEnd` 0x1c1930:30 sets the ended bit the
//! check reads], and the release check `weapon_data+0x44 <= AnimList(4)+0x90` lives in case 0xc
//! [M 0x1a2e38:505-519, disasm 0x1a3268-0x1a32c4] — i.e. it compares the +0xB4 script's playhead,
//! which starts at 1.0 [M `AnimScriptInit` 0x1c1600:37] and advances 1.0/tick [M `AnimScriptTick`
//! 0x1c2630 mode 0]. Case 9's identical check [M 0x1a2e38:411-422] never runs for this family:
//! the state leaves 9 inside the fire tick itself [M 0x1a5460:676-685].
//!
//! Frag facts [L ee_start_05.bin weapon_data 0x34]: +0x44 = 26, +0x6C = 0x8c08 (bit 0x400), +0xB0 =
//! 0x0600049d (70 frames), +0xB4 = 0x0600049e (35 frames) [D, the port's script parse]. The fire
//! script's playhead (1 + 1.5·k) reaches its 70-frame end at k = 46; the +0xB4 playhead (1 + k)
//! reaches 26 at k = 25 → the grenade spawns 71 ticks after the press.
//!
//! Opt-in via `NF_ISO`.

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn load(iso: &str, level: &str) -> nf_view::scene::Scene {
    let mut scene = nf_view::load(iso, level, false).expect("load");
    let intro: &[u32] = match level {
        "07000001.bin" => &[0x0600_0087],
        "07000002.bin" => &[0x0600_0084],
        _ => &[],
    };
    skip_active_cutscene(&mut scene, intro);
    scene
}

fn skip_active_cutscene(scene: &mut nf_view::scene::Scene, intros: &[u32]) {
    if intros.is_empty() { return; }
    let pending = |sim: &nf_view::scene::Sim| sim.drones.scripts.players.iter().any(|p| intros.contains(&p.scripts[0]) && (matches!(p.state, 1 | 2 | 7) || p.info[p.cur].as_ref().is_some_and(|s| s.play & 1 != 0)));
    for t in 0..600u32 {
        let sim = scene.sim.as_mut().unwrap();
        if t >= 2 && !pending(sim) && !sim.disabled { break; }
        sim.drones.scripts.skip_request = true;
        sim.step_bond(Default::default(), false, false, false);
    }
    assert!(!scene.sim.as_ref().unwrap().disabled, "the level-start cut-scene never handed control back");
}

fn tick_idle(sim: &mut nf_view::scene::Sim, n: u32) {
    for _ in 0..n { sim.step_bond(Default::default(), false, false, false); }
}

/// Equip a weapon id through the real take + switch path (a kind-0 pickup at Bond's feet).
fn equip(sim: &mut nf_view::scene::Sim, id: usize) {
    let model = sim.weapons.model(id);
    let bounds = *sim.models.get(&model).unwrap_or(&nf_game::pickup::Model { centre: [0.0; 3], radius: 0.3 });
    let at = sim.bond.pos;
    let p = nf_game::pickup::Pickup::create_placed(at, [0.0; 3], nf_game::pickup::kind::WEAPON, id as u16, 1, 0, 0xFFFF, 0, 0, model, bounds, &sim.weapons, &std::collections::HashMap::new())
        .expect("pickup builds");
    sim.drones.pickups.push(p);
    tick_idle(sim, 5);
    assert!(sim.drones.inventory.owned[id], "{id:#x} owned after the take");
    sim.gun.switch_request = id as u8;
    tick_idle(sim, 60);
    assert_eq!(sim.gun.weapon, id as u8, "equipped after the holster/draw cycle");
    assert!(sim.gun.timer <= 0.0, "the refire timer is run out before the throw");
}

/// The frag throw: press fire once, then hold nothing, and return the tick (counted from the press,
/// press tick excluded) at which the world grenade first exists, plus whether one ever appeared.
fn throw_and_measure(sim: &mut nf_view::scene::Sim) -> (Option<u32>, Option<(u32, u32)>) {
    sim.step_bond(Default::default(), true, false, false); // the press tick
    let mut first = None;
    let mut scripts = None;
    for t in 1..=200u32 {
        if scripts.is_none() { scripts = Some((t, sim.view_script)); }
        sim.step_bond(Default::default(), false, false, false);
        if first.is_none() && !sim.drones.grenades.is_empty() { first = Some(t); }
        if first.is_some() && scripts.is_some() && t > 90 { break; }
    }
    (first, scripts)
}

/// The frag spawns exactly on the game's release tick: the fire script (70 frames) plays out at 1.5×
/// (46 ticks), then the +0xB4 script's playhead (1 + k) reaches +0x44 = 26 after 25 more — press + 71.
#[test]
fn frag_spawns_on_the_game_release_tick() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso, "07000001.bin");
    let sim = scene.sim.as_mut().unwrap();
    tick_idle(sim, 2);
    equip(sim, 0x34);
    let (falen, fire_hash, fire_last, speed) = {
        let w = sim.weapons.records.get(0x34).expect("the frag record");
        (u16::from_le_bytes([w[0x44], w[0x45]]) as u32,
         u32::from_le_bytes(w[0xb0..0xb4].try_into().unwrap()),
         u32::from_le_bytes(w[0xb4..0xb8].try_into().unwrap()),
         f32::from_le_bytes(w[0x20..0x24].try_into().unwrap()))
    };
    let fire_len = sim.scripts.get(&fire_hash).map(|s| s.length as u32).expect("the fire script loaded");
    let fire_last_len = sim.scripts.get(&fire_last).map(|s| s.length as u32).expect("the fire_last script loaded");
    // The release tick from the decompile facts: ceil((fire_len - 1)/1.5) ticks of fire script, then
    // falen - 1 ticks into the +0xB4 script (its playhead starts at 1.0).
    let windup = ((fire_len - 1) as f32 / 1.5).ceil() as u32;
    let expected = windup + (falen - 1);
    let (first, scripts) = throw_and_measure(sim);
    // The old behaviour (the case-9 check during the fire script) released at the playhead-26
    // crossing — 17 ticks into the 46-tick wind-up: the user's "way too soon".
    let old_crossing = ((falen as f32 - 1.0) / 1.5).ceil() as u32;
    assert_ne!(first, Some(old_crossing), "the grenade must not spawn at the fire script's frame-{} crossing (the playhead 26 tick)", falen);
    assert_eq!(first, Some(expected),
        "the grenade spawns when the +0xB4 script's playhead reaches +0x44 ({falen}): {fire_len}-frame fire script at 1.5x = {windup} ticks, then {falen} - 1 = {} ticks of the {}-frame bring-up",
        falen - 1, fire_last_len);
    // Sister: the view was still playing the FIRE script well past the old crossing — the throw
    // animation is not interrupted by the release.
    let (t17, script_at_17) = scripts.expect("the walk recorded the early script");
    assert!(t17 <= 20 && script_at_17 == fire_hash,
        "at tick {t17} the view script is still the fire script ({script_at_17:08x})");
    let g = &sim.drones.grenades[0];
    assert_eq!(g.weapon_type, 0x34);
    assert!((g.speed - speed).abs() < 1e-3, "speed = weapon_data+0x20 [L ee_start_05.bin record 0x34]: {}", g.speed);
    // The release latches (+0x95C [M 0x1a2e38:517]): exactly one grenade per press.
    tick_idle(sim, 5);
    assert_eq!(sim.drones.grenades.iter().filter(|g| g.weapon_type == 0x34).count(), 1, "one grenade per press");
    let _ = fire_last_len;
}
