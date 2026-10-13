//! walkreal-a probe: dump the route-relevant data (start, triggers, doors, pickups, ladders, wires)
//! for the eight walkreal levels. Diagnostic only — run with --nocapture.

#[path = "common/walker.rs"]
mod walker;

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

#[test]
fn dump_levels() {
    let iso = iso();
    for code in ["07000001", "07000002", "07000003", "07000004", "07000005", "07000006", "07000007", "07000008"] {
        let scene = match nf_view::load(&iso, &format!("{code}.bin"), false) {
            Ok(s) => s,
            Err(e) => { eprintln!("=== {code}: LOAD FAIL {e}"); continue; }
        };
        let sim = scene.sim.as_ref().unwrap();
        eprintln!("=== {code} ===");
        let b = &sim.bond;
        eprintln!("start: bond=({:.2},{:.2},{:.2}) yaw={:.3} disabled={}", b.pos[0], b.pos[1], b.pos[2], b.yaw, sim.disabled);
        eprintln!("cels: {}", sim.cels.len());
        for t in &sim.drones.triggers {
            if t.tris.is_empty() && t.kind != walker::trigger_kind::LOAD_LEVEL && t.kind != walker::trigger_kind::AND
                && t.kind != walker::trigger_kind::OUT && t.kind != walker::trigger_kind::OR { continue; }
            let c = t.centre();
            let list = t.list.iter().map(|x| format!("{x:#x}")).collect::<Vec<_>>().join(",");
            eprintln!("trig kind={:2} out={:#04x} gate={} key={} fb={} next={:#x} mask={:#04x} lock={} at=({:.1},{:.1},{:.1}) list=[{}]",
                t.kind, t.out, t.gate, t.key_gate, t.fallback, t.next_level, t.mask, t.lock, c[0], c[1], c[2], list);
        }
        for (i, d) in sim.drones.doors.list.iter().enumerate() {
            eprintln!("door[{i}] at=({:.1},{:.1},{:.1}) kind={} ch_open={:#x} ch_lock={:#x} lock_flag={} approach={} state={:#x}",
                d.pos[0], d.pos[1], d.pos[2], d.kind, d.ch_open, d.ch_lock, d.locked_flag, d.approach, d.state);
        }
        for p in walker::pickups(sim) {
            eprintln!("pickup at=({:.1},{:.1},{:.1}) kind={:#x} ch={:#x}", p.0[0], p.0[1], p.0[2], p.1, p.2);
        }
        for (c, top, bot) in walker::ladders(sim) {
            eprintln!("ladder at=({:.1},{:.1},{:.1}) top={top:.1} bottom={bot:.1}", c[0], c[1], c[2]);
        }
        for w in &sim.wires {
            eprintln!("wire centre=({:.1},{:.1},{:.1}) r={:.1} yaw={:.2} type={} ch={}", w.centre[0], w.centre[1], w.centre[2], w.radius, w.yaw, w.wire_type, w.channel);
        }
        for (i, d) in sim.drones.list.iter().enumerate() {
            eprintln!("drone[{i}] skin={:#x} pos=({:.1},{:.1},{:.1}) started={} switch_ch={:#x} corpse={} health={:.0}",
                d.skin, d.pos[0], d.pos[1], d.pos[2], d.started, d.switch_channel, d.corpse, d.health);
        }
    }
}
