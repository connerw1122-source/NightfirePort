//! fix-gadgets + fix-laser-beam: the Laser (0x4E/0x4F — the disc's name label 0x05000043 "Laser";
//! the old port comments called it the watch/decryptor) beam through the real sim path.
//!
//! The game: the gadget beams are the SHOTS the held fire spawns — each refire tick (interval
//! +0x40 = 1 [L weapon_data]) runs `Player_WeaponFiring` 0x1a5460's held-fire branch →
//! `Player_WeaponInitBullet` 0x1a4bf0 → the beam/bullet reaches the lock's contact list;
//! `Lock_Update` 0x1ae3d8 case 0 reads the contact's weapon id and advances the burn; the open
//! writes the lock's out channel (07000008's safe bolts: ch 134/135 [D #207/#208]). fix-laser-beam:
//! the beam weapon ids are the +0x6C & 0x180 records (0x4E/0x4F laser, 0x4A/0x4C taser [L]) —
//! 0x54/0x55 are the Micro-Camera (no beam, no burn), so its old tests here equip the Laser.

use nf_game::player::Input;

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

/// The Laser (0x4E — the disc's cutting laser, name label 0x05000043 "Laser" [L USATxt.dat]) aimed
/// at the safe's top bolt (162.24, 43.88, −117.14 [D lock dump; fix_objects2]) burns it: the out
/// channel 134 rises. Sister: the beam pointed away leaves the lock intact.
#[test]
fn the_watch_laser_burns_the_safe_bolt() {
    let iso = iso();
    let mut scene = nf_view::load(&iso, "07000008.bin", false).expect("load 07000008");
    let sim = scene.sim.as_mut().unwrap();
    // Stand before the safe and equip the Laser through the real switch input.
    sim.bond.pos = [160.3, 44.6, -115.4];
    sim.bond.yaw = 0.85;
    let draw = Input { switch_to: Some(0x4e), ..Default::default() };
    for _ in 0..40 { sim.step_bond(draw, false, false, false); }
    assert_eq!(sim.gun.weapon, 0x4e, "the Laser must equip; weapon {:#x}", sim.gun.weapon);
    // Aim at the top bolt through the real look input and HOLD the fire: the beam burns it.
    // No aim gate on the Laser [M 0x1a5460:512-514's 0x54 check is the Micro-Camera's fire polarity].
    let target = [162.2365f32, 43.875843, -117.14455];
    for _ in 0..300 {
        let (turn, look) = {
            let sim = scene.sim.as_ref().unwrap();
            let bp = sim.bond.pos;
            let dx = target[0] - bp[0];
            let dz = target[2] - bp[2];
            let dy = target[1] - (bp[1] + sim.bond.eye - sim.bond.crouch_drop);
            let desired = dx.atan2(dz);
            let err = desired - sim.bond.yaw;
            let want_pitch = (dy / dxz(bp, target).max(0.5)).atan() / std::f32::consts::FRAC_PI_2;
            (err.clamp(-0.08, 0.08), (want_pitch - sim.bond.pitch).clamp(-0.08, 0.08))
        };
        scene.sim.as_mut().unwrap().step_bond(Input { turn, look, ..Default::default() }, true, false, false);
    }
    let sim = scene.sim.as_ref().unwrap();
    assert!(sim.drones.channels.on(134), "the top bolt's burn completes: the out channel 134 rises [M 0x1ae3d8:194-195]");
    assert!(sim.drones.locks.iter().find(|l| l.out == 134).map(|l| l.burned).unwrap_or(false), "the lock record is burned");
    assert!(sim.laser_beam.is_some(), "the beam endpoints are parked for the renderer while the laser is out");
}

/// fix-gadgets2: the burn follows the button. The same aim and hold, then the release: the channel
/// stops rising the tick the fire comes up (the game's shots stop spawning [M 0x1a5460:442-501's
/// no-fire branch]). The pre-fire ticks must NOT burn anything (the old model burned while equipped).
#[test]
fn the_watch_laser_burns_only_while_the_fire_is_held() {
    let iso = iso();
    let mut scene = nf_view::load(&iso, "07000008.bin", false).expect("load 07000008");
    let sim = scene.sim.as_mut().unwrap();
    sim.bond.pos = [160.3, 44.6, -115.4];
    sim.bond.yaw = 0.85;
    let draw = Input { switch_to: Some(0x4e), ..Default::default() };
    for _ in 0..40 { sim.step_bond(draw, false, false, false); }
    assert_eq!(sim.gun.weapon, 0x4e, "the Laser equips [D USATxt 0x05000043]");
    let target = [162.2365f32, 43.875843, -117.14455];
    // Aimed at the bolt WITHOUT the fire: nothing burns (the fail-first property — the pre-fix port
    // burned the lock in this state).
    for _ in 0..120 {
        let (turn, look) = {
            let sim = scene.sim.as_ref().unwrap();
            let bp = sim.bond.pos;
            let dx = target[0] - bp[0];
            let dz = target[2] - bp[2];
            let dy = target[1] - (bp[1] + sim.bond.eye - sim.bond.crouch_drop);
            let desired = dx.atan2(dz);
            let err = desired - sim.bond.yaw;
            let want_pitch = (dy / dxz(bp, target).max(0.5)).atan() / std::f32::consts::FRAC_PI_2;
            (err.clamp(-0.08, 0.08), (want_pitch - sim.bond.pitch).clamp(-0.08, 0.08))
        };
        scene.sim.as_mut().unwrap().step_bond(Input { turn, look, ..Default::default() }, false, false, false);
    }
    {
        let sim = scene.sim.as_ref().unwrap();
        assert!(!sim.drones.channels.on(134), "no burn without the fire held (the old model burned while equipped)");
    }
    // Hold the fire (aimed): the burn completes.
    for _ in 0..240 {
        let (turn, look) = {
            let sim = scene.sim.as_ref().unwrap();
            let bp = sim.bond.pos;
            let dx = target[0] - bp[0];
            let dz = target[2] - bp[2];
            let dy = target[1] - (bp[1] + sim.bond.eye - sim.bond.crouch_drop);
            let desired = dx.atan2(dz);
            let err = desired - sim.bond.yaw;
            let want_pitch = (dy / dxz(bp, target).max(0.5)).atan() / std::f32::consts::FRAC_PI_2;
            (err.clamp(-0.08, 0.08), (want_pitch - sim.bond.pitch).clamp(-0.08, 0.08))
        };
        scene.sim.as_mut().unwrap().step_bond(Input { turn, look, ..Default::default() }, true, false, false);
    }
    let sim = scene.sim.as_ref().unwrap();
    assert!(sim.drones.channels.on(134), "the held fire burns the bolt");
}

/// Sister: the same setup aimed at the wall (away from the bolts) burns nothing.
#[test]
fn the_laser_away_from_the_bolts_burns_nothing() {
    let iso = iso();
    let mut scene = nf_view::load(&iso, "07000008.bin", false).expect("load");
    let sim = scene.sim.as_mut().unwrap();
    sim.bond.pos = [160.3, 44.6, -115.4];
    sim.bond.yaw = 0.85;
    let draw = Input { switch_to: Some(0x4e), ..Default::default() };
    for _ in 0..40 { sim.step_bond(draw, false, false, false); }
    assert_eq!(sim.gun.weapon, 0x4e, "the Laser equips [D USATxt 0x05000043]");
    // Aim at the ceiling (the bolt is below the eye line) with the fire HELD: the beam never touches it.
    for _ in 0..300 {
        let (_, look) = {
            let sim = scene.sim.as_ref().unwrap();
            let want_pitch = 0.9f32;
            (0.0f32, (want_pitch - sim.bond.pitch).clamp(-0.08, 0.08))
        };
        scene.sim.as_mut().unwrap().step_bond(Input { look, ..Default::default() }, true, false, false);
    }
    let sim = scene.sim.as_ref().unwrap();
    assert!(!sim.drones.channels.on(134) && !sim.drones.channels.on(135), "no bolt burns when the beam points away");
    assert!(!sim.drones.locks.iter().any(|l| l.burned), "the lock records stay intact");
}

fn dxz(a: [f32; 3], b: [f32; 3]) -> f32 { ((a[0] - b[0]).powi(2) + (a[2] - b[2]).powi(2)).sqrt() }

/// The same lock burn through the GADGET CYCLE (the wheel walk, not switch_to) — 0x4E is the
/// Laser [L USATxt 0x05000043; the old comment called it the Q-Decryptor — the decryptor pair is
/// 0x56/0x57, `Lock_Update` 0x1ae3d8:57-63's `− 0x56 < 2` band]: the same safe, the same aim.
#[test]
fn the_decryptor_burns_the_safe_bolt() {
    let iso = iso();
    let mut scene = nf_view::load(&iso, "07000008.bin", false).expect("load 07000008");
    let sim = scene.sim.as_mut().unwrap();
    sim.bond.pos = [160.3, 45.5, -115.4];
    sim.bond.yaw = 0.85;
    let mut equipped = false;
    for _ in 0..12 {
        sim.step_bond(Input { gadget_cycle: 1, ..Default::default() }, false, false, false);
        for _ in 0..60 { sim.step_bond(Input::default(), false, false, false); }
        if sim.gun.weapon == 0x4e { equipped = true; break; }
    }
    assert!(equipped, "the decryptor must equip through the gadget cycle; weapon {:#x}", sim.gun.weapon);
    let target = [162.2365f32, 43.875843, -117.14455];
    // The decryptor's fire has no aim gate [M 0x1a5460:512-514 reads only 0x54] — hold the fire and burn.
    for _ in 0..300 {
        let (turn, look) = {
            let sim = scene.sim.as_ref().unwrap();
            let bp = sim.bond.pos;
            let dx = target[0] - bp[0];
            let dz = target[2] - bp[2];
            let dy = target[1] - (bp[1] + sim.bond.eye - sim.bond.crouch_drop);
            let desired = dx.atan2(dz);
            let err = desired - sim.bond.yaw;
            let want_pitch = (dy / dxz(bp, target).max(0.5)).atan() / std::f32::consts::FRAC_PI_2;
            (err.clamp(-0.08, 0.08), (want_pitch - sim.bond.pitch).clamp(-0.08, 0.08))
        };
        scene.sim.as_mut().unwrap().step_bond(Input { turn, look, ..Default::default() }, true, false, false);
    }
    let sim = scene.sim.as_ref().unwrap();
    assert!(sim.drones.channels.on(134), "the decryptor's beam burns the top bolt [M 0x1ae3d8:57-63]");
}

// ---- fix-laser-beam: the playtest symptoms ----

/// The user's playtest (1013-1758): holding fire with the Laser "sounds like a rapid fire of bullet
/// impacts" and shows no beam; the loop sound keeps playing after release. The game's model: every
/// held tick is a shot [L weapon_data 0x4E +0x40 = 1] whose world death is `Effect_Laser` [M
/// Bullet_CollisionHandler 0x12ce80:186-192] — NO bullet decal and NO wall-impact sound — while the
/// loop (0x12, the fire script's op-1 [L 0x060004e9]) holds ONE voice and the release script's
/// op-4.3 stops it [L 0x060004e8].
#[test]
fn the_laser_hold_spawns_no_bullet_impacts_and_keeps_one_loop_voice() {
    let iso = iso();
    let mut scene = nf_view::load(&iso, "07000008.bin", false).expect("load 07000008");
    let sim = scene.sim.as_mut().unwrap();
    sim.bond.pos = [160.3, 44.6, -115.4];
    sim.bond.yaw = 0.85;
    let draw = Input { switch_to: Some(0x4e), ..Default::default() };
    for _ in 0..40 { sim.step_bond(draw, false, false, false); }
    assert_eq!(sim.gun.weapon, 0x4e, "the Laser equips");
    // Aim at the wall behind the safe, hold fire 60 ticks through the real input path.
    let target = [162.2365f32, 43.875843, -117.14455];
    for _ in 0..60 {
        let (turn, look) = {
            let sim = scene.sim.as_ref().unwrap();
            let bp = sim.bond.pos;
            let dx = target[0] - bp[0];
            let dz = target[2] - bp[2];
            let dy = target[1] - (bp[1] + sim.bond.eye - sim.bond.crouch_drop);
            let desired = dx.atan2(dz);
            let err = desired - sim.bond.yaw;
            let want_pitch = (dy / dxz(bp, target).max(0.5)).atan() / std::f32::consts::FRAC_PI_2;
            (err.clamp(-0.08, 0.08), (want_pitch - sim.bond.pitch).clamp(-0.08, 0.08))
        };
        scene.sim.as_mut().unwrap().step_bond(Input { turn, look, ..Default::default() }, true, false, false);
    }
    let sim = scene.sim.as_ref().unwrap();
    // NO bullet decals from the hold [M Effect_Laser 0x1cef38 — no Effect_Bullet decal].
    assert_eq!(sim.drones.effects.decals.len(), 0,
        "the laser hold must not spawn bullet decals (got {})", sim.drones.effects.decals.len());
    // The fire script's loop (0x12) holds exactly ONE voice — the pre-fix port re-played the op-1
    // every tick (60 nodes/s) — and the pool is free of impact-sound churn [M :186-192: no
    // Effect_Bullet/impact sound path runs for the laser]. The level's own ambients (0x5fc/0x1d5…)
    // and the equip sounds are legitimate other voices.
    let ids = sim.drones.audio.active_ids();
    let loop_count = ids.iter().filter(|&&id| id == 0x12).count();
    assert_eq!(loop_count, 1, "exactly one loop voice 0x12 plays (active ids {ids:x?})");
    assert!(ids.len() < 10, "no voice churn during the hold (active ids {ids:x?})");
    // The beam is parked for the renderer every held tick.
    assert!(sim.laser_beam.is_some(), "the beam endpoints are parked while held");
    // Release: the fire script plays out (16 frames [L 0x060004e9]) and the release script's op-4.3
    // stops the loop [L 0x060004e8 op 4 args [1, 3, 12]] — the game's own ring-out through the
    // state-0xb → 0xb transition [M 0x1a2e38 case 0xb:499-507].
    for _ in 0..24 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    let sim = scene.sim.as_ref().unwrap();
    assert!(!sim.drones.audio.active_ids().contains(&0x12), "the loop is gone after the release ring-out");
    assert!(sim.laser_beam.is_none(), "the beam ends with the fire");
}

/// The beam's far end equals the crosshair ray's world hit [M Player_MuzzleFlash 0x1a4298:107-132 —
/// the shot ray's `Collide_RayIntersect` first hit caps the beam; Player_WeaponInitBullet 0x1a4bf0
/// :152-159 aims through `Player_GetAimingPoint`]. The old bond.look() ray missed the crosshair
/// offset — the user's "the laser isn't accurate" on the 0700000c display vent.
#[test]
fn the_beam_end_equals_the_crosshair_rays_hit() {
    let iso = iso();
    let mut scene = nf_view::load(&iso, "07000008.bin", false).expect("load 07000008");
    let sim = scene.sim.as_mut().unwrap();
    sim.bond.pos = [160.3, 44.6, -115.4];
    sim.bond.yaw = 0.85;
    let draw = Input { switch_to: Some(0x4e), ..Default::default() };
    for _ in 0..40 { sim.step_bond(draw, false, false, false); }
    assert_eq!(sim.gun.weapon, 0x4e, "the Laser equips");
    let target = [162.2365f32, 43.875843, -117.14455];
    // fix-weapon-anims2: the hold now covers the Laser's correct-rate equip+draw+first-shot chain.
    // The Laser's draw script 0x060004ee is 76 frames and the switch machine plays it at the game's
    // 1.25 [M 0x1a2e38 case 3:287-288] after the start weapon's holster (case 1:220-223) — measured
    // (wa2 probe): the draw settles at hold-tick 48 and the first beam parks at hold-tick 49 (the
    // fire gate `can_fire = !switching` matches the game's state-3/4 fire block). The old 30-tick
    // window was tuned to the double-advance bug's 2x-fast draw (~24 ticks). The assertion is
    // unchanged: the beam, once parked, must equal the crosshair ray's hit.
    for _ in 0..120 {
        let (turn, look) = {
            let sim = scene.sim.as_ref().unwrap();
            let bp = sim.bond.pos;
            let dx = target[0] - bp[0];
            let dz = target[2] - bp[2];
            let dy = target[1] - (bp[1] + sim.bond.eye - sim.bond.crouch_drop);
            let desired = dx.atan2(dz);
            let err = desired - sim.bond.yaw;
            let want_pitch = (dy / dxz(bp, target).max(0.5)).atan() / std::f32::consts::FRAC_PI_2;
            (err.clamp(-0.08, 0.08), (want_pitch - sim.bond.pitch).clamp(-0.08, 0.08))
        };
        scene.sim.as_mut().unwrap().step_bond(Input { turn, look, ..Default::default() }, true, false, false);
    }
    let sim = scene.sim.as_ref().unwrap();
    let (a, b) = sim.laser_beam.expect("the beam is parked");
    // The head end is the eye; the far end sits on the aimed-at surface — within the lock's reach
    // (the safe face the bolts sit on) and NOT short of it.
    let head = sim.bond.head();
    assert!((a[0] - head[0]).abs() < 0.5 && (a[2] - head[2]).abs() < 0.5, "the beam starts at the eye (got {a:?})");
    let d = ((b[0] - target[0]).powi(2) + (b[1] - target[1]).powi(2) + (b[2] - target[2]).powi(2)).sqrt();
    assert!(d < 1.5, "the beam's far end lands on the aimed-at face (dist {d:.2} from the bolt)");
    // The crosshair-ray construction: the far end lies along the aim direction from the head.
    let look = sim.bond.look();
    let (yaw, p) = (sim.bond.yaw, sim.bond.pitch * std::f32::consts::FRAC_PI_2);
    let right = [-yaw.cos(), 0.0, yaw.sin()];
    let up = [-p.sin() * yaw.sin(), p.cos(), -p.sin() * yaw.cos()];
    let (tx, ty) = (0.76966 * sim.cross[0], 0.57724 * sim.cross[1]);
    let d0 = [0, 1, 2].map(|k| look[k] + tx * right[k] + ty * up[k]);
    let l = (d0[0] * d0[0] + d0[1] * d0[1] + d0[2] * d0[2]).sqrt();
    let dir = d0.map(|v| v / l);
    let seg = [b[0] - head[0], b[1] - head[1], b[2] - head[2]];
    let sl = (seg[0] * seg[0] + seg[1] * seg[1] + seg[2] * seg[2]).sqrt();
    let seg = [seg[0] / sl, seg[1] / sl, seg[2] / sl];
    let dot = dir[0] * seg[0] + dir[1] * seg[1] + dir[2] * seg[2];
    assert!(dot > 0.999, "the beam runs down the crosshair ray (dot {dot:.5})");
}

/// The Stunner (0x4A — the disc name 0x05000042 "Stunner" [L]; the old port called it the Q-Claw)
/// spawns NO shot bullet and draws NO beam when the aim ray has no guard in range [M
/// Player_WeaponInitBullet 0x1a4bf0:187-200's +0x95d miss arm returns before Bullet_init;
/// Draw_TaserBeam's gate reads +0x95d too [M 0x1a4298:134]].
#[test]
fn the_stunner_without_a_target_shoots_nothing() {
    let iso = iso();
    let mut scene = nf_view::load(&iso, "07000008.bin", false).expect("load 07000008");
    let sim = scene.sim.as_mut().unwrap();
    // `--give 0x4e,0x4a` is the playtest's equip path; the test equips through the same switch.
    sim.bond.pos = [160.3, 44.6, -115.4];
    sim.bond.yaw = 0.85;
    let draw = Input { switch_to: Some(0x4a), ..Default::default() };
    let mut ok = false;
    for _ in 0..40 {
        sim.step_bond(draw, false, false, false);
        if sim.gun.weapon == 0x4a { ok = true; break; }
    }
    if !ok {
        // The 07000008 start owns the Stunner (the 07 party loadout's 0x4a/999 [M 0x1a7ef8 case 7]);
        // the switch input must find it owned.
        panic!("the Stunner (0x4a) did not equip — owned {:?}", sim.drones.inventory.owned.get(0x4a));
    }
    let decals_before = scene.sim.as_ref().unwrap().drones.effects.decals.len();
    let target = [162.2365f32, 43.875843, -117.14455];
    for _ in 0..60 {
        let (turn, look) = {
            let sim = scene.sim.as_ref().unwrap();
            let bp = sim.bond.pos;
            let dx = target[0] - bp[0];
            let dz = target[2] - bp[2];
            let dy = target[1] - (bp[1] + sim.bond.eye - sim.bond.crouch_drop);
            let desired = dx.atan2(dz);
            let err = desired - sim.bond.yaw;
            let want_pitch = (dy / dxz(bp, target).max(0.5)).atan() / std::f32::consts::FRAC_PI_2;
            (err.clamp(-0.08, 0.08), (want_pitch - sim.bond.pitch).clamp(-0.08, 0.08))
        };
        // The safe face is ~2.4 away — beyond nothing alive: no guard in the ray → no shot, no beam
        // (the wall hit never happens: no bullet exists to hit it).
        scene.sim.as_mut().unwrap().step_bond(Input { turn, look, ..Default::default() }, true, false, false);
    }
    let sim = scene.sim.as_ref().unwrap();
    assert_eq!(sim.drones.effects.decals.len(), decals_before, "no impact decals from a targetless stunner hold");
    assert!(sim.laser_beam.is_none(), "the taser beam needs +0x95d (a target) [M 0x1a4298:134]");
    assert_eq!(sim.stunner_target, Some(false), "the ray test ran and found no guard");
}
