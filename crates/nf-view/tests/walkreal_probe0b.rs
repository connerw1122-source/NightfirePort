#[path = "common/walker.rs"]
mod walker;
use walker::Walker;

#[test]
fn probe_0b_deck_edges() {
    let iso = std::env::var("NF_ISO").unwrap();
    let mut w = Walker::new(nf_view::load_with_carry(&iso, "0700000b.bin", false, None).unwrap());
    w.wait(5);
    // Stand where the walk test stands after taking the parachute.
    {
        let sim = w.scene.sim.as_mut().unwrap();
        sim.bond.pos = [2.87, 57.6, 41.3];
        sim.bond.vel = [0.0; 3];
    }
    w.wait(20);
    // Floor samples around the deck (where does the y≈58 surface end?)
    {
        let sim = w.scene.sim.as_ref().unwrap();
        for (x, z) in [(2.5f32, 35.0), (2.5, 40.0), (2.5, 41.0), (2.5, 42.0), (2.5, 44.0), (2.5, 46.0), (2.5, 50.0),
                       (-5.0, 41.0), (-8.0, 41.0), (8.0, 41.0), (12.0, 41.0), (16.0, 41.0), (2.5, 25.0), (2.5, 20.0)] {
            let f = sim.world.floor_below([x, 58.0, z], 12.0);
            eprintln!("[probe] floor at ({x},{z}) y58-12: {f:?}");
        }
    }
    // Drive each direction from the stance and see which one moves.
    for (label, tx, tz) in [("south", 2.5f32, 60.0), ("east", 30.0, 41.3), ("west", -20.0, 41.3), ("north", 2.5, 5.0)] {
        {
            let sim = w.scene.sim.as_mut().unwrap();
            sim.bond.pos = [2.87, 57.6, 41.3];
            sim.bond.vel = [0.0; 3];
        }
        w.wait(10);
        let start = w.total_ticks;
        let mut landed = false;
        while w.total_ticks - start < 400 {
            let sim = w.scene.sim.as_ref().unwrap();
            let p = sim.bond.pos;
            if p[1] < 55.0 { landed = true; eprintln!("[probe] {label}: FELL to {p:?} at +{}", w.total_ticks - start); break; }
            let desired = (tx - p[0]).atan2(tz - p[2]);
            let err = walker::wrap_pi(desired - sim.bond.yaw);
            let jump = w.total_ticks % 30 == 0 && sim.bond.grounded;
            let input = nf_game::player::Input {
                turn: err.clamp(-0.08, 0.08),
                forward: if err.abs() > 1.0 { 0.0 } else { 1.0 },
                jump,
                ..Default::default()
            };
            w.tick(input, false);
        }
        let p = w.scene.sim.as_ref().unwrap().bond.pos;
        eprintln!("[probe] {label}: end pos {p:?} landed {landed}");
    }
}

/// Warm load (the carried channels) + parachute, then drive east — bisect the probe-vs-walk difference.
#[test]
fn probe_0b_warm_east_fall() {
    let iso = std::env::var("NF_ISO").unwrap();
    let cold = nf_view::load_with_carry(&iso, "0700000b.bin", false, None).unwrap();
    let save = {
        let sim = cold.sim.as_ref().unwrap();
        nf_game::player::RamSave::capture(&sim.drones.health, sim.gun.switch_request, &sim.drones.inventory, &sim.drones.channels)
    };
    let mut w = Walker::new(nf_view::load_with_carry(&iso, "0700000b.bin", false, Some(&save)).unwrap());
    for c in [0x46u8, 0x47, 0x48, 0x49, 0x4a] { w.scene.sim.as_mut().unwrap().drones.set_channel(c); }
    w.wait(5);
    // Take the parachute the same way (stance + forward).
    let pp = {
        let sim = w.scene.sim.as_ref().unwrap();
        let p = sim.drones.pickups.iter().find(|p| p.switch_channel == 0x3f).unwrap().m[3];
        [p[0], p[1], p[2]]
    };
    w.scene.sim.as_mut().unwrap().bond.pos = [pp[0], pp[1] - 1.2, pp[2] + 1.2];
    w.wait(30);
    let mut taken = false;
    'stances: for c in [[0.0f32, 0.0, 0.0], [0.6, 0.0, 0.6], [-0.6, 0.0, 0.6], [0.6, 0.0, -0.6], [-0.6, 0.0, -0.6], [0.0, 0.6, 0.0], [0.0, 1.2, 0.0]] {
        for _ in 0..40 {
            w.tick(nf_game::player::Input { forward: 0.4, ..Default::default() }, false);
            if w.scene.sim.as_ref().unwrap().drones.channels.on(0x3f) { taken = true; break 'stances; }
        }
        let sim = w.scene.sim.as_mut().unwrap();
        sim.bond.pos = [pp[0] + c[0], pp[1] + c[1] - 1.2, pp[2] + c[2]];
        sim.bond.vel = [0.0; 3];
    }
    eprintln!("[probe] parachute taken {taken}; ch0x3f {}", w.scene.sim.as_ref().unwrap().drones.channels.on(0x3f));
    eprintln!("[probe] bond at east-drive start {:?}", w.scene.sim.as_ref().unwrap().bond.pos);
    let start = w.total_ticks;
    while w.total_ticks - start < 600 {
        if w.scene.sim.as_ref().unwrap().drones.channels.on(0x40) { eprintln!("[probe] ch 0x40 at +{}", w.total_ticks - start); break; }
        let sim = w.scene.sim.as_ref().unwrap();
        let p = sim.bond.pos;
        if (w.total_ticks - start) % 20 == 0 { eprintln!("[probe] t+{} bond {:?}", w.total_ticks - start, p); }
        let desired = (30.0f32 - p[0]).atan2(41.3 - p[2]);
        let err = walker::wrap_pi(desired - sim.bond.yaw);
        let jump = w.total_ticks % 30 == 0 && sim.bond.grounded;
        let input = nf_game::player::Input { turn: err.clamp(-0.08, 0.08), forward: if err.abs() > 1.0 { 0.0 } else { 1.0 }, jump, ..Default::default() };
        w.tick(input, false);
    }
    for (c, name) in [(0x40u32, "0x40"), (0x41, "0x41"), (0x5a, "0x5a"), (100, "100"), (0x7a, "0x7a"), (0x7b, "0x7b")] {
        eprintln!("[probe] ch {name} = {}", w.scene.sim.as_ref().unwrap().drones.channels.on(c));
    }
    let p = w.scene.sim.as_ref().unwrap().bond.pos;
    eprintln!("[probe] warm east-fall end pos {p:?}");
}

/// Dump the ch 0x40 band volumes' trigger params (lock/gate/hide) — the walk test sets ch 9, 0x08, 0x59.
#[test]
fn probe_0b_band_params() {
    let iso = std::env::var("NF_ISO").unwrap();
    let scene = nf_view::load_with_carry(&iso, "0700000b.bin", false, None).unwrap();
    let sim = scene.sim.as_ref().unwrap();
    for t in &sim.drones.triggers {
        if t.out == 0x40 || t.list.contains(&0x40) || t.out == 0x41 {
            eprintln!("[probe] trig pl{} kind{} out {:#x} lock {} mask {:#x} list {:?} hide {} gate {} tris {} lo {:?} hi {:?}",
                t.placement, t.kind, t.out, t.lock, t.mask, t.list, t.hide, t.gate, t.tris.len(), t.lo, t.hi);
        }
    }
}

/// Cold load, wait ~8k ticks (the walk test's age), then drive east — does the deck drift?
#[test]
fn probe_0b_late_east_fall() {
    let iso = std::env::var("NF_ISO").unwrap();
    let mut w = Walker::new(nf_view::load_with_carry(&iso, "0700000b.bin", false, None).unwrap());
    w.wait(5);
    {
        let sim = w.scene.sim.as_mut().unwrap();
        sim.bond.pos = [2.87, 57.6, 41.3];
        sim.bond.vel = [0.0; 3];
    }
    w.wait(8000);
    eprintln!("[probe] after 8000 ticks bond {:?}", w.scene.sim.as_ref().unwrap().bond.pos);
    {
        let sim = w.scene.sim.as_ref().unwrap();
        for (x, z) in [(2.5f32, 41.0), (2.5, 35.0), (8.0, 41.0), (12.0, 41.0), (2.5, 25.0)] {
            let f = sim.world.floor_below([x, 58.0, z], 12.0);
            eprintln!("[probe] late floor at ({x},{z}): {f:?}");
        }
    }
    {
        let sim = w.scene.sim.as_mut().unwrap();
        sim.bond.pos = [2.87, 57.6, 41.3];
        sim.bond.vel = [0.0; 3];
    }
    w.wait(10);
    let start = w.total_ticks;
    while w.total_ticks - start < 600 {
        if w.scene.sim.as_ref().unwrap().drones.channels.on(0x40) { eprintln!("[probe] ch 0x40 at +{}", w.total_ticks - start); break; }
        let sim = w.scene.sim.as_ref().unwrap();
        let p = sim.bond.pos;
        if (w.total_ticks - start) % 20 == 0 { eprintln!("[probe] t+{} bond {:?}", w.total_ticks - start, p); }
        let desired = (30.0f32 - p[0]).atan2(41.3 - p[2]);
        let err = walker::wrap_pi(desired - sim.bond.yaw);
        let input = nf_game::player::Input { turn: err.clamp(-0.08, 0.08), forward: 1.0, ..Default::default() };
        w.tick(input, false);
    }
    for (c, name) in [(0x40u32, "0x40"), (0x41, "0x41"), (0x7b, "0x7b")] {
        eprintln!("[probe] ch {name} = {}", w.scene.sim.as_ref().unwrap().drones.channels.on(c));
    }
    let p = w.scene.sim.as_ref().unwrap().bond.pos;
    eprintln!("[probe] late east-fall end pos {p:?}");
}

/// Drive east off the deck edge and watch ch 0x40 through the fall.
#[test]
fn probe_0b_east_fall() {
    let iso = std::env::var("NF_ISO").unwrap();
    let mut w = Walker::new(nf_view::load_with_carry(&iso, "0700000b.bin", false, None).unwrap());
    w.wait(5);
    {
        let sim = w.scene.sim.as_mut().unwrap();
        sim.bond.pos = [2.87, 57.6, 41.3];
        sim.bond.vel = [0.0; 3];
    }
    w.wait(10);
    let start = w.total_ticks;
    while w.total_ticks - start < 400 {
        if w.scene.sim.as_ref().unwrap().drones.channels.on(0x40) { eprintln!("[probe] ch 0x40 at +{}", w.total_ticks - start); break; }
        let sim = w.scene.sim.as_ref().unwrap();
        let p = sim.bond.pos;
        let desired = (30.0f32 - p[0]).atan2(41.3 - p[2]);
        let err = walker::wrap_pi(desired - sim.bond.yaw);
        let input = nf_game::player::Input { turn: err.clamp(-0.08, 0.08), forward: 1.0, ..Default::default() };
        w.tick(input, false);
        if (w.total_ticks - start) % 30 == 0 { eprintln!("[probe] t+{} bond {:?}", w.total_ticks - start, w.scene.sim.as_ref().unwrap().bond.pos); }
    }
    for (c, name) in [(0x40u32, "0x40"), (0x41, "0x41"), (0x5a, "0x5a"), (100, "100"), (0x7b, "0x7b"), (0x62, "0x62")] {
        eprintln!("[probe] ch {name} = {}", w.scene.sim.as_ref().unwrap().drones.channels.on(c));
    }
    let p = w.scene.sim.as_ref().unwrap().bond.pos;
    eprintln!("[probe] east-fall end pos {p:?}");
}
