//! fix-stunner-press (109b): the Stunner 0x4A fires on the PRESS and the tase stuns the guard.
//!
//! Ground truth [D docs/live/2026-10-14-weapon-measurements-pcsx2.md]: the real stunner activates on
//! the press (the device swings forward, the tip lights blue), holds while fire is held, swings back
//! on the release, and spends no charge with no target. The user's playtest 1014-1725: "Doesn't
//! fucking work. It only really activates on releasing the key."
//!
//! The port-side failure this file pins: the tase kill route. The game's shot spawns a zero-damage
//! bullet with the fired record itself [M Player_WeaponInitBullet 0x1a4bf0:176-203 — the 0x4A/0x4C
//! target test's hit arm falls through to Bullet_init; the record: damage 0.0, radius 0.0, puff 0.0
//! [L ee_start_05.bin @ 0x2bf150 + 0x4a·0x10c]] whose guard hit routes msg 7 (taser)
//! [M NDrone2_DealWithObjHit 0x13fff0:269-300]. Master routed 0x4A/0x4C to msg 8 — a zero-damage
//! bullet hit: the guard flinched and alerted, nothing else. The taser family (DSTATE_Taser 0x4A,
//! DSTATE_Stunned 0x4B, DSTATE_Stunned_Recover 0x4C) was unported [M dronetype-tables.md:257].

use nf_game::player::Input;

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

fn dxz(a: [f32; 3], b: [f32; 3]) -> f32 { ((a[0] - b[0]).powi(2) + (a[2] - b[2]).powi(2)).sqrt() }

/// Bond 2.5 u in front of a placed guard from 07000005's start pocket, the Stunner equipped through
/// the real switch input, the aim converged on the guard WITHOUT the fire (steering like the laser
/// tests). Returns the guard index and the aim target.
fn setup_stunner_on_guard(scene: &mut nf_view::scene::Scene) -> (usize, [f32; 3]) {
    let gi = {
        let sim = scene.sim.as_ref().unwrap();
        *sim.drones.llist.iter().find(|&&i| {
            let dr = &sim.drones.list[i];
            dr.present && dr.enabled && !dr.corpse && dr.health > 0.0
                && (dr.pos[0] - sim.bond.pos[0]).powi(2) + (dr.pos[2] - sim.bond.pos[2]).powi(2) < 400.0
        }).expect("a placed guard within 20 u of Bond's start on 07000005")
    };
    let gpos = scene.sim.as_ref().unwrap().drones.list[gi].pos;
    let mut dev = nf_view::dev::Dev { scenario: None, ..Default::default() };
    dev.teleport(scene, [gpos[0] - 2.5, gpos[1], gpos[2]], 0.0);
    let draw = Input { switch_to: Some(0x4a), ..Default::default() };
    let mut ok = false;
    for _ in 0..60 {
        scene.sim.as_mut().unwrap().step_bond(draw, false, false, false);
        if scene.sim.as_ref().unwrap().gun.weapon == 0x4a { ok = true; break; }
    }
    assert!(ok, "the Stunner (0x4a) did not equip");
    // Settle: the draw finishes — the user sees the device in hand before pressing.
    for _ in 0..70 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    // Converge the aim on the guard without the fire.
    for _ in 0..60 {
        let (turn, look) = {
            let sim = scene.sim.as_ref().unwrap();
            let bp = sim.bond.pos;
            let g = sim.drones.list[gi].pos;
            let dx = g[0] - bp[0];
            let dz = g[2] - bp[2];
            let dy = g[1] - (bp[1] + sim.bond.eye - sim.bond.crouch_drop);
            let desired = dx.atan2(dz);
            let err = desired - sim.bond.yaw;
            let want_pitch = (dy / dxz(bp, g).max(0.5)).atan() / std::f32::consts::FRAC_PI_2;
            (err.clamp(-0.08, 0.08), (want_pitch - sim.bond.pitch).clamp(-0.08, 0.08))
        };
        scene.sim.as_mut().unwrap().step_bond(Input { turn, look, ..Default::default() }, false, false, false);
    }
    (gi, gpos)
}

fn steer_and_fire(scene: &mut nf_view::scene::Scene, gi: usize, fire: bool) {
    let (turn, look) = {
        let sim = scene.sim.as_ref().unwrap();
        let bp = sim.bond.pos;
        let g = sim.drones.list[gi].pos;
        let dx = g[0] - bp[0];
        let dz = g[2] - bp[2];
        let dy = g[1] - (bp[1] + sim.bond.eye - sim.bond.crouch_drop);
        let desired = dx.atan2(dz);
        let err = desired - sim.bond.yaw;
        let want_pitch = (dy / dxz(bp, g).max(0.5)).atan() / std::f32::consts::FRAC_PI_2;
        (err.clamp(-0.08, 0.08), (want_pitch - sim.bond.pitch).clamp(-0.08, 0.08))
    };
    scene.sim.as_mut().unwrap().step_bond(Input { turn, look, ..Default::default() }, fire, false, false);
}

/// The press arms the fire machine on the PRESS tick: one settled press enters view state 9 with the
/// fire script queued [M Player_WeaponFiring 0x1a5460:601-695 — the press with the timer run out sets
/// +0x936/+0x92C, spends through Player_RoundToFire and calls Player_SetFiringAnim 0x1a3c70; the
/// +0x44 = 0 record spawns the same tick 0x1a5460:688-690]. The release rings the fire script out
/// into the release script +0xB4 at 1.5× [M 0x1a2e38 case 0xb:499-507] and spawns nothing.
#[test]
fn the_stunner_fires_on_the_press_tick() {
    let iso = iso();
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005");
    let (gi, _) = setup_stunner_on_guard(&mut scene);
    {
        let sim = scene.sim.as_ref().unwrap();
        assert!(!sim.gun.fire_state, "idle before the press");
        assert_eq!(sim.view_script, 0x600048b, "the stunner's idle script [L ee_start_05 +0xa0]");
    }
    // ONE press tick.
    steer_and_fire(&mut scene, gi, true);
    {
        let sim = scene.sim.as_ref().unwrap();
        assert!(sim.gun.fire_state, "the press enters view state 9 (the fire machine) [M 0x1a3c70:50-54]");
        assert_eq!(sim.view_script, 0x600048a, "the fire script is queued on the press tick [M 0x1a3c70]");
        assert_eq!(sim.gun.clip, 99, "the press spends one round [M Player_RoundToFire 0x1a18e0]");
    }
    // The release: the fire script rings out (16 frames [L 0x0600048a family]) into the release
    // script 0x6000488 at 1.5×, then the idle. No further spend.
    let clip_after_press = scene.sim.as_ref().unwrap().gun.clip;
    for _ in 0..40 { steer_and_fire(&mut scene, gi, false); }
    let sim = scene.sim.as_ref().unwrap();
    assert_eq!(sim.gun.clip, clip_after_press, "the release spends nothing [M 0x1a5460:601-607]");
    assert!(!sim.gun.fire_state, "the fire machine ends through the release script [M 0x1a2e38 case 0xb]");
    assert_eq!(sim.view_script, 0x600048b, "back to the idle [M 0x1a2e38 case 0xc → 0]");
}

/// The tased guard falls into the unported taser family: TASER 0x4a → (weapon dropped) STUNNED 0x4b,
/// his health UNTOUCHED (the taser bullet's damage is 0.0 [L record +0x0c]) and the alert at status 2.
/// Master sent msg 8 (a zero-damage bullet hit) — the guard flinched forever, never fell.
#[test]
fn the_tased_guard_falls_drops_his_weapon_and_stays_down() {
    let iso = iso();
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005");
    let (gi, _) = setup_stunner_on_guard(&mut scene);
    let health0 = scene.sim.as_ref().unwrap().drones.list[gi].health;
    // Hold fire on the guard: the tase msgs accumulate (+0xBC) past 2·health and he falls
    // [M DroneFunc_HandleImpact 0x1469b8:58-131 → SetState(0x4a)].
    let mut fell = None;
    for t in 0..400u32 {
        steer_and_fire(&mut scene, gi, true);
        let st = scene.sim.as_ref().unwrap().drones.list[gi].state;
        if st == nf_game::drone::state::TASER { fell = Some(t); break; }
    }
    let fell = fell.expect("the guard falls into DSTATE_Taser 0x4a under a held tase [M 0x1469b8 case 7]");
    eprintln!("fell at hold tick {fell}");
    {
        let sim = scene.sim.as_ref().unwrap();
        let dr = &sim.drones.list[gi];
        assert!((dr.health - health0).abs() < 0.01, "the tase deals no damage [L record 0x4a +0x0c = 0.0]");
        assert!(dr.alert >= 0.66, "AlertStatusSet(2) [M 0x162b98:42]");
    }
    // The held tase keeps him in the family; the weapon drops at the TASER enter
    // [M 0x162b98:27-34 DropWeapon + ChangeWeapon(1) for non-civilians].
    for _ in 0..30 { steer_and_fire(&mut scene, gi, true); }
    {
        let sim = scene.sim.as_ref().unwrap();
        let dr = &sim.drones.list[gi];
        assert!(matches!(dr.state, nf_game::drone::state::TASER), "the held tase re-arms the fall timer [M 0x162b98 case 7]");
        assert_eq!(dr.weapon, 1, "the tased guard drops to weapon 1 [M 0x162b98:31-32]");
    }
    // Release and let the SM run: the TASER timer (FRAME_RATE_DIV·30 = 1800) expires into
    // STUNNED 0x4b [M 0x162b98 case 0xc → SetState(0x4b)]; Bond stands 2.5 u away — within the
    // AnyoneNear(2.0→<4.0) band, so the STUNNED timer re-arms and he stays down.
    for _ in 0..2200 { steer_and_fire(&mut scene, gi, false); }
    {
        let sim = scene.sim.as_ref().unwrap();
        let dr = &sim.drones.list[gi];
        assert_eq!(dr.state, nf_game::drone::state::STUNNED,
            "the taser twitch ends into DSTATE_Stunned 0x4b [M 0x162b98 case 0xc]");
        assert!(matches!(dr.state, nf_game::drone::state::STUNNED | nf_game::drone::state::STUNNED_RECOVER),
            "he stays in the stunned family while Bond is near [M 0x162eb0 case 0xc]");
    }
}

/// Sister: the P2K's bullet hit keeps the old msg-8 route — real damage, and the taser family
/// never entered by a gun bullet [M 0x13fff0:276-300 — the 0x4A/0x4C ids pick msg 7, everything
/// else keeps Drone_BulletHit].
#[test]
fn a_p2k_hit_is_not_a_tase() {
    let iso = iso();
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005");
    let gi = {
        let sim = scene.sim.as_ref().unwrap();
        *sim.drones.llist.iter().find(|&&i| {
            let dr = &sim.drones.list[i];
            dr.present && dr.enabled && !dr.corpse && dr.health > 0.0
                && (dr.pos[0] - sim.bond.pos[0]).powi(2) + (dr.pos[2] - sim.bond.pos[2]).powi(2) < 400.0
        }).expect("a placed guard")
    };
    let gpos = scene.sim.as_ref().unwrap().drones.list[gi].pos;
    let mut dev = nf_view::dev::Dev { scenario: None, ..Default::default() };
    dev.teleport(&mut scene, [gpos[0] - 2.5, gpos[1], gpos[2]], 0.0);
    let draw = Input { switch_to: Some(0x02), ..Default::default() };
    let mut ok = false;
    for _ in 0..60 {
        scene.sim.as_mut().unwrap().step_bond(draw, false, false, false);
        if scene.sim.as_ref().unwrap().gun.weapon == 0x02 { ok = true; break; }
    }
    assert!(ok, "the P2K did not equip");
    for _ in 0..70 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    let health0 = scene.sim.as_ref().unwrap().drones.list[gi].health;
    // A one-tick press at the guard, then settle: the bullet hits (or dies at 5.0) — either way
    // the guard never enters the taser family, and real damage lands (or he dies outright).
    for _ in 0..8 { steer_and_fire(&mut scene, gi, true); }
    for _ in 0..60 { steer_and_fire(&mut scene, gi, false); }
    let sim = scene.sim.as_ref().unwrap();
    let dr = &sim.drones.list[gi];
    assert!(dr.health < health0 || dr.corpse || dr.health <= 0.0,
        "the P2K hit applies real damage (health {health0} → {})", dr.health);
    assert!(!matches!(dr.state,
            nf_game::drone::state::TASER | nf_game::drone::state::STUNNED | nf_game::drone::state::STUNNED_RECOVER),
        "a P2K hit never enters the taser family [M 0x13fff0:276-300]");
}
