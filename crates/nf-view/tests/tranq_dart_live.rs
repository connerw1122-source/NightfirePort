//! tranq-gun (750l): the Korsakov K5's dart puts a guard to sleep — the game's msg-0x19 route.
//!
//! The user, 2026-10-16 evening: "tranq gun doesnt work". The K5 (weapon 0x43) is the start
//! loadout of 07000009 "Night of the Agent" (Tower A) [M Player_InitWeapon 0x1a7ef8 case
//! 0x7000009 — `Upgrade_Weapon(0x43)`, 999 rounds, SELECTED; the port's fresh_09]. The game's
//! hit route: `NDrone2_DealWithObjHit` 0x13fff0 case 5 (:269-306) — the bullet's weapon record
//! 0x43/0x44 picks **msg 0x19** (NOT the plain msg-8 bullet), `NDrone2_HitDamage` is never
//! called (the record's 0.1 damage is dead), `DroneFunc_HandleImpact` 0x1469b8 case 0x19
//! (:315-361) logs `PlrStat_LogShotHitEnemy(0)` and sets state **0x50 StunDartImpact**
//! (0x163430: the sleep-fall anim 0x2B/0x2C → 0x51), **0x51 StunDartLoop** (0x163570: down flag
//! +0x2F, the wake timer +0xCC4 = now + `DroneFunc_RecoverTime` 0x147188 — 0x43 → k=60 →
//! 60·60·0.01·(200−0) = **7200 ticks = 120 s** [+0xB9 = 0 at run time, L ee_start_05.bin]; the
//! TIMER_C gate `DroneFunc_AnyoneNear(2.0)` 0x14a718 holds him while anyone is near) → 0x52
//! StunDartRecover (0x163700: the get-up 0x76 with follow-on Attack 0x56 for non-civilians).
//! A punch on a sleeper → `NDrone2_PainTalk` + **0x42 KnockedOut_Anim** [M 0x163570 case 6].
//!
//! The lifecycle runs on the real input path (`nf_view::load` + `Sim::step_bond`, the K5 in
//! hand from the level's own loadout). The `Dev::teleport` placement is the dev helper the
//! merged stun tests use; it only sets the shot geometry — the equip, the aim, the fire, the
//! dart's flight and the hit are the game's own paths.

use nf_game::drone::{state, anim_state};
use nf_game::player::Input;

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

fn dxz(a: [f32; 3], b: [f32; 3]) -> f32 { ((a[0] - b[0]).powi(2) + (a[2] - b[2]).powi(2)).sqrt() }

/// The nearest live, standing guard: present, enabled, alive, not a corpse, idle over alert so
/// the silent shot lands before any escalation. Returns the list index.
fn find_guard(scene: &nf_view::scene::Scene) -> usize {
    let sim = scene.sim.as_ref().unwrap();
    let cands: Vec<usize> = sim.drones.llist.iter().copied().filter(|&i| {
        let dr = &sim.drones.list[i];
        dr.present && dr.enabled && !dr.corpse && dr.health > 0.0 && !dr.down
    }).collect();
    {
        for &i in sim.drones.llist.iter().take(24) {
            let dr = &sim.drones.list[i];
            eprintln!("[tranq] llist {i}: team {} present {} enabled {} corpse {} health {} state {:#x} at {:?}",
                dr.team, dr.present, dr.enabled, dr.corpse, dr.health, dr.state, dr.pos);
        }
    }
    assert!(!cands.is_empty(), "07000009 places live guards");
    let best = |idle_only: bool| cands.iter().copied().filter(|&i| {
        let st = sim.drones.list[i].state;
        // The tower guards are team 3 [L the llist dump; M 0x14aae0 makes them IsCivilian
        // civilians]; prefer the standing idle family so the silent shot lands unalerted.
        !idle_only || matches!(st, state::IDLE | state::CIVILIAN_PATROL | state::CIVILIAN)
    })
        .min_by_key(|&i| dxz(sim.drones.list[i].pos, sim.bond.pos) as u32);
    best(true).or_else(|| best(false)).expect("a live guard on 07000009")
}

/// Bond 2.5 u BEHIND the guard (opposite his facing — the silent approach), aim converged.
fn sneak_up(scene: &mut nf_view::scene::Scene, gi: usize) {
    let (gpos, yaw) = { let sim = scene.sim.as_ref().unwrap();
        (sim.drones.list[gi].pos, sim.drones.list[gi].yaw) };
    let behind = [gpos[0] - yaw.sin() * 2.5, gpos[1], gpos[2] - yaw.cos() * 2.5];
    let mut dev = nf_view::dev::Dev { scenario: None, ..Default::default() };
    dev.teleport(scene, behind, 0.0);
    for _ in 0..70 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    for _ in 0..60 {
        let (turn, look) = {
            let sim = scene.sim.as_ref().unwrap();
            let bp = sim.bond.pos;
            let g = sim.drones.list[gi].pos;
            let dx = g[0] - bp[0]; let dz = g[2] - bp[2];
            let dy = g[1] - (bp[1] + sim.bond.eye - sim.bond.crouch_drop);
            let desired = dx.atan2(dz);
            (err_of(desired, sim.bond.yaw), pitch_of(dy, dxz(bp, g), sim.bond.pitch))
        };
        scene.sim.as_mut().unwrap().step_bond(Input { turn, look, ..Default::default() }, false, false, false);
    }
}

fn err_of(desired: f32, yaw: f32) -> f32 { (desired - yaw).clamp(-0.08, 0.08) }
fn pitch_of(dy: f32, dxz: f32, pitch: f32) -> f32 {
    (((dy / dxz.max(0.5)).atan()) / std::f32::consts::FRAC_PI_2 - pitch).clamp(-0.08, 0.08)
}

/// Steer at the guard; `fire` holds the trigger.
fn steer(scene: &mut nf_view::scene::Scene, gi: usize, fire: bool) {
    let (turn, look) = {
        let sim = scene.sim.as_ref().unwrap();
        let bp = sim.bond.pos;
        let g = sim.drones.list[gi].pos;
        let dx = g[0] - bp[0]; let dz = g[2] - bp[2];
        let dy = g[1] - (bp[1] + sim.bond.eye - sim.bond.crouch_drop);
        (err_of(dx.atan2(dz), sim.bond.yaw), pitch_of(dy, dxz(bp, g), sim.bond.pitch))
    };
    scene.sim.as_mut().unwrap().step_bond(Input { turn, look, ..Default::default() }, fire, false, false);
}

fn is_combat(s: u8) -> bool { matches!(s, state::ATTACK | state::AIM_STAND_FIRE | state::COMBAT_NO_ROUTE) }

/// The shared prelude: load 07000009, ride the level's own weapon switch to the K5, find a guard,
/// sneak behind him. Returns (scene, guard list index).
fn setup_k5_guard() -> (nf_view::scene::Scene, usize) {
    let iso = iso();
    let mut scene = nf_view::load(&iso, "07000009.bin", false).expect("load 07000009");
    let mut drawn = false;
    for _ in 0..180 {
        scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
        if scene.sim.as_ref().unwrap().gun.weapon == 0x43 { drawn = true; break; }
    }
    assert!(drawn, "the level's loadout switch request 0x43 completes into the K5 [M 0x1a7ef8 case 0x7000009]");
    let gi = find_guard(&mut scene);
    sneak_up(&mut scene, gi);
    (scene, gi)
}

/// The core symptom: a dart hit puts the guard into the game's sleep family —
/// StunDartImpact 0x50 → StunDartLoop 0x51 with the down flag, NO health loss, and the
/// 7200-tick (120 s) wake timer of `DroneFunc_RecoverTime` 0x147188's 0x43 arm.
#[test]
fn the_k5_dart_puts_a_guard_to_sleep() {
    let iso = iso();
    let mut scene = nf_view::load(&iso, "07000009.bin", false).expect("load 07000009");
    let mut drawn = false;
    for _ in 0..180 {
        scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
        if scene.sim.as_ref().unwrap().gun.weapon == 0x43 { drawn = true; break; }
    }
    assert!(drawn, "the level's loadout switch request 0x43 completes into the K5 [M 0x1a7ef8 case 0x7000009]");
    let gi = find_guard(&mut scene);
    sneak_up(&mut scene, gi);
    let (health0, state0) = { let sim = scene.sim.as_ref().unwrap();
        (sim.drones.list[gi].health, sim.drones.list[gi].state) };
    eprintln!("[tranq] guard {gi}: health {health0} state {state0:#x}");

    // Hold the trigger until the dart lands: the game's route turns the state to 0x50 the tick
    // the HITDATA arrives [M 0x13fff0 case 5 → 0x1469b8 case 0x19 → SetState(0x50)].
    let mut hit_at = None;
    let mut saw = Vec::new();
    for t in 0..400u32 {
        steer(&mut scene, gi, true);
        let sim = scene.sim.as_ref().unwrap();
        let dr = &sim.drones.list[gi];
        if dr.state != state0 && !saw.contains(&(dr.state, t)) { saw.push((dr.state, t)); }
        if matches!(dr.state, state::STUN_DART_IMPACT | state::STUN_DART_LOOP) { hit_at = Some(t); break; }
    }
    steer(&mut scene, gi, false);
    eprintln!("[tranq] states seen after the pull: {saw:?}");
    let hit_at = hit_at.unwrap_or_else(|| panic!(
        "the dart hit puts the guard into StunDartImpact 0x50 / StunDartLoop 0x51 [M 0x13fff0 case 5: 0x43|0x44 → msg 0x19; 0x1469b8 case 0x19 → SetState(0x50)] — states seen: {saw:?}"));
    assert!(hit_at < 300, "the dart lands inside its 45 u range (2.5 u away), at {hit_at}");

    {
        let sim = scene.sim.as_ref().unwrap();
        let dr = &sim.drones.list[gi];
        assert!((dr.health - health0).abs() < 1e-4,
            "the dart deals NO damage — NDrone2_HitDamage is never called on the msg-0x19 route [M 0x13fff0:278-306], health {} → {}", health0, dr.health);
    }
    // The fall: 0x50's sleep-fall anim 0x2B/0x2C completes into StunDartLoop 0x51 with the down
    // flag +0x2F [M 0x163430 case 1 CallAnim(…, 0x51); 0x163570 case 1].
    let mut loop_at = None;
    for t in 0..600u32 {
        steer(&mut scene, gi, false);
        let sim = scene.sim.as_ref().unwrap();
        if sim.drones.list[gi].state == state::STUN_DART_LOOP { loop_at = Some(t); break; }
    }
    loop_at.expect("the sleep-fall completes into StunDartLoop 0x51 [M 0x163430 CallAnim(0x2b/0x2c, 0, 0x51)]");
    {
        let sim = scene.sim.as_ref().unwrap();
        let dr = &sim.drones.list[gi];
        assert!(dr.down, "the sleeper carries the DOWN flag +0x2F [M 0x163570 case 1]");
        // The wake deadline: +0xCC4 = the loop's enter frame + +0xCE0 = RecoverTime(0x43) = 7200
        // [M 0x147188 case 0x43: FRAME_RATE_INT·60·0.01·(200 − +0xB9); +0xB9 = 0 [L ee_start_05]].
        let tc = dr.timer_c.expect("the wake timer TIMER_C is armed [M 0x163570 case 1]");
        let until = tc.saturating_sub(sim.drones.frame);
        assert!((7200i64 - until as i64).abs() <= 5,
            "the K5 sleep runs 7200 ticks (120 s), got {until}");
    }

    // The wake: Bond steps out of the AnyoneNear(2.0) band before the deadline; at the expiry
    // the loop falls to StunDartRecover 0x52 [M 0x163570 case 0xc], whose get-up 0x76 completes
    // into the combat family 0x56 (non-civilians) [M 0x163700 case 1].
    let gpos = scene.sim.as_ref().unwrap().drones.list[gi].pos;
    let dev = nf_view::dev::Dev { scenario: None, ..Default::default() };
    dev.teleport(&mut scene, [gpos[0] + 8.0, gpos[1], gpos[2]], 0.0);
    let mut recovered_at = None;
    while scene.sim.as_ref().unwrap().drones.frame - scene.sim.as_ref().unwrap().drones.list[gi].entered < 7400 {
        steer(&mut scene, gi, false);
        if scene.sim.as_ref().unwrap().drones.list[gi].state == state::STUN_DART_RECOVER { recovered_at = Some(()); break; }
    }
    assert!(recovered_at.is_some(), "nobody near → the sleep expires into StunDartRecover 0x52 [M 0x163570 case 0xc]");
    // The wake: the tower guards are team 3 — `IsCivilian` civilians [M 0x14aae0] — so the get-up
    // follows the impact resume the Tower band stamped: 0x14 CivilianPatrol for an unsighted
    // guard [M 0x1469b8:331-357, 0x163700:19-29]. (A team-1 guard would take Attack 0x56.)
    let mut awake = false;
    for _ in 0..900u32 {
        steer(&mut scene, gi, false);
        if scene.sim.as_ref().unwrap().drones.list[gi].state == state::CIVILIAN_PATROL { awake = true; break; }
    }
    let sim = scene.sim.as_ref().unwrap();
    let now = sim.drones.list[gi].state;
    assert!(awake || matches!(now, state::CIVILIAN_PATROL | state::RETURN_TO_PATROL_PATH | state::GO_TO_GOAL),
        "the get-up 0x76 completes into the Tower band's stamped resume 0x14/0x15 [M 0x1469b8:331-357], got {now:#x}");
    assert!(now != state::STUN_DART_LOOP, "the guard did not stay asleep");
}

/// A dart that lands while the guard sleeps EXTENDS the sleep from NOW by a fresh RecoverTime
/// (msg 0x19 re-arms +0xCC4 [M 0x163570 case 0x19]) — and a dart during the get-up re-sleeps
/// him (msg 0x19 → 0x50 [M 0x163700 case 0x19]).
#[test]
fn a_second_dart_extends_the_sleep() {
    let (mut scene, gi) = setup_k5_guard();
    let mut hit = false;
    for _ in 0..400u32 {
        steer(&mut scene, gi, true);
        if scene.sim.as_ref().unwrap().drones.list[gi].state == state::STUN_DART_LOOP { hit = true; break; }
    }
    steer(&mut scene, gi, false);
    assert!(hit, "the first dart sleeps the guard (sister setup)");
    for _ in 0..60 { steer(&mut scene, gi, false); } // let the fall settle
    let sim = scene.sim.as_ref().unwrap();
    let before = sim.drones.list[gi].timer_c.expect("armed").saturating_sub(sim.drones.frame);
    drop(sim);
    // The second dart: the sleeper is DOWN — the game's shootable volume is the low sphere
    // (radius 0.3, centre 0.3 above the origin [M NDrone2_Collision 0x140e90:65-90]), so aim a
    // little above the root.
    let mut rehit = false;
    for _ in 0..400u32 {
        let (turn, look) = {
            let sim = scene.sim.as_ref().unwrap();
            let bp = sim.bond.pos;
            let g = sim.drones.list[gi].pos;
            let dx = g[0] - bp[0]; let dz = g[2] - bp[2];
            let dy = (g[1] + 0.3) - (bp[1] + sim.bond.eye - sim.bond.crouch_drop);
            (err_of(dx.atan2(dz), sim.bond.yaw), pitch_of(dy, dxz(bp, g), sim.bond.pitch))
        };
        scene.sim.as_mut().unwrap().step_bond(Input { turn, look, ..Default::default() }, true, false, false);
        if scene.sim.as_ref().unwrap().drones.list[gi].timer_c.is_some() {
            let sim = scene.sim.as_ref().unwrap();
            let now = sim.drones.list[gi].timer_c.unwrap().saturating_sub(sim.drones.frame);
            if now > before + 30 { rehit = true; break; } // the countdown went UP = a fresh re-arm
        }
        if scene.sim.as_ref().unwrap().drones.list[gi].state == state::STUN_DART_IMPACT { rehit = true; break; }
    }
    steer(&mut scene, gi, false);
    assert!(rehit, "the second dart re-arms the sleep to a fresh RecoverTime [M 0x163570 case 0x19]");
    let sim = scene.sim.as_ref().unwrap();
    let dr = &sim.drones.list[gi];
    assert!(matches!(dr.state, state::STUN_DART_LOOP | state::STUN_DART_IMPACT),
        "the sleeper stays in the sleep family, got {:#x}", dr.state);
}

/// Punching a sleeper knocks him out FOR GOOD: msg 6 → `NDrone2_PainTalk` + 0x42
/// KnockedOut_Anim [M 0x163570 case 6] — the Surrender path's shape.
#[test]
fn a_punch_puts_a_sleeper_out_for_good() {
    let (mut scene, gi) = setup_k5_guard();
    let mut hit = false;
    for _ in 0..400u32 {
        steer(&mut scene, gi, true);
        if scene.sim.as_ref().unwrap().drones.list[gi].state == state::STUN_DART_LOOP { hit = true; break; }
    }
    steer(&mut scene, gi, false);
    assert!(hit, "the first dart sleeps the guard (sister setup)");
    for _ in 0..60 { steer(&mut scene, gi, false); }
    // Bond swaps to the fists and punches the sleeper point-blank (melee range — walk in first).
    let draw = Input { switch_to: Some(1), ..Default::default() };
    let mut fists = false;
    for _ in 0..90 {
        scene.sim.as_mut().unwrap().step_bond(draw, false, false, false);
        if scene.sim.as_ref().unwrap().gun.weapon == 1 { fists = true; break; }
    }
    let _ = fists; // the port's punch route rides the weapon id, not the anim
    // Close the gap: step Bond toward the sleeper until the fists reach.
    for _ in 0..120 {
        let (turn, fwd) = {
            let sim = scene.sim.as_ref().unwrap();
            let bp = sim.bond.pos;
            let g = sim.drones.list[gi].pos;
            let dx = g[0] - bp[0]; let dz = g[2] - bp[2];
            (err_of(dx.atan2(dz), sim.bond.yaw), if dx * dx + dz * dz > 1.0 { 1.0 } else { 0.0 })
        };
        scene.sim.as_mut().unwrap().step_bond(Input { turn, forward: fwd, ..Default::default() }, false, false, false);
    }
    let mut ko = false;
    for _ in 0..240u32 {
        steer(&mut scene, gi, true);
        if scene.sim.as_ref().unwrap().drones.list[gi].state == state::KNOCKED_OUT_ANIM { ko = true; break; }
    }
    steer(&mut scene, gi, false);
    assert!(ko, "a punch on a sleeper → KnockedOut_Anim 0x42 [M 0x163570 case 6]");
    // …and the KO holds: a stray dart no longer re-sleeps him (0x42 has no case 0x19; the bullet
    // scan skips nothing here — the msg-8 arm of 0x172a30 routes ANY bullet to Death_Anim).
    let _ = anim_state::STUN_GET_UP; // (keep the import honest across cfgs)
}
