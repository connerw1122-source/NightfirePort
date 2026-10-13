//! probe: the nav route from 05's spawn to the wire and to the zip door.
#[path = "common/walker.rs"]
mod walker;
fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }
#[test]
fn probe_05_nav() {
    let iso = iso();
    let scene = nf_view::load(&iso, "07000005.bin", false).expect("load");
    let sim = scene.sim.as_ref().unwrap();
    let start = sim.bond.pos;
    for (name, dest) in [("the wire", [-23.5f32, 34.5, 6.3]), ("the zip door", [37.3, 50.1, -68.4]), ("the gate door", [30.0, 38.8, 38.2])] {
        match walker::leg(sim, start, dest) {
            pts if pts.len() > 2 => {
                eprintln!("{name}: {} nodes:", pts.len() - 2);
                for p in pts.iter().skip(1).take(pts.len() - 2).step_by(3) {
                    eprintln!("  ({:.1},{:.1},{:.1})", p[0], p[1], p[2]);
                }
            }
            _ => eprintln!("{name}: no nav route"),
        }
    }
}
