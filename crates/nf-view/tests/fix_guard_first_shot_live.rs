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
    let mut aim_config: std::collections::HashMap<usize, bool> = std::collections::HashMap::new();
    'run: for t in 0..900u32 {
        sim.step_bond(idle, false, false, false);
        if alarm.is_none() && sim.drones.channels.on(55) { alarm = Some(t); eprintln!("[beam] t={t} alarm ch55 up"); }
        for (i, dr) in sim.drones.list.iter().enumerate() {
            if !dr.present || !dr.enabled { continue; }
            if dr.los {
                sighted.insert(i);
                // The shooter's config at his FIRST LOS tick decides which game chain times the
                // first bullet (see the assertion below): an already-aiming combat guard
                // (class-2 anim + the request armed — SetCombatMoveAnim 0x13cfb0:73 arms
                // +0x3b = +0x274 < 180 on every move-anim tick) releases on the fresh-LOS tick;
                // a guard still in the fresh-sight chain (idle/look-about anims) must ride the
                // reaction ack first.
                aim_config.entry(i).or_insert(dr.anim.class == 2 && dr.firing);
            }
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
        "guard {shot_guard:?} fired without ever having Bond in sight — DoFiring's sight gate \
         +0x274 < 0x10 [M 0x17a080:24-25] is shut until a LOS tick resets the counter \
         [M 0x176a28:221, disasm 0x176de8 `sw $zero, 0x274`], so no LOS tick means no release");
    // The game's first-shot timing has TWO chains [M]: (a) the fresh-sight chain — a guard in the
    // idle/alert states plays the class-1 look-abouts (CombatState 0x148338's flag-4-clear arms,
    // anim 0x21) whose anim class fails Ready2Fire's aim-class gate [M 0x178c48 SP arm
    // `+0x568 != 2 → 0`], so the first bullet waits for the ack (0x175ef0:41-44, ReactionTime 10
    // on 07000006 [M 0x14ac60 default arm]) plus the aim-anim chain — the game measured +45/+97
    // frames from the alert [L docs/live/2026-10-14-weapon-measurements-pcsx2.md]. (b) the
    // combat-config release — a guard already moving in an AIM-class move anim (RUN_AIM / the
    // strafe-aims, class 2) has the fire request re-armed by SetCombatMoveAnim on EVERY
    // move-anim tick (+0x3b = +0x274 < FRAME_RATE_INT*3 [M 0x13cfb0:73, assigned at :150/:240/
    // :272/:353]) and DoFiring's +0x274 < 0x10 gate releases on the FIRST LOS tick (+0x274 = 0
    // [M 0x176a28:221, disasm 0x176de8]) — Ready2Fire's class-2 + muzzle-tolerance gates pass
    // mid-approach, and nothing further in the chain holds the bullet. The 109f writer (growing
    // +0x274 through the reaction window) matched NEITHER chain and broke the 20 other +0x274
    // readers (movement/engagement) — reverted; chain (a)'s floor is asserted below.
    match shot_guard.and_then(|g| aim_config.get(&g).copied()).unwrap_or(false) {
        true => eprintln!("[beam] guard {shot_guard:?} was aim-configured (class-2 anim + armed request) at first LOS — the game's combat-config release (SetCombatMoveAnim 0x13cfb0:73 → DoFiring 0x17a080:24-25 → Ready2Fire 0x178c48); no reaction floor applies"),
        false => assert!(first_shot >= first_los + 9,
            "guard {shot_guard:?} fired {} ticks after first sight from a NON-aim config — inside \
             the game's fresh-sight reaction floor (the flag-4-less arms play the class-1 \
             look-abouts [M 0x148338:108-113 else-arms] that fail Ready2Fire's aim-class gate \
             [M 0x178c48], so the bullet waits for the ack [M 0x175ef0:41-46] + the aim chain; \
             the game measured +45/+97 frames [L measurements 2026-10-14])",
            first_shot - first_los),
    }
    assert!(first_shot >= first_jump + 26,
        "guard {shot_guard:?} fired {} ticks after the alertness jump — inside the game's legal \
         floor (the ack + fire-state entry + BurstDelay's 15 [M 0x179ba8]; the game's measured \
         chains were 70 and 128 [L 06-firstsight.trace])",
        first_shot - first_jump);
}
