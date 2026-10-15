//! fix-guard-first-shot: 07000006 — "Shots are still happening instantly" (playtest 1014-1725,
//! real-input build). Through the REAL path (`nf_view::load` + `Sim::step_bond` with the window's
//! `Input`), Bond stands in the courtyard searchlight's sweep (the alarm path the user's walk-in
//! takes), the alarm wakes the guards [M PreDroneControl 0x148db0:51-61], and per guard we measure
//! FIRST SIGHT (the LOS tick) → first bullet (`Drone.shots`) against the game's chain:
//! - the reaction ack: `HaveOpponentSight` 0x175ef0:41-44 returns until `+0x270 >= ReactionTime`
//!   (0x14ac60's default arm = 10 ticks on 07000006) — no sight flag, no combat, no fire request;
//! - the post-ack choreography [L 06-firstsight.trace: d9 Idle→AIM_STAND +31f, d1 Idle→STEP_AIM
//!   +116f; first hits 70/128 frames after the jump] — never a same-tick shot;
//! - `DroneWeap_Fire` 0x17a8c0's burst schedule (`+0xBC4 = max(now, +0xBC8+BurstDelay+interval)`
//!   [M 0x179d40:134-142]).
//! Opt-in via NF_ISO.

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// The beam spot of the 06-searchlight trace's opening (Bond inside head #2's sweep at the
/// courtyard entrance; the game's alarm fired within ~1 s of standing there).
const BEAM_SPOT: [f32; 3] = [4.8, 42.09, -16.0];

#[test]
fn a_real_stand_in_the_beam_never_draws_a_shot_inside_the_game_s_reaction_chain() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000006.bin", false).expect("load 07000006");
    let sim = scene.sim.as_mut().unwrap();
    for _ in 0..300 { sim.step_bond(Default::default(), false, false, false); if !sim.disabled { break; } }
    assert!(!sim.disabled, "Bond is free after the intro");

    // Bond stands in the sweep, armed (the P2K), through the real input path.
    sim.bond = nf_game::player::Bond::stand_at(&sim.world, BEAM_SPOT, -2.2091);
    sim.gun.weapon = 2;
    sim.gun.switch_request = 2;
    sim.gun.switch_state = 0;
    let idle = nf_game::player::Input::default();

    let mut alarm = None;
    let mut first_los: Option<u32> = None;
    let mut first_jump: Option<u32> = None;
    let mut first_shot: Option<u32> = None;
    let mut shot_guard: Option<usize> = None;
    let mut sighted: std::collections::HashSet<usize> = std::collections::HashSet::new();
    let mut shot_sighted: bool = false;
    'run: for t in 0..900u32 {
        sim.step_bond(idle, false, false, false);
        if alarm.is_none() && sim.drones.channels.on(55) { alarm = Some(t); eprintln!("[beam] t={t} alarm ch55 up"); }
        for (i, dr) in sim.drones.list.iter().enumerate() {
            if !dr.present || !dr.enabled { continue; }
            if dr.los { sighted.insert(i); }
            if first_los.is_none() && dr.los && dr.dist < 30.0 {
                first_los = Some(t);
                eprintln!("[beam] t={t} guard {i} FIRST LOS (dist {:.1} state {:#x} vis {:.2})", dr.dist, dr.state, dr.visibility);
            }
            if first_jump.is_none() && dr.alert >= 1.0 { first_jump = Some(t); }
            if !dr.shots.is_empty() {
                first_shot = Some(t); shot_guard = Some(i);
                shot_sighted = sighted.contains(&i);
                eprintln!("[beam] t={t} guard {i} FIRST SHOT (state {:#x} anim {:#x}.{} los={} seen={} alert {:.2} seen_before={})",
                    dr.state, dr.anim.class, dr.anim.state, dr.los, dr.seen, dr.alert, sighted.contains(&i));
                break 'run;
            }
        }
    }
    let alarm = alarm.expect("standing in the beam raises alarm channel 55 (the searchlight test proves the path)");
    let (first_los, first_jump, first_shot) = match (first_los, first_jump, first_shot) {
        (Some(l), Some(j), Some(s)) => (l, j, s),
        _ => { eprintln!("[beam] no sighting-then-shot in 900 ticks (los={first_los:?} jump={first_jump:?} shot={first_shot:?}) — vacuous"); return; }
    };
    eprintln!("[beam] RESULT alarm t={alarm} guard {shot_guard:?}: sight t={first_los} jump t={first_jump} shot t={first_shot} | sight→shot {} jump→shot {}",
        first_shot - first_los, first_shot - first_jump);

    // The game's hard floor: no bullet before the reaction ack — HaveOpponentSight 0x175ef0:41-44
    // holds flag 4 until +0x270 >= ReactionTime (10 on 07000006 [M 0x14ac60 default arm]), and the
    // fire request +0x3B rides the sight flag through CombatState 0x148338:108-113.
    assert!(shot_sighted,
        "guard {shot_guard:?} fired without ever having Bond in sight — the game's fire request \
         +0x3B is armed from the sight flag [M DroneFunc_CombatState 0x148338:108-113] which \
         HaveOpponentSight sets only after the reaction [M 0x175ef0:41-46]");
    assert!(first_shot >= first_los + 9,
        "guard {shot_guard:?} fired {} ticks after first sight — inside the game's reaction floor \
         (HaveOpponentSight 0x175ef0:41-44 holds the sight flag until +0x270 >= ReactionTime, 10 on \
         07000006 [M 0x14ac60 default arm]; the counter counts the sighting tick itself \
         [M 0x176a28:222], so the ack — and the earliest legal bullet — land on the 10th LOS tick)",
        first_shot - first_los);
    assert!(first_shot >= first_jump + 26,
        "guard {shot_guard:?} fired {} ticks after the alertness jump — inside the game's legal \
         floor (the ack + fire-state entry + BurstDelay's 15 [M 0x179ba8]; the game's measured \
         chains were 70 and 128 [L 06-firstsight.trace])",
        first_shot - first_jump);
}
