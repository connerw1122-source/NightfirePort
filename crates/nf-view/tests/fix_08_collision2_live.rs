//! fix-08-collision2 (100m): 100j's scoped collision broke the 01 escort walk — the contact
//! settled at (-5.2, 6.3, 14.1) with the LoadPointActivator plate at (-33.1, 2.3, 15.4) because
//! the walk's per-cel pass disagreed with the flat one at the room seams. Two causes, both the
//! game's own mechanism:
//!
//! 1. **The straddle root** [M `build_LinkToRoom` 0x1c9680:35-40 → `build_FindCel` 0x1c8cb0]:
//!    the per-tick relink of obj+0x20 is the room cels' TWO-RAY tie-break (the up ray, then the
//!    down ray capped at min(up hit, len); the nearest hit wins; no hits → the first candidate),
//!    so the actor's cel carries the geometry under him. The port used the coarse room-bbox
//!    mapping (`cel_at` → `room_cell`), which at a seam roots the straddle at the neighbour cel —
//!    the pass then tests the same triangles in a different ORDER than the flat world and the
//!    resolved push differs (the game's `Collide_Pick` 0x1eb320 walks the straddle array in BFS
//!    order too, but from the FindCel cel the order agrees with the flat resolution).
//! 2. **The portal reference point** [M `build_alloc_portal` 0x1c8928 stores the 4 corners'
//!    average at portal+0x70; `Intersect_Portal` 0x1ea7b8's 0x800 path projects the capsule
//!    segment against THAT point]: the port projected against corner 0.
//!
//! The assertions pin NUMBERS (positions, set sizes) only. Opt-in via NF_ISO.
use nf_game::player::{Bond, Input};

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// The first seam of the 01 route where the scoped pass diverged from the flat one (the merged
/// tree's t2460 trace: Bond's capsule grazing the seam wall at cel 60 while the pusher hangs on
/// the neighbouring cel's bucket). The flat push, the scoped push FROM THE GAME'S ROOT and the
/// straddle set must agree — the game's straddle-set cels carry everything that blocks him there.
#[test]
fn the_01_seam_scoped_pass_matches_the_flat_one() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000001.bin", false).expect("load 07000001");
    {
        let sim = scene.sim.as_mut().unwrap();
        // Walk Bond to the seam by hand (the intro's cut-scene must release him first).
        let idle = Input::default();
        for _ in 0..2000 { if !sim.disabled { break; } sim.step_bond(idle, false, false, false); }
        assert!(!sim.disabled, "the intro never released Bond");
        sim.bond.pos = [-2.62, 6.468, -1.12];
        sim.bond.vel = [0.0; 3];
        for _ in 0..3 { sim.step_bond(idle, false, false, false); }
    }
    let (root, a, b) = {
        let sim = scene.sim.as_ref().unwrap();
        let (root, cap) = (sim.bond.cel, sim.bond.capsule_seg());
        (root, cap.0, cap.1)
    };
    let world = &scene.sim.as_ref().unwrap().world;
    let Some(root) = root else { panic!("Bond's relink resolved (build_FindCel 0x1c8cb0)") };
    let set = world.debug_straddle(root, a, b, nf_game::player::RADIUS);
    println!("root cel {root} straddle {set:?}");
    assert!(set.len() >= 2, "the seam's straddle set keeps the neighbouring cel too [M 0x1eac98's portal walk] (got {set:?})");
    let (flat, flat_w) = world.capsule_push(a, b, nf_game::player::RADIUS);
    let (scoped, scoped_w) = world.capsule_push_scoped(Some(root), a, b, nf_game::player::RADIUS);
    // The per-component agreement: the game's per-cel pass resolves the same push the flat world
    // does when the straddle root is the game's relink (the order matches; the stray run the
    // coarse root tested first is out of the set).
    for k in 0..3 {
        assert!((flat[k] - scoped[k]).abs() < 1e-3,
            "the scoped pass from the FindCel root matches the flat push at the seam [M 0x194df0's obj+0x20 root] axis {k}: flat {flat:?} scoped {scoped:?}");
    }
    assert_eq!(flat_w, scoped_w, "the walkable verdict agrees too");
}

/// Sister: the FindCel root is the FLOOR's cel — at the seam the two-ray tie-break's down ray
/// hits the geometry under Bond in the root's own bucket, and the root differs from the coarse
/// room-bbox stand-in that pre-fix the port used (the traced t2460 divergence: root 60 while the
/// flat-matching order lives at 6).
#[test]
fn the_findcel_root_carries_the_floor_under_bond() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000001.bin", false).expect("load 07000001");
    {
        let sim = scene.sim.as_mut().unwrap();
        let idle = Input::default();
        for _ in 0..2000 { if !sim.disabled { break; } sim.step_bond(idle, false, false, false); }
        sim.bond.pos = [-2.62, 6.468, -1.12];
        sim.bond.vel = [0.0; 3];
        for _ in 0..3 { sim.step_bond(idle, false, false, false); }
    }
    let sim = scene.sim.as_ref().unwrap();
    let root = sim.bond.cel.expect("the relink resolved");
    // The floor under Bond: the scoped floor ray from the root hits (the root's chain carries
    // the floor — the game's tie-break guarantees it [M 0x1c8cb0's down-ray rule]).
    let ray = sim.bond.cc + 0.6;
    let floor = sim.world.segment_hit_scoped(Some(root), sim.bond.pos, [sim.bond.pos[0], sim.bond.pos[1] - ray, sim.bond.pos[2]]);
    assert!(floor.is_some(), "the FindCel root's own chain carries the floor under Bond [M 0x1c8cb0]");
    // The coarse room-bbox stand-in's cel differs here (the pre-fix root) — the divergence the
    // 01 walk tripped over. Numbers only: print both.
    let room_root = sim.cel_at(sim.bond.pos).and_then(|room| sim.room_cell.get(room).copied().flatten());
    println!("findcel root {root}, room-bbox root {room_root:?}");
}
