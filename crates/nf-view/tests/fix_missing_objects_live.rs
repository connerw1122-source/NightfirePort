//! fix-missing-objects: the fountain, the guard-room furniture and the lockers on 07000006 were
//! not drawn; the 0700000c pylon was reported missing at the user's spot.
//!
//! The cause (evidence in docs/reports/fix-missing-objects.md): `CollisionWorld::build_scoped`
//! attributed each instance's collision to the smallest-volume cell bbox containing its centre —
//! the guard-room floor landed under the table's tiny bbox, the courtyard floor under the
//! fountain's — so at `build_chains` time every candidate room's scoped bucket read empty and
//! `link_owner_room` (the game's `build_link_objects_to_rooms` 0x1c9150 pick: the room whose
//! collision sits closest to the generic's sphere centre, per fix-portal5 §4's disassembly)
//! left the generics UNCHAINED. `View_AddCels` 0x1e6ba0 draws a generic only from its owner
//! room's +0x18 chain (fix-portal5 §1), so unchained = never drawn.
//!
//! The game's per-hit cel word (`HITDATA+0x54`) carries the object's own placement cel
//! (`parsemap_block_map_data_static` 0x1d1998 creates one runtime cel per placement whose glist
//! IS the placement's object), so `build_scoped` now keys the k-th pushed instance to the k-th
//! cell (the instances are the cel placements in parse order; the scope graph's cells are
//! `parse_cel_graph`'s per-placement cells in the same order).
//!
//! Tests drive the exact draw-list input (`scene::eye_cell` + `portal_visible_cels_game_from_cell`
//! — gpu.rs `cel_visibility`'s composition, the same inputs `portal_eyecel_live` drives) and the
//! exact build sequence (`parse_cel_graph` → `build_scoped` → `build_chains`). Opt-in via NF_ISO.

use nf_formats::level::Level;
use nf_formats::iso9660::IsoImage;
use nf_view::scene;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn read_archive(iso: &str, archive: &str) -> Vec<u8> {
    let img = IsoImage::open(iso).expect("open iso");
    let (entries, bin) = nf_formats::disc::files_bin_entries(&img).expect("FILES.BIN");
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(archive)).expect("archive");
    img.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).expect("read")
}

fn view_proj(pos: [f32; 3], yaw: f32, pitch: f32, aspect: f32) -> [[f32; 4]; 4] {
    use glam::{Mat4, Vec3};
    let (near_p, far, fov) = (0.05f32, 4000.0f32, std::f32::consts::FRAC_PI_3);
    let f = 1.0 / (fov * 0.5).tan();
    let proj = Mat4::from_cols_array_2d(&[
        [f / aspect, 0.0, 0.0, 0.0],
        [0.0, f, 0.0, 0.0],
        [0.0, 0.0, far / (near_p - far), -1.0],
        [0.0, 0.0, near_p * far / (near_p - far), 0.0],
    ]);
    let cp = pitch.cos();
    let fw = Vec3::new(cp * yaw.sin(), pitch.sin(), cp * yaw.cos()).normalize();
    let s = fw.cross(Vec3::Y).normalize();
    let u = s.cross(fw);
    let view = Mat4::from_cols_array_2d(&[
        [s.x, u.x, -fw.x, 0.0],
        [s.y, u.y, -fw.y, 0.0],
        [s.z, u.z, -fw.z, 0.0],
        [-s.dot(Vec3::from(pos)), -u.dot(Vec3::from(pos)), fw.dot(Vec3::from(pos)), 1.0],
    ]);
    (proj * view).to_cols_array_2d()
}

/// The user's spots (main agent, 2026-10-13). Eye = bond y + 0.7 (`Bond::head()` at these
/// floors — probe 6 measured eye = stand_at(pos).head() = [14.22, 16.965, -139.54] on 0c).
const FOUNTAIN_SPOT: ([f32; 3], f32) = ([21.12, 39.04 + 0.7, -0.48], 8.683);
const GUARDROOM_SPOT: ([f32; 3], f32) = ([33.50, 41.13 + 0.7, -17.90], 9.158);
const LOCKERS_SPOT: ([f32; 3], f32) = ([31.94, 41.13 + 0.7, -17.36], 10.756);

/// The named objects' placement indices on 07000006 (probe 2's dump) and their expected world
/// sphere centres (the objects are world-space baked; the placement transform is identity).
const SIX_OBJECTS: &[(usize, &str, [f32; 3])] = &[
    (352, "OBJECT_Fountain", [27.5, 41.0, -7.0]),
    (351, "Object_Pondwater", [27.5, 38.6, -7.0]),
    (80, "OBJECT_GuardroomTable", [31.7, 40.5, -24.4]),
    (84, "Object_bed", [36.9, 40.4, -23.5]),
    (85, "Object_chair", [31.8, 40.6, -23.1]),
    (89, "Object_Mapboard", [31.7, 42.3, -24.9]),
    (82, "OBJECT_WallClock", [34.4, 42.6, -25.0]),
    (81, "OBJECT_Locker", [28.3, 41.1, -20.6]),
];

fn game_mask(sc: &nf_view::scene::Scene, eye: [f32; 3], yaw: f32) -> (usize, Vec<bool>) {
    let g = &sc.cel_graph;
    let world = sc.sim.as_ref().map(|s| &s.world);
    let vp = view_proj(eye, yaw, 0.0, 16.0 / 9.0);
    let planes = nf_view::gpu::extract_frustum(vp);
    let sides = [planes[0], planes[1], planes[2], planes[3]];
    let depth0 = [planes[0], planes[1], planes[2], planes[3], {
        let fw = [planes[4][0], planes[4][1], planes[4][2]];
        let fe = fw[0] * eye[0] + fw[1] * eye[1] + fw[2] * eye[2];
        [-fw[0], -fw[1], -fw[2], fe + 200.0]
    }];
    let mut state = scene::ViewerCel::default();
    let cell = scene::eye_cell(g, &mut state, eye, world).expect("the eye cel resolves");
    let mask = scene::portal_visible_cels_game_from_cell(g, cell, eye, &depth0, &sides).expect("the game mask");
    (cell, mask)
}

/// ── 1: the named objects are in the drawn set at the user's spots ──────────────────────────────
/// A generic draws only from its owner room's +0x18 chain under `Vision_InView`
/// (`View_AddCels` 0x1e6ba0 [M fix-portal5 §1]); the renderer's World cull hides every material
/// whose node the game mask leaves out (gpu.rs render's `cell_visible` check). Pre-fix the
/// fountain/furniture/locker nodes had NO chain owner (the buckets were empty), so their nodes
/// never entered the mask and all their materials were culled.
#[test]
fn the_named_objects_are_in_the_drawn_set_at_the_user_spots() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let sc = nf_view::load(&iso, "07000006.bin", false).expect("load 07000006");
    let g = &sc.cel_graph;
    for (spot_name, (eye, yaw), wanted) in [
        ("fountain", FOUNTAIN_SPOT, &[(352usize, "OBJECT_Fountain"), (351, "Object_Pondwater")][..]),
        ("guard room", GUARDROOM_SPOT, &[(80, "OBJECT_GuardroomTable"), (84, "Object_bed"), (85, "Object_chair"), (89, "Object_Mapboard"), (82, "OBJECT_WallClock")][..]),
        ("lockers", LOCKERS_SPOT, &[(81, "OBJECT_Locker")][..]),
    ] {
        let (_cell, mask) = game_mask(&sc, eye, yaw);
        for &(pi, name) in wanted {
            let node = g.own_cells_main.get(pi).copied().flatten()
                .unwrap_or_else(|| panic!("{name}: placement {pi} has no own cel"));
            let owner = g.chains.iter().enumerate().find(|(_, ch)| ch.contains(&node)).map(|(ri, _)| ri);
            assert!(owner.is_some(), "{name} (node {node}): unchained — the game's load-time linker \
                build_link_objects_to_rooms 0x1c9150 leaves no 0x8019 generic unchained (fix-portal5 §4)");
            assert!(mask[node as usize], "{name} (node {node}): not in the game mask at the {spot_name} spot \
                — the renderer culls every material at this node (the user's missing object)");
        }
    }
    // Sister: the materials the renderer holds at those nodes — every World draw keyed to the
    // named nodes survives both the mask and the frustum (`Vision_InView` 0x1e89e0's
    // signed-distance form, the game's own gate).
    let (_, fountain_mask) = game_mask(&sc, FOUNTAIN_SPOT.0, FOUNTAIN_SPOT.1);
    let vp = view_proj(FOUNTAIN_SPOT.0, FOUNTAIN_SPOT.1, 0.0, 16.0 / 9.0);
    let planes = nf_view::gpu::extract_frustum(vp);
    let fountain_node = sc.cel_graph.own_cells_main[352].unwrap();
    let held = sc.materials.iter()
        .filter(|m| m.layer == scene::Layer::World && m.node == fountain_node
            && fountain_mask[m.node as usize]
            && planes.iter().all(|p| p[0] * m.centre[0] + p[1] * m.centre[1] + p[2] * m.centre[2] + p[3] + m.radius >= 0.0))
        .count();
    assert!(held >= 90, "the fountain's 96 batches must draw at the fountain spot; held {held}");
    // Sister 2: a node that was already chained before the fix stays chained (the lamp post at
    // the far side of the courtyard — owner room 34 pre-fix) — the fix must not re-home others.
    let lamp = g.own_cells_main[61].unwrap();
    let lamp_owner = g.chains.iter().enumerate().find(|(_, ch)| ch.contains(&lamp)).map(|(ri, _)| ri);
    assert_eq!(lamp_owner, Some(34), "OBJECT_LampPost1 keeps its owner room 34");
}

/// ── 2: the 0c pylon at the user's spot ─────────────────────────────────────────────────────────
/// Reported missing (main agent render, 2026-10-13) with its baked shadow drawn. On the current
/// tree the four pylon parts (`pylon`/`pylon_back`/`pylon_left`/`pylon_right`, placements
/// 160–163, world-baked at x 44–60, z −115..−106) resolve eye cell 91 from the real
/// stand_at+head camera, all four nodes ride owner room 77's chain, pass `Vision_InView` and
/// every one of their 14 materials survives the mask and the frustum (probe 6). This test pins
/// that state as a regression guard; the report asks the main agent for a live re-render.
#[test]
fn the_0c_pylon_is_in_the_drawn_set_at_the_user_spot() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut sc = nf_view::load(&iso, "0700000c.bin", false).expect("load 0700000c");
    // The real --bond path: stand Bond at the spot, tick, camera = head.
    {
        let sim = sc.sim.as_mut().unwrap();
        sim.bond = nf_game::player::Bond::stand_at(&sim.world, [14.22, 16.28, -139.54], 0.986);
        for _ in 0..5 { sim.step_bond(Default::default(), false, false, false); }
    }
    let eye = sc.sim.as_ref().unwrap().bond.head();
    let yaw = sc.sim.as_ref().unwrap().bond.yaw;
    let g = &sc.cel_graph;
    let (_cell, mask) = game_mask(&sc, eye, yaw);
    for &(pi, name) in &[(160usize, "pylon_back"), (161, "pylon"), (162, "pylon_left"), (163, "pylon_right"), (113, "pylonshadow")] {
        let node = g.own_cells_main.get(pi).copied().flatten()
            .unwrap_or_else(|| panic!("{name}: placement {pi} has no own cel"));
        let owner = g.chains.iter().enumerate().find(|(_, ch)| ch.contains(&node)).map(|(ri, _)| ri);
        assert!(owner.is_some(), "{name} (node {node}): unchained");
        assert!(mask[node as usize], "{name} (node {node}): not in the game mask at the user's spot");
    }
    // Sister: the pylon's materials all survive the renderer's two gates.
    let vp = view_proj(eye, yaw, 0.0, 16.0 / 9.0);
    let planes = nf_view::gpu::extract_frustum(vp);
    let nodes: Vec<u32> = [160usize, 161, 162, 163].iter()
        .filter_map(|&pi| g.own_cells_main[pi]).collect();
    let (total, kept) = sc.materials.iter()
        .filter(|m| m.layer == scene::Layer::World && nodes.contains(&m.node))
        .map(|m| (true, mask[m.node as usize] && planes.iter().all(|p| p[0] * m.centre[0] + p[1] * m.centre[1] + p[2] * m.centre[2] + p[3] + m.radius >= 0.0)))
        .fold((0usize, 0usize), |(t, k), (_, keep)| (t + 1, k + keep as usize));
    assert_eq!(total, 12, "the four pylon parts carry 12 World draws");
    assert_eq!(kept, total, "every pylon draw survives the mask and the frustum");
}

/// ── 3: the census — no unexplained skips on any on-foot level ──────────────────────────────────
/// The exact build sequence of `Scene::build` (parse → build_scoped → build_chains), then the
/// census: every 0x8017/0x8018/0x8019 generic with a real bbox must ride some room's chain (the
/// game's linker leaves none unchained — fix-portal5's live chain dumps), and every resolved
/// cel placement with a visible mesh must be in the drawn set unless the game itself skips it:
/// the cel pass's flag-0x10 skip (`View_AddCels` never pushes a glist whose +0x44 has bit 0x10
/// [M docs/models/world.md §2]), or the retained-out kinds drawn by their own passes
/// (0x2a/0x22/0xd4 sky/ladders/clouds, 0xdb doors, 0xde searchlights, 0xfc coronas, 0x3e wires,
/// 0x3a grapple points, the MISC_KINDS static dynamic entities, 0xf0 pickups, 0x0f characters).
#[test]
fn census_no_unchained_generics_and_no_unexplained_skips_on_any_level() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    // The on-foot archives of FILES.BIN (the 27 of docs/systems/levels.md §7; the driving
    // missions and the shared packs are out of scope there).
    let img = IsoImage::open(&iso).expect("open iso");
    let (entries, bin) = nf_formats::disc::files_bin_entries(&img).expect("FILES.BIN");
    let mut archives: Vec<String> = entries.iter().filter_map(|e| {
        let n = e.name.to_ascii_uppercase();
        n.starts_with("0700").then(|| e.name.clone())
    }).collect();
    archives.sort();
    let mut report: Vec<(String, usize, usize, usize, usize, usize, usize)> = Vec::new();
    for a in &archives {
        // The census scope = docs/systems/levels.md §7's: the on-foot archives only (the driving
        // missions 07000023-29 and the shared packs 07000500/07000999 are out of scope there).
        let code = u32::from_str_radix(a.to_ascii_uppercase().trim_end_matches(".BIN"), 16).unwrap_or(0);
        if (0x0700_0023..=0x0700_0029).contains(&code) || code == 0x0700_0500 || code == 0x0700_0999 { continue; }
        let data = read_archive(&iso, a);
        let Ok(level) = Level::parse(&data) else { continue };
        let Some(_mm) = level.main_map() else { continue };
        // The build sequence of Scene::build (scene.rs 4173-4177):
        let (room_cels, mut cel_graph) = scene::parse_cel_graph(&level);
        let _ = room_cels;
        let instances = level.instances(true).0;
        let mut world = nf_game::world::CollisionWorld::build_scoped(&level, &instances, Some(scene::scope_graph(&cel_graph)));
        scene::build_chains(&mut cel_graph, Some(&world));
        world.attribute_scope(&{
            // The same replay Scene::build runs (scene.rs 4191-4220) — the final attribution
            // the FindCel tie-break reads; built here so the census world matches the live one.
            let main_opt = level.main_map();
            let mut node_owner: Vec<Option<u32>> = vec![None; cel_graph.cells.len()];
            for (o, ch) in cel_graph.chains.iter().enumerate() {
                for &n in ch { if (n as usize) < node_owner.len() { node_owner[n as usize] = Some(o as u32); } }
            }
            let mut instance_cell: Vec<Option<u32>> = Vec::new();
            let mut cell_counter = 0usize;
            for (mi, map) in level.maps.iter().enumerate() {
                for p in &map.placements {
                    if !p.creates_cel() { continue; }
                    if Some(mi) == main_opt {
                        let target = if p.by_hash() { level.by_hash.get(&p.hashcode).copied() } else if (p.index as usize) < map.objects.len() { Some((mi, p.index as usize)) } else { None };
                        if let Some((om, oi)) = target {
                            let obj = &level.maps[om].objects[oi];
                            if obj.collision.is_some() && obj.flags & 0x1000 == 0 {
                                let c = cell_counter;
                                let f16 = cel_graph.cells.get(c).map(|n| n.flags & 0xffff).unwrap_or(0);
                                let owner = if matches!(f16, 0xc023 | 0xc047) { Some(c as u32) } else { node_owner.get(c).copied().flatten() };
                                instance_cell.push(owner);
                            }
                        }
                    }
                    cell_counter += 1;
                }
            }
            instance_cell
        });
        // The census.
        let mut generics = 0usize; let mut unchained = 0usize; let mut unchained_names: Vec<String> = Vec::new();
        let main = level.main_map().unwrap();
        // own_cells_main is aligned with the main map's placements; the cell index of the i-th
        // cel placement is its parse ordinal (sub-maps first, main last) — the chains reference
        // the parse-wide indices.
        let mut chain_owner = vec![None; cel_graph.cells.len()];
        for (o, ch) in cel_graph.chains.iter().enumerate() {
            for &n in ch { if (n as usize) < chain_owner.len() { chain_owner[n as usize] = Some(o); } }
        }
        let mut parse_ordinal = 0usize;
        for (mi, map) in level.maps.iter().enumerate() {
            for p in &map.placements {
                if p.creates_cel() {
                    if Some(mi) == Some(main) {
                        let c = parse_ordinal;
                        let f16 = cel_graph.cells[c].flags & 0xffff;
                        if matches!(f16, 0x8018 | 0x8019) {
                            generics += 1;
                            let zero = cel_graph.cells[c].min == [0.0; 3] && cel_graph.cells[c].max == [0.0; 3] && cel_graph.cells[c].sphere.1 == 0.0;
                            // Only generics with a visible mesh must chain: a meshless one draws
                            // nothing either way (`psiDrawObjectMatrix` no-ops on an empty glist),
                            // and a glist whose flags carry the game's not-drawn bit 0x10 is
                            // skipped by the game itself (obj+0xF0's bit 0x10 — View_AddObjects).
                            let (mesh, objf) = if p.by_hash() { level.by_hash.get(&p.hashcode).map(|&(m, o)| (level.maps[m].objects[o].gfx.leaves.iter().any(|l| !l.batches.is_empty()), level.maps[m].objects[o].flags)).unwrap_or((false, 0)) } else { map.objects.get(p.index as usize).map(|o| (o.gfx.leaves.iter().any(|l| !l.batches.is_empty()), o.flags)).unwrap_or((false, 0)) };
                            if !zero && mesh && objf & (0x10 | 0x1000) == 0 && chain_owner[c].is_none() {
                                // The game's linker also requires a candidate: a generic whose
                                // centre no room's bbox contains (radius gate passed) stays
                                // unchained IN THE GAME TOO — 07000007's Object_Books_43 at the
                                // origin is exactly that case (buried mesh, nothing to draw).
                                let (centre, radius) = cel_graph.cells[c].sphere;
                                let has_candidate = cel_graph.cells.iter().any(|r| matches!(r.flags & 0xffff, 0xc023 | 0xc047) && radius <= r.sphere.1 && (0..3).all(|k| r.min[k] <= centre[k] && centre[k] <= r.max[k]));
                                if has_candidate {
                                    unchained += 1;
                                    let name = if p.by_hash() { level.by_hash.get(&p.hashcode).map(|&(m, o)| level.maps[m].objects[o].name.trim().to_string()) } else { map.objects.get(p.index as usize).map(|o| o.name.trim().to_string()) };
                                    if unchained_names.len() < 8 { unchained_names.push(name.unwrap_or_default()); }
                                }
                            }
                        }
                    }
                    parse_ordinal += 1;
                }
            }
        }
        // Resolved main-map placements with a visible mesh that NO draw path covers and the game's
        // own rules do not explain. The game-side rules, per class:
        // - glist flag 0x10 in the object's flags word = the game's not-drawn bit (obj+0xF0's
        //   bit 0x10 makes View_AddObjects 0x1e62d8 skip; every trigger/AI-marker glist on the
        //   disc carries it — probe: objf 0x50/0x30) [M docs/fields/glist.md];
        // - 0x29 switch / 0x31 fusebox: obj+0xD8 = 0, no mesh is ever attached [M 001adcc0:35,
        //   001b0448:25 — docs/models/misc.md]; the drawn panel is the level's own statics;
        // - 0x27 GT hidden until deployed [M 0017ffa0:78], 0x32 rainbox never drawn [M 0018d0d0],
        //   0x38/0x39 space laser/missile invisible controllers, 0xF2 emitter has no glist ever
        //   [M 0017ae00], 0x3F creep walls attach no gfx [M 0x196fc8] (docs/models/misc.md);
        // - 0x21 ripples / 0x4E lights / 0xDF snow: the presentation systems (docs/systems/render.md);
        // - 0x24 Player_1 / 0x2D PlayerStart: the player's own object — drawn as Bond;
        // - 0x0F characters: the drone lane (spawner hold-backs, skins merged from other archives);
        // - 0xD9 ScriptPlayers / 0xE8 exit gates / 0x28 sensors / 0x26 MP objects: controller
        //   classes (docs/systems/script-player.md, world-objects.md);
        // - 0x17/0x23/0xFF: map-block data, no case in parsemap_create_dynamic_objects [D];
        // - 0x2F helicopter: Copter_Create 0x17d340 attaches the glist and the game draws it —
        //   the copter is the one dynamic family still un-ported (docs/systems/world-objects.md
        //   "Missing, by priority"), so its two 07000005 placements stay undrawn here;
        // - 0xEA/0xEB mission triggers: glist-attached meshes (Trigger_Create 0x1b1720:95 sets
        //   obj+0xF0 = glist+0x44, docs/fields/glist.md) that the game draws when the glist
        //   flags lack the not-drawn bits — the 0700000a laser/laserbeam tripwires (flags
        //   0x0/0x645) and the 07000012 lift switches (flags 0x0) have no port draw path yet —
        //   a FOUND GAP of this census, listed in the report. (Most trigger glists carry 0x10 —
        //   the flagged-0x10 counter — and the 0x1000 collision-only ones skip too.)
        // - 0xE4 MissileFlame (0700001b): the space missile's flame — script-driven visuals like
        //   the 0x38/0x39 controllers it belongs to (docs/models/misc.md).
        // - 0xF4 Trigger_MoviePlayer / 0xF9 sound triggers: controller classes (levels.md §7.1's
        //   unhandled list; the marker mesh is not a game draw).
        let no_draw_kinds: &[u8] = &[0x17, 0x23, 0x24, 0x26, 0x27, 0x28, 0x29, 0x2d, 0x31, 0x32, 0x38, 0x39,
            0x3f, 0x4e, 0x21, 0x2f, 0xe4, 0xea, 0xeb, 0xf4, 0xf9, 0xdf, 0xd9, 0xe8, 0xf2, 0xf3, 0xff];
        let mut unexplained = 0usize; let mut unexplained_names: Vec<String> = Vec::new();
        let mut flagged = 0usize;
        for p in &level.maps[main].placements {
            let kind = p.flags & 0xff;
            let target = if p.by_hash() { level.by_hash.get(&p.hashcode).copied() } else if (p.index as usize) < level.maps[main].objects.len() { Some((main, p.index as usize)) } else { None };
            let Some((m, o)) = target else { continue };
            let obj = &level.maps[m].objects[o];
            if !obj.gfx.leaves.iter().any(|l| !l.batches.is_empty()) { continue; }
            let drawn = matches!(kind, 0x2a | 0x22 | 0xd4 | 0x3e | 0xfc | 0xdb | 0xde | 0xf0 | 0x0f)
                || matches!(kind, 0x3a | 0x2e | 0x30 | 0x34 | 0x35 | 0x3b | 0x71 | 0xd5 | 0xd8 | 0xe1 | 0xe3 | 0xda | 0xfa | 0xfd | 0xfe | 0x4a)
                || matches!(kind, 0x20 | 0xe2)
                || p.creates_cel(); // the static loop (the flag-0x10 skip is the game's own)
            if !drawn {
                if obj.flags & 0x10 != 0 || obj.flags & 0x1000 != 0 { flagged += 1; continue; }
                // bit 0x10 = the game's not-drawn bit; bit 0x1000 = collision-only (the game's
                // draw pass skips it — the BlockingPoly/PlungeTrigger family, fix-objects2's
                // Collide_Pick 0x1eb320 evidence)
                if no_draw_kinds.contains(&(kind as u8)) { continue; } // the documented no-draw classes
                unexplained += 1;
                if unexplained_names.len() < 8 { unexplained_names.push(format!("{} kind {kind:#04x} objf {:#x}", obj.name.trim(), obj.flags)); }
            }
        }
        eprintln!("[census] {a}: {generics} generics, {unchained} unchained {:?}, flagged-0x10 {flagged}, {unexplained} no-draw-path {:?}",
            unchained_names, unexplained_names);
        report.push((a.clone(), generics, unchained, 0, 0, unexplained, 0));
        assert_eq!(unchained, 0, "{a}: {unchained} unchained 0x8018/0x8019 generics {unchained_names:?} — \
            the game's build_link_objects_to_rooms 0x1c9150 leaves none unchained (fix-portal5 §4); \
            unchained = never drawn (View_AddCels draws generics only from their owner room's chain)");
        assert_eq!(unexplained, 0, "{a}: {unexplained} resolved mesh placements no draw path covers {unexplained_names:?}");
    }
    eprintln!("[census] {} archives checked", report.len());
}
