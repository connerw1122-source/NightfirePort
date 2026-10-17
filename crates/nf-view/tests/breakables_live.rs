//! glass-shatter: the breakables (placement 0x20 → `Break_Create` 0x12c230) — a shot through a
//! real gun shatters the 07000008 control-room glass panes (`Break_Update` 0x12bfd8 subtracts the
//! bullet's damage from the pane's health; `Break_Kill` 0x12b970 plays the glass sound 0x2e for
//! kind 2 and leaves the pane gone), a second round through the same spot breaks nothing, and the
//! corridor's NON-breakable world glass (material 0x10, fix-rocket-glass-08) never breaks.
//! Opt-in: set `NF_ISO` to the disc image.

use nf_game::player::Input;

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

/// The 07000008 control-room panes' bounds centres (the runtime `Breakables::list` order —
/// placement-vec indices 170/176/177; the disc's `Glass_Controlroom*` map objects, kind 2,
/// health 0 [D placements + probe]): 170 = the x 105.25 side pane, 176 = the z −122.0 front
/// pane, 177 = the x 109.25 side pane.
const FRONT: usize = 176;
const SIDE: usize = 170;
const FRONT_C: [f32; 3] = [107.25008, 35.3124, -121.99846];
const SIDE_C: [f32; 3] = [105.25008, 35.3124, -121.05769];

fn count_2e(sim: &nf_view::scene::Sim) -> usize {
    sim.drones.audio.live_ids().iter().filter(|&&id| id == 0x2e).count()
}

fn broken(sim: &nf_view::scene::Sim) -> Vec<usize> {
    sim.drones.breakables.list.iter().filter(|b| b.broken).map(|b| b.placement).collect()
}

/// Walks the aim onto `at` from the eye and taps the trigger (the solar-panels walker: turn/look
/// inputs, fire through the real gun path, reload when the clip runs dry).
fn fire_at(scene: &mut nf_view::scene::Scene, at: [f32; 3], ticks: u32) -> bool {
    let mut last_tap = 0u32;
    let mut fired = false;
    for t in 0..ticks {
        let (input, trigger, reload) = {
            let sim = scene.sim.as_ref().unwrap();
            let (bp, dp) = (sim.bond.pos, at);
            let eye = [bp[0], bp[1] + sim.bond.eye - sim.bond.crouch_drop, bp[2]];
            let dx = dp[0] - eye[0];
            let dy = dp[1] - eye[1];
            let dz = dp[2] - eye[2];
            let dxz = (dx * dx + dz * dz).sqrt().max(0.5);
            let desired = dx.atan2(dz);
            let err = { let mut e = desired - sim.bond.yaw; while e > std::f32::consts::PI { e -= 2.0 * std::f32::consts::PI; } while e < -std::f32::consts::PI { e += 2.0 * std::f32::consts::PI; } e };
            let want_pitch = (dy / dxz).atan() / std::f32::consts::FRAC_PI_2;
            let perr = want_pitch - sim.bond.pitch;
            let clip_empty = sim.gun.clip == 0 && sim.gun.spare > 0;
            let on_target = err.abs() < 0.02 && perr.abs() < 0.028 && t.saturating_sub(last_tap) > 15;
            (Input { turn: err.clamp(-0.08, 0.08), look: perr.clamp(-0.08, 0.08), ..Default::default() },
             on_target && !clip_empty,
             clip_empty && t % 20 == 0)
        };
        if trigger { last_tap = t; fired = true; }
        scene.sim.as_mut().unwrap().step_bond(input, trigger, reload, false);
        let sim = scene.sim.as_ref().unwrap();
        if fired && t.saturating_sub(last_tap) > 40 { break; }
    }
    fired
}

/// A shot from inside the control room shatters the front pane: the pane is gone, the glass
/// sound 0x2e plays, a second round through the same spot breaks nothing, and the neighbouring
/// pane still shatters after that (the sister: the system is alive, the first pane's silence is
/// its death). [M 0x12bfd8:36-51, 0x12b970:60-76; D 07000008 placements]
#[test]
fn a_shot_shatters_the_controlroom_pane_and_a_second_bullet_passes() {
    unsafe { std::env::set_var("NF_SILENT", "1"); }
    let iso = iso();
    let mut scene = nf_view::load_weapons(&iso, "07000008.bin", false, &[0x00]).expect("load 08");
    {
        let sim = scene.sim.as_mut().unwrap();
        for _ in 0..120 { sim.step_bond(Input::default(), false, false, false); }
        // Bond stands 3.5 units off the front pane with a clear ray (probe: no world surface
        // between the eye and the pane plane).
        sim.bond = nf_game::player::Bond::stand_at(&sim.world, [107.25003, 35.3124, -118.49846], 0.0);
        for _ in 0..30 { sim.step_bond(Input::default(), false, false, false); }
        assert_eq!(broken(sim), Vec::<usize>::new(), "the panes start whole");
    }
    assert!(fire_at(&mut scene, FRONT_C, 400), "a shot was fired");
    for _ in 0..30 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    let sim = scene.sim.as_ref().unwrap();
    assert_eq!(broken(sim), vec![FRONT], "the front pane shatters on the first round (health 0 [D], damage > 0 [M 0x12bfd8:36-37])");
    let after_first = count_2e(sim);
    assert!(after_first >= 1, "Break_Kill's glass sound 0x2e plays [M 0x12b970:65-76], got {after_first}");

    // The second round through the same spot: the dead pane registers no hit [M 0x12b970:60] —
    // nothing new breaks, no second glass sound node.
    assert!(fire_at(&mut scene, FRONT_C, 400), "a second shot was fired");
    for _ in 0..30 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    let sim = scene.sim.as_ref().unwrap();
    assert_eq!(broken(sim), vec![FRONT], "the second bullet passes where the pane was — nothing left to break");
    assert!(count_2e(sim) <= after_first, "no second 0x2e play for the gone pane");

    // The sister: the SIDE pane is still whole and still shatters.
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.bond = nf_game::player::Bond::stand_at(&sim.world, [107.25008, 34.60022, -121.05769], 0.0);
        for _ in 0..30 { sim.step_bond(Input::default(), false, false, false); }
    }
    assert!(fire_at(&mut scene, SIDE_C, 400), "a shot at the side pane was fired");
    for _ in 0..30 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    let sim = scene.sim.as_ref().unwrap();
    let got = broken(sim);
    assert!(got.contains(&SIDE), "the neighbouring pane still shatters after the first died, got {got:?}");
    assert!(!got.contains(&FRONT), "the first pane stays dead");
    assert!(count_2e(sim) > after_first, "the side pane's break plays its own 0x2e");
}

/// The corridor window (the fix-rocket-glass-08 pane, world surface byte 0x10) is NOT a
/// breakable: a bullet fired from the level start pierces it (impact past the window plane) and
/// no breakable in the level breaks. [D 07000008 collision; M 0x12cf44..0x12cf64]
#[test]
fn the_corridor_window_is_not_a_breakable() {
    unsafe { std::env::set_var("NF_SILENT", "1"); }
    let iso = iso();
    let mut scene = nf_view::load_weapons(&iso, "07000008.bin", false, &[0x00]).expect("load 08");
    {
        let sim = scene.sim.as_mut().unwrap();
        for _ in 0..150 { sim.step_bond(Input::default(), false, false, false); }
    }
    // Fire straight ahead from the level start (the default facing — the rocket-glass-08 aim):
    // settle, tap once through the real path, fly the round out.
    let mut fired = false;
    for t in 0..200 {
        let trigger = t == 100;
        scene.sim.as_mut().unwrap().step_bond(Input::default(), trigger, false, false);
        if trigger { fired = true; }
    }
    assert!(fired, "a corridor shot was fired");
    for _ in 0..30 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    let sim = scene.sim.as_ref().unwrap();
    let impact = sim.drones.player_impacts.iter().find_map(|i| match i {
        nf_game::drone::PlayerImpact::World(p) => Some(*p),
        _ => None,
    });
    let impact = impact.expect("the corridor bullet's world impact was recorded");
    assert!(impact[2] < -98.5, "the bullet pierced the non-breakable glass and stopped past it (the wall face z −100.0 [D 07000008 collision]), got {:?}", impact);
    assert_eq!(broken(sim), Vec::<usize>::new(), "no breakable broke — the corridor glass is world geometry, not a 0x20 pane");
}
