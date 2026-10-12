#[path = "common/walker.rs"]
mod walker;
use walker::Walker;

#[test]
fn probe_09_stairs() {
    let iso = std::env::var("NF_ISO").unwrap();
    let mut w = Walker::new(nf_view::load(&iso, "07000009.bin", false).unwrap());
    w.wait(10);
    let sim = w.scene.sim.as_ref().unwrap();
    let start = sim.bond.pos;
    // A lobby anchor (the lift mouth area).
    let dest = [-3.5, -368.5, 14.0];
    eprintln!("nav route spawn→lobby: {:?}", walker::nav_route(sim, start, dest).map(|p| p.len()));
    if let Some(pts) = walker::nav_route(sim, start, dest) {
        for p in &pts { eprintln!("  nav ({:.2},{:.2},{:.2})", p[0], p[1], p[2]); }
    }
    // Floors inside the stairwell beyond the first door (21.5, -377.08, 40.9): sample along z.
    for z in [42.0, 45.0, 48.0, 51.0, 54.0, 57.0, 60.0] {
        for y in [-377.0, -375.0, -373.0, -371.0, -369.0] {
            if let Some(f) = sim.world.floor_below([17.5, y, z], 3.0) {
                eprintln!("floor at z {z} y {y}: {:.2}", f);
                break;
            }
        }
    }
}
