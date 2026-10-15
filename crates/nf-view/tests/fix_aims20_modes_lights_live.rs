//! fix-aims20-modes-lights: the AIMS-20's mode swap (0x1a ⇄ 0x1b) and the four side lights,
//! through the real input path (`Sim::step_bond` with the `Input` the window builds).
//!
//! The game, measured in PCSX2 (docs/live/2026-10-14-weapon-measurements-pcsx2.md, the AIMS-20
//! section): the four round lights on the receiver's left side are a live display — lit red from
//! the moment the gun is drawn; with grenade rounds 0 the pattern is red, red, dark, red; after
//! picking up 90 9mm rounds red, dark, red, red; in Grenade mode with 4 rounds all four lit. The
//! mode press only swaps when the grenade twin has ammo (with 0 the press did nothing; after the
//! 0x1b rounds the press swapped, HUD "4-0 Grenade").
//!
//! Code [M]: the lights are `AnimDatumSetEntity(vm, 8..0xB, 0x2000132, 0xff)` every tick while
//! 0x1a is held, cleared per gate — `Player_WeaponFiring` 0x1a5460:256-327: bone 8 blinks when the
//! idle counter BL+0x93a > 15 (on GameState+0x76 % 40's 20/40 duty), bone 9 dark when the raise
//! cooldown BL+0x93e has drained to 0 (0x1a5460:275-281), bone 10 dark when the 0x1b clip is dry
//! and blinking in grenade mode below 3 rounds on FlashMod > 0.75 (0x1a5460:282-294), bone 11 dark
//! when the 0x1a clip is dry and blinking in burst mode below 7 (0x1a5460:298-308), all four off
//! when 0x93a > 45 (0x1a5460:314). The counter's writer is `Player_SetWeaponAnimObj` 0x1a2e38:40-58:
//! it climbs only while the view anim is idle AND no analog input is deflected (`Input_Update`
//! 0x17ee88:54,80-84 sets DAT_002a3a1c per slot when any of the pad's analog inputs moves); any
//! non-idle view anim resets it and refills +0x93e to FRAME_RATE_INT·0x14 = 1200 [L ee_start_05
//! BL+0x93e = 1200]. The mode press is `Player_Weapon` 0x1a24c8:204-260: twin-linked records switch
//! to `w + twin` through `Player_WeaponHasAmmo` 0x1a16c0 — the 0x1a press needs the 0x1b slot, the
//! 0x1b press reads the 0x1a side back.

use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn load(iso: &str) -> nf_view::scene::Scene {
    nf_view::load_weapons(iso, "07000005.bin", false, &[0x1a, 0x1b]).expect("load 05 with the AIMS pair")
}

/// Equip the AIMS-20 the way its weapon pickup does (`Player_EquipWeapon` 0x1a8a58: owned, a full
/// clip) and let the draw machine settle.
fn draw_aims(scene: &mut nf_view::scene::Scene) {
    {
        let sim = scene.sim.as_mut().unwrap();
        let wd = sim.weapons.clone();
        let rounds = wd.clip_size(0x1a).max(1);
        if sim.drones.inventory.equip_weapon(&wd, 0x1a, rounds, true)
            || sim.drones.inventory.owned.get(0x1a).copied().unwrap_or(false) {
            sim.gun.switch_request = 0x1a;
        }
    }
    for _ in 0..200 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    assert_eq!(scene.sim.as_ref().unwrap().gun.weapon, 0x1a, "the AIMS-20 is drawn");
}

/// One walking tick (the window maps W to forward 1.0) — the deflected stick that keeps the idle
/// counter at 0 in play.
fn walk_tick(sim: &mut nf_view::scene::Sim) {
    let mut input = Input::default();
    input.forward = 1.0;
    sim.step_bond(input, false, false, false);
}

/// The idle counter crosses the 45-frame all-off gate during any no-input settle (0x1a5460:314);
/// the live patterns are judged after one stick tick, the way the game sees them in play.
fn wake(sim: &mut nf_view::scene::Sim) { walk_tick(sim); }

fn aims_light_rows(scene: &nf_view::scene::Scene) -> Vec<(u8, [[f32; 4]; 4])> {
    let vm = *scene.view_models.get(&0x1a).expect("the AIMS view model");
    let fs = scene.frame_state(None, 0.0, Default::default());
    scene.attachments.iter().zip(fs.attach.iter())
        .filter(|(a, _)| a.aims_light.is_some() && a.character as usize == vm)
        .map(|(a, m)| (a.aims_light.unwrap(), *m)).collect()
}

/// A collapsed matrix lost its rotation (the port collapses a hidden mesh to a point by zeroing
/// the 3×3, `Scene::frame_state`'s `collapse`).
fn collapsed(m: [[f32; 4]; 4]) -> bool { m[0][0] == 0.0 && m[1][1] == 0.0 }

fn lit_bones(scene: &nf_view::scene::Scene) -> Vec<u8> {
    aims_light_rows(scene).iter().filter(|(_, m)| !collapsed(*m)).map(|(b, _)| *b).collect()
}

/// The lights must survive walking: the game resets the idle counter on every deflected analog
/// input (`Input_Update` 0x17ee88 → `Player_SetWeaponAnimObj` 0x1a2e38:41-57), so in play the row
/// never crosses the 15/45-frame blink/off thresholds. Before the fix the port's counter climbed
/// while the stick moved, and the whole row went dark 0.75 s into any walk — the user's "lights on
/// the aim 20 arent on". Sister: dead-still idle for 60 ticks DOES kill the row (0x1a5460:314).
#[test]
fn the_side_lights_survive_stick_input_and_die_when_dead_still() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso);
    draw_aims(&mut scene);
    // The twin's rounds (the armoury's 0x1b pickup) so the row is judged complete.
    {
        let sim = scene.sim.as_mut().unwrap();
        let wd = sim.weapons.clone();
        sim.drones.inventory.equip_ammo(&wd, 0x1b, 4);
    }
    // Walk forward for 60 ticks through the real input.
    {
        let sim = scene.sim.as_mut().unwrap();
        for _ in 0..60 {
            walk_tick(sim);
            assert_eq!(sim.drones.aims_idle, 0, "a deflected stick resets the idle counter every tick [M 0x17ee88:80-84, 0x1a2e38:41-44]");
        }
    }
    let rows = aims_light_rows(&scene);
    assert_eq!(rows.len(), 4, "bones 8..0xB each carry one light [M 0x1a5460:259-266]");
    for (bone, m) in &rows {
        assert!(!collapsed(*m), "light bone {bone} stays lit while the stick moves (idle counter pinned at 0)");
    }
    // Sister: dead-still idle crosses the 45-frame gate and the row goes dark [M 0x1a5460:314].
    {
        let sim = scene.sim.as_mut().unwrap();
        for _ in 0..60 { sim.step_bond(Input::default(), false, false, false); }
        assert!(sim.drones.aims_idle > 45, "dead-still idle climbs the counter");
    }
    for (bone, m) in aims_light_rows(&scene) {
        assert!(collapsed(m), "light bone {bone} goes dark after 45 dead-still ticks [M 0x1a5460:314]");
    }
}

/// The lights are a display, not decoration (the ground truth): with the grenade slot dry the
/// third light is dark (bone 10, the 0x1b clip gate 0x1a5460:282-288) while 8/9/11 stay lit —
/// "red, red, dark, red"; after the 0x1b rounds land the row is complete; and bone 9 follows the
/// raise cooldown +0x93e (lit while the 1200-tick refill lasts, dark once it drains without a
/// view anim — the ground truth's "red, dark, red, red" state). Before the fix the port kept
/// 9/10/11 lit unconditionally.
#[test]
fn the_lights_show_the_ammo_pattern() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso);
    draw_aims(&mut scene);
    // Just drawn, grenade slot dry: red, red, dark, red.
    {
        let sim = scene.sim.as_mut().unwrap();
        wake(sim);
    }
    assert_eq!(lit_bones(&scene), vec![8, 9, 11], "grenade rounds 0: bone 10 dark, the rest lit (the ground-truth pattern)");
    // The 0x1b rounds arrive through the game's own equip path (the pickup's take).
    {
        let sim = scene.sim.as_mut().unwrap();
        let wd = sim.weapons.clone();
        sim.drones.inventory.equip_ammo(&wd, 0x1b, 4);
        for _ in 0..10 { sim.step_bond(Input::default(), false, false, false); }
        wake(sim);
    }
    assert_eq!(lit_bones(&scene), vec![8, 9, 10, 11], "grenade rounds in: all four lit (the ground-truth pattern)");
    // 1300 dead-still ticks drain +0x93e (1200 at the start [L ee_start_05]) without any view
    // anim to refill it; bone 9 goes dark while the ammo lights stay.
    {
        let sim = scene.sim.as_mut().unwrap();
        for _ in 0..1300 { sim.step_bond(Input::default(), false, false, false); }
        wake(sim);
    }
    assert_eq!(lit_bones(&scene), vec![8, 10, 11], "raise cooldown drained: bone 9 dark, the ammo lights lit (the ground-truth pattern)");
}

/// The mode press swaps like the game. ○ on the drawn AIMS-20 with the grenade twin's slot filled
/// requests 0x1b through the switch machine and completes, and the round trip returns; with the
/// slot dry the 0x1a-side press does nothing [M 0x1a24c8:246-260 + the ground truth "with no
/// grenade rounds the press did nothing"]. The give is the port's acquire affordance (the dev
/// menu's Weapons page): it must leave the pair in the state the game's armoury leaves — the
/// twin's rounds present. Before the fix the give filled only the given record's slot and the
/// twin stayed dry forever, so the press could never swap in a give-started session.
#[test]
fn the_mode_press_swaps_only_with_grenade_rounds() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso);
    // The dev give (the user's path): the panel's `do=weapon&id=0x1a` command.
    let (status, switch_to) = nf_view::panel::apply(&mut scene, &[("do".to_string(), "weapon".to_string()), ("id".to_string(), "0x1a".to_string())]);
    assert!(status.contains("gave"), "the give equips: {status}");
    {
        let sim = scene.sim.as_mut().unwrap();
        let sw = switch_to.expect("the give returns the equip switch");
        let mut input = Input::default();
        input.switch_to = Some(sw);
        sim.step_bond(input, false, false, false);
    }
    for _ in 0..200 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    assert_eq!(scene.sim.as_ref().unwrap().gun.weapon, 0x1a, "the AIMS-20 is drawn from the give");
    {
        let sim = scene.sim.as_ref().unwrap();
        let g = sim.drones.inventory.clip.get(0x1b).copied().unwrap_or(0);
        assert!(g > 0, "the give leaves the grenade twin's slot filled (the game's armoury state) — got {g}");
    }
    // The press with rounds: swaps to the grenade twin and completes.
    {
        let sim = scene.sim.as_mut().unwrap();
        let mut input = Input::default();
        input.mode_switch = true;
        sim.step_bond(input, false, false, false);
        assert_eq!(sim.gun.switch_request, 0x1b, "○ requests the grenade twin [M 0x1a24c8:246-260]");
    }
    for _ in 0..200 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    assert_eq!(scene.sim.as_ref().unwrap().gun.weapon, 0x1b, "the swap completes onto the grenade twin");
    // The press back: the 0x1b record's twin is −1, the burst side has rounds — the swap returns.
    {
        let sim = scene.sim.as_mut().unwrap();
        let mut input = Input::default();
        input.mode_switch = true;
        sim.step_bond(input, false, false, false);
        assert_eq!(sim.gun.switch_request, 0x1a, "○ on the twin requests the burst record back (0x1b + twin −1)");
    }
    for _ in 0..200 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    assert_eq!(scene.sim.as_ref().unwrap().gun.weapon, 0x1a, "the round trip lands on the burst record");
    // Drain the grenade magazine through the game's own fire path (the launcher throws while held),
    // then the 0x1a-side press must do nothing — the ground-truth gate.
    {
        let sim = scene.sim.as_mut().unwrap();
        // Swap to the grenade twin first: the drain spends ITS magazine.
        let mut input = Input::default();
        input.mode_switch = true;
        sim.step_bond(input, false, false, false);
        assert_eq!(sim.gun.switch_request, 0x1b, "the press swaps to the grenade twin for the drain");
    }
    for _ in 0..200 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    assert_eq!(scene.sim.as_ref().unwrap().gun.weapon, 0x1b, "the grenade twin is drawn for the drain");
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.bond.pitch = 0.9; // fire at the sky: the grenades land far from Bond (the fuse is 5 s)
        for _ in 0..12 {
            let g = sim.drones.inventory.clip.get(0x1b).copied().unwrap_or(0);
            if g == 0 { break; }
            sim.step_bond(Input::default(), true, false, false); // hold fire: the launcher spends a round
            for _ in 0..30 { sim.step_bond(Input::default(), false, false, false); }
            sim.drones.health.health = 100.0; // the playtest's god cheat (Dev::after_tick's exact write)
        }
        let g = sim.drones.inventory.clip.get(0x1b).copied().unwrap_or(0);
        assert_eq!(g, 0, "the launcher's fire path drained the grenade magazine");
        // The dry fire tick may trigger the game's no-ammo auto-switch (Player_HandleHasNoAmmo) —
        // settle it out; the gate check below needs the burst record in hand.
    }
    for _ in 0..200 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    {
        let sim = scene.sim.as_mut().unwrap();
        if sim.gun.weapon == 0x1b {
            // From the dry grenade side the press reads the BURST side back (its twin −1, rounds
            // there) — the twin rule is per-side [M 0x1a24c8:246-260].
            let mut input = Input::default();
            input.mode_switch = true;
            sim.step_bond(input, false, false, false);
            assert_eq!(sim.gun.switch_request, 0x1a, "the dry twin's press reads the burst side back (rounds there)");
        }
    }
    for _ in 0..200 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    {
        let sim = scene.sim.as_mut().unwrap();
        assert_eq!(sim.gun.weapon, 0x1a, "back on the burst record for the gate check");
        // THE ground-truth gate: holding the burst record with the grenade slot dry, the press does
        // nothing [M 0x1a24c8 — Player_WeaponHasAmmo 0x1a16c0 refuses the 0x1b side].
        let mut input = Input::default();
        input.mode_switch = true;
        sim.step_bond(input, false, false, false);
        assert_eq!(sim.gun.switch_request, 0x1a, "grenade slot dry: the press does nothing (the ground-truth gate)");
    }
    // Rounds back through the game's own equip path: the press swaps again.
    {
        let sim = scene.sim.as_mut().unwrap();
        let wd = sim.weapons.clone();
        sim.drones.inventory.equip_ammo(&wd, 0x1b, 4);
        let mut input = Input::default();
        input.mode_switch = true;
        sim.step_bond(input, false, false, false);
        assert_eq!(sim.gun.switch_request, 0x1b, "rounds back: the press swaps to the grenade twin");
    }
}
