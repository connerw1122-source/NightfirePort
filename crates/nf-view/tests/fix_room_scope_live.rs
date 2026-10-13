//! fix-room-scope: the actor capsule on the room-scoped collision — fix-safe08b's floor-mapping
//! gap, ported from `build_FindCel` 0x1c8cb0. Driven through the REAL level data (`nf_view::load`,
//! the same scoped world the Sim walks on) and the REAL queries the walk physics makes.
//!
//! The gap (fix-safe08b report, measured on 07000008): with the scope on, Bond fell through the
//! GuardRoom floor at the fusebox (153.15, 43.38, −119.67, cell 40) because the only collision under
//! that spot is the shared `Room_Stairwell_T_2` mesh (instance 32), which the port's smallest-volume
//! containment rule mapped to cell 59 — a cell the capsule's straddle set from 40 never keeps. The
//! game links it with `build_FindCel` 0x1c8cb0's tie-break: every containing ROOM cel is a
//! candidate; when several, two vertical rays from the centre (up 256 first, then down clamped to
//! the up hit) through each candidate's own linked geometry decide — nearest hit wins, no hits = the
//! first candidate [M 0x1c8cb0 disassembly; `build_LinkToRoom` 0x1c9680 → `Control_BuildWorldSph`
//! 0x133070 feeds the world-sphere centre]. Objects link at creation, in placement order
//! (`Control_CreateObjEx` 0x1343c8 calls `build_LinkToRoom`; every entity creator goes through it).

use nf_game::player::Input;
use nf_formats::placement::transform;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn load(iso: &str, level: &str) -> nf_view::scene::Scene {
    nf_view::load(iso, level, false).expect("load")
}

/// The fusebox facts on 07000008 (the fix-safe08b probe, [D] level data):
/// the floor point sits in cell 40 (the GuardRoom), the under-floor collision is instance 32
/// (`Room_Stairwell_T_2`), and cell 59 (the old smallest-volume mapping) is in no straddle set
/// from 40.
const FUSEBOX: [f32; 3] = [153.15, 43.38, -119.67];
const FUSEBOX_CELL: usize = 40;
const GUARDROOM_FLOOR_Y: f32 = 42.0; // the down-ray hits t 0.524/0.648 from y 44.2 → y ≈ 42.0/41.5

fn fusebox_cell(scene: &nf_view::scene::Scene) -> usize {
    let sim = scene.sim.as_ref().unwrap();
    let room = sim.cel_at(FUSEBOX).expect("the fusebox sits in a room cel");
    sim.room_cell[room].expect("the room cel maps to a graph cell")
}

/// The walk's floor ray at the fusebox, through the scoped world, from Bond's cell: the game never
/// falls there [D playtest], so the GuardRoom floor must be inside cell 40's own bucket —
/// `build_FindCel`'s tie-break, not the smallest-volume rule that parked the stairwell mesh in
/// cell 59. Sister: the flat floor ray hits too (the fix must MOVE the mapping, not eat geometry).
#[test]
fn the_scoped_floor_ray_hits_the_guardroom_floor_at_the_fusebox() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let scene = load(&iso, "07000008.bin");
    assert_eq!(fusebox_cell(&scene), FUSEBOX_CELL, "the fusebox's cell (the fix-safe08b probe)");
    let world = &scene.sim.as_ref().unwrap().world;
    let from = [FUSEBOX[0], FUSEBOX[1] + 0.6, FUSEBOX[2]];
    let to = [FUSEBOX[0], GUARDROOM_FLOOR_Y - 1.0, FUSEBOX[2]];
    assert!(world.segment_hit(from, to).is_some(),
        "the flat floor ray hits the GuardRoom floor (Room_Stairwell_T_2 [D])");
    assert!(world.segment_hit_scoped(Some(FUSEBOX_CELL), from, to).is_some(),
        "the scoped floor ray from cell 40 must hit the GuardRoom floor — build_FindCel 0x1c8cb0's \
         tie-break links the shared stairwell mesh to the cel the floor belongs to, not the \
         smallest-volume cel 59");
}

/// The capsule's push query at the fusebox — the exact call the walk physics makes
/// (`capsule_push_scoped(self.cel, …)`) — must find the floor walkable from cell 40.
/// Sister: the flat capsule at the same spot reports the same floor (no geometry eaten).
#[test]
fn the_scoped_capsule_stands_on_the_guardroom_floor_at_the_fusebox() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let scene = load(&iso, "07000008.bin");
    assert_eq!(fusebox_cell(&scene), FUSEBOX_CELL, "the fusebox's cell (the fix-safe08b probe)");
    let world = &scene.sim.as_ref().unwrap().world;
    // Bond's capsule standing at the fusebox: the floor is at y ≈ 42.0, the capsule segment spans
    // it (the walk's RADIUS 0.55 [M player::RADIUS]).
    let a = [FUSEBOX[0], GUARDROOM_FLOOR_Y + 0.3, FUSEBOX[2]];
    let b = [FUSEBOX[0], FUSEBOX[1] + 0.4, FUSEBOX[2]];
    let (push, walkable) = world.capsule_push_scoped(Some(FUSEBOX_CELL), a, b, 0.55);
    assert!(walkable || push[1] > 1e-3,
        "the scoped capsule must stand on the GuardRoom floor from cell 40 (walkable={walkable}, \
         push={push:?}) — the floor's stairwell mesh belongs in cell 40's bucket via the tie-break");
    let (_, flat_walkable) = world.capsule_push(a, b, 0.55);
    assert!(flat_walkable, "the flat capsule stands on the same floor (the sister)");
}

/// The straddle discipline on real data: from the bedroom (cell 48) no scoped query may reach
/// `Room_Outside_3`'s collision (the user's blue block, map51 obj16), while the flat world still
/// collides it (the block is real geometry — the game just scopes it out indoors). Sister: SOME
/// cell's scoped query reaches the mesh (outdoors the game collides it) — the scope must MOVE
/// geometry out of indoor queries without orphaning it.
#[test]
fn the_scoped_query_misses_the_exterior_mesh_from_indoors() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let scene = load(&iso, "07000008.bin");
    // Room_Outside_3's collision instance (map51 obj16, the fix-safe08b probe).
    let lvl = level(&iso, "07000008.bin");
    let (instances, _) = lvl.instances(true);
    let (ei, inst) = instances.iter().enumerate()
        .find(|(_, i)| i.map == 51 && i.object == 16)
        .expect("Room_Outside_3's placement (map51 obj16 [D])");
    let obj = &lvl.maps[51].objects[16];
    let Some(c) = &obj.collision else { panic!("the block has collision") };
    // A point just above the mesh's first collision triangle (world space), and a bedroom point in
    // cell 48.
    let tri = c.triangles_with_material()[0].0.map(|p| transform(&inst.matrix, p));
    let on_mesh = [0, 1, 2].map(|k| (tri[0][k] + tri[1][k] + tri[2][k]) / 3.0);
    let bedroom = [161.87f32, 43.39, -116.54]; // the safe doorway (the fix-safe08b probe: cell 48)
    let sim = scene.sim.as_ref().unwrap();
    let world = &sim.world;
    let room = sim.cel_at(bedroom).expect("the doorway sits in a room cel");
    let cell48 = sim.room_cell[room].expect("the bedroom maps to a graph cell");
    assert_eq!(cell48, 48, "the bedroom's cell (the fix-safe08b probe)");
    // The ray from just above the mesh straight down: the flat world hits; the bedroom's scoped
    // query (the vertical ray there — the walk's floor shape) must not.
    let from = [on_mesh[0], on_mesh[1] + 0.5, on_mesh[2]];
    let to = [on_mesh[0], on_mesh[1] - 0.5, on_mesh[2]];
    assert!(world.segment_hit(from, to).is_some(), "the flat world hits the mesh (inst {ei})");
    assert!(world.segment_hit_scoped(Some(cell48), from, to).is_none(),
        "the scoped query from the bedroom must miss the exterior mesh");
    // Sister: some cell's scoped query DOES hit it (the mesh is not orphaned by the scope).
    let any = (0..scene.cel_graph.cells.len())
        .any(|c| world.segment_hit_scoped(Some(c), from, to).is_some());
    assert!(any, "an outdoor query (some cell's scope) still collides the exterior mesh");
}

/// The end-to-end symptom: Bond placed at the fusebox and ticked through the REAL input path must
/// keep standing (the scoped capsule engaged for the walk). Before the capsule switch this passes
/// on the flat world; its fail-first is the two scoped-query tests above plus the unit law test.
#[test]
fn bond_does_not_fall_at_the_fusebox_with_the_scoped_capsule() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso, "07000008.bin");
    for _ in 0..60 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.bond.pos = FUSEBOX;
        sim.bond.yaw = 0.0;
        sim.bond.pitch = 0.0;
    }
    for _ in 0..120 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    let p = scene.sim.as_ref().unwrap().bond.pos;
    eprintln!("[room-scope] Bond at the fusebox after 2s: ({:.2}, {:.2}, {:.2})", p[0], p[1], p[2]);
    assert!(p[1] > GUARDROOM_FLOOR_Y + 0.3,
        "Bond does not fall through the GuardRoom floor at the fusebox (stood at y {:.2})", p[1]);
    assert!(((p[0] - FUSEBOX[0]).powi(2) + (p[2] - FUSEBOX[2]).powi(2)).sqrt() < 2.0,
        "Bond stays at the fusebox (no slide-off): ({:.2}, {:.2})", p[0], p[2]);
}

// ── helpers: the raw level parse for instance identity (the same pattern the fix-safe08b probe
// uses; nf_view::load owns the Scene's own copy).

fn iso_data(iso: &str, name: &str) -> Vec<u8> {
    let img = nf_formats::iso9660::IsoImage::open(iso).expect("open iso");
    let (entries, bin) = nf_formats::disc::files_bin_entries(&img).expect("FILES.BIN");
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(name)).expect("level entry");
    img.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).expect("read archive")
}

fn level(iso: &str, name: &str) -> nf_formats::level::Level {
    nf_formats::level::Level::parse(&iso_data(iso, name)).expect("parse level")
}
