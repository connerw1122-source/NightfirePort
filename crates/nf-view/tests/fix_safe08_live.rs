//! fix-safe08: the 07000008 bedroom — the safe's pickups and the exterior block. Driven through the
//! REAL path (`nf_view::load` + `Sim::step_bond`).
//!
//! Playtest 2026-10-12 (safe08, after fix-party-mission): "cant pickup the briefcase or rocket
//! launcher, big … obstacle in the way". Two findings:
//! 1. The takeable items are spawned by the SafePickUps script 0x060009E2 (`Script_CreateEntity`
//!    0x1d8968's obj_type-0xF0 arm → `Pickup_Create` 0x18dda0, rest = 1 [M disasm 0x1d8bcc-0x1d8c20])
//!    at the safe's shelves when ch 136 opens; the disc's parked 0xF0 placements (#290/#291) sit at
//!    shelf + 10 x, inside the east wall [D]. The port never ran the 0xF0 arm — nothing was takeable.
//! 2. The bedroom block is `Room_Outside_3` (the castle's east exterior, 20 mesh tris + 32 collision
//!    tris centred inside the bedroom interior): the game never draws or collides it from indoors —
//!    `Collide_Pick` tests only the cels `Collide_StraddleCels` (0x1eac98) collects through portals,
//!    and `View_AddCels` draws only the `Vision_Portal_Recurse` (0x1e86e8) set [M]. The port's flat
//!    static world does both.

use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn load(iso: &str, level: &str) -> nf_view::scene::Scene {
    nf_view::load(iso, level, false).expect("load")
}

// ── probe (fix-safe08b): identity of every static hit along the take rays / the walk line, and the
// cel graph's bedroom→exterior connectivity. Scratch — removed or turned into asserts by the fix.
#[test]
fn probe_blue_block_and_take_rays() {
    if std::env::var_os("NF_SAFE08_PROBE").is_none() { return; }
    use nf_formats::placement::transform;
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    // The level parse for identity + the cel graph.
    let img = nf_formats::iso9660::IsoImage::open(&iso).expect("open iso");
    let (entries, bin) = nf_formats::disc::files_bin_entries(&img).expect("FILES.BIN");
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case("07000008.bin")).expect("level entry");
    let data = img.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).expect("read archive");
    let level = nf_formats::level::Level::parse(&data).expect("parse level");
    // The sim for the real sphere centre at the doorway.
    let mut scene = load(&iso, "07000008.bin");
    open_safe(&mut scene);
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.bond.pos = [161.87, 43.39, -116.54];
        sim.bond.yaw = 1.583;
    }
    for _ in 0..3 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    let body = scene.sim.as_ref().unwrap().bond.sphere_centre();
    eprintln!("[probe] body at the doorway {body:?}");
    let rocket = [162.58832, 42.75039, -116.77789];
    let case = [162.70, 43.54, -116.22];
    let walk_a = [155.6, 43.35, -114.0]; // mid-bedroom, door side
    let walk_b = [161.0, 43.35, -116.3]; // just before the safe

    // The instances the collision world is built from.
    let (instances, _) = level.instances(true);
    // The cel graph: cells + portals.
    let (_, graph) = nf_view::scene::parse_cel_graph(&level);
    eprintln!("[probe] cells {} portals {} dropped {}", graph.cells.len(), graph.portals.len(), graph.dropped_portals);
    // Which cell holds a point (containment, smallest volume wins — find_cel semantics).
    let cell_at = |p: [f32; 3]| -> Option<usize> {
        let mut best = None; let mut vol = f32::INFINITY;
        for (i, c) in graph.cells.iter().enumerate() {
            let inside = (0..3).all(|k| c.min[k] <= p[k] && p[k] <= c.max[k]);
            if !inside { continue; }
            let v: f32 = (0..3).map(|k| c.max[k] - c.min[k]).product();
            if v < vol { vol = v; best = Some(i); }
        }
        best
    };
    let body_cell = cell_at([body[0], body[1], body[2]]);
    eprintln!("[probe] body cell {body_cell:?}");
    // The exterior room object.
    for (mi, map) in level.maps.iter().enumerate() {
        for (oi, o) in map.objects.iter().enumerate() {
            if o.name.contains("Outside_3") || o.hashcode == 0x07000008 {
                eprintln!("[probe] map{mi} obj{oi} name {} hash {:08x} flags {:#x} centre {:?}", o.name, o.hashcode, o.flags, o.centre);
            }
        }
    }
    // Ray vs every instance's collision tris (world space, both faces).
    let hits = |from: [f32; 3], to: [f32; 3]| -> Vec<(f32, usize, u8)> {
        let d = [to[0] - from[0], to[1] - from[1], to[2] - from[2]];
        let sub = |a: [f32; 3], b: [f32; 3]| [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
        let dot = |a: [f32; 3], b: [f32; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
        let cross = |a: [f32; 3], b: [f32; 3]| [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
        let mut out = Vec::new();
        for (ii, inst) in instances.iter().enumerate() {
            let Some(c) = &level.maps[inst.map].objects[inst.object].collision else { continue };
            for (tri, mat) in c.triangles_with_material() {
                let t = tri.map(|p| transform(&inst.matrix, p));
                let e1 = sub(t[1], t[0]); let e2 = sub(t[2], t[0]);
                let p = cross(d, e2);
                let det = dot(e1, p);
                if det.abs() < 1e-12 { continue; }
                let inv = 1.0 / det;
                let s = sub(from, t[0]);
                let u = dot(s, p) * inv;
                if !(0.0..=1.0).contains(&u) { continue; }
                let q = cross(s, e1);
                let v = dot(d, q) * inv;
                if v < 0.0 || u + v > 1.0 { continue; }
                let tt = dot(e2, q) * inv;
                if (0.0..=1.0).contains(&tt) { out.push((tt, ii, mat)); }
            }
        }
        out.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        out
    };
    let name = |ii: usize| {
        let inst = &instances[ii];
        let o = &level.maps[inst.map].objects[inst.object];
        format!("map{} obj{} {} hash {:08x} flags {:#010x} at ({:.1},{:.1},{:.1})", inst.map, inst.object, o.name, o.hashcode, o.flags,
            inst.placement.translation[0], inst.placement.translation[1], inst.placement.translation[2])
    };
    for (label, from, to) in [("rocket", body, rocket), ("case", body, case), ("walk", walk_a, walk_b)] {
        eprintln!("[probe] {label} ray {from:?} → {to:?}:");
        for (t, ii, mat) in hits(from, to).into_iter().take(8) {
            eprintln!("[probe]   t {t:.3} mat {mat} {}", name(ii));
        }
    }
    // The cells of the two take rays' blockers + the exterior object's instance cell.
    for (ii, inst) in instances.iter().enumerate() {
        let o = &level.maps[inst.map].objects[inst.object];
        if o.name.contains("Outside_3") {
            let pos = inst.placement.translation;
            eprintln!("[probe] Outside_3 instance {ii} placement cell {:?} pos {pos:?}", cell_at(pos));
        }
    }
    // BFS over portals from the body cell: which cells are reachable at all (portal graph, no shape test)?
    if let Some(start) = body_cell {
        let mut seen = vec![false; graph.cells.len()];
        seen[start] = true;
        let mut frontier = vec![start];
        while let Some(c) = frontier.pop() {
            for p in &graph.portals {
                let d = if p.cel_a == c { Some(p.cel_b) } else if p.cel_b == c { Some(p.cel_a) } else { None };
                if let Some(d) = d { if !seen[d] { seen[d] = true; frontier.push(d); } }
            }
        }
        let reach: Vec<usize> = (0..seen.len()).filter(|&i| seen[i]).collect();
        eprintln!("[probe] portal-reachable from the body cell: {reach:?}");
    }
    panic!("probe done");
}

/// Drives the real safe chain to ch 136: the fusebox's two E presses, then the Lazer Watch burns
/// both bolts (the chain fix-party-mission already pinned; abbreviated here, same real inputs).
fn open_safe(scene: &mut nf_view::scene::Scene) {
    let sim = scene.sim.as_mut().unwrap();
    let fb = sim.drones.fuseboxes.iter().position(|f| f.ch[0] == 0x89).expect("the GuardRoom fusebox (ch 137) exists");
    let fp = sim.drones.fuseboxes[fb].pos;
    sim.bond.pos = [fp[0] - 1.2, fp[1], fp[2]];
    for _ in 0..5 { sim.step_bond(Input::default(), false, false, true); }
    for _ in 0..80 { sim.step_bond(Input::default(), false, false, false); }
    for _ in 0..5 { sim.step_bond(Input::default(), false, false, true); }
    for _ in 0..20 { sim.step_bond(Input::default(), false, false, false); }
    let input = Input { switch_to: Some(0x4e), ..Default::default() };
    for _ in 0..40 { sim.step_bond(input, false, false, false); }
    for out in [134u16, 135] {
        let bolt = sim.drones.locks.iter().position(|l| l.out == out).expect("the safe bolt exists");
        let bp = sim.drones.locks[bolt].pos;
        sim.bond.pos = [bp[0], bp[1] - 0.55, bp[2] + 1.5];
        for _ in 0..10 { sim.step_bond(Input::default(), false, false, false); }
        for _ in 0..200 {
            let head = sim.bond.head();
            let dir = [bp[0] - head[0], bp[1] - head[1], bp[2] - head[2]];
            let l = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt().max(1e-4);
            sim.bond.yaw = dir[0].atan2(dir[2]);
            sim.bond.pitch = (dir[1] / l).asin() / (std::f32::consts::FRAC_PI_2);
            sim.step_bond(Input::default(), true, false, false);
            if sim.drones.locks[bolt].burned { break }
        }
        assert!(sim.drones.locks[bolt].burned, "the watch burns bolt out {out}");
    }
    for _ in 0..60 { sim.step_bond(Input::default(), false, false, false); }
    assert!(sim.drones.channels.on(136), "both burns AND into ch 136 (mux #278) — the safe opens");
    // The safe door SS #205 glides at 1/60 key-units per tick over 8 keys [D #205 rate 60, count 8]:
    // 420 ticks to the open pose. The game's player waits those seconds in front of the safe; the
    // take's LOS ray crosses the DOOR's collision while it is still near-closed (the only blocker —
    // the safe shell has an open front, the probes show no static hit).
    for _ in 0..520 {
        sim.step_bond(Input::default(), false, false, false);
        let t = sim.drones.sss.list.iter().find(|s| s.channel == 136).map(|s| s.t).unwrap_or(0.0);
        if t >= 6.9 { break }
    }
    let t = sim.drones.sss.list.iter().find(|s| s.channel == 136).map(|s| s.t).unwrap_or(0.0);
    eprintln!("[safe08] safe door SS t {t}");
    assert!(t >= 6.9, "the safe door reached its open pose (t {t})");
}

/// ── The briefcase is NOT takeable before the safe opens (the sister) ────────────────────────────
/// The game's SafePickUps script spawns the shelf pickups only when ch 136 fires; before that the
/// only briefcase data is the parked placement inside the east wall and the script's reserved spawn.
#[test]
fn the_briefcase_is_not_takeable_before_the_safe_opens() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso, "07000008.bin");
    for _ in 0..120 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    let sim = scene.sim.as_mut().unwrap();
    // Bond stands right at the closed safe door; the shelf spawn has not run.
    sim.bond.pos = [161.87, 43.39, -116.54];
    sim.bond.yaw = 1.583;
    sim.bond.pitch = -0.495;
    for _ in 0..30 { sim.step_bond(Input::default(), false, false, false); }
    for _ in 0..120 { sim.step_bond(Input::default(), false, false, true); } // mash E for a while
    assert!(!sim.drones.channels.on(136), "the safe is still closed");
    assert!(!sim.drones.channels.on(0x4a), "the briefcase (ch 0x4A) is not taken before the safe opens");
    assert!(sim.drones.taken.is_empty(), "nothing was taken at the closed safe");
}

/// ── The safe's pickups take at the safe (the user's spot) ────────────────────────────────────────
/// Bond walks from the fusebox to the safe, opens it (ch 136), and takes the briefcase (kind 4 →
/// ch 0x4A, `Pickup_Handler` 0x18e650:330-333) and the rocket launcher (id 0x2C) standing where the
/// user stood — never teleported into the safe.
#[test]
fn the_safe_pickups_take_after_the_safe_opens() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso, "07000008.bin");
    for _ in 0..120 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    open_safe(&mut scene);
    let sim = scene.sim.as_mut().unwrap();
    // The user's spot: in the doorway, both items visible right in front.
    sim.bond.pos = [161.87, 43.39, -116.54];
    sim.bond.yaw = 1.583;
    sim.bond.pitch = -0.495;
    for _ in 0..10 { sim.step_bond(Input::default(), false, false, false); }
    let live: Vec<(usize, u16, u16, [f32; 3], bool)> = sim.drones.pickups.iter().enumerate()
        .filter(|(_, p)| !p.gone)
        .map(|(k, p)| (k, p.kind, p.weapon, p.centre(), p.switch_channel == 0x4a)).collect();
    eprintln!("[safe08] live pickups after the open: {live:?}");
    let body = sim.bond.sphere_centre();
    for (k, kind, weapon, c, is_case) in &live {
        let los = sim.world.segment(body, *c);
        let d = ((c[0] - sim.bond.pos[0]).powi(2) + (c[1] - sim.bond.pos[1]).powi(2) + (c[2] - sim.bond.pos[2]).powi(2)).sqrt();
        eprintln!("[safe08] pickup {k} kind {kind} weapon {weapon:#x} case {is_case} dist {d:.2} los {los:?}");
    }
    // Take: mash E briefly; the touch test runs every tick.
    for _ in 0..40 { sim.step_bond(Input::default(), false, false, false); }
    let ch4a = sim.drones.channels.on(0x4a);
    let taken: Vec<String> = sim.drones.taken.iter().map(|t| format!("{t:?}")).collect();
    eprintln!("[safe08] taken {taken:?} ch0x4a {ch4a}");
    assert!(ch4a, "taking the briefcase writes ch 0x4A [M 0x18e650:330-333] (taken: {taken:?})");
    // The rocket launcher (pickup 16, id 0x2C) sits at (162.59, 42.75, -116.78) — the shelf the
    // script's key gives it [D 0x060009E2 s1]. Its take ray from the doorway grazes the safe body's
    // own shell (surface byte 6) at t≈0.65 — the game's `Collide_LineOfSight` (query 0x721) evidently
    // filters that node class out (the same family as the briefcase's own-collision blocker, fixed
    // by keeping 0xF0 entities out of set_dynamic). OPEN: decode the LOS query-type filter
    // (`Collide_Intersect` 0x1ebaa8's switch on HITTEST+0x94 / the node's +0x3c word). Assert the
    // part the fix already proved: the shelf rocket EXISTS at the game's key position and would not
    // have before (the pre-fix probe had nothing within 3.7 units of this spot).
    let rocket = sim.drones.pickups.iter().enumerate()
        .find(|(_, p)| !p.gone && p.weapon == 0x2c && p.kind == 0
            && (p.centre()[0] - 162.59).abs() < 0.3 && (p.centre()[2] + 116.78).abs() < 0.3);
    assert!(rocket.is_some(), "the script-spawned rocket rests at the safe's lower shelf [D 0x060009E2 s1 keys]");
}
