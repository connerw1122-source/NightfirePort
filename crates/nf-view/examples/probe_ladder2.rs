//! fix-ladder2 probe: weapon 0x47 ('G' hands) record fields, the 07000002 water geometry,
//! and the ladder placements' draw resolution. Run:
//!   cargo run --offline --release -p nf-view --example probe_ladder2 -- <iso>
use nf_view::scene::Sim;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let iso = std::env::args().nth(1).expect("iso path");
    let mut scene = nf_view::load(&iso, "07000002.bin", false)?;
    let sim = scene.sim.take().expect("sim");

    // Weapon 0x47 ('G' hands) [M Player_WeaponNone 0x1a9858 sets the class byte to 0x47].
    let w = &sim.weapons.records[0x47];
    let u32_at = |o: usize| u32::from_le_bytes(w[o..o + 4].try_into().unwrap());
    println!("== weapon 0x47: ammo_type(+0x90)={} ammo_per_shot(+0x91)={} fire(+0xb0)={:08x} fire_last(+0xb4)={:08x} fire_anim_len(+0x44)={} idle(+0xa0)={:08x}",
        w[0x90], w[0x91], u32_at(0xb0), u32_at(0xb4), u32_at(0x44), u32_at(0xa0));
    for id in [1usize, 2] {
        let w = &sim.weapons.records[id];
        let u32_at = |o: usize| u32::from_le_bytes(w[o..o + 4].try_into().unwrap());
        println!("   weapon {id:#x}: ammo_type={} fire(+0xb0)={:08x} fire_last={:08x} len={}", w[0x90], u32_at(0xb0), u32_at(0xb4), u32_at(0x44));
    }

    // The water cels [M cel+0x90]: surface vs the collision floor under the centre.
    println!("== cels with water:");
    for (i, c) in sim.cels.iter().enumerate() {
        if c.water <= -400.0 { continue; }
        let cx = 0.5 * (c.min[0] + c.max[0]);
        let cz = 0.5 * (c.min[2] + c.max[2]);
        let floor = sim.world.floor_below([cx, c.water, cz], 14.0);
        println!("  cel {i}: min {:?} max {:?} water {} floor_at_centre {:?}", c.min, c.max, c.water, floor);
    }
    println!("== bond start {:?} yaw {}  cel {:?}", sim.bond.pos, sim.bond.yaw, sim.cel_at(sim.bond.pos));

    // The pool's collision from above: does a surface quad stop Bond at the water level?
    println!("== pool (cel 16) collision probes:");
    for &(px, pz) in &[(-42.0f32, 32.0f32), (-35.0, 25.0), (-45.0, 40.0), (-31.0, 22.0)] {
        let f = sim.world.floor_below([px, 6.0, pz], 12.0);
        println!("  ({px},{pz}): floor below 6.0 = {:?}", f);
    }
    // Ladders: the port's gameplay list (Ladder::all) vs the instances Scene::build drew.
    println!("== ladders ({}):", sim.ladders.len());
    for l in &sim.ladders {
        println!("  centre {:?} r {} yaw {} top {} bottom {}", l.centre, l.radius, l.yaw, l.top(), l.bottom());
    }
    Ok(())
}
// (fix-ladder2 probe continued inline: the pool's collision from above)
