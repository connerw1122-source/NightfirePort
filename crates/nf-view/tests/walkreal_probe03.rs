//! walkreal probe for 07000003 "Alpine Escape" (Castle C): every interactable, door, trigger,
//! pickup, lock, SS, searchlight, script player and drone the disc places, through the loaded sim.
//! Diagnostic only — run with --nocapture.

#[path = "common/walker.rs"]
mod walker;

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

#[test]
fn dump_07000003() {
    let iso = iso();
    let scene = nf_view::load(&iso, "07000003.bin", false).expect("load 07000003");
    let sim = scene.sim.as_ref().unwrap();
    eprintln!("=== 07000003 ===");
    let b = &sim.bond;
    eprintln!("start: bond=({:.2},{:.2},{:.2}) yaw={:.3} disabled={}", b.pos[0], b.pos[1], b.pos[2], b.yaw, sim.disabled);
    eprintln!("cels: {}", sim.cels.len());

    eprintln!("--- triggers ---");
    for t in &sim.drones.triggers {
        let c = t.centre();
        let list = t.list.iter().map(|x| format!("{x:#x}")).collect::<Vec<_>>().join(",");
        eprintln!("trig kind={:2} out={:#04x} gate={} key={} fb={} next={:#x} mask={:#04x} lock={} at=({:.1},{:.1},{:.1}) list=[{}]",
            t.kind, t.out, t.gate, t.key_gate, t.fallback, t.next_level, t.mask, t.lock, c[0], c[1], c[2], list);
    }
    eprintln!("--- doors ---");
    for (i, d) in sim.drones.doors.list.iter().enumerate() {
        eprintln!("door[{i}] at=({:.1},{:.1},{:.1}) kind={} ch_open={:#x} ch_lock={:#x} lock_flag={} approach={} state={:#x}",
            d.pos[0], d.pos[1], d.pos[2], d.kind, d.ch_open, d.ch_lock, d.locked_flag, d.approach, d.state);
    }
    eprintln!("--- locks (safes) ---");
    for l in &sim.drones.locks {
        eprintln!("lock placement={} at=({:.2},{:.2},{:.2}) r={:.1} out={:#04x} keys={:?} burn={} unlock_on_op={}",
            l.placement, l.pos[0], l.pos[1], l.pos[2], l.radius, l.out, l.key_channels, l.burn, l.unlock_on_operative);
    }
    eprintln!("--- switches (0x29) ---");
    for s in &sim.drones.useobjs.switches {
        eprintln!("switch at=({:.1},{:.1},{:.1}) ch={:#x} gate={:#x} rest={} busy={} auto={}", s.pos[0], s.pos[1], s.pos[2], s.ch, s.gate, s.rest, s.busy, s.auto_delay);
    }
    eprintln!("--- simple scripts ---");
    for (i, s) in sim.drones.sss.list.iter().enumerate() {
        eprintln!("ss[{i}] placement={} ch={:#04x} state={:#x} mask={:#x} keys={} hash={:#x} at=({:.1},{:.1},{:.1})",
            s.placement, s.channel, s.state, s.mask, s.keys.len(), s.hash, s.matrix[3][0], s.matrix[3][1], s.matrix[3][2]);
    }
    eprintln!("--- searchlights ---");
    for (i, sl) in sim.drones.searchlights.parts.iter().enumerate() {
        eprintln!("searchlight[{i}] placement={} kind={} at=({:.1},{:.1},{:.1}) on={:#x} off={:#x} alarm={:#x} state={}",
            sl.placement, sl.kind, sl.spawn_pos[0], sl.spawn_pos[1], sl.spawn_pos[2], sl.on, sl.off, sl.alarm, sl.state);
    }
    eprintln!("--- pickups ---");
    for p in walker::pickups(sim) {
        eprintln!("pickup at=({:.1},{:.1},{:.1}) kind={:#x} ch={:#x}", p.0[0], p.0[1], p.0[2], p.1, p.2);
    }
    eprintln!("--- ladders ---");
    for (c, top, bot) in walker::ladders(sim) {
        eprintln!("ladder at=({:.1},{:.1},{:.1}) top={top:.1} bottom={bot:.1}", c[0], c[1], c[2]);
    }
    eprintln!("--- wires ---");
    for w in &sim.wires {
        eprintln!("wire centre=({:.1},{:.1},{:.1}) r={:.1} yaw={:.2} type={} ch={}", w.centre[0], w.centre[1], w.centre[2], w.radius, w.yaw, w.wire_type, w.channel);
    }
    eprintln!("--- script players (cut-scenes/SPs) ---");
    for (i, p) in scene.sim.as_ref().unwrap().drones.scripts.players.iter().enumerate() {
        eprintln!("sp[{i}] placement={} scripts={:?} modes={:?} mask={:#06x} out={:#06x} gate={:#06x} at=({:.1},{:.1},{:.1})",
            p.placement, p.scripts, p.modes, p.mask, p.out, p.gate, p.live_matrix[3][0], p.live_matrix[3][1], p.live_matrix[3][2]);
    }
    eprintln!("--- drones ---");
    for (i, d) in sim.drones.list.iter().enumerate() {
        eprintln!("drone[{i}] skin={:#x} pos=({:.1},{:.1},{:.1}) team={} started={} enabled={} corpse={} switch_ch={:#x} death_ch={:#x} health={:.0}",
            d.skin, d.pos[0], d.pos[1], d.pos[2], d.team, d.started, d.enabled, d.corpse, d.switch_channel, d.death_channel, d.health);
    }
    eprintln!("--- nav paths: {} ---", sim.nav.paths.len());
    eprintln!("--- scripts: channel writes (event 0xD / 0x11), loads (0x12), texts ---");
    let mut hashes: Vec<_> = sim.drones.scripts.lib.keys().copied().collect();
    hashes.sort_unstable();
    for h in hashes {
        let n = &sim.drones.scripts.lib[&h];
        for (si, st) in n.streams.iter().enumerate() {
            for c in st.decode() {
                match c {
                    nf_formats::nis::Cmd::Event { kind, ref params } if kind == 0xd && !params.is_empty() => {
                        eprintln!("script {h:#x} stream {si}: event 0xD ch={:#x} val={}", params[0], params.get(1).copied().unwrap_or(0));
                    }
                    nf_formats::nis::Cmd::Event { kind, ref params } if kind == 0x11 && params.len() >= 3 => {
                        eprintln!("script {h:#x} stream {si}: event 0x11 if ch={:#x} then={:#x} else={:#x}", params[0], params[1], params[2]);
                    }
                    nf_formats::nis::Cmd::Event { kind: 0x12, ref params } if !params.is_empty() => {
                        eprintln!("script {h:#x} stream {si}: LOAD LEVEL {:#x}", params[0]);
                    }
                    nf_formats::nis::Cmd::Text { label, time } => {
                        eprintln!("script {h:#x} stream {si}: text label {label:#x} {time}f");
                    }
                    _ => {}
                }
            }
        }
    }
}

/// Recon: the mux-overwrite semantics (hand the leaf channels), the gate-plate approach lane, and
/// the ch 0x62 latch suspicion.
#[test]
fn recon03() {
    let iso = iso();

    // 1. The mux family: the AND(0x4f,0x50) drives 0x51 every tick, so a hand of 0x51 is
    //    overwritten; the leaves 0x4f+0x50 + 0x4e must carry it.
    let mut scene = nf_view::load(&iso, "07000003.bin", false).expect("load");
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.drones.channels.set(0x4e, 1, sim.drones.frame);
        sim.drones.channels.set(0x51, 1, sim.drones.frame); // the WRONG hand (overwritten?)
        for _ in 0..30 { sim.step_bond(Default::default(), false, false, false); }
        eprintln!("recon1 hand 0x4e+0x51: 0x51={} 0x52={}", sim.drones.channels.on(0x51), sim.drones.channels.on(0x52));
    }
    let mut scene = nf_view::load(&iso, "07000003.bin", false).expect("reload");
    {
        let sim = scene.sim.as_mut().unwrap();
        for &c in &[0x4eu32, 0x4f, 0x50] { sim.drones.channels.set(c, 1, sim.drones.frame); }
        for t in 0..30 {
            sim.step_bond(Default::default(), false, false, false);
            if t < 3 || t == 29 {
                eprintln!("recon1b t{}: 4e={} 4f={} 50={} 51={} 52={}", t,
                    sim.drones.channels.on(0x4e), sim.drones.channels.on(0x4f), sim.drones.channels.on(0x50),
                    sim.drones.channels.on(0x51), sim.drones.channels.on(0x52));
            }
        }
        eprintln!("recon1 hand leaves: states={:?} fail_label={:#x} internal={} ch99={}", sim.drones.mission.states[6], sim.drones.mission.fail_label, sim.drones.mission.internal, sim.drones.channels.on(99));
    }

    // 2. Standable spots around the front gate (the stuck walk approached from the courtyard).
    let mut scene = nf_view::load(&iso, "07000003.bin", false).expect("reload");
    for (name, at) in [
        ("courtyard S of wall", [6.35f32, 6.3, -28.93]),
        ("near plate N", [11.8, 6.0, -35.6]),
        ("plate S", [11.8, 6.0, -34.5]),
        ("gate front", [14.7, 6.3, -33.5]),
        ("gate back", [14.7, 6.3, -36.5]),
        ("west lane", [5.0, 6.0, -33.0]),
        ("far west", [0.0, 6.0, -33.0]),
        ("SW corner", [0.0, 6.0, -36.0]),
        ("castle door 2", [11.8, 6.3, -19.0]),
        ("east lane 1", [16.0, 6.3, -28.0]),
        ("east lane 2", [16.0, 6.3, -31.0]),
        ("east lane 3", [15.0, 6.3, -33.0]),
        ("east lane 4", [13.5, 6.3, -34.2]),
        ("west lane east", [8.0, 6.0, -36.0]),
        ("west lane east 2", [10.5, 6.0, -36.0]),
    ] {
        let sim = scene.sim.as_mut().unwrap();
        sim.bond.pos = at;
        sim.bond.vel = [0.0; 3];
        for _ in 0..45 { sim.step_bond(Default::default(), false, false, false); }
        let p = sim.bond.pos;
        eprintln!("recon2 {name}: dropped to ({p:?}) grounded={} ch62={}", sim.bond.grounded, sim.drones.channels.on(0x62));
    }
}
