//! U-032 probe 2: when does ch 0x43 (spawner 268's kill channel) turn on in a 07000005 session?
fn main() {
    let iso = std::env::var("NF_ISO").expect("NF_ISO");
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load");
    let sim = scene.sim.as_mut().unwrap();
    for t in 0..400u32 {
        sim.step_bond(Default::default(), false, false, false);
        if t % 20 == 0 || sim.drones.channels.on(0x43) {
            let sp = sim.drones.spawners.iter().find(|s| s.placement == 268);
            eprintln!("t={:3} ch0x43={:b} ch0x22={:b} spawner268 kill_latch={:b} active={:b} gone={:b}",
                t, sim.drones.channels.on(0x43) as u8, sim.drones.channels.on(0x22) as u8,
                sp.map(|s| s.kill_latch as u8).unwrap_or(9), sp.map(|s| s.active as u8).unwrap_or(9), sp.map(|s| s.gone as u8).unwrap_or(9));
        }
        if sim.drones.channels.on(0x43) && t > 5 { break; }
    }
}
