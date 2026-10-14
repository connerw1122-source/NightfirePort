//! fix-qclaw: the Q-Claw's real ids and fire path through the real input path.
//!
//! The playtest (results 1013-1758, 07000005 at `--bond 16.5,29.6,58.6,0,0.25 --give 0x50`, beside
//! grapple point #282): "the grapple hook isn't there; using the hook within range makes a bullet
//! impact". What the disc says [L ee_start_05.bin @ weapon_data 0x2bf150 + USATxt.dat]:
//! - **0x50/0x51 are the Q-Claw** — the bank's own name "Grapple" (+0x38 label 0x05000044),
//!   `weapon_data+0x70 & 0x40` (the grapple-bolt flag `Bullet_Delete` 0x12e660:33 latches on), max
//!   range +0x1C = 50.0 / 100.0, clip 0 (not a clip weapon), damage 0.
//! - 0x4A/0x4C are the **Stunner** (the watch taser, clip 100, regenerating), 0x4E/0x4F the
//!   **Laser**, 0x54/0x55 the **Micro-Camera**. The old port's gadget.rs put the claw on 0x4A/0x4C
//!   and the laser on 0x50/0x51 — the hardcoded table's mistake again — so `--give 0x50` ran the
//!   LASER beam (`is_laser_weapon`) whose endpoint is the "bullet impact", and the grapple gate
//!   (`0x4a | 0x4c`) never matched the claw.
//! - The game's grapple bolt IS a bullet: `Bullet_CollisionHandler` 0x12ce80's `uVar2 - 0x50 < 2`
//!   band + `Bullet_Delete` 0x12e660 latch it on the class-0x51 point; its effect kind is 0 (the
//!   claw's +0x70 = 0x40 has no bit 2), so `Effect_Create` 0x1ce688 spawns NO decal/debris.
//! - The rope: `Player_InitGrapple` 0x195d10 creates mesh 0x2000138 at BLData+0x158;
//!   `Player_GrappleSetRope` 0x192728 draws it only while `weapon − 0x50 < 2`.

use nf_game::player::Input;

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

/// The playtest spot, ~4.05 units from point #282 (13.7525, 32.2919, 57.3034 [L ee_start_05.bin obj+0x30]).
const SPOT: [f32; 3] = [16.5, 29.6, 58.6];
const TARGET: [f32; 3] = [13.752469, 32.291878, 57.303413];

/// Equips the claw the way `--give 0x50` does (lib.rs: `Player_EquipWeapon` 0x1a8a58 + three clips
/// of ammo + the switch request) through the real inventory/switch path. 07000005 owns the claw at
/// the start [L Inventory::level_start] and the game's own equip declines the re-equip (the
/// ammo-type-0 cap [M 0x1a8a58's owned branch]) — the fixed --give still DRAWS it (lib.rs's
/// already-owned branch).
fn give_claw(scene: &mut nf_view::scene::Scene) {
    let sim = scene.sim.as_mut().unwrap();
    let wd = sim.weapons.clone();
    let rounds = wd.clip_size(0x50).max(1);
    if !sim.drones.inventory.equip_weapon(&wd, 0x50, rounds, true) {
        assert!(*sim.drones.inventory.owned.get(0x50).unwrap_or(&false),
            "--give 0x50: refused but not owned — the fp-skin gate, not the owned draw");
    }
    sim.gun.switch_request = 0x50;
}

/// The main playtest flow: `--give 0x50` (the real claw id) draws the claw view model, and the held
/// fire at point #282 latches, reels Bond to the point and hangs him — with NO laser beam (the old
/// "bullet impact" was `is_laser_weapon(0x50)`'s beam endpoint) and NO bullet decals (the game's
/// claw bolt spawns none: effect kind 0 [M 0x1ce688 dispatch; L +0x70 = 0x40]).
#[test]
fn the_claw_draws_and_the_held_fire_latches_reels_and_hangs_with_no_beam_and_no_decals() {
    let iso = iso();
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005");
    assert!(!scene.sim.as_ref().unwrap().drones.grapple_points.is_empty(),
        "07000005 carries grapple points [D #281/#282]");
    give_claw(&mut scene);
    // Settle the switch (the holster/draw cycle runs through the real step_bond path).
    let mut equipped = false;
    for _ in 0..120 {
        scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
        if scene.sim.as_ref().unwrap().gun.weapon == 0x50 { equipped = true; break; }
    }
    assert!(equipped, "the claw (0x50) must equip; weapon {:#x}", scene.sim.as_ref().unwrap().gun.weapon);
    // The view model is the claw's own skin: 0x05000044 with its rigid claw parts on bones 38..42
    // [D probe: the skin ships in 07000005; its skinned main mesh 0x02100717 resolves on no archive].
    {
        let scene = &scene;
        let sim = scene.sim.as_ref().unwrap();
        let &ci = scene.view_models.get(&0x50).expect("the claw has a first-person view model");
        assert_eq!(scene.characters[ci].skin.hashcode, 0x0500_0044, "the view model is the claw's skin");
        assert!(!scene.characters[ci].skin.extra.is_empty(), "the claw's rigid parts (bones 38..42) draw");
        assert!(sim.view_model_ids.contains(&0x50), "the gadget cycle's D-003 gate lists the claw");
    }
    scene.sim.as_mut().unwrap().bond.pos = SPOT;
    // Aim at the point through the real look input.
    for _ in 0..200 {
        let (turn, look) = aim_inputs(&scene, TARGET);
        scene.sim.as_mut().unwrap().step_bond(Input { turn, look, ..Default::default() }, false, false, false);
    }
    // HOLD the fire: latch (substate 2), reel, hang — never a beam, never a decal.
    let mut hung = false;
    for _ in 0..1200 {
        let (turn, look) = aim_inputs(&scene, TARGET);
        scene.sim.as_mut().unwrap().step_bond(Input { turn, look, ..Default::default() }, true, false, false);
        let scene = &scene;
        let sim = scene.sim.as_ref().unwrap();
        assert!(scene.sim.as_ref().unwrap().laser_beam.is_none(),
            "the claw (0x50) is NOT a laser weapon — the old is_laser_weapon drew the 'bullet impact' beam");
        assert!(sim.drones.effects.decals.is_empty(),
            "the claw bolt spawns no bullet decals [M Effect_Create 0x1ce688's kind-0 dispatch]");
        if sim.bond.grapple.as_ref().map(|g| g.phase) == Some(2) {
            let d = dist(sim.bond.pos, TARGET);
            if d < 2.5 { hung = true; break; }
        }
        assert!(!(sim.bond.substate == nf_game::player::SS_WALK && sim.bond.grapple.is_none()),
            "the grapple must not detach while the fire is held [M 0x192860:57-62]");
    }
    {
        let sim = scene.sim.as_ref().unwrap();
        assert!(hung, "the held fire must reel Bond to #282 and hang him (pos {:?}, phase {:?})",
            sim.bond.pos, sim.bond.grapple.as_ref().map(|g| g.phase));
        assert!(sim.drones.effects.decals.is_empty(), "0 bullet decals through the whole grapple");
    }
    // The release detaches [M 0x192860:64-66].
    scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
    let sim = scene.sim.as_ref().unwrap();
    assert_eq!(sim.bond.substate, nf_game::player::SS_WALK, "the release returns Bond to the walk substate");
    assert!(sim.bond.grapple.is_none(), "the grapple state clears on the release");
}

/// Sister: the ids the old port grappled with are the STUNNER's — firing 0x4A at #282 does NOT
/// latch (and never beams), while 0x50 stays out of the laser set that made the "bullet impact".
#[test]
fn the_stunner_does_not_grapple_and_the_claw_does_not_beam() {
    let iso = iso();
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005");
    // The id sets themselves (the regression that made the playtest's "bullet impact").
    assert!(nf_game::gadget::is_grapple(0x50) && nf_game::gadget::is_grapple(0x51),
        "0x50/0x51 are the Q-Claw [L bank 'Grapple' 0x05000044; +0x70 & 0x40]");
    assert!(!nf_game::gadget::is_grapple(0x4a) && !nf_game::gadget::is_grapple(0x4c),
        "0x4A/0x4C are the Stunner [L bank 'Stunner' 0x05000042]");
    assert!(!nf_game::gadget::is_laser_weapon(0x50), "0x50 must NOT run the beam (the playtest's bug)");
    assert!(nf_game::gadget::is_laser_weapon(0x4e), "the Laser 0x4E keeps the beam [L +0x68 laser-pointer]");
    // The ranges are the table's own +0x1C: 50.0 for the claw (0x50), 100.0 for the upgrade (0x51),
    // NOT the old 5.0/10.0 (the Stunner's ranges).
    let sim = scene.sim.as_ref().unwrap();
    assert!((nf_game::gadget::grapple_range(&sim.weapons, 0x50) - 50.0).abs() < 1e-3,
        "0x50's +0x1C = 50.0 [L ee_start_05.bin]");
    assert!((nf_game::gadget::grapple_range(&sim.weapons, 0x51) - 100.0).abs() < 1e-3,
        "0x51's +0x1C = 100.0 [L ee_start_05.bin]");
    // Through the sim: the Stunner 0x4a (owned at the level start) fires at #282 and does NOT latch.
    let mut equipped = false;
    for _ in 0..12 {
        scene.sim.as_mut().unwrap().step_bond(Input { gadget_cycle: 1, ..Default::default() }, false, false, false);
        for _ in 0..60 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
        if scene.sim.as_ref().unwrap().gun.weapon == 0x4a { equipped = true; break; }
    }
    assert!(equipped, "the Stunner must equip through the gadget cycle; weapon {:#x}", scene.sim.as_ref().unwrap().gun.weapon);
    scene.sim.as_mut().unwrap().bond.pos = SPOT;
    for _ in 0..200 {
        let (turn, look) = aim_inputs(&scene, TARGET);
        scene.sim.as_mut().unwrap().step_bond(Input { turn, look, ..Default::default() }, false, false, false);
    }
    for _ in 0..30 { scene.sim.as_mut().unwrap().step_bond(Input::default(), true, false, false); }
    for _ in 0..10 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    let scene = &scene;
    let sim = scene.sim.as_ref().unwrap();
    assert_ne!(sim.bond.substate, nf_game::player::SS_GRAPPLE, "the Stunner does not grapple");
    assert!(sim.bond.grapple.is_none(), "no grapple state from the Stunner");
    assert!(scene.sim.as_ref().unwrap().laser_beam.is_none(), "the Stunner does not beam either");
}

/// The user's other report spot (`--bond -18.28,38.19,-71.83,-4.717,0.151`): the nearest point is
/// 15.4 units out — beyond the OLD 5.0 "range" the port grappled with, well inside the claw's real
/// 50.0 bolt range [L +0x1C], so the held fire latches there too.
#[test]
fn the_users_report_spot_latches_within_the_real_range() {
    let iso = iso();
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005");
    give_claw(&mut scene);
    for _ in 0..120 {
        scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
        if scene.sim.as_ref().unwrap().gun.weapon == 0x50 { break; }
    }
    let spot = [-18.28f32, 38.19, -71.83];
    scene.sim.as_mut().unwrap().bond.pos = spot;
    scene.sim.as_mut().unwrap().bond.yaw = -4.717;
    scene.sim.as_mut().unwrap().bond.pitch = 0.151;
    for _ in 0..30 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    // The nearest point along the game's own acceptance test (the bolt's sphere sweep): from the
    // spot, the aim direction at #282's neighbours — use the head→point ray the sim picks with.
    let sim = scene.sim.as_ref().unwrap();
    let head = sim.bond.head();
    let nearest = sim.drones.grapple_points.iter()
        .map(|p| (dist(p.pos, spot), p.pos))
        .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap()).unwrap();
    assert!(nearest.0 < nf_game::gadget::grapple_range(&sim.weapons, 0x50),
        "the user's spot is {:.1} from the nearest point — inside the claw's real 50.0 range", nearest.0);
    let dir = {
        let d = [nearest.1[0] - head[0], nearest.1[1] - head[1], nearest.1[2] - head[2]];
        let l = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
        [d[0] / l, d[1] / l, d[2] / l]
    };
    assert!(nf_game::gadget::pick_grapple_point(&sim.drones.grapple_points, head, dir,
        nf_game::gadget::grapple_range(&sim.weapons, 0x50)).is_some(),
        "the bolt sweep accepts the point from the user's spot");
}

fn aim_inputs(scene: &nf_view::scene::Scene, target: [f32; 3]) -> (f32, f32) {
    let sim = scene.sim.as_ref().unwrap();
    let bp = sim.bond.pos;
    let dx = target[0] - bp[0];
    let dz = target[2] - bp[2];
    let dy = target[1] - (bp[1] + sim.bond.eye - sim.bond.crouch_drop);
    let desired = dx.atan2(dz);
    let err = desired - sim.bond.yaw;
    let want_pitch = (dy / dxz(bp, target).max(0.5)).atan() / std::f32::consts::FRAC_PI_2;
    (err.clamp(-0.08, 0.08), (want_pitch - sim.bond.pitch).clamp(-0.08, 0.08))
}

fn dxz(a: [f32; 3], b: [f32; 3]) -> f32 { ((a[0] - b[0]).powi(2) + (a[2] - b[2]).powi(2)).sqrt() }
fn dist(a: [f32; 3], b: [f32; 3]) -> f32 { ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt() }
