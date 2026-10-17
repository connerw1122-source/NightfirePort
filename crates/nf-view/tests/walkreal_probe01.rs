//! walkreal probe for 07000001 "Paris Prelude" (Castle A): every interactable, door, trigger,
//! pickup, lock, SS, searchlight, script player and drone the disc places, through the loaded sim.
//! Diagnostic only — run with --nocapture.

#[path = "common/walker.rs"]
mod walker;

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

#[test]
fn dump_07000001() {
    let iso = iso();
    let scene = nf_view::load(&iso, "07000001.bin", false).expect("load 07000001");
    let sim = scene.sim.as_ref().unwrap();
    eprintln!("=== 07000001 ===");
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

#[test]
fn dump_scripts_01() {
    let iso = iso();
    let scene = nf_view::load(&iso, "07000001.bin", false).expect("load 07000001");
    let sim = scene.sim.as_ref().unwrap();
    for h in [0x0600_0086u32, 0x0600_0087, 0x0600_0096, 0x0600_0097] {
        let Some(n) = sim.drones.scripts.lib.get(&h) else { eprintln!("script {h:#x} MISSING"); continue };
        eprintln!("=== script {h:#x} streams {} ===", n.streams.len());
        for (si, st) in n.streams.iter().enumerate() {
            eprintln!("--- stream {si} ---");
            for c in st.decode() {
                eprintln!("  {c:?}");
            }
        }
    }
}

#[test]
fn find_scripts_96_97() {
    use nf_formats::{iso9660::IsoImage, levelbin::LevelBin};
    let iso = iso();
    let image = IsoImage::open(&iso).expect("open iso");
    let (entries, bin) = nf_formats::disc::files_bin_entries(&image).expect("files");
    for fe in entries.iter().filter(|e| e.name.to_ascii_lowercase().ends_with(".bin")) {
        let data = image.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).expect("read");
        let Ok(lb) = LevelBin::parse(&data) else { continue };
        for e in lb.entries.iter().filter(|e| e.kind == 7) {
            let h = u32::from_str_radix(&e.name, 16).unwrap_or(0);
            if h == 0x0600_0096 || h == 0x0600_0097 || h == 0x0600_0086 {
                eprintln!("script {h:08x} found in {} (kind 7, len {})", fe.name, e.length);
            }
        }
    }
    eprintln!("scan done");
}
