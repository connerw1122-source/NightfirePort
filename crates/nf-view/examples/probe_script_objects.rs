//! Scratch probe (task 289): the urn SP's prime state + the truck script's stream 2.
use nf_formats::nis::Cmd;

fn main() {
    let iso_path = std::env::var("NF_ISO").expect("NF_ISO");

    // ── 07000007: the urn SP after one tick ──
    let mut scene = nf_view::load(&iso_path, "07000007.bin", false).unwrap();
    let sim = scene.sim.as_mut().unwrap();
    sim.step_bond(nf_game::player::Input::default(), false, false, false);
    let players = &sim.drones.scripts.players;
    println!("07000007: {} script players", players.len());
    for (i, p) in players.iter().enumerate() {
        if p.scripts[0] == 0x0600_007a {
            println!("urn SP at players[{i}]: state {} cur {} mask {} modes {:?}", p.state, p.cur, p.mask, p.modes);
            if let Some(s) = &p.info[p.cur] {
                println!("  script clock {} play {:04b}", s.clock, s.play);
                for (si, (kind, obj, fk, lk)) in s.stream_bindings().iter().enumerate().take(14) {
                    println!("  stream {si}: kind {kind} obj {obj:?} keys {fk}..{lk}");
                }
            } else { println!("  info[cur] = None"); }
        }
    }
    let by_src = sim.drones.scripts.objects.list.iter().flatten()
        .filter(|o| matches!(o, nf_game::nis::Obj::Entity { src: (s, _), .. } if *s == 0x0600_007a)).count();
    println!("urn entities in objects.list: {by_src} (list len {})", sim.drones.scripts.objects.list.len());
    // The blast flow: fire → replay → count.
    sim.drones.explosions.push(([49.26, 43.01, -53.46], 3.0, 100.0, usize::MAX, 0x34));
    for t in 1..=5 {
        sim.step_bond(nf_game::player::Input::default(), false, false, false);
        let p = sim.drones.scripts.players.iter().find(|p| p.scripts[0] == 0x0600_007a).unwrap();
        let n = sim.drones.scripts.objects.list.iter().flatten()
            .filter(|o| matches!(o, nf_game::nis::Obj::Entity { src: (s, _), .. } if *s == 0x0600_007a)).count();
        println!("t{t}: state {} clock {:?} play {:?} shards {n} sound1523 {}",
            p.state, p.info[p.cur].as_ref().map(|s| s.clock), p.info[p.cur].as_ref().map(nf_game::nis::Script::playing),
            sim.drones.sounds.iter().any(|s| s.id == 1523));
    }

    // ── 07000005: the truck script's stream 2 ──
    let mut scene2 = nf_view::load(&iso_path, "07000005.bin", false).unwrap();
    let sim2 = scene2.sim.as_mut().unwrap();
    let nis = sim2.drones.scripts.lib.get(&0x0600_0a22).expect("truck script in lib");
    for (si, st) in nis.streams.iter().enumerate().take(4) {
        println!("truck stream {si}:", );
        for c in st.decode().iter().take(6) {
            match c {
                Cmd::Entity { hash, obj_type, params, .. } => println!("   Entity {hash:08x} type {obj_type:02x} params {params:?}"),
                other => println!("   {other:?}"),
            }
        }
    }
    let before = sim2.volumes.len();
    sim2.drones.scripts.spawn_once(0, 0x0600_0a22,
        [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [98.49, 27.05, 106.36, 1.0]]);
    for t in 1..=64 {
        sim2.step_bond(nf_game::player::Input::default(), false, false, false);
        if t == 1 || t == 8 || t == 64 {
            let sp = sim2.drones.scripts.players.iter().find(|p| p.scripts[0] == 0x0600_0a22);
            println!("t{t}: pending_volumes {} volumes {} sp {:?}", sim2.drones.scripts.objects.pending_volumes.len(),
                sim2.volumes.len() - before,
                sp.map(|p| (p.state, p.info[p.cur].as_ref().map(|s| (s.clock, s.play)))));
        }
    }
    println!("volumes with truck src: {}", sim2.volumes.iter().filter(|v| v.src == Some((0x0600_0a22, 2))).count());
}
