//! map-tower2 probe: live sim state of 07000011..07000014 at load and after stepping — the scratch
//! evidence behind docs/levels/*.md and the walk_*_live tests. Opt-in via NF_ISO.
//! Run: `NF_ISO=… cargo test -p nf-view --test walk_tower2_probe -- --nocapture --test-threads=1`

use nf_game::player::Input;

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

fn probe(name: &str, level: u32, ticks: u32) {
    let iso = iso();
    let mut scene = match nf_view::load(&iso, name, false) {
        Ok(s) => s,
        Err(e) => { println!("================ {name}: LOAD FAILED: {e}"); return; }
    };
    let sim = scene.sim.as_mut().expect("sim");
    println!("================ {name} ================");
    println!("level {:08x}  bond {:?} yaw {:.2}  disabled {}  drones {}  spawners {}",
        sim.level, sim.bond.pos, sim.bond.yaw, sim.disabled, sim.drones.list.len(), sim.drones.spawners.len());
    let mdef = |lv: u32| nf_game::mission::LEVELS.iter().enumerate().find(|(_, d)| d.level == lv);
    if let Some((i, d)) = mdef(level) {
        println!("mission {:08x} part {}: {}", d.level, d.part,
            d.objectives.iter().zip(&sim.drones.mission.states[i])
                .map(|(o, s)| format!("[ch{:02x} en{:02x} st{}]", o.channel, o.enable, s)).collect::<String>());
    } else { println!("mission: none"); }
    println!("exits {:?}",
        sim.drones.triggers.iter().filter(|t| t.kind == nf_game::triggers::kind::LOAD_LEVEL)
            .map(|t| (t.next_level, t.gate, t.key_gate, t.fallback)).collect::<Vec<_>>());
    let mut done = 0;
    for t in 0..ticks {
        sim.step_bond(Input::default(), false, false, false);
        if sim.drones.level_exit.is_some() { println!("level_exit {:08x} at tick {t}", sim.drones.level_exit.unwrap()); break; }
        if !sim.disabled && t > 5 && done == 0 { done = t; }
    }
    let on: Vec<String> = (0..256u32).filter(|&c| sim.drones.channels.on(c)).map(|c| format!("{c:02x}")).collect();
    println!("after {ticks}: bond {:?} disabled {} free_at {done}", sim.bond.pos, sim.disabled);
    println!("channels on: {}", on.join(","));
    if let Some((i, d)) = mdef(level) {
        println!("mission after: {}", d.objectives.iter().zip(&sim.drones.mission.states[i])
            .map(|(o, s)| format!("[ch{:02x} st{}]", o.channel, s)).collect::<String>());
    }
    let alive = sim.drones.list.iter().filter(|d| !d.corpse).count();
    println!("drones alive {}/{}  corpses {}", alive, sim.drones.list.len(), sim.drones.list.iter().filter(|d| d.corpse).count());
}

#[test]
fn tower2_levels_load_and_step() {
    probe("07000011.bin", 0x0700_0011, 900);
    probe("07000012.bin", 0x0700_0012, 900);
    probe("07000013.bin", 0x0700_0013, 900);
    probe("07000014.bin", 0x0700_0014, 900);
}

/// Every kind-5 script's command ops for one level — the SimpleScript/anim scripts (SCRIPT_SHUTTER_*,
/// lift screens). Shows which of them write switch channels (op 0xD) or gates (op 0x11).
#[test]
fn kind5_script_ops() {
    let iso = iso();
    for name in ["07000011.bin", "07000012.bin", "07000013.bin"] {
        let img = nf_formats::iso9660::IsoImage::open(&iso).unwrap();
        let (entries, bin) = nf_formats::disc::files_bin_entries(&img).unwrap();
        let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(name)).unwrap();
        let data = img.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).unwrap();
        let lb = nf_formats::levelbin::LevelBin::parse(&data).unwrap();
        println!("==== {name} kind-5 scripts:");
        for e in lb.entries.iter().filter(|e| e.kind == 5) {
            if let Ok(s) = nf_formats::anim::Script::parse(lb.data(&data, e)) {
                let ops: Vec<String> = s.commands.iter().map(|c| {
                    if c.op == 0xd || c.op == 0x11 { format!("op{:x}{:x?}", c.op, c.args) } else { format!("{:x}", c.op) }
                }).collect();
                let has_ch = ops.iter().any(|o| o.contains("opd"));
                if has_ch { println!("  {:08x}: {}", s.hashcode, ops.join(",")); }
            }
        }
        println!("  (no channel writes in the rest)");
    }
}
