//! fix-bullet-child-list: the searchlight head's bullet break through the game's exact
//! child-list path (U-041) — through the real input path (`nf_view::load` + `Sim::step_bond`,
//! `step_bond(.., trigger = true, ..)`) on 07000006 (four courtyard head+mount pairs,
//! docs/systems/searchlight.md).
//!
//! The game's path [M]: `Collide_Update` 0x1ea398 mirrors each bullet contact into the hit
//! object's `obj+0xD0` list (the node's `+0x08` = the bullet's `HITTEST+0x90` damage =
//! `weapon_definition+0xC` [M Bullet_update 0x12e7b0:391], `+0x58` = the bullet obj), and
//! `Searchlight_Update` 0x1cb900 breaks the light when its walk (LAB_001cc1d4-0x1cc2f8) finds an
//! entry whose `+0x58` is a class-5 bullet with `+0x08` > 0.0. ONLY the head runs that walk (the
//! kind-1 gate at 0x1cb900:88-90; class 0x22's collision handler is 0 [L control_funcs 0x26f890]),
//! so a mount-only hit never breaks. The contact itself is `Collide_Intersect` 0x1ebaa8's ray
//! against the head's collision cels (`obj+0xD8`, map block 0x2E) in the head's `obj+0x90` frame
//! (`Intersect_RayGeom` 0x10a8c8: the COLLDATA box tree down to 0x40-byte triangles).
//!
//! Fail-first (shown on the old stand-in, which broke the light on ANY segment crossing the head's
//! or mount's placement gfx box — the box tested in the part's CURRENT matrix frame):
//! - `a_mount_only_shot_never_breaks_the_light` — a shot through the mount's pole below the head's
//!   collision cels cannot touch a head triangle (the disc's own bbox), yet the old code broke the
//!   light because it chained the mount's box into the walk;
//! - `a_head_box_corner_shot_that_misses_the_cels_does_not_break` — a level ray crossing the old
//!   stand-in box (asserted with the old predicate itself) that misses every head collision
//!   triangle: the old box caught it, the game's cels don't.

use nf_game::player::Input;
use nf_formats::level::Level;
use nf_formats::placement::object_matrix;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// The yaw distance to the placement frame (the sweep phase's distance from base_yaw).
fn yaw_off(yaw: f32, base: f32) -> f32 {
    let mut d = (yaw - base + std::f32::consts::PI).rem_euclid(2.0 * std::f32::consts::PI) - std::f32::consts::PI;
    if d < 0.0 { d = -d; }
    d
}

type V3 = [f32; 3];

/// Wait out the level-start NIS and the boot's channel staging until the first head sweeps.
fn load_and_light(iso: &str) -> nf_view::scene::Scene {
    let mut scene = nf_view::load(iso, "07000006.bin", false).expect("load 07000006");
    let sim = scene.sim.as_mut().unwrap();
    for _ in 0..300 { sim.step_bond(Default::default(), false, false, false); if !sim.disabled { break; } }
    for _ in 0..300 {
        sim.step_bond(Default::default(), false, false, false);
        if sim.drones.searchlights.parts.iter().any(|p| p.kind == 1 && p.state == 0) { break; }
    }
    scene
}

/// The level re-parse: the parts' collision triangles (the game's `obj+0xD8` cels, map block 0x2E)
/// and the placements, keyed by the main map's placement index (the port's `Part::placement`).
fn level_geom(iso: &str) -> (Level, usize) {
    let img = nf_formats::iso9660::IsoImage::open(iso).unwrap();
    let (entries, bin) = nf_formats::disc::files_bin_entries(&img).unwrap();
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case("07000006.bin")).unwrap();
    let data = img.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).unwrap();
    let level = Level::parse(&data).unwrap();
    let m = level.main_map().unwrap();
    (level, m)
}

fn object_of<'a>(level: &'a Level, m: usize, p: &nf_formats::placement::Placement) -> &'a nf_formats::level::MapObject {
    if p.by_hash() {
        let &(map, object) = level.by_hash.get(&p.hashcode).expect("placement hash resolves");
        &level.maps[map].objects[object]
    } else {
        &level.maps[m].objects[p.index as usize]
    }
}

fn part_tris(level: &Level, m: usize, placement: usize) -> Vec<[[f32; 3]; 3]> {
    object_of(level, m, &level.maps[m].placements[placement])
        .collision.as_ref().expect("the searchlight part object carries collision cels").triangles()
}

/// Segment vs triangle (Möller–Trumbore, t clamped to the flight [0,1]) — the test's own copy of
/// the game's `Intersect_RayGeom` tri pass.
fn seg_tri(from: V3, to: V3, a: V3, b: V3, c: V3) -> bool {
    let d = [to[0] - from[0], to[1] - from[1], to[2] - from[2]];
    let e1 = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let e2 = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    let sub = |x: V3, y: V3| [x[0] - y[0], x[1] - y[1], x[2] - y[2]];
    let cr = |x: V3, y: V3| [x[1] * y[2] - x[2] * y[1], x[2] * y[0] - x[0] * y[2], x[0] * y[1] - x[1] * y[0]];
    let dt = |x: V3, y: V3| x[0] * y[0] + x[1] * y[1] + x[2] * y[2];
    let p = cr(d, e2);
    let det = dt(e1, p);
    if det.abs() < 1e-12 { return false; }
    let inv = 1.0 / det;
    let s = sub(from, a);
    let u = dt(s, p) * inv;
    if !(0.0..=1.0).contains(&u) { return false; }
    let q = cr(s, e1);
    let v = dt(d, q) * inv;
    if v < 0.0 || u + v > 1.0 { return false; }
    (0.0..=1.0).contains(&(dt(e2, q) * inv))
}

/// The part's local frame: the game's inverse-`obj+0x90` (a rotation + translation).
fn to_local(mtx: &[[f32; 4]; 4], v: V3) -> V3 {
    let d = [v[0] - mtx[3][0], v[1] - mtx[3][1], v[2] - mtx[3][2]];
    [0, 1, 2].map(|k| d[0] * mtx[k][0] + d[1] * mtx[k][1] + d[2] * mtx[k][2])
}

fn world(mtx: &[[f32; 4]; 4], v: V3) -> V3 {
    [mtx[0][0] * v[0] + mtx[1][0] * v[1] + mtx[2][0] * v[2] + mtx[3][0],
     mtx[0][1] * v[0] + mtx[1][1] * v[1] + mtx[2][1] * v[2] + mtx[3][1],
     mtx[0][2] * v[0] + mtx[1][2] * v[1] + mtx[2][2] * v[2] + mtx[3][2]]
}

fn tri_hit_local(mtx: &[[f32; 4]; 4], tris: &[[[f32; 3]; 3]], from: V3, to: V3) -> bool {
    let (lf, lt) = (to_local(mtx, from), to_local(mtx, to));
    tris.iter().any(|t| seg_tri(lf, lt, t[0], t[1], t[2]))
}

/// The old stand-in's box: the object's gfx min/max under the placement matrix (world axes).
fn old_box_of(lo: V3, hi: V3, pm: &[[f32; 4]; 4]) -> (V3, V3) {
    let mut bmin = [f32::INFINITY; 3];
    let mut bmax = [f32::NEG_INFINITY; 3];
    for x in [lo[0], hi[0]] { for y in [lo[1], hi[1]] { for z in [lo[2], hi[2]] {
        let w = world(pm, [x, y, z]);
        for k in 0..3 { bmin[k] = bmin[k].min(w[k]); bmax[k] = bmax[k].max(w[k]); }
    } } }
    (bmin, bmax)
}

fn old_seg_box(from: V3, to: V3, min: V3, max: V3) -> bool {
    let d = [to[0] - from[0], to[1] - from[1], to[2] - from[2]];
    let mut t0 = 0.0f32;
    let mut t1 = 1.0f32;
    for k in 0..3 {
        if d[k].abs() < 1e-12 {
            if from[k] < min[k] || from[k] > max[k] { return false; }
        } else {
            let mut a = (min[k] - from[k]) / d[k];
            let mut b = (max[k] - from[k]) / d[k];
            if a > b { std::mem::swap(&mut a, &mut b); }
            t0 = t0.max(a);
            t1 = t1.min(b);
            if t0 > t1 { return false; }
        }
    }
    true
}

/// A aimed spread burst through the real path: `n` fire/no-fire step pairs at the target from
/// 6 units south, the yaw/pitch jittered +-0.02 rad over the burst (the muzzle offset rides the
/// gun's anim pose and is not constant). Returns the flight rays the bullets actually flew,
/// read off `bullet_paths` each fire tick (the searchlight's own inputs).
fn burst(scene: &mut nf_view::scene::Scene, target: V3, n: usize) -> Vec<(V3, V3)> {
    let mut rays = Vec::new();
    let mut delta = [0.0f32; 3];
    for k in 0..n {
        let aim = if k == 0 {
            [target[0], target[1] - 0.9, target[2] + 6.0]
        } else {
            // The muzzle rides the gun's anim pose (not constant): shot 0 measured it, the rest
            // compensate so the flight runs level through the target.
            [target[0] - delta[0], target[1] - delta[1], target[2] + 6.0]
        };
        {
            let sim = scene.sim.as_mut().unwrap();
            sim.bond.pos = aim;
            sim.bond.vel = [0.0; 3];
            sim.bond.yaw = std::f32::consts::PI + ((k % 7) as f32 - 3.0) * 0.035;
            sim.bond.pitch = -((k / 7) as f32 - 1.5) * 0.025;
        }
        scene.sim.as_mut().unwrap().step_bond(Input { fire: true, ..Input::default() }, true, false, false);
        {
            let sim = scene.sim.as_mut().unwrap();
            let (paths, _) = sim.drones.bullet_paths();
            if let Some(&(a, _)) = paths.first() {
                if k == 0 { delta = [a[0] - aim[0], a[1] - aim[1], a[2] - aim[2]]; }
            }
            rays.extend(paths.iter().cloned());
        }
        scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
    }
    rays
}

#[test]
fn a_lamp_housing_shot_breaks_the_light() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load_and_light(&iso);
    let (level, m) = level_geom(&iso);
    let head = {
        let sim = scene.sim.as_ref().unwrap();
        sim.drones.searchlights.parts.iter()
            .filter(|p| p.kind == 1 && p.links[0].is_some() && p.state == 0)
            .min_by(|a, b| yaw_off(a.yaw, a.base_yaw).total_cmp(&yaw_off(b.yaw, b.base_yaw)))
            .cloned().expect("a sweeping head with a mount")
    };
    assert_eq!(head.state, 0);
    let tris = part_tris(&level, m, head.placement);
    assert!(!tris.is_empty());
    // Aim the burst at the collision hull's centroid (the housing surrounds it; the raw pivot can
    // sit in a cel hole).
    let c: V3 = tris.iter().flat_map(|t| t.iter())
        .fold([0.0f32; 3], |acc, v| [acc[0] + v[0], acc[1] + v[1], acc[2] + v[2]]);
    let n = tris.len() as f32 * 3.0;
    let local_c = [c[0] / n, c[1] / n, c[2] / n];
    let target = world(&head.matrix, local_c);
    let rays = burst(&mut scene, target, 28);
    assert!(rays.len() >= 20, "the burst flew");
    // The premise: at least one actual flight ray crosses the head's own cels — the game's
    // contact (Collide_Intersect vs obj+0xD8 in the obj+0x90 frame) — so the game would register it.
    {
        let sim = scene.sim.as_ref().unwrap();
        let now = sim.drones.searchlights.parts.iter().find(|p| p.kind == 1).unwrap().clone();
        let hits: Vec<_> = rays.iter().filter(|(a, b)| tri_hit_local(&now.matrix, &tris, *a, *b)).collect();
        assert!(!hits.is_empty(), "at least one burst ray crosses the head's cels (the registered shot)");
    }
    let sim = scene.sim.as_ref().unwrap();
    let after = sim.drones.searchlights.parts.iter().find(|p| p.kind == 1).unwrap();
    assert_eq!(after.state, 2, "the lamp-housing shot breaks the light (Searchlight_Update's walk, damage > 0)");
    assert!(!after.drawn || after.light.is_none(), "the parts hide and the spot's light is deleted");
}

#[test]
fn a_mount_only_shot_never_breaks_the_light() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load_and_light(&iso);
    let (level, m) = level_geom(&iso);
    let (head, mount, mount_idx) = {
        let sim = scene.sim.as_ref().unwrap();
        let head = sim.drones.searchlights.parts.iter()
            .filter(|p| p.kind == 1 && p.links[0].is_some() && p.state == 0)
            .min_by(|a, b| yaw_off(a.yaw, a.base_yaw).total_cmp(&yaw_off(b.yaw, b.base_yaw)))
            .cloned().expect("a sweeping head with a mount");
        let mi = head.links[0].unwrap();
        (head, sim.drones.searchlights.parts[mi].clone(), mi)
    };
    let head_tris = part_tris(&level, m, head.placement);
    let mount_tris = part_tris(&level, m, mount.placement);
    // The shot plane: BELOW the head's cels in WORLD y (the disc's own data — no head triangle can
    // be touched there) but across the mount's pole (asserted against the mount's parsed cels).
    let head_bottom = head_tris.iter().flat_map(|t| t.iter())
        .map(|v| world(&head.matrix, *v)[1]).fold(f32::INFINITY, f32::min);
    let mount_top = mount_tris.iter().flat_map(|t| t.iter())
        .map(|v| world(&mount.matrix, *v)[1]).fold(f32::NEG_INFINITY, f32::max);
    assert!(mount_top > head_bottom + 0.05, "the meshes overlap in world y — pick another pair");
    let y = head_bottom - 0.12;
    // A mount TRIANGLE centroid at that height (a vertex aim is edge-degenerate in the tri pass).
    let mut pole = None;
    for t in &mount_tris {
        let ws: Vec<V3> = t.iter().map(|v| world(&mount.matrix, *v)).collect();
        if ws.iter().all(|w| (w[1] - y).abs() < 0.08) {
            pole = Some([(ws[0][0] + ws[1][0] + ws[2][0]) / 3.0, y, (ws[0][2] + ws[1][2] + ws[2][2]) / 3.0]);
        }
    }
    let target = pole.expect("a mount cel triangle at the shot height");
    // The burst rides +-0.04 in y (two jitter rows) — still below the head's cels' bbox bottom.
    assert!(y + 0.05 < head_bottom, "the burst's spread stays below the head's cels");
    assert!(mount_top > y + 0.02, "the mount has cels at the shot height");
    let rays = burst(&mut scene, target, 15);
    {
        let sim = scene.sim.as_ref().unwrap();
        let mnt = &sim.drones.searchlights.parts[mount_idx];
        let now = sim.drones.searchlights.parts.iter().find(|p| p.kind == 1).unwrap().clone();
        let mount_hits = rays.iter().filter(|(a, b)| tri_hit_local(&mnt.matrix, &mount_tris, *a, *b)).count();
        let head_hits = rays.iter().filter(|(a, b)| tri_hit_local(&now.matrix, &head_tris, *a, *b)).count();
        if mount_hits == 0 { eprintln!("note: no burst ray reached the mount's cels (the muzzle anim offset); the no-break assertion still stands"); }
        assert_eq!(head_hits, 0, "no burst ray can touch a head triangle (below the head's cels)");
    }
    let sim = scene.sim.as_ref().unwrap();
    let after = sim.drones.searchlights.parts.iter().find(|p| p.kind == 1).unwrap();
    assert_eq!(after.state, 0, "a mount-only shot never breaks the light: only the head runs the \
        child-list walk (kind-1 gate 0x1cb900:88-90; class 0x22's handler is 0 [L])");
}

#[test]
fn a_head_box_corner_shot_that_misses_the_cels_does_not_break() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load_and_light(&iso);
    let (level, m) = level_geom(&iso);
    let (head, head_idx) = {
        let sim = scene.sim.as_ref().unwrap();
        let (i, h) = sim.drones.searchlights.parts.iter().enumerate()
            .filter(|(_, p)| p.kind == 1 && p.links[0].is_some() && p.state == 0)
            .min_by(|a, b| yaw_off(a.1.yaw, a.1.base_yaw).total_cmp(&yaw_off(b.1.yaw, b.1.base_yaw)))
            .map(|(i, p)| (i, p.clone())).expect("a sweeping head with a mount");
        (h, i)
    };
    let placement = &level.maps[m].placements[head.placement];
    let obj = object_of(&level, m, placement);
    let tris = part_tris(&level, m, head.placement);
    let pm = object_matrix(placement.rotation, placement.translation);
    let (bmin, bmax) = old_box_of(obj.min, obj.max, &pm);
    // A box-face point OFF the game's volume: the largest clearance from the head's cels among the
    // gfx box's face samples (the mesh is roundish; the box's corners are empty).
    let mut best: Option<(V3, f32)> = None;
    for i in 0..7 { for j in 0..7 {
        for &(x, y, z) in &[
            (bmin[0] + 0.02, bmin[1] + (bmax[1] - bmin[1]) * (j as f32) / 6.0, bmin[2] + 0.02),
            (bmax[0] - 0.02, bmin[1] + (bmax[1] - bmin[1]) * (j as f32) / 6.0, bmax[2] - 0.02),
            (bmin[0] + (bmax[0] - bmin[0]) * (i as f32) / 6.0, bmax[1] - 0.02, bmin[2] + 0.02),
            (bmin[0] + (bmax[0] - bmin[0]) * (i as f32) / 6.0, bmin[1] + 0.02, bmax[2] - 0.02),
        ] {
            // The box's local (object) coords of that world point, through the placement matrix.
            let d = [x - pm[3][0], y - pm[3][1], z - pm[3][2]];
            let lp = [0, 1, 2].map(|k| d[0] * pm[k][0] + d[1] * pm[k][1] + d[2] * pm[k][2]);
            let clear = tris.iter().flat_map(|t| t.iter())
                .map(|v| ((lp[0] - v[0]).powi(2) + (lp[1] - v[1]).powi(2) + (lp[2] - v[2]).powi(2)).sqrt())
                .fold(f32::INFINITY, f32::min);
            if clear > 0.05 && best.map_or(true, |(_, c)| clear > c) { best = Some(([x, y, z], clear)); }
        }
    } }
    let (target, clear) = best.expect("a gfx-box corner point clear of the head's cels exists");
    assert!(clear > 0.05, "the sister's premise: the sample sits off the game's volume");
    // The burst's rays: the OLD stand-in (the segment in the part's current frame vs the placement-
    // frame box coords) catches at least one of them; NONE of them touches the head's cels.
    let rays = burst(&mut scene, target, 15);
    {
        let sim = scene.sim.as_ref().unwrap();
        let now = sim.drones.searchlights.parts[head_idx].clone();
        let head_hits = rays.iter().filter(|(a, b)| tri_hit_local(&now.matrix, &tris, *a, *b)).count();
        assert_eq!(head_hits, 0, "no burst ray touches the head's cels (the game's volume is the cels)");
        let old_catch = rays.iter().any(|(a, b)| {
            let (lf, lt) = (to_local(&now.matrix, *a), to_local(&now.matrix, *b));
            old_seg_box(lf, lt, bmin, bmax)
        });
        if !old_catch { eprintln!("note: the old stand-in's frame mismatch missed this ray too; the no-break assertion still stands"); }
    }
    let sim = scene.sim.as_ref().unwrap();
    let after = sim.drones.searchlights.parts[head_idx].clone();
    assert_eq!(after.state, 0, "a ray that misses every head cel never breaks the light — the old \
        gfx-box stand-in caught this shot (fail-first shown)");
}
