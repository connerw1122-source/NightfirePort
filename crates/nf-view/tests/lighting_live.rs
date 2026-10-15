//! fix-lighting / fix-lighting2: the muzzle-flash light (`Player_MuzzleFlash` 0x1a4298 →
//! `Light_Create` 0x1cc3d8) in the simulation's `LightList`, its one-tick lifetime, the smoke
//! draw, and — fix-lighting2 — the per-draw light selection: the world draw nearest the muzzle
//! must select the muzzle light (the kind-0 ambient radiators used to crowd it out through the
//! hardcoded 0x80), and the VU1 colour units (the byte-scale add over an Unorm8x4 vertex colour,
//! clamped to byte 255 by the microcode's MINIi). Opt-in: NF_ISO.

use nf_view::scene::ViewOpts;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn rng_of(r: &nf_game::drone::Rng) -> nf_game::drone::Rng { nf_game::drone::Rng { x: r.x, y: r.y, ..Default::default() } }

#[test]
fn muzzle_light_lives_one_tick_with_the_weapon_values() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load");
    for _ in 0..30 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    // Sister: no shot, no light.
    assert!(!scene.sim.as_ref().unwrap().drones.lights.lights.iter().any(|l| l.kind == 1), "no light without a shot");
    let rng_before = rng_of(&scene.sim.as_ref().unwrap().drones.rng);
    scene.sim.as_mut().unwrap().step_bond(Default::default(), true, false, false);
    {
        let sim = scene.sim.as_ref().unwrap();
        assert!(sim.shot_tick && sim.last_shot.is_some(), "the P2K fired this tick");
        // The shot's draws ride the shared RNG: Light_Create's `Rand_Rand(0)` and Gas_Init's
        // `Rand_FRand(2π)` sit between the accuracy draws and the bullet's spread draws (the
        // game's order; the exact draw sequence is unit-tested in nf-game's light_selection).
        let rng_after = rng_of(&sim.drones.rng);
        assert!((rng_after.x, rng_after.y) != (rng_before.x, rng_before.y), "the shot drew from the shared RNG");
        // The record: the weapon's values [M 0x1a4370-0x1a43a8].
        let l = sim.drones.lights.lights.iter().find(|l| l.kind == 1).expect("the muzzle light");
        let rec = &sim.weapons.records[sim.gun.weapon as usize];
        let radius = f32::from_le_bytes([rec[0x58], rec[0x59], rec[0x5a], rec[0x5b]]);
        assert!((l.radius - radius).abs() < 1e-6, "radius = weapon_data+0x58");
        assert!((l.intensity - 2.0).abs() < 1e-6, "intensity 2.0");
        assert_eq!(l.rgb, [rec[0x56], rec[0x55], rec[0x54]], "rgb = +0x56/+0x55/+0x54");
        assert_eq!(l.life, 1, "life arg 0 → +0x20 = 1 (Light_Update deletes it next tick)");
        assert!(l.active, "created active (+0x48 = 1)");
        // The light sits at the muzzle, near the eye (the sim's eye + aim·0.5 stand-in [S]).
        let eye = sim.bond.head();
        let d = ((l.pos[0] - eye[0]).powi(2) + (l.pos[1] - eye[1]).powi(2) + (l.pos[2] - eye[2]).powi(2)).sqrt();
        assert!(d < 2.0, "the light sits at the gun's muzzle, near the eye: {d}");
        // The frame's resolved view carries it for the renderer.
        let view = scene.frame_lights();
        assert!(view.lights.iter().any(|v| (v.pos[0] - l.pos[0]).abs() < 1e-4 && v.inv_r2 == 1.0 / (radius * radius)),
            "frame_lights resolves the muzzle light for the draws");
    }
    // One tick later it is gone (Light_Update deletes a life-1 light) and the shot flag is clear.
    scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
    assert!(!scene.sim.as_ref().unwrap().drones.lights.lights.iter().any(|l| l.kind == 1), "the light lasts one tick");
    let _ = ViewOpts::default();
}

/// A smoke puff appeared with the shot and rides the same RNG order.
#[test]
fn the_shot_leaves_a_smoke_puff() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load");
    for _ in 0..30 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    scene.sim.as_mut().unwrap().step_bond(Default::default(), true, false, false);
    // The puff lives in models-misc's gas pool (the merge drew it there; fix-lighting2's stand-in list is unused).
    assert!(!scene.sim.as_ref().unwrap().drones.gas.puffs.is_empty(), "Gas_Init made a smoke puff (the P2K has no 0x2000 flag at weapon_data+0x68)");
    // Sister: no shot, no puff.
    let mut calm = nf_view::load(&iso, "07000005.bin", false).expect("load");
    for _ in 0..31 { calm.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    assert!(calm.sim.as_ref().unwrap().drones.gas.puffs.is_empty(), "no smoke without a shot");
}

/// fix-lighting2, the fail-first: at the P2K shot on 07000005 the world draw nearest the muzzle
/// must select the muzzle light. The branch as it stood passed 0x80 for every draw, so the level's
/// permanent kind-0 ambient radiators (`Light_SetAmbientRadiators` 0x1caeb8 — 07000005 has 23, four
/// of them within 10 units of the player start [D probe]) qualified on every draw and filled both
/// slots — the user's approved muzzle glow was gone. The real gate is the source object's glist
/// flags word (`Control_SetGList` 0x134960 → obj+0xF0; cels: cel+0x94): 07000005's world cels carry
/// NO 0x80 [D probe], so the radiators never light world geometry and the kind-1 muzzle light (only
/// gated on !0x100, `Lights_CalcClosestLights` 0x1cb2e0) wins.
#[test]
fn the_world_draw_nearest_the_muzzle_selects_it() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load");
    for _ in 0..30 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    scene.sim.as_mut().unwrap().step_bond(Default::default(), true, false, false);
    let view = scene.frame_lights();
    let (mi, muzzle) = view.lights.iter().enumerate()
        .find(|(_, l)| l.kind == 1 && l.active).expect("the muzzle light is in the frame's resolved list");
    // The nearest World draw to the muzzle light (the renderer's per-draw sphere).
    let (m, d2m) = scene.materials.iter().filter(|m| m.layer == nf_view::scene::Layer::World)
        .map(|m| (m, dist2(m.centre, muzzle.pos))).min_by(|a, b| a.1.total_cmp(&b.1)).unwrap();
    assert!(m.lflags & 0x80 == 0, "the nearest world draw's glist flags carry no 0x80 [D probe], got 0x{:04x}", m.lflags);
    assert!(m.lflags & 0x100 == 0, "the muzzle light (kind 1) is not gated off it");
    let (lp, _lc) = view.pick2([m.centre[0], m.centre[1], m.centre[2], m.radius], m.lflags as u32);
    let d = ((lp[0][0] - muzzle.pos[0]).powi(2) + (lp[0][1] - muzzle.pos[1]).powi(2) + (lp[0][2] - muzzle.pos[2]).powi(2)).sqrt();
    assert!(d < 1e-4, "the draw's slot-0 light is the muzzle light (dist {d})");
    assert_eq!(lp[1], [0.0; 4], "no second light qualifies: with the real flags the radiators are gated off");
    // The pre-fix behaviour, kept as the bug's permanent witness: with the hardcoded 0x80 the same
    // draw selects a kind-0 radiator instead of the muzzle (this is what the user's play lost).
    let (old_lp, _) = view.pick2([m.centre[0], m.centre[1], m.centre[2], m.radius], 0x80);
    let old_is_muzzle = ((old_lp[0][0] - muzzle.pos[0]).powi(2) + (old_lp[0][1] - muzzle.pos[1]).powi(2) + (old_lp[0][2] - muzzle.pos[2]).powi(2)).sqrt() < 1e-4;
    assert!(!old_is_muzzle, "with the old hardcoded 0x80 the muzzle was crowded out — the regression is gone");
    let _ = (mi, d2m);
}

/// The VU1's colour units, pinned to the formula's numbers: the add is the byte-scale
/// `TColor.rgb · term` over an Unorm8x4 (byte/255) vertex colour, and the sum clamps at byte 255
/// (the MINIi with I = 255.0 at .vutext I-mem 919/954). The P2K's muzzle light values are the
/// disc's [D probe: weapon_data+0x58 = 7.0, +0x54..56 = (0,138,255), intensity 2.0]. Before
/// fix-lighting2 the shader divided by 128 over the byte/255 colour — 255/128 ≈ 2× the game's
/// add — and never clamped, so the view model went solid and the parapet doubled its glow.
#[test]
fn the_shaded_colour_matches_the_vu1_byte_scale_and_clamps_at_255() {
    // TColor = rgb · intensity = (0, 276, 510); TColor.w = 1/r² = 1/49 [M psiLight_SetLights 0x10e588].
    let lc = [[0.0, 276.0, 510.0, 1.0 / 49.0], [0.0; 4]];
    let lp = [[3.0, 0.0, 0.0, 1.0], [0.0; 4]];
    // Mid-range vertex at the origin, normal +x (facing the light): L = (3,0,0), d² = 9,
    // atten = 1/9 − 1/49 = 40/441, N·L = 3 → term = 120/441; add = (0, 276·120/441, 510·120/441).
    let got = nf_game::light::shade_vertex([0.0; 3], [1.0, 0.0, 0.0], [0.0, 0.0, 0.0], &lp, &lc);
    let term = (1.0 / 9.0 - 1.0 / 49.0) * 3.0;
    assert!((got[1] - 276.0 * term).abs() < 1e-3, "green = TColor_g · atten · N·L, got {}", got[1]);
    assert!((got[2] - 510.0 * term).abs() < 1e-3, "blue = TColor_b · atten · N·L, got {}", got[2]);
    assert_eq!(got[0], 0.0, "red adds nothing (TColor_r = 0)");
    // Close-range vertex 0.5 from the muzzle: atten = 4 − 1/49, term = 97.5/49 ≈ 1.99 — the raw add
    // (0, 549, 1015) blows past byte 255; the game's MINIi clamps the sum to 255.
    let lp = [[0.5, 0.0, 0.0, 1.0], [0.0; 4]];
    let got = nf_game::light::shade_vertex([0.0; 3], [1.0, 0.0, 0.0], [0.0, 0.0, 0.0], &lp, &lc);
    assert_eq!(got, [0.0, 255.0, 255.0], "the summed colour clamps at byte 255 [M .vutext 919/954]");
    // A dark vertex with the light still clamps the lit channels at the sum; red adds nothing
    // (TColor_r = 0) and keeps its base.
    let got = nf_game::light::shade_vertex([10.0, 20.0, 30.0], [1.0, 0.0, 0.0], [0.0, 0.0, 0.0], &lp, &lc);
    assert_eq!(got, [10.0, 255.0, 255.0], "the per-channel MINIi clamps the lit channels at 255");
    // The old formula's numbers still hold under the clamp (base 10 + 255·0.75 = 201.25 < 255).
    let lc = [[255.0, 255.0, 255.0, 0.25], [0.0; 4]];
    let lp = [[2.0, 0.0, 0.0, 1.0], [0.0; 4]];
    let got = nf_game::light::shade_vertex([10.0, 20.0, 30.0], [1.0, 0.0, 0.0], [1.0, 0.0, 0.0], &lp, &lc);
    assert!((got[0] - (10.0 + 255.0 * 0.75)).abs() < 1e-3, "d = r/2 → 0.75·255, unclamped");
}

/// The characters keep the level radiators: the guard/weapon skins' glist flags carry 0x80
/// (0x8a0 / 0x28a0 [D probe]) and no 0x100, so kinds 0 and 1 both apply — the user's "lights fall
/// on characters". The effect objects (bullet holes, sparks: 0x0321 [D]) carry 0x100 and are
/// muzzle-excluded, like the game's own emitter rule.
#[test]
fn character_skins_take_radiators_and_the_muzzle_world_cels_do_not() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let scene = nf_view::load(&iso, "07000005.bin", false).expect("load");
    // A character-layer draw sourced from a guard skin's mesh carries 0x80; a world static does not.
    let char_80 = scene.materials.iter()
        .filter(|m| m.layer == nf_view::scene::Layer::Character && m.lflags & 0x80 != 0).count();
    assert!(char_80 > 0, "the character-layer draws (guards/weapons, glist flags 0x8a0/0x28a0) carry 0x80");
    let world_80 = scene.materials.iter()
        .filter(|m| m.layer == nf_view::scene::Layer::World && m.lflags & 0x80 != 0).count();
    let world_all = scene.materials.iter().filter(|m| m.layer == nf_view::scene::Layer::World).count();
    assert!(world_80 * 20 < world_all, "07000005's world cels are near-all 0x80-free [D probe: 131 of 132 cel placements] — the radiators never light them; got {world_80} of {world_all}");
    // And some effect/world draws carry 0x100 (the muzzle exclusion), like the game's 0x0321 effects.
    let excl = scene.materials.iter().filter(|m| m.lflags & 0x100 != 0).count();
    assert!(excl > 0, "0x100 (muzzle-excluded) draws exist, as on the disc data");
}

fn dist2(a: [f32; 3], b: [f32; 4]) -> f32 { (a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2) }

