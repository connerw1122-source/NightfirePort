//! walkreal-b probe: which mask-4 SPs live near 07000001's courtyard door, and does the
//! live_matrix fix change the E-press hit set there.

#[path = "common/walker.rs"]
mod walker;
use walker::Walker;

#[test]
fn probe_01_door_sps() {
    let iso = std::env::var("NF_ISO").unwrap();
    let mut w = Walker::new(nf_view::load_with_carry(&iso, "07000001.bin", false, None).unwrap());
    // Play the intro out like the escort test does.
    for t in 0..3200u32 {
        w.scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
        w.total_ticks += 1;
        if w.scene.sim.as_ref().unwrap().drones.channels.on(0xb) && t > 100 && !w.scene.sim.as_ref().unwrap().disabled { break; }
    }
    {
        let sim = w.scene.sim.as_ref().unwrap();
        for (i, sp) in sim.drones.scripts.players.iter().enumerate() {
            let lm = sp.live_matrix[3];
            let pm = sp.matrix[3];
            eprintln!("[p01] sp{i} mask {:#x} out {:#x} gate {:#x} activated {} placement ({:.2},{:.2},{:.2}) live ({:.2},{:.2},{:.2}) same {}",
                sp.mask, sp.out, sp.gate, sp.activated, pm[0], pm[1], pm[2], lm[0], lm[1], lm[2],
                (pm[0]-lm[0]).abs() < 0.01 && (pm[1]-lm[1]).abs() < 0.01 && (pm[2]-lm[2]).abs() < 0.01);
        }
    }
    // The door press: stand where the escort test stands (1.2 from the door, facing it) and press.
    let door = {
        let sim = w.scene.sim.as_ref().unwrap();
        sim.drones.doors.list.iter().enumerate()
            .min_by(|(_, a), (_, b)| ((a.pos[0] - 7.0).abs() + (a.pos[2] - 11.0).abs())
                .total_cmp(&((b.pos[0] - 7.0).abs() + (b.pos[2] - 11.0).abs())))
            .map(|(i, _)| (i, sim.drones.doors.list[i].pos)).unwrap()
    };
    eprintln!("[p01] door {} at {:?}", door.0, door.1);
    for back in [1.2f32, 2.2] {
        let (bx, bz) = (door.1[0] - 1.0 * back, door.1[2] + 0.0 * back); // Bond south of the door
        {
            let sim = w.scene.sim.as_mut().unwrap();
            let floor = sim.world.floor_below([bx, 7.0, bz], 6.0).unwrap_or(7.0);
            sim.bond.pos = [bx, floor + 1.1, bz];
            sim.bond.yaw = (door.1[0] - bx).atan2(door.1[2] - bz);
        }
        w.wait(2);
        // The press with look 1 unit ahead.
        let (head, look) = {
            let sim = w.scene.sim.as_ref().unwrap();
            let h = [sim.bond.pos[0], sim.bond.pos[1] + sim.bond.eye - sim.bond.crouch_drop, sim.bond.pos[2]];
            ([h[0], h[1], h[2]], [sim.bond.yaw.sin(), 0.0, sim.bond.yaw.cos()])
        };
        let used = w.scene.sim.as_mut().unwrap().drones.useobjs.player_activate(
            &mut w.scene.sim.as_mut().unwrap().drones.scripts,
            &mut w.scene.sim.as_mut().unwrap().drones.channels,
            head.into(), look.into(), 1);
        eprintln!("[p01] press at back {back}: used {used}; door pos01 {}", w.scene.sim.as_ref().unwrap().drones.doors.list[door.0].pos01);
    }
}
