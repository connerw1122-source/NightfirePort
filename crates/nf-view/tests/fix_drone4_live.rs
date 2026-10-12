//! fix-drone4 repro tests, driven through the real path (`nf_view::load` + `Sim::step_bond`).
//!
//! The user's retest of the party level (docs/playtests/2026-10-11.md C3 follow-up): "the 07000007
//! guests no longer T-pose, but they don't move or walk at all". Root cause [M InitAIPath 0x153720]:
//! the guests carry prop 0x1D and the level places kind-8 (mission) paths under them, so the game
//! starts them in CivilianMission (0x16) walking the path chain — the port had no 0x16 handler and its
//! `Route::for_path` collapsed every open chain (endpoint nodes) to a single node.
//!
//! State ids are written raw (`0x16` …) so this file compiles — and fails — against the pre-fix master.
//! Opt-in via `NF_ISO`.

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// C3 follow-up. A guest placed on a kind-8 mission path with no switch channel runs CivilianMission
/// (0x16): CivilianInit/PartyGirlInit ENTER assign the mask-8 route [M 0x172730:29-39, 0x1728c0:17-22],
/// ENTER calls the walk danim 0x37 [M 0x15d860:19-21] and UPDATE follows the route [M 0x15d860 case 3,
/// UpdateMissionRoute 0x154a80]. Pre-fix: no guest ever got a route (the port skipped the mask-8 branch
/// entirely) and none reached 0x16 — "they don't move or walk at all".
#[test]
fn a_party_guest_walks_her_mission_route() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000007.bin", false).expect("load");
    let sim = scene.sim.as_mut().unwrap();
    // The walker: a dtype-10 guest with no switch channel (starts immediately) and a kind-8 path node
    // within the game's 0.3 acceptance of her placement [M AssignAIPath 0x154190: fVar11 = 0.3].
    let data = sim.data();
    let walker = sim.drones.list.iter().position(|dr| {
        dr.present && dr.dtype == 10 && dr.switch_channel == 0
            && data.nav.paths.iter().any(|p| p.kind & 8 != 0 && p.nodes.iter().any(|n| {
                (n.pos[1] - dr.feet()[1]).abs() <= 2.0
                    && (dr.pos[0] - n.pos[0]).powi(2) + (dr.pos[2] - n.pos[2]).powi(2) < 0.09
            }))
    }).expect("07000007 places a guest on a kind-8 mission path (paths 2/4/5 node 0)");
    drop(data);

    for t in 0..300 { sim.step_bond(Default::default(), false, false, false); if t == 60 {
        let dr = &sim.drones.list[walker];
        assert_eq!(dr.state, 0x16, "the guest runs CivilianMission [M 0x15d860] — got {:#x}", dr.state);
        assert_eq!(dr.anim.state, 0x37, "CivilianMission's ENTER walk danim plays [M 0x15d860 CallAnim(0x37)]");
    } }
    let dr = &sim.drones.list[walker];
    assert!(dr.route.is_some(), "the guest holds her assigned mission route");
    assert!(dr.route.as_ref().unwrap().nodes.len() >= 4,
        "the mission route is the real node chain [M InitAIPath 0x153720] — got {} node(s): the collapsed \
         single-node route (the pre-fix for_path endpoint bug) 'arrives' where it stands",
        dr.route.as_ref().unwrap().nodes.len());
    assert_eq!(dr.state, 0x16, "still walking her route at tick 300 (pre-fix: fallen to Civilian 0x11)");
    assert_eq!(dr.anim.state, 0x37, "the walk danim holds while the route runs");

    // Sister: she actually covers ground — the user's symptom is the standing still.
    let start = dr.pos;
    for _ in 0..300 { sim.step_bond(Default::default(), false, false, false); }
    let dr = &sim.drones.list[walker];
    let moved = ((dr.pos[0] - start[0]).powi(2) + (dr.pos[2] - start[2]).powi(2)).sqrt();
    assert!(moved > 2.0, "the guest walks her route (moved {moved:.2} in 300 ticks; pre-fix: 0.04)");
}

/// The route-chain fix must not disturb the guard patrols (the kind-4 loops the existing tests walk):
/// a 07000005 patroller still patrols and his torso stays untwisted (the fix-drone3 gate).
#[test]
fn the_07000005_patrol_still_walks_after_the_route_fix() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load");
    let sim = scene.sim.as_mut().unwrap();
    for _ in 0..60 { sim.step_bond(Default::default(), false, false, false); }
    let pi = sim.drones.list.iter().position(|dr| dr.present && dr.enabled && dr.state == 0x07)
        .expect("07000005 starts a patrolling guard");
    let pp = sim.drones.list[pi].pos;
    for _ in 0..120 { sim.step_bond(Default::default(), false, false, false); }
    let dr = &sim.drones.list[pi];
    assert_eq!(dr.state, 0x07, "the guard keeps patrolling after the for_path change");
    let moved = ((dr.pos[0] - pp[0]).powi(2) + (dr.pos[2] - pp[2]).powi(2)).sqrt();
    assert!(moved > 0.5, "the patroller still covers ground (moved {moved:.2})");
    assert!(dr.aim_yaw.abs() < 1e-4, "a patrolling guard does not aim his torso [M the aim-state gate]");
}
