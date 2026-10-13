//! fix-safe08 probe: which triangle blocks the take LOS to the shelf pickups. Opt-in via NF_ISO.

use nf_formats::iso9660::IsoImage;
use nf_formats::level::Level;
use nf_formats::placement::transform;
use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn read_archive(iso: &str, archive: &str) -> Vec<u8> {
    let img = IsoImage::open(iso).expect("open iso");
    let (entries, bin) = nf_formats::disc::files_bin_entries(&img).expect("FILES.BIN");
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(archive)).expect("archive");
    img.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).expect("read")
}

fn seg_tri(from: [f32; 3], to: [f32; 3], t: [[f32; 3]; 3]) -> Option<f32> {
    fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] { [a[0] - b[0], a[1] - b[1], a[2] - b[2]] }
    fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] { [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]] }
    fn dot(a: [f32; 3], b: [f32; 3]) -> f32 { a[0] * b[0] + a[1] * b[1] + a[2] * b[2] }
    let d = sub(to, from);
    let e1 = sub(t[1], t[0]);
    let e2 = sub(t[2], t[0]);
    let p = cross(d, e2);
    let det = dot(e1, p);
    if det.abs() < 1e-12 { return None }
    let inv = 1.0 / det;
    let s = sub(from, t[0]);
    let u = dot(s, p) * inv;
    if !(0.0..=1.0).contains(&u) { return None }
    let q = cross(s, e1);
    let v = dot(d, q) * inv;
    if v < 0.0 || u + v > 1.0 { return None }
    let tt = dot(e2, q) * inv;
    (0.0..=1.0).contains(&tt).then_some(tt)
}

#[test]
fn probe_take_los_blockers() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let data = read_archive(&iso, "07000008.bin");
    let level = Level::parse(&data).expect("parse");
    let (insts, _) = level.instances(true);

    // The exact rays the take test runs (Bond's sphere centre → the shelf pickups), from the failed
    // test log: bond pos (161.87, 43.39, -116.54) → centre ≈ pos + up 1.0? Use the sim to be exact.
    let mut scene = nf_view::load(&iso, "07000008.bin", false).expect("load");
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.bond.pos = [161.87, 43.39, -116.54];
        sim.bond.yaw = 1.583;
        sim.bond.pitch = -0.495;
        for _ in 0..10 { sim.step_bond(Input::default(), false, false, false); }
        // Open the safe the fast way (the chain's channels), then re-pose Bond at the user's spot.
        sim.drones.set_channel(134);
        sim.drones.set_channel(135);
        for _ in 0..520 { sim.step_bond(Input::default(), false, false, false); }
        eprintln!("ch136 {} door t {:?}", sim.drones.channels.on(136),
            sim.drones.sss.list.iter().find(|s| s.channel == 136).map(|s| (s.t, [s.matrix[3][0], s.matrix[3][1], s.matrix[3][2]])));
    }
    let sim = scene.sim.as_ref().unwrap();
    // Step Bond into the doorway (the natural take spot after the door glides open).
    {
        let sim = scene.sim.as_mut().unwrap();
        for spot in [[161.87, 43.39, -116.54], [162.05, 43.2, -116.6], [162.25, 43.1, -116.65]] {
            sim.bond.pos = spot;
            sim.bond.yaw = 1.583;
            sim.bond.pitch = -0.3;
            for _ in 0..10 { sim.step_bond(Input::default(), false, false, false); }
            let body = sim.bond.sphere_centre();
            eprintln!("spot {spot:?} settled {:?} body {body:?}", sim.bond.pos);
            for (k, p) in sim.drones.pickups.iter().enumerate() {
                if p.gone { continue }
                let c = p.centre();
                let d = ((c[0] - sim.bond.pos[0]).powi(2) + (c[1] - sim.bond.pos[1]).powi(2) + (c[2] - sim.bond.pos[2]).powi(2)).sqrt();
                if d > 2.0 { continue }
                let los = sim.world.segment(body, c);
                eprintln!("  near pickup {k} kind {} weapon {:#x} dist {d:.2} los {los:?}", p.kind, p.weapon);
            }
        }
    }
    {
        let sim = scene.sim.as_mut().unwrap();
        // Test B: the door SS's open pose may push Bond back; drop its collision and re-take.
        for s in sim.drones.sss.list.iter_mut() { if s.channel == 136 { s.hash = 0; } }
        sim.bond.pos = [162.15, 43.2, -116.6];
        for _ in 0..30 { sim.step_bond(Input::default(), false, false, false); }
        eprintln!("[no-door] bond settled {:?} taken {:?} ch0x4a {}", sim.bond.pos, sim.drones.taken, sim.drones.channels.on(0x4a));
        let body3 = sim.bond.sphere_centre();
        for (k, p) in sim.drones.pickups.iter().enumerate() {
            if p.gone { continue }
            let c = p.centre();
            let d = ((c[0] - sim.bond.pos[0]).powi(2) + (c[1] - sim.bond.pos[1]).powi(2) + (c[2] - sim.bond.pos[2]).powi(2)).sqrt();
            if d > 2.0 { continue }
            eprintln!("[no-door] pickup {k} weapon {:#x} dist {d:.2} los {:?}", p.weapon, sim.world.segment_actor(body3, c));
        }
        for _ in 0..40 { sim.step_bond(Input::default(), false, false, false); }
        eprintln!("[no-door] after take ticks: taken {:?} ch0x4a {}", sim.drones.taken, sim.drones.channels.on(0x4a));
        for _ in 0..60 { sim.step_bond(Input::default(), false, false, false); }
        eprintln!("[take-at-spot] taken {:?} ch0x4a {}", sim.drones.taken, sim.drones.channels.on(0x4a));
        let live: Vec<(usize, u16, [f32; 3], f32)> = sim.drones.pickups.iter().enumerate()
            .filter(|(_, p)| !p.gone)
            .map(|(k, p)| (k, p.kind, p.centre(), ((p.centre()[0] - sim.bond.pos[0]).powi(2) + (p.centre()[1] - sim.bond.pos[1]).powi(2) + (p.centre()[2] - sim.bond.pos[2]).powi(2)).sqrt()))
            .filter(|(_, _, _, d)| *d < 2.0).collect();
        eprintln!("[take-at-spot] live near: {live:?}");
        let body2 = sim.bond.sphere_centre();
        for (k, _, c, _) in &live {
            eprintln!("[take-at-spot] pickup {k} los-actor {:?}", sim.world.segment_actor(body2, *c));
        }
        // Hunt the dynamic set: every door's glist shape at its matrix, every entity hash.
        for (k, x) in sim.drones.doors.list.iter().enumerate() {
            if x.glist == 0 { continue }
            let Some(tris) = sim.world.shape(x.glist) else { continue };
            let mut best = f32::INFINITY;
            for t in tris {
                let t3 = t.map(|p| transform(&x.matrix, p));
                if let Some(tt) = seg_tri(body2, [162.72339f32, 43.52936, -116.22817], t3) { best = best.min(tt); }
            }
            let trans = [x.matrix[3][0], x.matrix[3][1], x.matrix[3][2]];
            if best.is_finite() || (trans[0] - 162.0).abs() < 12.0 {
                eprintln!("[door {k}] glist {:#x} trans {trans:?} hit {best:.3}", x.glist);
            }
        }
        for (oi, o) in sim.drones.scripts.objects.list.iter().enumerate() {
            let Some(nf_game::nis::Obj::Entity { hash, matrix, kind, src, .. }) = o else { continue };
            if let Some(tris) = sim.world.shape(*hash) {
                let mut best = f32::INFINITY;
                for t in tris {
                    let t3 = t.map(|p| transform(matrix, p));
                    if let Some(tt) = seg_tri(body2, [162.72339f32, 43.52936, -116.22817], t3) { best = best.min(tt); }
                }
                if best.is_finite() {
                    let nm = level.by_hash.get(hash).map(|(m, ob)| level.maps[*m].objects[*ob].name.trim().to_string()).unwrap_or_else(|| "?".into());
                    eprintln!("[ent {oi}] hash {hash:08x} \"{nm}\" kind {kind} src {src:?} hit {best:.3} trans {:?}", [matrix[3][0], matrix[3][1], matrix[3][2]]);
                }
            }
        }
    }
    let sim = scene.sim.as_ref().unwrap();
    let body = sim.bond.sphere_centre();
    eprintln!("bond body {body:?}");
    // Cage + every instance tri against both rays (the segment_actor blocker hunt).
    for (name, tgt) in [("briefcase", [162.72339f32, 43.52936, -116.22817]), ("rocket", [162.58832, 42.75039, -116.77789])] {
        for (i, inst) in insts.iter().enumerate() {
            let obj = &level.maps[inst.map].objects[inst.object];
            let nm = obj.name.trim();
            if !nm.contains("BlockingPoly") && !nm.contains("Room_Outside") && nm != "Object_Safe" && nm != "Object_SafeDoor" && nm != "Object_chest" && nm != "Object_bed" { continue }
            let Some(c) = &obj.collision else { continue };
            let mut best = f32::INFINITY;
            for (t, mat) in c.triangles_with_material() {
                let t = t.map(|p| transform(&inst.matrix, p));
                if let Some(tt) = seg_tri(body, tgt, t) { if tt < best { best = tt; } }
            }
            if best.is_finite() { eprintln!("  [cage-hunt {name}] {nm} t {best:.3} objflags {:#x}", obj.flags); }
        }
    }
    let targets: Vec<([f32; 3], &str)> = vec![
        ([162.72339, 43.52936, -116.22817], "briefcase"),
        ([162.58832, 42.75039, -116.77789], "rocket"),
    ];

    // Static instances:
    for (tgt, name) in &targets {
        eprintln!("--- ray to {name} {tgt:?} ---");
        for (i, inst) in insts.iter().enumerate() {
            let obj = &level.maps[inst.map].objects[inst.object];
            let Some(c) = &obj.collision else { continue };
            if obj.flags & 0x1000 != 0 { continue } // the actor path skips the cage
            let mut best = f32::INFINITY;
            let mut best_mat: Option<u8> = None;
            for (t, mat) in c.triangles_with_material() {
                let t = t.map(|p| transform(&inst.matrix, p));
                if let Some(tt) = seg_tri(body, *tgt, t) { if tt < best { best = tt; best_mat = Some(mat); } }
            }
            if best.is_finite() {
                eprintln!("  [s{i}] \"{}\" t {best:.3} mat {best_mat:?} objflags {:#x}", obj.name.trim(), obj.flags);
            }
        }
        // Dynamic (the safe door SS): its posed tris via the sim's world — the port's segment already
        // told us it hits; here identify which dynamic part by re-pushing the SS matrices.
        for s in &sim.drones.sss.list {
            let Some(t) = sim.world.shape(s.hash) else { continue };
            let mut best = f32::INFINITY;
            for tri in t {
                let t3 = tri.map(|p| transform(&s.matrix, p));
                if let Some(tt) = seg_tri(body, *tgt, t3) { best = best.min(tt); }
            }
            if best.is_finite() {
                eprintln!("  [ss placement {}] hash {:08x} t {best:.3} matrix trans {:?}", s.placement, s.hash,
                    [s.matrix[3][0], s.matrix[3][1], s.matrix[3][2]]);
            }
        }
    }
}
