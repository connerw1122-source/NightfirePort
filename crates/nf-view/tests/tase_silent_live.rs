//! Playtest 1016: "literally cant even stun the starting guard without him shooting me instantly before he can
//! even turn." The held Stunner was heard as a P2K gunshot every tick: the port's hearing used a hardcoded
//! weapon→sound table whose default was 0x403 (weight 65, outer 40), while the game's guards hear the fire
//! script's own op-1 `Sound_Play3D` [M AnimProcessScriptCmds 0x1c1b38 op 1; Sound_Alertness 0x1ddc48 skips
//! looped sounds]. The Stunner's fire script has no gunshot, so the tase is silent; the guard's alertness
//! stays put, his reaction time stays (59 ticks on 07000005 at alertness 0 [M 0x14ac60:15-24]), and the tase
//! drops him first (accumulator past 2·health [M 0x1469b8:126-130]).
//!
//! (The setup below is fix_stunner_press_live's, with Bond placed BEHIND the guard.)
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
    // Behind the guard: opposite his facing (yaw convention: forward = (sin yaw, cos yaw)).
    let gyaw = scene.sim.as_ref().unwrap().drones.list[gi].yaw;
    eprintln!("guard {gi} at {:?} yaw {:.2} state {:#x}", gpos, gyaw, scene.sim.as_ref().unwrap().drones.list[gi].state);
    dev.teleport(scene, [gpos[0] - 2.2 * gyaw.sin(), gpos[1], gpos[2] - 2.2 * gyaw.cos()], gyaw);
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


#[test]
fn a_guard_tased_from_behind_goes_down_without_shooting_bond() {
    let iso = iso();
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005");
    let (gi, _) = setup_stunner_on_guard(&mut scene);
    let h0 = scene.sim.as_ref().unwrap().drones.health.health;
    let mut fell = None;
    for t in 0..120u32 {
        steer_and_fire(&mut scene, gi, true);
        let sim = scene.sim.as_ref().unwrap();
        assert_eq!(sim.drones.health.health, h0, "tick {t}: the tased guard shot Bond (state {:#x}, alert {:.2})",
            sim.drones.list[gi].state, sim.drones.list[gi].alert);
        if sim.drones.list[gi].state == nf_game::drone::state::TASER { fell = Some(t); break; }
    }
    let fell = fell.expect("the guard falls into DSTATE_Taser under a held tase [M 0x1469b8 case 7]");
    assert!(fell <= 2 * scene.sim.as_ref().unwrap().drones.list[gi].health as u32 + 2, "fell at {fell}");
    assert!(scene.sim.as_ref().unwrap().drones.list[gi].alert < 1.0, "the silent tase never maxed his alertness before the fall");
}

/// Sister: a real gunshot behind an idle guard is still heard — the P2K's fire script plays 0x403 (weight 65,
/// inner 2, outer 40 [L SFXOutputData]) and that play feeds the hearing.
#[test]
fn a_p2k_shot_behind_a_guard_still_alerts_him() {
    let iso = iso();
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005");
    let (gi, _) = setup_stunner_on_guard(&mut scene);
    let draw = Input { switch_to: Some(0x02), ..Default::default() };
    for _ in 0..60 {
        scene.sim.as_mut().unwrap().step_bond(draw, false, false, false);
        if scene.sim.as_ref().unwrap().gun.weapon == 0x02 { break; }
    }
    for _ in 0..70 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    let a0 = scene.sim.as_ref().unwrap().drones.list[gi].alert;
    // One shot aimed away from him (straight up), then a few ticks for the hearing to land.
    scene.sim.as_mut().unwrap().step_bond(Input { look: 1.0, ..Default::default() }, true, false, false);
    for _ in 0..6 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    let a1 = scene.sim.as_ref().unwrap().drones.list[gi].alert;
    assert!(a1 > a0 + 0.3, "the gunshot was heard: alert {a0:.2} -> {a1:.2}");
}
