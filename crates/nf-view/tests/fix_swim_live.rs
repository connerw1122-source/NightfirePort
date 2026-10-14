//! fix-swim: the 07000002 garden pond swim (substate 3) and the water ladder under the hut, through
//! the REAL viewer path (`nf_view::load` + `Sim::step_bond`). Opt-in via `NF_ISO`.
//!
//! User playtest (results 1013-1825, `--bond -82.3,5.3,32.05,1.57`): "This is hardly swimming …
//! spamming the crouch button, and I very, very slowly got into the water", "I could hear my
//! footsteps too", "I can get in the water, but I can't crouch in water. I'm not sure how to swim",
//! "There's a ladder in the water under that hut. I can't latch onto it and climb it".
//!
//! Game evidence (ACTION.ELF):
//! - `Player_InWater` 0x195e50: cel+0x90 > pos.y − 0.3 → `ChangeSubState(3)`, run as the FIRST
//!   statement of `Player_SSWalk` 0x192310:29-32 (and `Player_SSCrouch` 0x1924c0:40-43).
//! - Swim case 3 of `Player_Update` 0x193600:181-222: `Player_Move(1.0, …)` — walk-speed movement
//!   whose substate-3 branch [M `Player_Move` 0x191dd0:149-165] rides the LOOK direction — dive by
//!   looking down, surface by looking up; the case tail clamps the world rise at `surface − 0.3`;
//!   exit when grounded and `surface <= pos.y − 0.3` → `ChangeSubState(0)`.
//! - No crouch/jump input exists in swim (case 3 reads none; `Player_HandleJump` 0x1916b0 is only
//!   called from `Player_SSWalk` 0x192310:69).
//! - `Player_CollLadder` 0x196118:16-17: the grab excludes ONLY substates 1 (climb) and 2 (grapple)
//!   — swim (3) grabs; the y-snap `pos.y = max(pos.y, bottom + 0.02)` lifts a swimmer onto the rungs.
//! - Footsteps: `AnimObjectUpdate` 0x1befd8's step fires `Effect_Create` off the CACHED ground hit,
//!   which early-outs with no hit [M 0x1ce688:63] — the swim capsule never touches the pond bed, so
//!   swimming is silent (the dive loop 0x27 / gasp 0x60c / drown 0x60b come from `Player_MonitorAir`
//!   0x191a70 instead).

use nf_view::scene::Sim;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn load() -> Sim {
    let iso = iso().expect("NF_ISO");
    let mut scene = nf_view::load(&iso, "07000002.bin", false).expect("load 07000002");
    scene.sim.take().expect("sim")
}

/// One step through the real path; the level's scripts grab Bond into cut-scenes within seconds,
/// so each tick carries the game's own skip action like the walkreal walker [M nis.rs].
fn step(sim: &mut Sim, input: nf_game::player::Input) {
    if sim.disabled { sim.drones.scripts.skip_request = true; }
    sim.step_bond(input, false, false, false);
}

/// The pond's water cels [D probe]: 4 and 16 at 3.765, 6 at 4.339, 17 at 3.768.
const BANK: [f32; 3] = [-82.3, 5.3, 32.05]; // the user's `--bond` spot, yaw 1.57 walks +X into the pond

/// Walk Bond from the bank into the pond on the real path; returns (entry tick, sim).
fn swim_in(sim: &mut Sim) -> usize {
    sim.bond = nf_game::player::Bond::stand_at(&sim.world, BANK, 1.57);
    let walk = nf_game::player::Input { forward: 1.0, ..Default::default() };
    for t in 0..600 {
        step(sim, walk);
        if sim.bond.substate == nf_game::player::SS_SWIM { return t; }
    }
    panic!("walking off the bank never entered the swim (pos {:?})", sim.bond.pos);
}

/// Walking forward off the bank enters the swim at the game's speed: `Player_InWater` 0x195e50's
/// threshold (`pos.y − 0.3 < cel+0x90`) fires from the real walk, and the swim stroke covers ground
/// at the WALK rate — case 3 calls `Player_Move(1.0, …)` [M 0x193600:181-182], the same smoothed
/// ±0.1/tick forward cap as the walk branch. (Regression pin: entry worked pre-fix; the speed cap
/// pins the multiplier against a future crouch-flag leak, `k` must stay 1.0 in swim.)
#[test]
fn walking_off_the_bank_enters_the_swim_at_the_game_speed() {
    let Some(_) = iso() else { eprintln!("NF_ISO not set — skipping"); return; };
    let mut sim = load();
    let entered = swim_in(&mut sim);
    assert!(entered < 600, "the swim did not enter within 10 s of walking in");
    // The steady stroke: over a 60-tick window the horizontal speed reaches the walk cap
    // (0.1/tick smoothed target ±0.1 [M Player_Move 0x191dd0:92-138] — ≥ 0.085/tick allows the ramp).
    let walk = nf_game::player::Input { forward: 1.0, ..Default::default() };
    for _ in 0..60 { step(&mut sim, walk); }
    let a = sim.bond.pos;
    for _ in 0..60 { step(&mut sim, walk); }
    let b = sim.bond.pos;
    let speed = ((b[0] - a[0]).powi(2) + (b[2] - a[2]).powi(2)).sqrt() / 60.0;
    assert!(speed >= 0.085, "the swim stroke is slower than the game's walk-parity cap: {:.4}/tick", speed);
    // Sister: still swimming (no spurious exit mid-pond — the exit needs grounded AND feet 0.3 above).
    assert_eq!(sim.bond.substate, nf_game::player::SS_SWIM, "the swim dropped out mid-pond");
}

/// The swim stroke rides the LOOK direction: looking down + forward DIVES, looking up + forward
/// surfaces and the rise clamps at `surface − 0.3` [M 0x191dd0:149-165 + 0x193600:201-220]. Pre-fix
/// the stroke was planar-only and crouch/jump sank/rose ±2.4/tick ([S] fiction — the game's case 3
/// reads no such input, the user's "I can't crouch in water. I'm not sure how to swim").
#[test]
fn swim_dives_and_surfaces_with_the_look_direction() {
    let Some(_) = iso() else { eprintln!("NF_ISO not set — skipping"); return; };
    let mut sim = load();
    swim_in(&mut sim);
    let y_surf = sim.bond.pos[1];
    // Dive: pitch down over a second while holding forward — Bond must lose depth.
    let dive = nf_game::player::Input { forward: 1.0, look: -0.03, ..Default::default() };
    for _ in 0..60 { step(&mut sim, dive); }
    assert!(sim.bond.pos[1] < y_surf - 0.4, "looking down + forward did not dive (y {} -> {})", y_surf, sim.bond.pos[1]);
    // Surface: pitch up — the rise stops at surface − 0.3 and never passes it [M the case-3 clamp].
    let up = nf_game::player::Input { forward: 1.0, look: 0.03, ..Default::default() };
    for _ in 0..180 { step(&mut sim, up); }
    let w = sim.cel_at(sim.bond.pos).map(|i| sim.cels[i].water).filter(|w| *w > -400.0)
        .expect("swimming with no water cel");
    assert!(sim.bond.pos[1] <= w - 0.3 + 1e-3, "the swim rose past surface − 0.3 (y {} vs surface {})", sim.bond.pos[1], w);
    assert!(sim.bond.pos[1] > y_surf - 0.4, "looking up + forward did not surface (y {})", sim.bond.pos[1]);
    // Sister: crouch/jump spam does NOT sink or rise a swimmer (no such input in case 3 [M]).
    let y_at = sim.bond.pos[1];
    let spam = nf_game::player::Input { jump: true, crouch: true, ..Default::default() };
    for _ in 0..30 { step(&mut sim, spam); }
    assert!((sim.bond.pos[1] - y_at).abs() < 1e-2, "crouch/jump moved the swimmer (y {} -> {})", y_at, sim.bond.pos[1]);
}

/// No footstep sounds while swimming: the game's step needs a cached ground hit the swim capsule
/// never gets [M Effect_Create 0x1ce688:63]; the pond surface's slosh id (129 [D probe]) played every
/// stride pre-fix — the user's "I could hear my footsteps too".
#[test]
fn no_footstep_sounds_play_while_swimming() {
    let Some(_) = iso() else { eprintln!("NF_ISO not set — skipping"); return; };
    let mut sim = load();
    swim_in(&mut sim);
    let listener = ([0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]);
    let mut ids = Vec::new();
    let walk = nf_game::player::Input { forward: 1.0, ..Default::default() };
    for _ in 0..180 {
        step(&mut sim, walk);
        sim.drones.audio.update(None, listener, &|_| true);
        for c in sim.drones.audio.out.iter() {
            if let nf_game::sound::Cmd::Start { id, .. } = c { ids.push(*id); }
        }
        sim.drones.audio.out.clear();
    }
    // The pond surface's slosh footstep 129 played every stride pre-fix (10x in 240 ticks [probe]);
    // the dive loop / gasp only fire on head crossings, which a surface swim has none of. Ambient
    // level one-shots (524/468/1532/469 [probe]) are NOT steps and may play.
    assert!(!ids.contains(&129), "the pond footstep slosh played while swimming: {ids:?}");
    assert!(!ids.iter().any(|&id| id == 0x27 || id == 0x60c), "swim sounds fired with the head above the surface: {ids:?}");
}

/// The hut ladder (placement centre −74.84, 4.04, 22.22, bottom 2.83 — it reaches 0.94 into the
/// pond whose surface sits at 3.765-3.768 [D probe]) grabs FROM THE SWIM and climbs out:
/// `Player_CollLadder` 0x196118 excludes only substates 1/2, and the y-snap bottoms a swimmer onto
/// the rungs. Pre-fix the port's grab gate rejected everything but the walk substate.
#[test]
fn the_hut_ladder_grabs_from_the_water_and_climbs_out() {
    let Some(_) = iso() else { eprintln!("NF_ISO not set — skipping"); return; };
    let mut sim = load();
    let li = (0..sim.ladders.len()).min_by(|a, b| {
        let d = |l: &nf_game::ladder::Ladder| (l.centre[0] - -74.84).powi(2) + (l.centre[2] - 22.22).powi(2);
        d(&sim.ladders[*a]).total_cmp(&d(&sim.ladders[*b]))
    }).expect("07000002 has ladders");
    let lad = sim.ladders[li].clone();
    assert!(lad.bottom() < 3.5, "the picked ladder does not reach into the pond: bottom {}", lad.bottom());
    // Bond at the ladder's front, in the water (the bed there puts the root at the swim depth);
    // facing the ladder, holding forward — the grab fires within a couple of ticks.
    let front = [lad.centre[0] + lad.yaw.sin() * 1.2, 0.0, lad.centre[2] + lad.yaw.cos() * 1.2];
    sim.bond = nf_game::player::Bond::stand_at(&sim.world, front, lad.yaw + std::f32::consts::PI);
    let walk = nf_game::player::Input { forward: 1.0, ..Default::default() };
    let mut grabbed = None;
    let mut swam = false;
    for t in 0..300 {
        step(&mut sim, walk);
        if sim.bond.substate == nf_game::player::SS_SWIM { swam = true; }
        if t % 30 == 0 { eprintln!("ladder t {t}: sub {} pos {:?} water {:?}", sim.bond.substate, sim.bond.pos, sim.cel_at(sim.bond.pos).map(|i| sim.cels[i].water)); }
        if sim.bond.substate == nf_game::player::SS_CLIMB { grabbed = Some(t); break; }
    }
    let t = grabbed.expect("walking/swimming into the hut ladder never attached");
    eprintln!("grabbed from swim: {swam}");
    assert_eq!(sim.bond.ladder, Some(li), "the climb latched the wrong ladder");
    // The grab snapped Bond onto the ladder axis at centre + fwd(yaw)·0.5 [M 0x196118:32-48].
    let (s, c) = (lad.yaw.sin(), lad.yaw.cos());
    let dxz = ((sim.bond.pos[0] - (lad.centre[0] + s * 0.5)).powi(2) + (sim.bond.pos[2] - (lad.centre[2] + c * 0.5)).powi(2)).sqrt();
    assert!(dxz < 0.2, "the grab did not snap to the ladder axis ({dxz:.3} off)");
    // Climb out: hold forward until the top dismount (the +0.6 lunge [M Player_Climb 0x196310:50-62]);
    // assert on the FIRST walk tick (the lunge lands him on the platform before any fall).
    let mut landed = None;
    for _ in 0..600 {
        step(&mut sim, walk);
        if sim.bond.substate == nf_game::player::SS_WALK { landed = Some(sim.bond.pos[1]); break; }
    }
    let y = landed.expect("the climb never dismounted at the top");
    assert!(y > lad.top() - 0.6, "the top dismount did not reach the platform (y {y:.3} vs top {:.3})", lad.top());
    eprintln!("grab at +{t}, climb-out at y {:.3} (top {:.3})", sim.bond.pos[1], lad.top());
}

