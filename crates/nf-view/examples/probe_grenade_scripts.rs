//! Probe: the grenade family's fire scripts — lengths, stop commands, release frames.
use nf_view::scene::Scene;

fn main() {
    let iso = std::env::var("NF_ISO").expect("NF_ISO");
    let level = std::env::args().nth(1).unwrap_or_else(|| "07000001.bin".into());
    let mut scene: Scene = nf_view::load_weapons(&iso, &level, false, &[0x34, 0x35, 0x36, 0x37]).expect("load");
    let sim = scene.sim.as_ref().unwrap();
    let wd = &sim.weapons;
    println!("level {level}");
    for id in [0x34u8, 0x35, 0x36, 0x37] {
        let Some(w) = wd.records.get(id as usize) else { println!("{id:#04x}: NO RECORD"); continue };
        let u32_at = |o: usize| u32::from_le_bytes(w[o..o + 4].try_into().unwrap());
        let falen = u16::from_le_bytes([w[0x44], w[0x45]]);
        let fire = u32_at(0xb0);
        let fire_last = u32_at(0xb4);
        println!("{id:#04x}: +0x44(fire_anim_len)={falen} +0x68={:#010x} +0x6C={:#010x} fire={fire:08x} fire_last={fire_last:08x}",
            wd.flags(id as usize), u32_at(0x6c));
        for (name, h) in [("fire(+0xB0)", fire), ("fire_last(+0xB4)", fire_last)] {
            match sim.scripts.get(&h) {
                Some(s) => {
                    println!("      {name} {h:08x}: length={} commands={}", s.length, s.commands.len());
                    for c in &s.commands {
                        println!("        op {} args {:?} frame {}", c.op, c.args, c.args.first().copied().unwrap_or(0));
                    }
                }
                None => println!("      {name} {h:08x}: NOT LOADED"),
            }
        }
    }
}
