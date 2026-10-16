//! sg5-semi-laser — the SG5 Commando's semi-auto mode lights its laser sight, through the real
//! path (`nf_view::load` + `Sim::step_bond`).
//!
//! Playtest 1015-1738: "the SG5, when you turn on semi-auto, it's supposed to have a laser ...
//! turn on, and it doesn't". The game: the sight is the weapon record's `+0x68` bit 0x8 —
//! `Player_SetWeaponAnim` 0x1a9338:21-26 latches it into `BLData+0x964` on every completed
//! draw/switch, and `Player_LaserPointer` 0x1a65d8 (per frame from `Player_CollisionHandler`
//! 0x1949a8:188) beams from the first-person model's bone-4 sight datum along the bone's local
//! +Z (the ray: head → datum + Z·100, `Collide_RayIntersect`'s bit-0x8 drop-pass query), clamps
//! the end at the hit and dots the wall (hit + normal·0.01). The disc gives the bit to the SG5's
//! SEMI twin 0x17 (`+0x68` = 0x00000088) and not to the full-auto 0x16 (0x00000080) [D
//! weapon_data 0x2bf150] — semi lights it, full-auto doesn't.

use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// The 07000005 load + the dev give of the SG5 (the user's weapon), settled to idle.
fn scene_with_sg5(iso: &str) -> nf_view::scene::Scene {
    let mut scene = nf_view::load(iso, "07000005.bin", false).expect("load 07000005");
    for _ in 0..4 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    let (status, switch_to) = nf_view::panel::apply(&mut scene, &[("do".to_string(), "weapon".to_string()), ("id".to_string(), "0x16".to_string())]);
    assert!(status.contains("gave"), "the give equips: {status}");
    {
        let sim = scene.sim.as_mut().unwrap();
        let mut input = Input::default();
        input.switch_to = Some(switch_to.expect("the give returns the equip switch"));
        sim.step_bond(input, false, false, false);
    }
    for _ in 0..200 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    scene
}

/// Switching the SG5 to semi (the ○ twin swap 0x16 → 0x17 through `fire_mode_switch`) lights the
/// sight: the latch sets from the semi record's `+0x68 & 0x8` [M 0x1a9338:21-26], the beam's end
/// is the aim trace's hit on the floor Bond is looking at, and the wall dot sits a hair off the
/// surface along the hit normal [M Player_LaserPointer 0x1a65d8:129-141, 189-205].
#[test]
fn semi_auto_lights_the_sight_and_the_beam_ends_on_a_surface() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = scene_with_sg5(&iso);
    assert_eq!(scene.sim.as_ref().unwrap().gun.weapon, 0x16, "the SG5 full-auto is drawn from the give");

    // Aim down at the floor (the pitch unit is π/2; the negative half looks 45° down) so the
    // 100-unit sight ray always meets a surface — deterministic without level-layout knowledge.
    scene.sim.as_mut().unwrap().bond.pitch = -0.5;
    for _ in 0..4 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }

    // FULL-AUTO (0x16, no record bit [D]): the sight is off — the playtest's "it doesn't" is
    // correct for this mode; the port must show nothing.
    {
        let s = scene.sim.as_ref().unwrap();
        assert!(!s.gun.laser_on, "full-auto 0x16 carries no +0x68 bit 0x8 [D] — the latch must be off");
        assert!(s.laser_sight().is_none(), "full-auto: no sight beam");
    }

    // The ○ press (pad action 12, the keyboard B) — one tick, then released. `fire_mode_switch`
    // takes the SG5's twin byte (+0x05 = 1) to 0x17 and writes the request [M 0x1a24c8:246-260];
    // the swap commits and `Player_SetWeaponAnim`'s latch read runs on the commit [M 0x1a9338].
    {
        let sim = scene.sim.as_mut().unwrap();
        let mut input = Input::default();
        input.mode_switch = true;
        sim.step_bond(input, false, false, false);
    }
    for _ in 0..200 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }

    let head = scene.sim.as_ref().unwrap().bond.head();
    {
        let s = scene.sim.as_ref().unwrap();
        assert_eq!(s.gun.weapon, 0x17, "the twin switch lands on the SG5's semi variant");
        assert!(s.gun.laser_on, "semi 0x17 carries the +0x68 bit 0x8 [D] — the latch must be lit");
        let (start, end, dot) = s.laser_sight().expect("semi: the sight beam is on");
        // The beam starts at the FP model's bone-4 sight datum — ahead of the eye, not at it.
        let d0 = ((start[0] - head[0]).powi(2) + (start[1] - head[1]).powi(2) + (start[2] - head[2]).powi(2)).sqrt();
        assert!(d0 > 0.05, "the beam starts at the bone-4 datum (eye distance {d0:.4}), not at the eye");
        // The end is the trace's clamped hit: extend the head→end segment and re-trace — the first
        // hit must land back on `end` (the bit-0x8 query semantics [M 0x1ec690; foliage-bullets.md]).
        let dir = [end[0] - head[0], end[1] - head[1], end[2] - head[2]];
        let far2 = [head[0] + dir[0] * 2.0, head[1] + dir[1] * 2.0, head[2] + dir[2] * 2.0];
        let (t, _, _) = s.world.segment_surface_bullet(head, far2).expect("the sight ray meets the floor");
        let hit = [head[0] + dir[0] * 2.0 * t, head[1] + dir[1] * 2.0 * t, head[2] + dir[2] * 2.0 * t];
        let miss = ((hit[0] - end[0]).powi(2) + (hit[1] - end[1]).powi(2) + (hit[2] - end[2]).powi(2)).sqrt();
        assert!(miss < 0.05, "the beam end sits on the re-traced hit (off by {miss:.4}) — 0x1a65d8 clamps to the hit");
        assert!(t < 0.99, "the hit clamps the 100-unit ray (t = {t:.3}) — the floor is closer than the far end");
        // The dot: hit + normal·0.01, the normal unit and up (the floor) [M 0x1a65d8:189-205].
        let (dot_pos, dot_n) = dot.expect("a floor hit carries the wall dot");
        let nl = (dot_n[0] * dot_n[0] + dot_n[1] * dot_n[1] + dot_n[2] * dot_n[2]).sqrt();
        assert!((nl - 1.0).abs() < 1e-3, "the dot's normal is unit, got len {nl}");
        assert!(dot_n[1] > 0.7, "a down ray hits the floor — the normal points up, got {dot_n:?}");
        let dm = ((dot_pos[0] - (end[0] + dot_n[0] * 0.01)).powi(2) + (dot_pos[1] - (end[1] + dot_n[1] * 0.01)).powi(2) + (dot_pos[2] - (end[2] + dot_n[2] * 0.01)).powi(2)).sqrt();
        assert!(dm < 1e-4, "the dot sits hit + normal·0.01, off by {dm:.5}");
        eprintln!("semi sight: datum-eye dist {d0:.3}, beam len {:.2}, floor t {t:.3}", (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt());
    }

    // Sister: switching BACK to full-auto kills the sight — the same ○ path, the latch re-read
    // from 0x16's bitless record on the swap commit [M 0x1a9338:21-26].
    {
        let sim = scene.sim.as_mut().unwrap();
        let mut input = Input::default();
        input.mode_switch = true;
        sim.step_bond(input, false, false, false);
    }
    for _ in 0..200 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    let s = scene.sim.as_ref().unwrap();
    assert_eq!(s.gun.weapon, 0x16, "the second ○ press swaps back to full-auto");
    assert!(!s.gun.laser_on, "full-auto: the latch is off again");
    assert!(s.laser_sight().is_none(), "full-auto: no sight beam (head {head:?})");
}

/// The draw's own gate [M Player_LaserPointer 0x1a65d8:95-99]: the beam lives when the record OR
/// its `+0x05` twin carries the bit — so the latch, once set by the semi draw, is what matters;
/// this test pins the latch to the RECORD the draw committed (the port sets it at every swap
/// commit, the game's `Player_SetWeaponAnim` call per completed draw).
#[test]
fn the_latch_follows_every_committed_draw_not_just_the_switch() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let scene = scene_with_sg5(&iso);
    // The initial P2K draw (Gun::new(2, ...)) — the P2K's records carry no sight bit [D]:
    // the fresh sim must not beam before the first swap commits anything.
    let fresh = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005");
    {
        let s = fresh.sim.as_ref().unwrap();
        assert!(!s.gun.laser_on, "the fresh P2K (0x02, +0x68 = 0x1004 [D]) has no sight bit");
        assert!(s.laser_sight().is_none(), "no sight before any draw commits");
    }
    // And the SG5's own give path (the switch commit) latches OFF for the full-auto 0x16.
    let s = scene.sim.as_ref().unwrap();
    assert_eq!(s.gun.weapon, 0x16);
    assert!(!s.gun.laser_on, "the 0x16 draw commit re-read the bitless record");
    assert!(s.laser_sight().is_none());
}
