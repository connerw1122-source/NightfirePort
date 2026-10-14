//! fix-08-collision (100j): the playtest's "the wall is still there but invisible" inside
//! 07000008's missile room (`Room_Guardhouse_F2`, graph cell 48) at
//! `--bond 155.19,43.12,-113.38,1.57,0` with portal culling ON.
//!
//! The exterior `Room_Outside_3` (cell 15) wall slabs — fix-08-walls measured their tris across
//! the room interior [D per-tri probe; this file's probe: 64 collision tris inside the room box,
//! bands x 155.0..165.9, y 36.75..57.4, z −125.0..−118.0] — cull visually at master, but their
//! COLLISION still blocks Bond: the port's movement pass calls the FLAT
//! `CollisionWorld::capsule_push`/`segment_hit` (every static triangle, one shared world)
//! [M crates/nf-game/src/player.rs `physics_tick`/`physics_tick_climb`], while the game resolves
//! the whole pass through the actor's own cel: `Player_Collision` 0x194df0 stores the actor's
//! obj+0x20 cel pointer into the BLData+0x740 HITTEST's start-cel field (`*(BLData+0x7c0) =
//! *(obj+0x20)` — the first store of the function), `Collide_Pick` 0x1eb320 walks
//! `Collide_StraddleCels` 0x1eac98's result and tests each straddled cel's OWN object list
//! (`*(cel + 0x18)` list head), and `Player_CollisionHandler` 0x1949a8 runs the floor test
//! (`Player_FeetOnPoint`) off the same HITTEST. So a triangle hanging on another room's cel never
//! blocks indoors — culling and collision share the per-cel attribution.
//!
//! The port already HAS the per-cel machinery: `capsule_push_scoped`/`segment_hit_scoped`
//! (the straddle walk + the chain-owner buckets fix-portal-eyecel's `attribute_scope` re-keyed).
//! These tests pin the switch: the northward walk across the slab face must cross it (fails
//! pre-fix), the room's real walls must still block (sisters), and the scoped query must
//! disagree with the flat one exactly at the slab face (the mechanism pin).
//!
//! Opt-in via NF_ISO (the slim ISO is enough).

use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn load_08(iso: &str) -> nf_view::scene::Scene {
    let mut scene = nf_view::load(iso, "07000008.bin", false).expect("load 07000008");
    let idle = Input::default();
    for _ in 0..600 { if !scene.sim.as_ref().unwrap().disabled { break; } scene.sim.as_mut().unwrap().step_bond(idle, false, false, false); }
    for _ in 0..30 { scene.sim.as_mut().unwrap().step_bond(idle, false, false, false); }
    assert!(!scene.sim.as_ref().unwrap().disabled, "the intro never released Bond");
    scene
}

/// Fail-first repro: from the user's playtest spot inside the missile room, walking NORTH (yaw π,
/// the −z facing) hits the exterior slabs' west face (z −118.0 [D probe]) — pre-fix the flat
/// capsule push stopped Bond at z ≈ −117.5 (face + radius) and squeezed him UP the invisible
/// wall (y 43.75 → 44.17 over ticks 59-68) before shoving him over the top; post-fix the slabs
/// hang on cell 15 and never enter the pass [M Player_Collision 0x194df0: the HITTEST's start cel
/// is obj+0x20; Collide_Pick 0x1eb320 tests each straddled cel's own list (cel+0x18)] — Bond
/// crosses the band on the level floor.
#[test]
fn the_missile_room_walk_across_the_slab_band_meets_no_wall() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load_08(&iso);
    let sim = scene.sim.as_mut().unwrap();
    sim.bond = nf_game::player::Bond::new([155.19, 43.12, -113.38], std::f32::consts::PI);
    for _ in 0..90 { sim.step_bond(Input::default(), false, false, false); }
    eprintln!("settle: pos {:?} cel {:?} grounded {}", sim.bond.pos, sim.bond.cel, sim.bond.grounded);
    assert!(sim.bond.pos[1] > 40.0, "Bond stands on the missile room floor (got {:?})", sim.bond.pos);
    let forward = Input { forward: 1.0, ..Input::default() };
    let mut max_y = f32::NEG_INFINITY;
    for t in 0..90 {
        sim.step_bond(forward, false, false, false);
        max_y = max_y.max(sim.bond.pos[1]);
        if t == 65 {
            eprintln!("crossing tick: pos {:?}", sim.bond.pos);
            assert!(sim.bond.pos[2] < -118.1,
                "walking north crosses the slab face (z −118.0 [D probe]) at the walk's own pace — \
                 the invisible wall stopped the walk [M Player_Collision 0x194df0 scopes the pass \
                 to obj+0x20's cel; Collide_Pick 0x1eb320 walks the straddled cels' own lists] \
                 (got z {:.2} y {:.2})", sim.bond.pos[2], sim.bond.pos[1]);
        }
    }
    eprintln!("crossing max y {max_y:.3}");
    // The pre-fix flat pass not only stopped Bond at the face — it squeezed him UP it (y 43.75 →
    // 44.17) and over the top. The crossing must stay on the level floor.
    assert!(max_y < 43.9,
        "the crossing rides the level floor — no wall to climb (the pre-fix flat pass squeezed \
         Bond up the slab face) (max y {:.2})", max_y);
    for _ in 0..120 { sim.step_bond(forward, false, false, false); }
    eprintln!("north walk: pos {:?} grounded {}", sim.bond.pos, sim.bond.grounded);
    assert!(sim.bond.pos[2] > -122.9,
        "the room's real north wall still blocks — the walk stayed inside Room_Guardhouse_F2 \
         (z to −122.33 [D cell bbox]) (got z {:.2})", sim.bond.pos[2]);
    assert!(sim.bond.pos[1] > 40.0, "the floor held across the band (got {:?})", sim.bond.pos);
}

/// Sister: the room's real west wall still blocks the same walk westward — the per-cel
/// attribution must not delete the room's own collision.
#[test]
fn the_missile_rooms_real_west_wall_still_blocks() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load_08(&iso);
    let sim = scene.sim.as_mut().unwrap();
    sim.bond = nf_game::player::Bond::new([160.0, 43.12, -114.5], -std::f32::consts::FRAC_PI_2);
    for _ in 0..90 { sim.step_bond(Input::default(), false, false, false); }
    let west = Input { forward: 1.0, ..Input::default() };
    for _ in 0..210 { sim.step_bond(west, false, false, false); }
    eprintln!("west walk: pos {:?}", sim.bond.pos);
    assert!(sim.bond.pos[0] > 153.5 && sim.bond.pos[0] < 156.6,
        "the missile room's real west wall blocks (room x from 154.5 [D cell bbox]) (got x {:.2})", sim.bond.pos[0]);
    assert!(sim.bond.pos[1] > 40.0, "the floor held (got {:?})", sim.bond.pos);
}

/// Sister (the playtest's own walk): eastward from the user spot reaches the room's real east
/// wall — the fix must not open the building.
#[test]
fn the_missile_rooms_real_east_wall_still_blocks() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load_08(&iso);
    let sim = scene.sim.as_mut().unwrap();
    sim.bond = nf_game::player::Bond::new([155.19, 43.12, -113.38], std::f32::consts::FRAC_PI_2);
    for _ in 0..90 { sim.step_bond(Input::default(), false, false, false); }
    let east = Input { forward: 1.0, ..Input::default() };
    for _ in 0..210 { sim.step_bond(east, false, false, false); }
    eprintln!("east walk: pos {:?}", sim.bond.pos);
    assert!(sim.bond.pos[0] > 158.0 && sim.bond.pos[0] < 164.0,
        "the walk crosses the room and stops at the real east wall (room x to 163.14 [D cell bbox]) \
         (got x {:.2})", sim.bond.pos[0]);
    assert!(sim.bond.pos[1] > 40.0, "the floor held (got {:?})", sim.bond.pos);
}

/// The mechanism pin: at the slab face the FLAT capsule push still reports the exterior slabs
/// (they exist in the shared world — the geometry fix-08-walls measured), while the SCOPED query
/// with Bond's own cel finds no wall — the per-cel attribution the game's pass runs on
/// [M Collide_Pick 0x1eb320: `Collide_StraddleCels` result → each cel's own +0x18 list].
#[test]
fn the_scoped_capsule_disagrees_with_the_flat_one_at_the_slab_face() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load_08(&iso);
    let sim = scene.sim.as_mut().unwrap();
    sim.bond = nf_game::player::Bond::new([155.19, 43.12, -113.38], std::f32::consts::PI);
    for _ in 0..90 { sim.step_bond(Input::default(), false, false, false); }
    let cell = sim.bond.cel.expect("Bond's cel resolves inside the missile room");
    eprintln!("bond cel {cell} pos {:?}", sim.bond.pos);
    // A capsule segment crossing the slab's west face (z −118.0) at mid-capsule height,
    // x 157.5 (inside the room's x band, clear of the side walls).
    let a = [157.5, sim.bond.pos[1] + 0.9, -115.5];
    let b = [157.5, a[1], -121.0];
    let (flat, _) = sim.world.capsule_push(a, b, 0.55);
    let (scoped, _) = sim.world.capsule_push_scoped(Some(cell), a, b, 0.55);
    eprintln!("flat push {:?} scoped push {:?}", flat, scoped);
    assert!(flat[2] > 0.01,
        "the shared world still carries the exterior slabs (the flat push shoves the capsule back \
         south) [D probe: cell 15's tris z −125.0..−118.0 inside the room] (got {:?})", flat);
    assert!(scoped[2].abs() < 0.01,
        "Bond's own cel's lists carry no wall at the slab face [M Collide_Pick 0x1eb320 per-cel \
         lists] (got {:?})", scoped);
    // Sister: the scoped query against the room's OWN west wall still pushes — attribution, not a
    // blanket cull.
    let wa = [157.5, a[1], -114.5];
    let wb = [153.8, a[1], -114.5];
    let (wscop, _) = sim.world.capsule_push_scoped(Some(cell), wa, wb, 0.55);
    eprintln!("west scoped push {:?}", wscop);
    assert!(wscop[0] > 0.01,
        "the room's real west wall still collides through the scoped query (got {:?})", wscop);
}
