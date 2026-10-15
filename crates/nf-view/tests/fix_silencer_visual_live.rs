//! fix-silencer-visual: the silenced twins' silencer MESH on the view model (task 109m).
//!
//! User (playtest 1014-1725, level 05): "The suppressor still stays on the gun, but visually
//! it's not there" — the mode press lands the silenced twin (its dead flash and own sound are
//! right, fix-weapon-sounds / fire_modes3), but the port never draws the silencer tube on the gun.
//!
//! The game's attachment, decoded [M]: while the HELD record's flags (+0x68) carry 0x800,
//! `Player_WeaponFiring` 0x1a5460:96-110 attaches the mesh at `weapon_data+0x7C` to DATUM 0 of the
//! weapon AnimObj (BL+0x7e8) every tick — `AnimDatumSetEntity(vm, 0, weapon+0x7C, 0xff)`, skipped
//! (and cleared) in view-anim state 8, the swap/holster anim; the twin arm (0x1a5460:111-118)
//! clears datum 0 again while the bare record is held. `AnimObjectDraw` 0x1c4490 draws a datum's
//! entity at `QuatTransToMat(datum.q, datum.t) · bone world` (docs/formats/weapon-data.md
//! "Drawing"). The records [L the golden weapon table 2fd48d6d, the ACTION.ELF static initialiser
//! 0x1b3488]: the silenced twins 0x03/0x05/0x07/0x09 carry flags 0x1884/0x188c — bit 0x800 — over
//! the bare 0x02/0x04/0x06/0x08's 0x1004/0x100c; each pair shares +0x7C (the P2K's is 0x02000842)
//! and the fp skin (the P2K's 0x05000088). Level 07000005 carries 0x02000842 natively [D the disc
//! census]. This fills session 102b's open decode ("the game's persistent mount is a draw-side
//! gate this session did not decode" — fix_silencer_modes_live.rs).
//!
//! The port built the twin's view-model character but never attached +0x7C: the fix hangs the mesh
//! on the silenced record's own view-model character (datum 0's bone, the datum's q/t), which draws
//! only while that record is the held weapon — the game's per-tick gate.
//!
//! Opt-in via NF_ISO (the slim ISO is enough).

use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn load(iso: &str) -> nf_view::scene::Scene {
    // The P2K pair's assets merge like the dev give does; 05 carries them natively either way.
    nf_view::load_with_carry_weapons(iso, "07000005.bin", false, None, &[2, 3]).expect("load 07000005")
}

fn settle(scene: &mut nf_view::scene::Scene) {
    for _ in 0..600 { if !scene.sim.as_ref().unwrap().disabled { break; } scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    for _ in 0..30 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    assert!(!scene.sim.as_ref().unwrap().disabled, "the intro never released Bond");
}

/// Presses the mode button (the real B press) with the machine idle, up to 3 attempts, and reports
/// the equipped weapon after — fire_modes3's harness verbatim.
fn mode_press(scene: &mut nf_view::scene::Scene) -> u8 {
    for _ in 0..3 {
        for _ in 0..40 {
            let sim = scene.sim.as_ref().unwrap();
            if sim.gun.switch_state == 0 && sim.gun.reload_state == 0 && !sim.gun.fire_state { break; }
            scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
        }
        scene.sim.as_mut().unwrap().step_bond(Input { mode_switch: true, ..Default::default() }, false, false, false);
        // Wait out the variant swap (the requested record's +0xC8 at 1.3 [M `Player_WeaponSelect`
        // 0x1a2cc0]) before judging it: a re-press while it still plays would toggle straight back.
        for _ in 0..200 {
            scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
            let sim = scene.sim.as_ref().unwrap();
            if sim.gun.switch_state == 0 && !sim.gun.fire_state { break; }
        }
        let w = scene.sim.as_ref().unwrap().gun.weapon;
        let idle = { let sim = scene.sim.as_ref().unwrap(); sim.gun.switch_state == 0 && !sim.gun.fire_state };
        if idle { return w; }
    }
    scene.sim.as_ref().unwrap().gun.weapon
}

/// Plain rigid attachments of a view-model character, as CONTENT (bone, local) — no sim-driven
/// kind (the skin's `extra` parts are exactly this set; the muzzle flash, AIMS lights, tracers and
/// friends all carry a kind). Content, not indices: the twins are separate characters, so their
/// attachment indices never overlap.
fn rigid_content(scene: &nf_view::scene::Scene, vm: usize) -> Vec<(usize, [[f32; 4]; 4])> {
    let mut v: Vec<(usize, [[f32; 4]; 4])> = scene.attachments.iter().filter(|a| {
        a.character as usize == vm
            && a.flash.is_none() && a.tracer.is_none() && a.projectile.is_none()
            && !a.dropped && a.script.is_none() && a.aims_light.is_none() && a.gas.is_none() && a.effect.is_none()
    }).map(|a| (a.bone, a.local)).collect();
    v.sort_by_key(|(b, _)| *b);
    v
}

/// A collapsed matrix lost its rotation (the port hides a mesh by zeroing the 3×3, `collapse`).
fn collapsed(m: [[f32; 4]; 4]) -> bool { m[0][0] == 0.0 && m[1][1] == 0.0 }

/// The silencer must be ON the silenced twin and NOWHERE else: the silenced record's view model
/// gains exactly one rigid attachment over the bare record's (the same fp skin means the same
/// `extra` parts), hung on DATUM 0's bone at the datum's q/t — the game's per-tick
/// `AnimDatumSetEntity(vm, 0, weapon+0x7C, 0xff)` gate [M 0x1a5460:96-110] — and it draws (its
/// matrix keeps its rotation) while the silenced twin is the held weapon.
#[test]
fn the_silenced_twin_draws_the_silencer_mesh_on_the_view_model() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso);
    settle(&mut scene);
    let sim = scene.sim.as_ref().unwrap();
    assert_eq!(sim.gun.weapon, 2, "Bond starts on the bare P2K [L the 05 spawn]");
    let (vm2, vm3) = (*scene.view_models.get(&2).expect("the P2K view model"),
                      *scene.view_models.get(&3).expect("the silenced twin's view model"));
    let dt0 = scene.characters[vm3].skin.datums.iter().find(|d| d.id == 0 && d.bone >= 0)
        .expect("the P2K fp skin has a datum 0 (the muzzle datum the flash uses)").clone();
    let bare = rigid_content(&scene, vm2);

    // The REAL mode press lands the silenced twin (the twin's own has-ammo gate [M 0x1a24c8:210]).
    let w = mode_press(&mut scene);
    assert_eq!(w, 0x03, "the mode press landed the silenced twin 0x03 (twin +0x05 = 1 [L])");

    // The silenced twin's view model carries exactly ONE rigid attachment over the bare record's
    // set — the silencer (0x1a5460:96-110). FAILING-FIRST: on the pre-fix master this is 0 — the
    // twins share the fp skin 0x05000088 [L +0xDC], so the twin's set starts identical to the
    // bare record's and nothing new appears.
    let extra: Vec<(usize, [[f32; 4]; 4])> = rigid_content(&scene, vm3).iter()
        .filter(|(b, l)| !bare.iter().any(|(b2, l2)| b2 == b && l2 == l)).copied().collect();
    assert_eq!(extra.len(), 1,
        "the silenced twin's view model carries exactly ONE attachment over the bare record's set — the silencer (0x1a5460:96-110)");
    assert_eq!(extra[0].0, dt0.bone as usize,
        "the silencer hangs on DATUM 0's bone [M AnimDatumSetEntity(vm, 0, ·) 0x1a5460:107]");
    assert_eq!(extra[0].1, nf_formats::anim::quat_trans_to_mat(dt0.q, dt0.t),
        "the silencer's local is the datum's q/t [M AnimObjectDraw 0x1c4490's entity draw]");
    // The bare record stays bare — the game's twin-clear arm [M 0x1a5460:111-118].
    assert_eq!(rigid_content(&scene, vm2), bare, "the bare P2K's view model gains nothing");

    // While the silenced twin is the held weapon the silencer draws with the gun (first-person):
    // its palette matrix keeps its rotation — not collapsed to a point.
    let idx = scene.attachments.iter().position(|a| a.character as usize == vm3
        && a.bone == dt0.bone as usize && a.local == extra[0].1).expect("the silencer attachment");
    let fs = scene.frame_state(None, 0.0, Default::default());
    assert!(!collapsed(fs.attach[idx]),
        "the silencer draws with the held silenced twin (the palette matrix keeps its rotation)");
}

/// Sister: cycling back to the bare P2K restores the bare set (the twin-clear arm) and the
/// silencer never joins the bare record's view model.
#[test]
fn the_bare_p2k_stays_bare_through_a_round_trip() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso);
    settle(&mut scene);
    let vm2 = *scene.view_models.get(&2).expect("the P2K view model");
    let bare = rigid_content(&scene, vm2);
    assert_eq!(scene.sim.as_ref().unwrap().gun.weapon, 2, "Bond starts on the bare P2K");

    // Round trip through the REAL mode presses: to the silenced twin and back.
    let w = mode_press(&mut scene);
    assert_eq!(w, 0x03, "the first press lands the silenced twin");
    let w = mode_press(&mut scene);
    assert_eq!(w, 0x02, "the second press returns to the bare P2K");

    assert_eq!(rigid_content(&scene, vm2), bare,
        "the bare P2K's view model is unchanged after the round trip [M the twin-clear 0x1a5460:111-118]");
}
