#[path = "common/walker.rs"]
mod walker;
use walker::{Walker, Wp};
fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }
#[test]
fn probe_shaft_lobby() {
    let iso = iso();
    let mut w = Walker::new(nf_view::load_with_carry(&iso, "07000012.bin", false, None).unwrap());
    w.wait_enabled(3000, "intro");
    w.wait(120);
    // Open the lift doors (the SS pairs) by pressing the console E (ch 0x59).
    w.walk_to(Wp::new(-1.55, 0.5, 7.2).r(1.0), "console");
    for _ in 0..30 { w.tick(nf_game::player::Input::default(), true); }
    {
        let sim = w.scene.sim.as_ref().unwrap();
        eprintln!("[probe] ch59 {} ch49 {}", sim.drones.channels.on(0x59), sim.drones.channels.on(0x49));
    }
    for _ in 0..200 { w.tick(nf_game::player::Input::default(), false); }
    // Rays into the shaft lobby from every side at chest height.
    let sim = w.scene.sim.as_ref().unwrap();
    for (from, to, tag) in [
        ([0.5f32, 1.0, 8.5], [0.5, 1.0, 11.5], "south x0.5"),
        ([2.4f32, 1.0, 8.5], [2.4, 1.0, 11.5], "south x2.4"),
        ([4.5f32, 1.0, 9.5], [0.5, 1.0, 9.5], "east z9.5"),
        ([4.5f32, 1.0, 11.0], [0.5, 1.0, 11.0], "east z11"),
        ([-1.5f32, 1.0, 9.5], [3.0, 1.0, 9.5], "west z9.5"),
        ([-1.5f32, 1.0, 11.5], [3.0, 1.0, 11.5], "west z11.5"),
        ([2.3f32, 1.0, 12.0], [2.3, 1.0, 14.0], "north x2.3"),
    ] {
        match sim.world.segment_surface(from, to) {
            Some((t, n, class)) => {
                let at = [from[0] + (to[0]-from[0])*t, from[1], from[2] + (to[2]-from[2])*t];
                eprintln!("[probe] {tag}: hit ({:.1},{:.1}) n({:.1},{:.1},{:.1}) class {class}", at[0], at[2], n[0], n[1], n[2]);
            }
            None => eprintln!("[probe] {tag}: clear"),
        }
    }
}
