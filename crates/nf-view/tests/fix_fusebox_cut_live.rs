//! fix-fusebox-cut: the fusebox wires are cut by HITS (the Laser's beam bullets), not by pressing E.
//!
//! The user's play (2026-10-12): on 07000008 the guard-room fusebox has to be burned with the laser
//! gadget; the port opened it with two E presses (the fix-objects2 stand-in).
//!
//! The game's cut [M]: `FuseBox_Update` 0x1adfe8:88 filters each wire's hit list with
//! `Collide_FilterBullets(list, 0xfffd)` (0x1ece30) — class-5 hitters whose `weapon_definition+0x10`
//! category word ANDs non-zero are DROPPED; a wire with a surviving hit dies. Disc words [L
//! weapon_data dump]: every gun is 0x0008 (gunfire never cuts), the **Laser** (0x4E/0x4F, the disc's
//! own name) is 0x0002 and cuts. `Player_Activate` 0x1a5170 has NO class-0x3C arm — E never reaches
//! a fusebox. The wires are the box script's event-0xF registrations (`FuseBox_Register` 0x1b0970
//! through `Script_EventHandler` 0x1d94c0:172-175) — the port's decode: 07000008's 0x060008AD
//! stream 2 → group 0 [L NF_SCRIPTLOG].

use nf_game::player::Input;

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

fn load(iso: &str, level: &str) -> nf_view::scene::Scene {
    nf_view::load(iso, level, false).expect("load")
}

/// Walk Bond to a firing spot for `wire`: drop him near the box (known-good terrain, the old
/// E-test offset), let him settle on the floor, then close horizontally until the head→wire
/// distance is inside the Laser's 4.0 range [L weapon_data 0x4E +0x1C]. Returns the settled
/// head→wire distance.
fn take_burning_stance(sim: &mut nf_view::scene::Sim, box_pos: [f32; 3], wire: [f32; 3]) -> f32 {
    // Candidate stands around the box (the old E-test offset first), each settled on the floor;
    // the closest head→wire distance inside the Laser's range wins.
    let candidates = [[0.6f32, -1.2, 0.0], [-0.6, -1.2, 0.0], [0.0, -1.2, 0.6], [0.0, -1.2, -0.6],
        [1.2, -2.2, 0.0], [-1.2, -2.2, 0.0], [0.0, -2.2, 1.2], [0.0, -2.2, -1.2],
        [0.6, -1.2, 0.6], [-0.6, -1.2, -0.6]];
    let mut best = f32::MAX;
    for c in candidates {
        // Pre-screen the floor with a downward world ray: never teleport over a void/kill volume.
        let spot = [box_pos[0] + c[0], box_pos[1] + c[1], box_pos[2] + c[2]];
        let floor = sim.world.segment(spot, [spot[0], spot[1] - 30.0, spot[2]]).map(|t| spot[1] - 30.0 * t);
        match floor {
            Some(f) if (f - (box_pos[1] - 2.4)).abs() < 3.0 => {}
            _ => { eprintln!("[stance] candidate {c:?} skipped (floor {floor:?} not at box height)"); continue; }
        }
        sim.bond.pos = spot;
        for _ in 0..60 { sim.step_bond(Input::default(), false, false, false); }
        // Slide horizontally toward the wire while the vertical gap allows it.
        for _ in 0..4 {
            let head = sim.bond.head();
            let d = [wire[0] - head[0], wire[1] - head[1], wire[2] - head[2]];
            let dist = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
            if dist <= 2.2 { break; }
            let h = (d[0] * d[0] + d[2] * d[2]).sqrt();
            if h < 0.3 { break; } // the gap is vertical — sliding can't close it
            let step = (dist - 2.0).min(h);
            sim.bond.pos = [sim.bond.pos[0] + d[0] / h * step, sim.bond.pos[1], sim.bond.pos[2] + d[2] / h * step];
            for _ in 0..10 { sim.step_bond(Input::default(), false, false, false); }
        }
        let head = sim.bond.head();
        let d = [wire[0] - head[0], wire[1] - head[1], wire[2] - head[2]];
        let dist = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
        eprintln!("[stance] candidate {:?} -> bond {:?} dist {dist:.2}", c, sim.bond.pos);
        if dist < best { best = dist; }
        if best <= 2.6 { break; }
    }
    best
}

/// The laser burns the 07000008 GuardRoom fusebox's wire → ch 137 → the doorway bolt opens.
/// Fail-first on the pre-fix tree: no wire registration existed and E did the cut.
#[test]
fn the_laser_cuts_the_guardroom_fusebox_wire_to_ch137() {
    let iso = iso();
    let mut scene = load(&iso, "07000008.bin");
    for _ in 0..120 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    let (fb, wire_obj, wire_pos) = {
        let sim = scene.sim.as_ref().unwrap();
        let f = sim.drones.fuseboxes.iter().position(|f| f.ch[0] == 0x89).expect("the GuardRoom fusebox (ch 137) exists [D #219]");
        let fb = &sim.drones.fuseboxes[f];
        eprintln!("[laser] fusebox #{} ch {:?} (param 2 = the hurt gate)", fb.placement, fb.ch);
        let mut wires = fb.alive_wires();
        let (g, k, obj) = wires.next().expect("the box's script registered a group-0 wire [L NF_SCRIPTLOG stream 2 → group 0]");
        assert_eq!((g, k), (0, 0), "07000008's box registers ONE wire in group 0");
        assert!(wires.next().is_none(), "no second wire on this box");
        let pos = nf_game::drone::Drones::script_obj_pos(&sim.drones.scripts, &sim.data(), obj)
            .expect("the wire's script object has a world position");
        eprintln!("[laser] wire obj {obj} at {pos:?}");
        (f, obj, pos)
    };
    // Take a firing stance inside the Laser's 4.0 range [L weapon_data 0x4E +0x1C], aim through
    // the wire, equip the Laser (0x4E) and hold fire.
    {
        let sim = scene.sim.as_mut().unwrap();
        let bp = sim.drones.fuseboxes[fb].pos;
        let dist = take_burning_stance(sim, bp, wire_pos);
        eprintln!("[laser] stance dist {dist:.2}");
        assert!(dist < 3.6, "the stance reaches the wire (Laser range 4.0)");
    }
    let draw = Input { switch_to: Some(0x4e), ..Default::default() };
    for _ in 0..40 { scene.sim.as_mut().unwrap().step_bond(draw, false, false, false); }
    assert_eq!(scene.sim.as_ref().unwrap().gun.weapon, 0x4e, "the Laser is selected [D USATxt label 0x05000043]");
    let mut ch137 = false;
    for t in 0..200u32 {
        {   // re-aim every tick through the wire (the sim may nudge Bond).
            let sim = scene.sim.as_mut().unwrap();
            let head = sim.bond.head();
            let dir = [wire_pos[0] - head[0], wire_pos[1] - head[1], wire_pos[2] - head[2]];
            let l = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt().max(1e-4);
            sim.bond.yaw = dir[0].atan2(dir[2]);
            sim.bond.pitch = (dir[1] / l).asin() / (std::f32::consts::FRAC_PI_2);
        }
        scene.sim.as_mut().unwrap().step_bond(Input::default(), true, false, false);
        if t % 40 == 0 { eprintln!("[laser] t={t} bond {:?} wire alive {}", scene.sim.as_ref().unwrap().bond.pos,
            scene.sim.as_ref().unwrap().drones.fuseboxes[fb].alive_wires().count()); }
        if scene.sim.as_ref().unwrap().drones.channels.on(137) { ch137 = true; break; }
    }
    let sim = scene.sim.as_ref().unwrap();
    eprintln!("[laser] ch137 {ch137} health {:.1}", sim.drones.health.health);
    assert!(ch137, "the laser burn cuts the wire and FuseBox_Update writes ch 137 [M 0x1adfe8:110-115]");
    assert!(sim.drones.fuseboxes[fb].alive_wires().next().is_none(), "the wire is gone from its group [M 0x1adfe8 slot = 0]");
    assert!(sim.drones.scripts.objects.list.get(wire_obj).map_or(true, |o| o.is_none()),
        "the wire's entity is removed (Effect_RemoveSObjs 0x1cf360 [M 0x1adfe8:93-96])");
    // The doorway bolt SS opens on ch 137 [D #217 param 3] and the box never hurts here
    // (no group-1 wires were registered — the hurt needs a group-1 DEATH [M 0x1adfe8:98-101]).
    assert_eq!(sim.drones.health.health, 100.0, "no live-wire hurt: this box registered no group-1 wires");
    let bolt_t = sim.drones.sss.list.iter().find(|s| s.channel == 137).map(|s| s.t).unwrap_or(-1.0);
    assert!(bolt_t > 0.0, "the doorway bolt SS opens on ch 137 [D #217]");
}

/// Sister A: pressing E at the fusebox does NOTHING — `Player_Activate` 0x1a5170's dispatch has no
/// class-0x3C arm [M 0x1a5170:95-160]. The old two-E-presses stand-in is gone.
#[test]
fn e_presses_do_nothing_to_the_fusebox() {
    let iso = iso();
    let mut scene = load(&iso, "07000008.bin");
    for _ in 0..120 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    let (fb, wire_obj, wire_pos) = {
        let sim = scene.sim.as_ref().unwrap();
        let f = sim.drones.fuseboxes.iter().position(|f| f.ch[0] == 0x89).expect("the GuardRoom fusebox");
        let obj = sim.drones.fuseboxes[f].alive_wires().next().expect("the wire is registered").2;
        let pos = nf_game::drone::Drones::script_obj_pos(&sim.drones.scripts, &sim.data(), obj).unwrap();
        (f, obj, pos)
    };
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.bond.pos = [wire_pos[0] + 1.2, wire_pos[1] - 1.0, wire_pos[2]];
        sim.bond.yaw = (wire_pos[0] - sim.bond.pos[0]).atan2(wire_pos[2] - sim.bond.pos[2]);
        sim.bond.pitch = 0.0;
    }
    // The old stand-in's exact flow: the lid press, the 63-tick lid script, the cut press.
    for _ in 0..5 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, true); }
    for _ in 0..80 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    for _ in 0..5 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, true); }
    for _ in 0..20 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    let sim = scene.sim.as_ref().unwrap();
    assert!(!sim.drones.channels.on(137), "E never cuts a fusebox wire [M 0x1a5170:95-160 — no 0x3C arm]");
    assert!(sim.drones.fuseboxes[fb].alive_wires().next().is_some(), "the wire is untouched");
    assert!(sim.drones.scripts.objects.list.get(wire_obj).map_or(false, |o| o.is_some()), "the wire's entity is still there");
}

/// Sister B: ordinary gunfire does not cut the wire — the guns' category word 0x0008 [L
/// weapon_data 0x02/0x06] is DROPPED by `Collide_FilterBullets(list, 0xfffd)` [M 0x1ece30:24-31].
#[test]
fn gunfire_never_cuts_the_wire() {
    let iso = iso();
    let mut scene = load(&iso, "07000008.bin");
    for _ in 0..120 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    let (fb, wire_pos) = {
        let sim = scene.sim.as_ref().unwrap();
        let f = sim.drones.fuseboxes.iter().position(|f| f.ch[0] == 0x89).expect("the GuardRoom fusebox");
        let obj = sim.drones.fuseboxes[f].alive_wires().next().expect("the wire is registered").2;
        let pos = nf_game::drone::Drones::script_obj_pos(&sim.drones.scripts, &sim.data(), obj).unwrap();
        (f, pos)
    };
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.bond.pos = [wire_pos[0] + 1.8, wire_pos[1] - 1.0, wire_pos[2]];
    }
    let draw = Input { switch_to: Some(0x02), ..Default::default() };
    for _ in 0..40 { scene.sim.as_mut().unwrap().step_bond(draw, false, false, false); }
    let w = scene.sim.as_ref().unwrap().gun.weapon;
    eprintln!("[gun] weapon {w:#x}");
    assert!((0x02..=0x11).contains(&w), "a gun is selected (its kind word is 0x0008 [L])");
    let mut ch137 = false;
    for t in 0..120u32 {
        {
            let sim = scene.sim.as_mut().unwrap();
            let head = sim.bond.head();
            let dir = [wire_pos[0] - head[0], wire_pos[1] - head[1], wire_pos[2] - head[2]];
            let l = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt().max(1e-4);
            sim.bond.yaw = dir[0].atan2(dir[2]);
            sim.bond.pitch = (dir[1] / l).asin() / (std::f32::consts::FRAC_PI_2);
        }
        scene.sim.as_mut().unwrap().step_bond(Input::default(), true, false, false);
        if scene.sim.as_ref().unwrap().drones.channels.on(137) { ch137 = true; break; }
    }
    let sim = scene.sim.as_ref().unwrap();
    eprintln!("[gun] after 120 P2K ticks: ch137 {ch137} wire alive {}", sim.drones.fuseboxes[fb].alive_wires().count());
    assert!(!ch137 && sim.drones.fuseboxes[fb].alive_wires().next().is_some(),
        "gunfire never cuts a fusebox wire [L kind 0x0008 dropped by the 0xfffd filter]");
}

/// The 07000014 sabotage through the real cut: both generator-room boxes' wires (groups 0+1) burn
/// under the Laser → ch 0x87 + 0x88 → AND #391 raises 0x8C. Every box there carries param 2
/// non-zero [D placements 291/383/385/833/919], so the group-1 deaths never hurt [M 0x1adfe8:99-101].
#[test]
fn the_07000014_sabotage_cuts_through_the_real_laser() {
    let iso = iso();
    let mut scene = load(&iso, "07000014.bin");
    for _ in 0..120 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    {
        let sim = scene.sim.as_ref().unwrap();
        eprintln!("[sab] laser owned: {}", sim.drones.inventory.owned.get(0x4e).copied().unwrap_or(false));
    }
    for want in [0x87u32, 0x88] {
        let (fb, wires) = {
            let sim = scene.sim.as_ref().unwrap();
            let f = sim.drones.fuseboxes.iter().position(|f| f.ch[0] as u32 == want)
                .unwrap_or_else(|| panic!("the fusebox with group-0 channel {want:#x} exists [D]"));
            let fb = &sim.drones.fuseboxes[f];
            eprintln!("[sab] box {} ch {:?} wires {:?}", fb.placement, fb.ch,
                fb.alive_wires().map(|(g, k, o)| (g, k, o)).collect::<Vec<_>>());
            let list: Vec<(usize, usize, [f32; 3])> = fb.alive_wires()
                .filter_map(|(g, k, o)| nf_game::drone::Drones::script_obj_pos(&sim.drones.scripts, &sim.data(), o).map(|p| (g, k, p)))
                .collect();
            (f, list)
        };
        assert!(!wires.is_empty(), "box {fb}: the script registered wires (group 0 AND group 1 [L NF_SCRIPTLOG])");
        // Burn each wire: settle near the box, slide inside the Laser's range, aim through it,
        // hold fire.
        for &(g, k, wp) in &wires {
            {
                let sim = scene.sim.as_mut().unwrap();
                let bp = sim.drones.fuseboxes[fb].pos;
                let dist = take_burning_stance(sim, bp, wp);
                eprintln!("[sab] wire at {wp:?} stance dist {dist:.2}");
                assert!(dist < 3.6, "the stance reaches box {fb}'s wire (Laser range 4.0)");
            }
            let draw = Input { switch_to: Some(0x4e), ..Default::default() };
            for _ in 0..40 { scene.sim.as_mut().unwrap().step_bond(draw, false, false, false); }
            let mut dead = false;
            for _ in 0..200u32 {
                {
                    let sim = scene.sim.as_mut().unwrap();
                    let head = sim.bond.head();
                    let dir = [wp[0] - head[0], wp[1] - head[1], wp[2] - head[2]];
                    let l = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt().max(1e-4);
                    sim.bond.yaw = dir[0].atan2(dir[2]);
                    sim.bond.pitch = (dir[1] / l).asin() / (std::f32::consts::FRAC_PI_2);
                }
                scene.sim.as_mut().unwrap().step_bond(Input::default(), true, false, false);
                let sim = scene.sim.as_ref().unwrap();
                if sim.drones.fuseboxes[fb].groups[g].slots[k].is_none() { dead = true; break; }
            }
            eprintln!("[sab] box {fb} wire g{g}k{k} dead {dead} ch {want:#x} {}", scene.sim.as_ref().unwrap().drones.channels.on(want as u32));
            assert!(dead, "the laser burns box {fb}'s group-{g} wire");
        }
        assert!(scene.sim.as_ref().unwrap().drones.channels.on(want), "cutting box {fb} writes ch {want:#x} [M 0x1adfe8]");
    }
    for _ in 0..10 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    {
        let sim = scene.sim.as_ref().unwrap();
        for t in sim.drones.triggers.iter().filter(|t| t.out == 0x8c || t.list.contains(&0x87) || t.list.contains(&0x88)) {
            eprintln!("[sab] trigger kind {} out {:#x} list {:?} gone {}", t.kind, t.out, t.list, t.gone);
        }
        eprintln!("[sab] ch 0x87 {} 0x88 {} 0x8C {}", sim.drones.channels.on(0x87), sim.drones.channels.on(0x88), sim.drones.channels.on(0x8c));
    }
    let sim = scene.sim.as_ref().unwrap();
    assert!(sim.drones.channels.on(0x8c), "AND #391 [0x87, 0x88] raises 0x8C [D]");
    // The no-hurt polarity, precisely: the fusebox hurt is the port's `player_hits` marker
    // (attacker usize::MAX — `Player_Hurt(5.0, …)` at the dying group-1 wire [M 0x1adfe8:99-101]).
    // Every box here carries param 2 non-zero [D], so the group-1 deaths never hurt.
    assert!(!sim.drones.player_hits.iter().any(|(_, who, _)| *who == usize::MAX),
        "no live-wire hurt with +0x56 configured [M 0x1adfe8:99-101]");
}
