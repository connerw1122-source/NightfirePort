//! probe_eyecel: the trench tie-break data (fix-portal-eyecel).
//! What do the candidates 14/15 actually hit through the scoped queries, and where does the
//! floor under the eye live in the port's scope buckets?

use nf_view::scene;

fn main() {
    let Some(iso) = std::env::var("NF_ISO").ok() else { return };
    let sc = nf_view::load(&iso, "07000005.bin", false).expect("load 05");
    let g = &sc.cel_graph;
    let world = sc.sim.as_ref().map(|s| &s.world).expect("sim world");
    let eye = [16.67f32, 30.04 + 0.7, 58.60];

    // The candidates (room cels holding the eye).
    let cands: Vec<usize> = g.cells.iter().enumerate()
        .filter(|(_, c)| c.flags & 0x40000 != 0 && (0..3).all(|k| c.min[k] <= eye[k] && eye[k] <= c.max[k]))
        .map(|(i, _)| i).collect();
    println!("[cands] {cands:?}");

    // The port's tie-break per candidate: up ray then down ray through the scoped query.
    for &i in &cands {
        let up = world.segment_hit_scoped(Some(i), eye, [eye[0], eye[1] + 256.0, eye[2]]);
        let dlen = up.map_or(256.0, |(t, _)| t * 256.0);
        let down = world.segment_hit_scoped(Some(i), eye, [eye[0], eye[1] - dlen, eye[2]]);
        println!("[cand {i}] up {up:?} (dlen {dlen:.3}) down {down:?}");
    }
    // The unscoped ray for reference.
    let up_any = world.segment_hit(eye, [eye[0], eye[1] + 256.0, eye[2]]);
    let down_any = world.segment_hit(eye, [eye[0], eye[1] - 256.0, eye[2]]);
    println!("[any] up {up_any:?} down {down_any:?}");

    // The graph cells 14/15 and their chains.
    for i in [14usize, 15] {
        let c = &g.cells[i];
        println!("[cell {i}] flags {:08x} bbox ({:.1},{:.1},{:.1})..({:.1},{:.1},{:.1}) chain {}",
            c.flags, c.min[0], c.min[1], c.min[2], c.max[0], c.max[1], c.max[2], g.chains[i].len());
    }
    // The portals near the eye: which portal would the vertical ray cross?
    for p in &g.portals {
        if p.cel_a == 14 || p.cel_b == 14 || p.cel_a == 15 || p.cel_b == 15 {
            let mut lo = [f32::MAX; 3]; let mut hi = [-f32::MAX; 3];
            for v in &p.corners { for k in 0..3 { lo[k] = lo[k].min(v[k]); hi[k] = hi[k].max(v[k]); } }
            println!("[portal] {}<->{} lo ({:.2},{:.2},{:.2}) hi ({:.2},{:.2},{:.2})",
                p.cel_a, p.cel_b, lo[0], lo[1], lo[2], hi[0], hi[1], hi[2]);
        }
    }
}
