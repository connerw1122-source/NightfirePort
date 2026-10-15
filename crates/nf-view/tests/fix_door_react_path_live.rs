//! fix-door-react-path (G-035): the door anti-crush through the game's collide path.
//!
//! The game runs `Door_ReactToHit` 0x135980 as the class-0x1C collide function from
//! `Collide_Update` 0x1ea398, off the `obj+0xD0` contact list that `Collide_Update`'s forward
//! builds from each picker's `Collide_PickObj` 0x1eae80 hit (the player's pick: mode 0x800 —
//! the pick capsule vs the door's live sphere ×1.5 [M Player_Collision 0x194df0 writes
//! BLData+0x7D4 = 0x800]). A kind-2 door CLOSING on a contact reverses to OPENING [M
//! 0x135980:101-108]; with nobody in the doorway it closes [M the kind-2 arm returns on
//! `state & 0x10 == 0`].
//!
//! These tests drive the REAL sim path (`nf_view::load` + `Sim::step_bond`) on 07000001 —
//! its three kind-2 doors auto-close (flags 0x02/0x03, no lock channel [D 07000001.bin
//! placements 13/14/69, docs/atlas/door.md's disc table]).
//!
//! Opt-in via `NF_ISO`.

use nf_view::dev;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn load(iso: &str, level: &str) -> nf_view::scene::Scene {
    nf_view::load(iso, level, false).expect("load")
}

fn tick(scene: &mut nf_view::scene::Scene, n: u32) {
    if let Some(sim) = scene.sim.as_mut() {
        for _ in 0..n { sim.step_bond(Default::default(), false, false, false); }
    }
}

fn step_at(scene: &mut nf_view::scene::Scene, pos: [f32; 3], yaw: f32, use_key: bool) {
    if let Some(sim) = scene.sim.as_mut() {
        sim.bond.pos = pos;
        sim.bond.yaw = yaw;
        sim.bond.pitch = 0.0;
        sim.step_bond(Default::default(), false, false, use_key);
    }
}

/// A door closing on Bond reverses (the anti-crush): E opens the kind-2 door, Bond stays in the
/// doorway, the second E starts the close — the game's rule re-opens it (state 8) instead of
/// closing through him. Sister (deliberately reversed expectation, the same flow with Bond walked
/// away): a door with nobody in it finishes the close (state 2).
#[test]
fn a_closing_door_reverses_on_bond_on_07000001() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso, "07000001.bin");
    let door_idx = scene.sim.as_ref().unwrap().drones.doors.list.iter().enumerate()
        .find(|(_, d)| d.kind == 2)
        .map(|(i, _)| i)
        .expect("a kind-2 slide door exists on 07000001");
    let door_pos = scene.sim.as_ref().unwrap().drones.doors.list[door_idx].pos;
    eprintln!("[react] slide door #{} at {:?}", door_idx, door_pos);
    let at_door = [door_pos[0], door_pos[1], door_pos[2] - 0.5];
    // E opens; the door travels to OPEN.
    step_at(&mut scene, at_door, 0.0, true);
    tick(&mut scene, 120);
    assert_eq!(scene.sim.as_ref().unwrap().drones.doors.list[door_idx].state, 4, "open after E");
    // Second E closes — with Bond still in the doorway. The anti-crush must reverse it.
    step_at(&mut scene, at_door, 0.0, true);
    let mut saw_opening = false;
    let mut saw_closed = false;
    let mut states: Vec<u16> = Vec::new();
    for _ in 0..45 {
        tick(&mut scene, 1);
        let s = scene.sim.as_ref().unwrap().drones.doors.list[door_idx].state;
        states.push(s);
        if s & 8 != 0 { saw_opening = true; }
        if s == 2 { saw_closed = true; }
    }
    eprintln!("[react] states at Bond's feet: {:04x?}", states);
    assert!(saw_opening, "the closing door reversed to OPENING with Bond in the doorway (the game's 0x135980 kind-2 rule)");
    assert!(!saw_closed, "the door never finished closing through Bond");
    // Sister: the same close with Bond walked away completes (state 2 — nobody in it). A level
    // guard standing in the doorway keeps it open through the SAME rule — accepted, but only with
    // a live drone actually inside the door's pick zone ((1.5·r + 0.4) of the live sphere centre).
    let mut scene2 = load(&iso, "07000001.bin");
    let door_pos2 = scene2.sim.as_ref().unwrap().drones.doors.list[door_idx].pos;
    step_at(&mut scene2, [door_pos2[0], door_pos2[1], door_pos2[2] - 0.5], 0.0, true);
    tick(&mut scene2, 120);
    assert_eq!(scene2.sim.as_ref().unwrap().drones.doors.list[door_idx].state, 4, "open after E");
    step_at(&mut scene2, [door_pos2[0], door_pos2[1], door_pos2[2] - 0.5], 0.0, true);
    // Walk Bond 12 m off before the leaf travels: nobody of OURS in the doorway.
    step_at(&mut scene2, [door_pos2[0] + 12.0, door_pos2[1], door_pos2[2]], 0.0, false);
    let mut closed = false;
    let mut guard_in_door = false;
    for _ in 0..90 {
        tick(&mut scene2, 1);
        let sim = scene2.sim.as_ref().unwrap();
        if sim.drones.doors.list[door_idx].state == 2 { closed = true; break; }
        let d = &sim.drones.doors.list[door_idx];
        let c = { let m = &d.matrix; [d.sph_centre[0] * m[0][0] + d.sph_centre[1] * m[1][0] + d.sph_centre[2] * m[2][0] + m[3][0], d.sph_centre[0] * m[0][1] + d.sph_centre[1] * m[1][1] + d.sph_centre[2] * m[2][1] + m[3][1], d.sph_centre[0] * m[0][2] + d.sph_centre[1] * m[1][2] + d.sph_centre[2] * m[2][2] + m[3][2]] };
        let r = d.sph.1 * 1.5 + 0.4;
        guard_in_door = sim.drones.list.iter().any(|x| x.present && !x.corpse && {
            let dx = x.pos[0] - c[0]; let dy = x.pos[1] - c[1]; let dz = x.pos[2] - c[2];
            dx * dx + dy * dy + dz * dz < r * r
        });
    }
    assert!(closed || guard_in_door,
        "a door with nobody in it closes; if it stayed open, a live guard must be standing in its pick zone (the same anti-crush rule)");
    eprintln!("[react] sister: closed={} guard_in_door={} ✓", closed, guard_in_door);
}
