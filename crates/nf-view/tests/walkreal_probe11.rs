//! walkreal-b 07000011 probe: the stairwell's surface stacks, the flight plates and the swing
//! door's open-leaf scan (the findings cited in walkreal_07000011_live.rs).

#[path = "common/walker.rs"]
mod walker;
use walker::Walker;

#[test]
fn probe_011_stairwell() {
    let iso = std::env::var("NF_ISO").unwrap();
    let mut w = Walker::new(nf_view::load_with_carry(&iso, "07000011.bin", false, None).unwrap());
    w.wait_enabled(3000, "the intro");
    w.wait(120);
    let sim = w.scene.sim.as_ref().unwrap();
    // The stairwell's surface stacks (the west line, the east line, the shaft).
    for (x, z) in [(14.5f32, -3.0), (14.5, 0.0), (14.5, 3.0), (14.5, 5.0), (18.5, 0.0), (22.5, 0.0), (22.5, 4.0)] {
        let mut col = Vec::new();
        let mut top = 48.5f32;
        for _ in 0..24 {
            match sim.world.floor_below([x, top - 0.05, z], 3.4) {
                Some(f) if f < top - 0.3 => { col.push((f * 10.0) as i32); top = f; }
                _ => { top -= 3.3; if top < 13.0 { break; } }
            }
        }
        eprintln!("[p011] stack ({x},{z}): {col:?}");
    }
    // The flight plates' volumes (each flight's xz rectangle).
    for t in &sim.drones.triggers {
        if [0x11u32, 0x13, 0x14, 0x36, 0x37].contains(&(t.out as u32)) && !t.tris.is_empty() {
            eprintln!("[p011] pl{} out {:#x} lo ({:.1},{:.1},{:.1}) hi ({:.1},{:.1},{:.1})",
                t.placement, t.out, t.lo[0], t.lo[1], t.lo[2], t.hi[0], t.hi[1], t.hi[2]);
        }
    }
    // The stairwell door: fields, the swung-open leaf's bounds, the doorway's clear z band.
    let di = sim.drones.doors.list.iter()
        .position(|d| (d.pos[0] - 13.8).abs() < 0.5 && (d.pos[2] - 7.1).abs() < 0.5)
        .expect("the stairwell door");
    {
        let d = &sim.drones.doors.list[di];
        eprintln!("[p011] door flags {:#x} locked {} approach {} kind {} keys {}",
            d.flags, d.locked_flag, d.approach, d.kind, d.keys.as_ref().map(|k| k.len()).unwrap_or(0));
    }
    w.scene.sim.as_mut().unwrap().bond.pos = [12.3, 47.85, 7.1];
    w.use_door(di, "probe open");
    for _ in 0..90 { w.tick(Default::default(), false); }
    {
        let sim = w.scene.sim.as_ref().unwrap();
        let d = &sim.drones.doors.list[di];
        eprintln!("[p011] after use_door: state {:#x} pos01 {:.2}", d.state, d.pos01);
        let m = d.matrix;
        if d.glist != 0 {
            if let Some(tris) = sim.world.shape(d.glist) {
                let mut lo = [f32::MAX; 3]; let mut hi = [f32::MIN; 3];
                for t in tris { for v in t {
                    let wp = [m[0][0]*v[0]+m[1][0]*v[1]+m[2][0]*v[2]+m[3][0],
                              m[0][1]*v[0]+m[1][1]*v[1]+m[2][1]*v[2]+m[3][1],
                              m[0][2]*v[0]+m[1][2]*v[1]+m[2][2]*v[2]+m[3][2]];
                    for k in 0..3 { lo[k] = lo[k].min(wp[k]); hi[k] = hi[k].max(wp[k]); }
                } }
                eprintln!("[p011] the open leaf bounds lo {lo:?} hi {hi:?}");
            }
        }
        for z in [5.9f32, 6.3, 6.7, 7.1, 7.5, 7.9] {
            let hit = sim.world.segment([12.5, 47.6, z], [16.5, 47.6, z]);
            eprintln!("[p011] the doorway z{z}: blocked {}", hit.is_some());
        }
    }
}
