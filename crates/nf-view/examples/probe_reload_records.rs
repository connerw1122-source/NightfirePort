//! Probe: dump the reload-relevant weapon_data facts for the task's ids on a level.
use nf_view::scene::Scene;

fn main() {
    let iso = std::env::var("NF_ISO").expect("NF_ISO");
    let level = std::env::args().nth(1).unwrap_or_else(|| "07000005.bin".into());
    let mut scene: Scene = nf_view::load_weapons(&iso, &level, false, &[0x02, 0x03, 0x1c, 0x1d, 0x2a, 0x2b, 0x2c, 0x2d]).expect("load");
    let sim = scene.sim.as_ref().unwrap();
    let wd = &sim.weapons;
    println!("level {level}");
    for id in [0x02u8, 0x03, 0x1c, 0x1d, 0x2a, 0x2b, 0x2c, 0x2d] {
        let Some(w) = wd.records.get(id as usize) else { println!("{id:#04x}: NO RECORD"); continue };
        let u32_at = |o: usize| u32::from_le_bytes(w[o..o + 4].try_into().unwrap());
        let known = |h: u32| sim.scripts.contains_key(&h);
        println!("{id:#04x}: base={:#04x} twin(+5)={} mode_count={} clip_size={} ammo_type={} flags68={:#010x}",
            wd.base(id as usize), w[5], w[0x2e], wd.clip_size(id as usize), wd.ammo_type(id as usize), wd.flags(id as usize));
        for (name, off) in [("idle+0xA0", 0xa0), ("reload+0xA4", 0xa4), ("start+0xA8", 0xa8), ("end+0xAC", 0xac), ("fire+0xB0", 0xb0), ("fireL+0xB4", 0xb4)] {
            let h = u32_at(off);
            println!("      {name}: {h:08x} len={} known={}", sim.scripts.get(&h).map(|s| s.length).unwrap_or(0), known(h));
        }
        println!("      fp(+0xDC)={:08x} draw(+0xC0)={:08x} holster(+0xC4)={:08x} variant(+0xC8)={:08x} fire_anim_len(+0x44)={}",
            wd.first_person(id as usize), u32_at(0xc0), u32_at(0xc4), u32_at(0xc8), u16::from_le_bytes([w[0x44], w[0x45]]));
    }
}
