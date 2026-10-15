//! fix-sg5-casings — the SG5 Commando's ejected shell casings through the real path
//! (`nf_view::load` + `Sim::step_bond`, the `Input` the window builds).
//!
//! The playtest 1014-1725 (user 05, walking forward + firing the SG5): "I could see the bullet
//! casings flying across my screen, like super large". The PCSX2 ground truth
//! (docs/live/2026-10-14-weapon-measurements-pcsx2.md, "The SG5 Commando 0x16 casings while walking
//! forward"): the casings are **small brass shells** ejected from the right side of the receiver,
//! flying **up and to the LEFT, away from the camera**, gone off the top-left of the view within
//! ~2 frames; walking forward does not bring them back across the screen and they never get large.
//!
//! Cause [M `Bullet_init_casing_ex` 0x12f678:160]: the game re-positions the casing to the weapon
//! datum's bone world position (`AnimGetBoneWorldTrans(shooter, datum)`'s VECTOR out,
//! `Vec_Copy2(bone_pos, obj+0x30, obj+0x40)`) — the view-model gun, on the record's +0xE0
//! first-person offset ([L] ee_start_05 @0x2bf150+0x16·0x10C: SG5 (0.028, 0.203, −0.244), the P2K
//! (0.028, 0.075, −0.903)). The port ejected at `camera` — the EYE (scene.rs passed the fire
//! origin) — so the shell drew at distance ≈ 0 and filled the frame, and the forward walk swept
//! the camera straight through it.

use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// The viewer-left / up basis of the eye's aim (`Player_PositionGun` 0x1a3b40's gl = up×gf,
/// gu = gf×gl — the same construction `cam_view_to_world` uses, lib.rs).
fn view_basis(fwd: [f32; 3]) -> ([f32; 3], [f32; 3]) {
    let n = |v: [f32; 3]| { let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-9); v.map(|x| x / l) };
    let f = n(fwd);
    let l = n([f[2], 0.0, -f[0]]); // Y × f
    let u = [f[1] * l[2] - f[2] * l[1], f[2] * l[0] - f[0] * l[2], f[0] * l[1] - f[1] * l[0]]; // f × l
    (l, u)
}

fn give_sg5(scene: &mut nf_view::scene::Scene) {
    for _ in 0..4 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    // The user's path: the dev give (`do=weapon&id=0x16`), then the equip switch it returns.
    let (status, switch_to) = nf_view::panel::apply(scene, &[("do".to_string(), "weapon".to_string()), ("id".to_string(), "0x16".to_string())]);
    assert!(status.contains("gave"), "the give equips: {status}");
    {
        let sim = scene.sim.as_mut().unwrap();
        let mut input = Input::default();
        input.switch_to = Some(switch_to.expect("the give returns the equip switch"));
        sim.step_bond(input, false, false, false);
    }
    for _ in 0..200 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    assert_eq!(scene.sim.as_ref().unwrap().gun.weapon, 0x16, "the SG5 is drawn from the give");
}

/// Walking forward while firing the SG5 (the user's scenario): the shell ejects at the VIEW-MODEL
/// GUN — a pre-eject distance from the eye on the record's +0xE0 scale (~0.25 for the SG5), not
/// ≈ 0 (the eye itself, the "super large" frames) — and drifts up and to the LEFT in view space
/// [L the PCSX2 measurement]. Sister: through the whole walked burst no shell ever comes back to
/// the eye (the camera never overtakes one — "they never get large").
#[test]
fn the_sg5_shell_ejects_at_the_gun_not_the_eye() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005");
    give_sg5(&mut scene);

    // Walk forward + hold fire, the user's repro. Record each slot's FIRST flying frame.
    let mut ejects: Vec<(usize, f32, [f32; 3])> = Vec::new(); // (slot, eye distance, vel)
    let mut seen: std::collections::HashSet<usize> = Default::default();
    let mut min_seen = f32::MAX;
    for _ in 0..40 {
        let (eye, fwd) = {
            let s = scene.sim.as_ref().unwrap();
            (s.bond.head(), s.bond.look())
        };
        scene.sim.as_mut().unwrap().step_bond(Input { forward: 1.0, ..Default::default() }, true, false, false);
        let s = scene.sim.as_ref().unwrap();
        for (k, c) in s.drones.casings.list.iter().enumerate() {
            if c.state != 0 { continue; }
            let d = ((c.pos[0] - eye[0]).powi(2) + (c.pos[1] - eye[1]).powi(2) + (c.pos[2] - eye[2]).powi(2)).sqrt();
            min_seen = min_seen.min(d);
            if seen.insert(k) && ejects.len() < 4 {
                ejects.push((k, d, c.vel));
                eprintln!("eject slot {k}: eye-dist {d:.3} pos {:?} vel {:?}", c.pos, c.vel);
            }
        }
    }
    assert!(!ejects.is_empty(), "the walked SG5 burst ejected no shell (the test's premise)");
    let (l, u) = view_basis(scene.sim.as_ref().unwrap().bond.look());
    for (i, (slot, d, vel)) in ejects.iter().enumerate() {
        assert!(*d >= 0.15, "shell slot {slot} ejected at eye distance {d:.4} — the game ejects at the view-model gun bone (Bullet_init_casing_ex 0x12f678 Vec_Copy2; the SG5's +0xE0 gun sits ~0.25 ahead [L]), at the eye it draws 'super large'");
        let vl = vel[0] * l[0] + vel[1] * l[1] + vel[2] * l[2];
        let vu = vel[0] * u[0] + vel[1] * u[1] + vel[2] * u[2];
        assert!(vl > 0.0 && vu > 0.0, "shell slot {slot} eject velocity view-left {vl:.4} / up {vu:.4} — the measured shells fly up and to the LEFT [L docs/live/2026-10-14]");
    }
    // Sister: walking forward never brings a shell back to the eye ("they never get large").
    assert!(min_seen >= 0.1, "a shell came within {min_seen:.4} of the eye during the walked burst — the camera overtook it (the user's 'flying across my screen')");
}

/// The SG5 (ammo 5) ejects its own shell mesh 0x02000447 [L ammo-data.md, the +4 casing id].
#[test]
fn the_sg5_ejects_its_own_mesh() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005");
    give_sg5(&mut scene);
    scene.sim.as_mut().unwrap().step_bond(Input::default(), true, false, false);
    let s = scene.sim.as_ref().unwrap();
    let mesh = s.drones.casings.list.iter().filter(|c| c.state != 2).map(|c| c.mesh).collect::<Vec<_>>();
    assert!(mesh.contains(&0x0200_0447), "the SG5 (ammo 5) ejects mesh 0x02000447 [L ammo-data.md], got {mesh:?}");
}
