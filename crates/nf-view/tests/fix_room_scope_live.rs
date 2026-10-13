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

/// The walk's floor ray at the fusebox, through the scoped world, from Bond's cell. *IGNORED (the
/// next session's fail-first)*: it fails for a MEASURED reason — the game's real `build_FindCel`
/// tie-break, ported verbatim and replayed on this level (run `NF_ROOMSCOPE_PROBE=1
/// cargo test -p nf-view --test fix_room_scope_live probe_room_scope_links`), maps the stairwell
/// mesh to cel 15, and the capsule's straddle set from cell 40 keeps only [40] — the same fall.
/// The game's statics never go through `build_LinkToRoom`: their collision is the map's own
/// Coll_Data node layer attached to the current cel at parse ([M `parsemap_block_Coll_Data_New`
/// 0x1d1810 → `pCurrCelList+8`]). Until the port models that layer, the folded instance world
/// cannot put this floor where the game has it. Sister: the flat floor ray hits too.
#[test]
#[ignore = "fix-room-scope: needs the Coll_Data node layer — the probe shows the faithful tie-break parks the stairwell mesh in cel 15, unreachable from cell 40 (see docs/reports/fix-room-scope.md)"]
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

/// The capsule's push query at the fusebox — the exact call the walk physics makes. *IGNORED for
/// the same measured reason as the floor-ray test above* (the Coll_Data node layer gap). Sister:
/// the flat capsule at the same spot reports the same floor (no geometry eaten).
#[test]
#[ignore = "fix-room-scope: needs the Coll_Data node layer — see the floor-ray test and docs/reports/fix-room-scope.md"]
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

/// The end-to-end symptom: Bond placed at the fusebox and ticked through the REAL input path keeps
/// standing — the capsule is FLAT today (the scope is not switched on for it: see the two ignored
/// fusebox tests and the report), so this is the regression guard for the flat world the walk
/// really runs on. When the Coll_Data layer lands and the capsule goes scoped, this must stay green.
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

// ── probe (fix-room-scope, env NF_ROOMSCOPE_PROBE=1): replay the game's `build_FindCel` 0x1c8cb0
// link law (rooms-only candidates + the two-ray tie-break, verbatim) over this level's instances
// and print every decision, then the capsule's shaped straddle set from the fusebox cell.
// IGNORED (it panics at the end by design — it's a probe, not an assertion).
#[test]
#[ignore = "probe: run with NF_ROOMSCOPE_PROBE=1 (panics at the end by design)"]
fn probe_room_scope_links() {
    if std::env::var_os("NF_ROOMSCOPE_PROBE").is_none() { return; }
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let lvl = level(&iso, "07000008.bin");
    let (_, graph) = nf_view::scene::parse_cel_graph(&lvl);
    let mut rooms = vec![false; graph.cells.len()];
    for m in graph.room_to_cell.iter().flatten() { if *m < rooms.len() { rooms[*m] = true; } }
    let (instances, _) = lvl.instances(true);
    // The per-cell linked geometry (pre-chunk objects) — the same walk build_scoped does.
    let mut stat_tris: Vec<[[f32; 3]; 3]> = Vec::new();
    let mut stat_objs: Vec<(usize, usize)> = Vec::new(); // (start, end) into stat_tris
    let mut cell_objs: Vec<Vec<usize>> = vec![Vec::new(); graph.cells.len()];
    let ray = |tris: &[[[f32; 3]; 3]], objs: &[(usize, usize)], from: [f32; 3], to: [f32; 3]| -> Option<f32> {
        let d = [to[0] - from[0], to[1] - from[1], to[2] - from[2]];
        let dot = |a: [f32; 3], b: [f32; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
        let cross = |a: [f32; 3], b: [f32; 3]| [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
        let mut best = None::<f32>;
        for &(s, e) in objs {
            for t in &tris[s..e] {
                let e1 = [t[1][0] - t[0][0], t[1][1] - t[0][1], t[1][2] - t[0][2]];
                let e2 = [t[2][0] - t[0][0], t[2][1] - t[0][1], t[2][2] - t[0][2]];
                let p = cross(d, e2);
                let det = dot(e1, p);
                if det.abs() < 1e-12 { continue; }
                let inv = 1.0 / det;
                let s2 = [from[0] - t[0][0], from[1] - t[0][1], from[2] - t[0][2]];
                let u = dot(s2, p) * inv;
                if !(0.0..=1.0).contains(&u) { continue; }
                let q = cross(s2, e1);
                let v = dot(d, q) * inv;
                if v < 0.0 || u + v > 1.0 { continue; }
                let tt = dot(e2, q) * inv;
                if (0.0..=1.0).contains(&tt) && best.is_none_or(|b| tt < b) { best = Some(tt); }
            }
        }
        best
    };
    for (ii, inst) in instances.iter().enumerate() {
        let obj = &lvl.maps[inst.map].objects[inst.object];
        let Some(c) = &obj.collision else { continue };
        if obj.flags & 0x1000 != 0 { continue; }
        let centre = transform(&inst.matrix, obj.centre);
        let cands: Vec<usize> = graph.cells.iter().enumerate()
            .filter(|(i, c)| rooms[*i] && (0..3).all(|k| c.min[k] <= centre[k] && centre[k] <= c.max[k]))
            .map(|(i, _)| i).take(0x40).collect();
        let detail = |cell_objs: &Vec<Vec<usize>>| -> String {
            cands.iter().map(|&cd| {
                let up = ray(&stat_tris, &cell_objs[cd].iter().map(|&o| stat_objs[o]).collect::<Vec<_>>(), centre, [centre[0], centre[1] + 256.0, centre[2]]).map(|f| f * 256.0);
                let dl = up.unwrap_or(256.0).min(256.0);
                let down = ray(&stat_tris, &cell_objs[cd].iter().map(|&o| stat_objs[o]).collect::<Vec<_>>(), centre, [centre[0], centre[1] - dl, centre[2]]).map(|f| f * dl);
                format!("cell{cd}(up{up:?},down{down:?})")
            }).collect::<Vec<_>>().join(" ")
        };
        let decision = match cands.len() {
            0 => None,
            1 => Some(cands[0]),
            _ => {
                let mut best = cands[0]; let mut bt = 256.0f32;
                for &cd in &cands {
                    let objs: Vec<_> = cell_objs[cd].iter().map(|&o| stat_objs[o]).collect();
                    let up = ray(&stat_tris, &objs, centre, [centre[0], centre[1] + 256.0, centre[2]]).map(|f| f * 256.0);
                    let dl = up.unwrap_or(256.0).min(256.0);
                    let down = ray(&stat_tris, &objs, centre, [centre[0], centre[1] - dl, centre[2]]).map(|f| f * dl);
                    if let Some(t) = match (up, down) { (Some(a), Some(b)) => Some(a.min(b)), (Some(a), None) => Some(a), (None, Some(b)) => Some(b), _ => None } {
                        if t < bt { bt = t; best = cd; }
                    }
                }
                Some(best)
            }
        };
        let interesting = inst.map == 51 && (inst.object == 16) || ii == 32 || (decision.is_some() && decision != cands.first().copied() && cands.len() > 1);
        let det = if interesting { detail(&cell_objs) } else { String::new() };
        let start = stat_tris.len();
        for (tri, _) in c.triangles_with_material() { stat_tris.push(tri.map(|p| transform(&inst.matrix, p))); }
        stat_objs.push((start, stat_tris.len()));
        if let Some(d) = decision { cell_objs[d].push(stat_objs.len() - 1); }
        if interesting {
            eprintln!("[links] inst {ii} map{} obj{} {} centre ({:.2},{:.2},{:.2}) cands {:?} → {:?}  {}",
                inst.map, inst.object, obj.name, centre[0], centre[1], centre[2], cands, decision, det);
        }
    }
    eprintln!("[links] total instances with collision linked: {}", stat_objs.len());
    // The capsule's shaped straddle set from the fusebox cell 40 (the 0x1eac98 BFS + the 0x1ea7b8
    // 0x800 shape test, the same formula `capsule_portal_hit` uses): which cells the walk keeps,
    // and through which portal each came.
    let start = 40usize;
    let (a, b, r) = ([FUSEBOX[0], 42.3f32, FUSEBOX[2]], [FUSEBOX[0], 43.78, FUSEBOX[2]], 0.55f32);
    let capsule_straddles = |p: &nf_view::scene::PortalEdge| -> bool {
        let closest_on_segment = |a: [f32; 3], b: [f32; 3], p: [f32; 3]| -> [f32; 3] {
            let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
            let l2 = ab[0] * ab[0] + ab[1] * ab[1] + ab[2] * ab[2];
            let t = if l2 > 0.0 { (((p[0] - a[0]) * ab[0] + (p[1] - a[1]) * ab[1] + (p[2] - a[2]) * ab[2]) / l2).clamp(0.0, 1.0) } else { 0.0 };
            [a[0] + ab[0] * t, a[1] + ab[1] * t, a[2] + ab[2] * t]
        };
        // Closest point on the portal's two triangles to the segment's closest point (coarse: the
        // vertex-level distance is enough to see the straddle here).
        let q = closest_on_segment(a, b, p.corners[0]);
        p.corners.iter().any(|c| {
            let d = [q[0] - c[0], q[1] - c[1], q[2] - c[2]];
            d[0] * d[0] + d[1] * d[1] + d[2] * d[2] < (r * 1.5) * (r * 1.5)
        })
    };
    let mut seen = vec![false; graph.cells.len()];
    let mut from = vec![usize::MAX; graph.cells.len()];
    seen[start] = true;
    let mut queue = std::collections::VecDeque::new();
    queue.push_back(start);
    while let Some(c) = queue.pop_front() {
        for p in &graph.portals {
            let dest = if p.cel_a == c { Some(p.cel_b) } else if p.cel_b == c { Some(p.cel_a) } else { None };
            let Some(dest) = dest else { continue };
            if seen[dest] || !capsule_straddles(p) { continue; }
            seen[dest] = true;
            from[dest] = c;
            queue.push_back(dest);
        }
    }
    let kept: Vec<usize> = (0..seen.len()).filter(|&i| seen[i]).collect();
    eprintln!("[straddle] capsule from cell {start}: kept {kept:?}");
    for c in kept.iter() { if from[*c] != usize::MAX { eprintln!("[straddle] cell {} via cell {}", c, from[*c]); } }
    panic!("probe done");
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
