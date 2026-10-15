//! Scratch probe (109b): the stunner's fire timing through the real input path.

use nf_game::player::Input;

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

fn dxz(a: [f32; 3], b: [f32; 3]) -> f32 { ((a[0] - b[0]).powi(2) + (a[2] - b[2]).powi(2)).sqrt() }

#[test]
fn probe_stunner_fire_timing() {
    let iso = iso();
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005");
    let gi = {
        let sim = scene.sim.as_ref().unwrap();
        *sim.drones.llist.iter().find(|&&i| {
            let dr = &sim.drones.list[i];
            dr.present && dr.enabled && !dr.corpse && dr.health > 0.0
                && (dr.pos[0] - sim.bond.pos[0]).powi(2) + (dr.pos[2] - sim.bond.pos[2]).powi(2) < 400.0
        }).expect("a placed guard within 20 u of Bond's start on 07000005")
    };
    let (gpos, ghealth) = {
        let sim = scene.sim.as_ref().unwrap();
        (sim.drones.list[gi].pos, sim.drones.list[gi].health)
    };
    eprintln!("guard {gi} at {gpos:?} health {ghealth}");
    // Teleport Bond to 2.5 u from the guard, level pitch; the aim steering closes on him.
    let mut dev = nf_view::dev::Dev { scenario: None, ..Default::default() };
    let start = [gpos[0] - 2.5, gpos[1], gpos[2]];
    dev.teleport(&mut scene, start, 0.0);
    // Equip the stunner through the real switch input.
    let draw = Input { switch_to: Some(0x4a), ..Default::default() };
    let mut ok = false;
    for _ in 0..60 {
        scene.sim.as_mut().unwrap().step_bond(draw, false, false, false);
        if scene.sim.as_ref().unwrap().gun.weapon == 0x4a { ok = true; break; }
    }
    assert!(ok, "the Stunner (0x4a) did not equip — owned {:?}", scene.sim.as_ref().unwrap().drones.inventory.owned.get(0x4a));
    eprintln!("stunner equipped at frame {}", scene.sim.as_ref().unwrap().drones.frame);
    // Settle: let the draw finish (idle, no fire) — the user sees the device in hand before pressing.
    for _ in 0..70 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    {
        let sim = scene.sim.as_ref().unwrap();
        eprintln!("settled: script={:08x} vt={:.1} switching={}", sim.view_script, sim.view_time, sim.gun.switch_state);
    }
    let target = gpos;
    let mut held_ticks = 0u32;
    for t in 0..120 {
        let (turn, look) = {
            let sim = scene.sim.as_ref().unwrap();
            let bp = sim.bond.pos;
            let dx = target[0] - bp[0];
            let dz = target[2] - bp[2];
            let dy = target[1] - (bp[1] + sim.bond.eye - sim.bond.crouch_drop);
            let desired = dx.atan2(dz);
            let err = desired - sim.bond.yaw;
            let want_pitch = (dy / dxz(bp, target).max(0.5)).atan() / std::f32::consts::FRAC_PI_2;
            (err.clamp(-0.08, 0.08), (want_pitch - sim.bond.pitch).clamp(-0.08, 0.08))
        };
        scene.sim.as_mut().unwrap().step_bond(Input { turn, look, ..Default::default() }, true, false, false);
        held_ticks += 1;
        let sim = scene.sim.as_ref().unwrap();
        if t < 8 || t % 20 == 0 {
            let dr = &sim.drones.list[gi];
            eprintln!("hold t={t:3} f={} state9={} script={:08x} vt={:5.1} clip={} st={:?} beam={} ghp={:.1} gst={:#x}",
                sim.drones.frame, sim.gun.fire_state, sim.view_script, sim.view_time, sim.gun.clip,
                sim.stunner_target, sim.laser_beam.is_some(), dr.health, dr.state);
        }
    }
    // The release: watch what fires on the way up.
    for t in 0..40 {
        scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
        let sim = scene.sim.as_ref().unwrap();
        if t < 10 || t % 10 == 0 {
            let dr = &sim.drones.list[gi];
            eprintln!("rel  t={t:3} f={} state9={} script={:08x} vt={:5.1} clip={} st={:?} beam={} ghp={:.1} gst={:#x}",
                sim.drones.frame, sim.gun.fire_state, sim.view_script, sim.view_time, sim.gun.clip,
                sim.stunner_target, sim.laser_beam.is_some(), dr.health, dr.state);
        }
    }
    eprintln!("held {held_ticks} ticks; guard health now {}", scene.sim.as_ref().unwrap().drones.list[gi].health);
}
