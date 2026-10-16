//! stun-ai-recover (654): the stunned guard completes the animation and returns to combat, and
//! the re-hit rules behave like the game. The playtest 1015-1738 (07000005, the Stunner): guards
//! "get stunned infinitely and don't complete the animation".
//!
//! Ground truth [M]:
//! - the tase route: `NDrone2_DealWithObjHit` 0x13fff0:269-300 — the stunner's bullets 0x4A/0x4C
//!   pick msg 7; the base case 7 `DroneFunc_HandleImpact` 0x1469b8:58-131 — the accumulator
//!   +0xBC grows by FRAME_RATE_MUL per msg, past 2·health the drone falls into DSTATE_Taser
//!   0x4a (the accumulator's ONLY writer in the whole decompile is that grow — it never resets,
//!   so a once-fully-tased guard stays "charged" for the game's lifetime);
//! - DSTATE_Taser 0x162b98: ENTER stores RecoverTime (+0xCE0 [M 0x147188 cases 0x4a/0x4c —
//!   FRAME_RATE_INT·10 → 600·0.01·(200−0xB9) = 1200 ticks at the run-time stat 0 [L 07000005]]),
//!   rolls the fall anim 0x31/0x32 by Rand(2) (+0xCD8), arms the twitch timer
//!   FRAME_RATE_DIV·30 = 1800; case 7 (a re-tase) re-arms the same 1800; case 0xc falls to
//!   DSTATE_Stunned when the anim is still in the 0x31..0x32 band, else re-arms 300;
//! - DSTATE_Stunned 0x162eb0: ENTER arms +0xCC4 = now + +0xCE0 (1200), sets the down flag +0x2f
//!   (the EXIT clears it [M case 1/2]) and plays the twitch 0x30/0x2f by the stored fall anim;
//!   case 7 re-falls (Taser 0x4a) UNLESS the stored anim is 0x32 — then it falls through to
//!   case 0x19's re-arm (RecoverTime of the NEW hitdata); case 0xc: `DroneFunc_AnyoneNear(2.0)`
//!   (< 4.0 u) re-arms 300, else SetState(0x4c);
//! - DSTATE_Stunned_Recover 0x1630b8: ENTER plays the get-up 0x76 with the follow-on AI state
//!   0x56 (Attack, non-civilians) [M case 1]; case 7/0x19 routes back through HandleImpact (the
//!   accumulator grows — for a charged guard the re-tase re-falls him at once); case 8/9 death;
//! - the downed guard is SHOOTABLE through `NDrone2_Collision` 0x140e90's down branch: the low
//!   sphere (radius 0.3) replaces the standing volume while +0x2f is set [M :65-90] — the re-tase
//!   dart reaches him (the port's downed-pose bone boxes left the aimed ray uncovered, which is
//!   why the playtest's re-hit never landed).
//!
//! The stun lifecycle below runs on the real input path (`nf_view::load` + `Sim::step_bond`, the
//! stunner equipped through the switch input) on 07000005, like the playtest. The `Dev::teleport`
//! placements are the dev helper the other live tests use; they happen only at the decision
//! moments (a teleport early in the wait can hand the fallen guard to the level's proximity
//! spawner, whose despawn is the level's own behaviour — the playtest's user never leaves).

use nf_game::drone::{state, anim_state};
use nf_game::player::Input;

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

fn dxz(a: [f32; 3], b: [f32; 3]) -> f32 { ((a[0] - b[0]).powi(2) + (a[2] - b[2]).powi(2)).sqrt() }

fn steer(scene: &mut nf_view::scene::Scene, gi: usize, fire: bool) {
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

/// Bond at `dist` u WEST of the guard (the open courtyard side), aimed at him when asked. The
/// teleport rebuilds Bond (`Bond::stand_at`).
fn place_bond(scene: &mut nf_view::scene::Scene, gi: usize, dist: f32, aim: bool) {
    let g = scene.sim.as_ref().unwrap().drones.list[gi].pos;
    let mut dev = nf_view::dev::Dev { scenario: None, ..Default::default() };
    dev.teleport(scene, [g[0] - dist, g[1], g[2]], 0.0);
    if aim {
        for _ in 0..40 { steer(scene, gi, false); }
    }
}

/// Bond 2.5 u in front of a placed guard from 07000005's start pocket, the Stunner equipped
/// through the real switch input, the aim converged (the fix-stunner-press setup).
fn setup_stunner_on_guard(scene: &mut nf_view::scene::Scene) -> usize {
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
    for _ in 0..70 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    for _ in 0..60 { steer(scene, gi, false); }
    gi
}

/// Hold the beam until the guard falls into DSTATE_Taser 0x4a [M 0x1469b8 case 7], keep the
/// re-arms for 30 more ticks (case 7), then release.
fn stun_to_taser(scene: &mut nf_view::scene::Scene, gi: usize) {
    let mut fell = None;
    for t in 0..400u32 {
        steer(scene, gi, true);
        if scene.sim.as_ref().unwrap().drones.list[gi].state == state::TASER { fell = Some(t); break; }
    }
    let fell = fell.expect("the guard falls into DSTATE_Taser 0x4a under a held tase [M 0x1469b8 case 7]");
    for _ in 0..30 { steer(scene, gi, true); }
    steer(scene, gi, false);
    eprintln!("[stun] fell at hold tick {fell}");
}

fn is_combat(s: u8) -> bool {
    matches!(s, state::ATTACK | state::AIM_STAND_FIRE | state::COMBAT_NO_ROUTE)
}

/// The full game-timer ride: TASER 0x4a (the 1800-tick twitch timer) → STUNNED 0x4b → with
/// nobody near, STUNNED_RECOVER 0x4c → the get-up 0x76 completes into the combat family 0x56.
/// The playtest's "stunned infinitely and don't complete the animation" is this walk stalled;
/// the game's rules walk it to the end.
#[test]
fn the_stunned_guard_rides_the_game_timers_and_returns_to_combat() {
    let iso = iso();
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005");
    let gi = setup_stunner_on_guard(&mut scene);
    stun_to_taser(&mut scene, gi);
    // TASER → STUNNED at the twitch timer's expiry (the trace: rel+1798) [M 0x162b98 case 0xc].
    let mut stunned_at = None;
    for t in 0..2400u32 {
        steer(&mut scene, gi, false);
        if scene.sim.as_ref().unwrap().drones.list[gi].state == state::STUNNED { stunned_at = Some(t); break; }
    }
    let stunned_at = stunned_at.expect("the twitch ends into DSTATE_Stunned 0x4b [M 0x162b98 case 0xc]");
    assert!(stunned_at >= 1700, "the fall ran its 1800-tick twitch timer, not an early exit (rel+{stunned_at})");
    let stun_frame = scene.sim.as_ref().unwrap().drones.frame;
    // Bond steps 8 u back just before the RecoverTime timer fires (stun_frame + 1200): outside
    // the AnyoneNear band. The stun timers are drone-side and keep running.
    while scene.sim.as_ref().unwrap().drones.frame < stun_frame + 1150 { steer(&mut scene, gi, false); }
    place_bond(&mut scene, gi, 8.0, false);
    // STUNNED → STUNNED_RECOVER when nobody is near: the RecoverTime timer 1200 (+ slack)
    // [M 0x162eb0 case 0xc → 0x4c].
    let mut recovered_at = None;
    for t in 0..1700u32 {
        steer(&mut scene, gi, false);
        if scene.sim.as_ref().unwrap().drones.list[gi].state == state::STUNNED_RECOVER { recovered_at = Some(t); break; }
    }
    let recovered_at = recovered_at.expect("nobody near → Stunned_Recover 0x4c [M 0x162eb0 case 0xc]");
    assert!(recovered_at <= 600, "the recover lands at the timer's expiry, not an instant drop (away+{recovered_at})");
    // The get-up 0x76 completes with its follow-on Attack 0x56 [M 0x1630b8 case 1]: the guard is
    // BACK IN COMBAT (the trace: +425 ticks after the recover).
    let mut combat_at = None;
    for t in 0..900u32 {
        steer(&mut scene, gi, false);
        if is_combat(scene.sim.as_ref().unwrap().drones.list[gi].state) { combat_at = Some(t); break; }
    }
    let sim = scene.sim.as_ref().unwrap();
    let in_log = sim.drones.list[gi].log.iter().any(|&(_, s)| is_combat(s));
    assert!(combat_at.is_some() || in_log,
        "the get-up 0x76 completes into the combat family [M 0x1630b8 case 1 CallAnim(0x76, 0, 0x56)] — log {:?}",
        sim.drones.list[gi].log);
    eprintln!("[stun] stunned at rel+{stunned_at}, recovered at away+{recovered_at}, combat at {:?} — the animation completed", combat_at);
}

/// The re-hit rules with the player near: AnyoneNear(2.0 → < 4.0 u) keeps the stunned guard down
/// (the timer re-arms 300 [M 0x162eb0 case 0xc]), and one re-tase on the downed guard re-falls
/// him (stored fall anim != 0x32) or re-arms the stun (stored anim == 0x32, the case-0x19
/// fall-through) [M 0x162eb0 case 7 → 0x19]. The walk still completes after the re-hit.
#[test]
fn the_player_near_keeps_him_down_and_a_retase_behaves_like_the_game() {
    let iso = iso();
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005");
    let gi = setup_stunner_on_guard(&mut scene);
    stun_to_taser(&mut scene, gi);
    let mut stunned_at = None;
    for t in 0..2400u32 {
        steer(&mut scene, gi, false);
        if scene.sim.as_ref().unwrap().drones.list[gi].state == state::STUNNED { stunned_at = Some(t); break; }
    }
    let stunned_at = stunned_at.expect("the twitch ends into DSTATE_Stunned 0x4b");
    let stun_frame = scene.sim.as_ref().unwrap().drones.frame;
    // Bond stays where he tased from (2.5 u — inside AnyoneNear's band; the playtest's player
    // stands over the guard too). Ride PAST the RecoverTime expiry (stun_frame + 1200): the
    // check must re-arm, not recover [M 0x162eb0 case 0xc].
    while scene.sim.as_ref().unwrap().drones.frame < stun_frame + 1350 {
        steer(&mut scene, gi, false);
        let dr = &scene.sim.as_ref().unwrap().drones.list[gi];
        assert_ne!(dr.state, state::STUNNED_RECOVER, "the player within 4 u keeps the guard down [M 0x162eb0 case 0xc AnyoneNear(2.0)]");
        assert_ne!(dr.state, state::TASER, "no re-fall while nobody tases");
    }
    {
        let dr = &scene.sim.as_ref().unwrap().drones.list[gi];
        assert_eq!(dr.state, state::STUNNED, "still down past the RecoverTime expiry with the player near [M 0x162eb0 case 0xc]");
        let tc = dr.timer_c.expect("the kept-down arm re-armed the timer");
        assert!(tc > stun_frame + 1200, "the re-arm pushed the timer past the expiry ({tc} vs {})", stun_frame + 1200);
        eprintln!("[stun] kept down past frame {} with the player at 2.5 u (tc {tc})", stun_frame + 1200);
    }
    // ONE re-tase tap on the downed guard. The stored fall anim decides the arm [M 0x162eb0
    // case 7]: != 0x32 → SetState(0x4a) (a fresh fall anim rolls); == 0x32 → case 0x19's re-arm
    // (he stays STUNNED, the RecoverTime timer re-arms).
    place_bond(&mut scene, gi, 2.5, false); // the placement rebuilds Bond facing +Z
    // The placement rebuilt Bond alive, but the sim's health tracker + death sequence (armed by
    // the courtyard fire while the test watched the timers) still gate his trigger AND his input
    // — restore both like the placement restored the body, THEN converge the aim (a death-gated
    // Bond takes no turn input) and tap through the real input path.
    scene.sim.as_mut().unwrap().drones.health.health = 100.0;
    scene.sim.as_mut().unwrap().drones.death.active = false;
    for _ in 0..60 { steer(&mut scene, gi, false); }
    let stored = scene.sim.as_ref().unwrap().drones.list[gi].tase_fall_anim;
    let tc_before = scene.sim.as_ref().unwrap().drones.list[gi].timer_c;
    steer(&mut scene, gi, true);
    steer(&mut scene, gi, false);
    let mut seen = None;
    for t in 0..60u32 {
        steer(&mut scene, gi, false);
        let dr = &scene.sim.as_ref().unwrap().drones.list[gi];
        if stored != anim_state::TASER_FALL_B && dr.state == state::TASER { seen = Some((t, dr.anim.state)); break; }
    }
    let dr = &scene.sim.as_ref().unwrap().drones.list[gi];
    if stored != anim_state::TASER_FALL_B {
        let (at, anim) = seen.expect("the re-tase re-falls the guard (stored fall anim 0x31) [M 0x162eb0 case 7]");
        assert!(matches!(anim, anim_state::TASER_FALL_A | anim_state::TASER_FALL_B),
            "the re-fall rolls a fresh fall anim, got {anim:#x}");
        eprintln!("[stun] re-tase re-fell him into TASER at settle+{at} (fresh anim {anim:#x})");
    } else {
        assert_eq!(dr.state, state::STUNNED, "stored fall anim 0x32: the re-tase re-arms, no re-fall [M 0x162eb0 case 7 → 0x19]");
        let tc = dr.timer_c.expect("the re-arm re-arms the timer");
        assert!(tc > tc_before.unwrap_or(0), "the re-arm pushed the RecoverTime timer ({tc_before:?} → {tc}) [M 0x162eb0 case 0x19]");
        eprintln!("[stun] re-tase re-armed the stun (stored 0x32), tc {tc_before:?} → {tc}");
    }
    // And the walk still completes: Bond steps away, the guard rides the timers back to combat.
    place_bond(&mut scene, gi, 8.0, false);
    let mut combat = false;
    for _ in 0..3800u32 {
        steer(&mut scene, gi, false);
        let s = scene.sim.as_ref().unwrap().drones.list[gi].state;
        if is_combat(s) { combat = true; break; }
    }
    let in_log = scene.sim.as_ref().unwrap().drones.list[gi].log.iter().any(|&(_, s)| is_combat(s));
    assert!(combat || in_log, "after the re-tase the guard still completes the stun and returns to combat");
}
