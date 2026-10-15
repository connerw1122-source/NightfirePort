//! fix-casings-16 — 07000016's playtest symptoms through the real path (`nf_view::load` +
//! `Sim::step_bond`).
//!
//! 1. The walk continues: the arrival walkway's first door (glist 0x803f00a0 — kind 2 slide,
//!    straddle, ch_open 0, ch_lock 0, flags 0x3 → closed at create [M Door_Create 0x136100:
//!    state 4 only when doorinfo+0x4c|+0x50 carries bit 2]) opens on contact with NO use key,
//!    like every straddle door in the game — neither `Door_Update` (a ch_open==0 door skips all
//!    channel arms [M 0x1363c8]) nor `Door_ReactToHit` (0x135980 reverses only moving leaves)
//!    opens a closed idle one; the game's contact path (U-041) is stood in by the per-tick
//!    straddle walk-in with `Door_Update`'s own proximity metric
//!    `Vec_SqDist3D(door+0x80, actor+0x80) < (door+0x8C + actor+0x8C)² · 2` [M 0x1363c8
//!    @0x136678-0x136684]. The user's "mission complete thing pop up ... but I still couldn't
//!    continue": the banners at the arrival are the game's own ("OBJECTIVE COMPLETE" for the
//!    carried 07000015 objective — the spawn plate #252 writes ch 0x8e as Bond arrives [D
//!    docs/levels/07000016.md §2] — plus the enabled "Reach Drake's launch silo" announce), but
//!    the walk then dead-ended at this door.
//! 2. Walking backwards ejects nothing: `Bullet_init_casing` 0x12fde0 runs only from
//!    `Player_WeaponInitBullet` 0x1a4bf0:306 (the shot's release) and `DroneWeap_FireWeapon`
//!    0x178f68:264 (a guard's shot) — never from movement. The user's "casings flying when I
//!    walk backwards" is the eject pool's 240-tick life [M Bullet_casing_update 0x12faa0]:
//!    casings from their own shots are still airborne during the backpedal, not new spawns.

use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn alive_casings(scene: &nf_view::scene::Scene) -> usize {
    scene.sim.as_ref().unwrap().drones.casings.list.iter().filter(|c| c.state != 2).count()
}

/// Walking forward through the arrival door with no use key: the door opens on approach (the
/// straddle walk-in) and Bond passes z −75 (the door sits at z −74.17). Sister: the LOCKED door
/// down the walkway (glist 0x803f0010, ch_lock 0x28 — its channel never rises here) stays shut.
#[test]
fn the_arrival_door_opens_on_contact_and_the_walk_continues() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000016.bin", false).expect("load");
    {
        let sim = scene.sim.as_mut().unwrap();
        for _ in 0..4 { sim.step_bond(Input::default(), false, false, false); }
    }
    // Pre-fix state: the door closed, Bond free to walk at it.
    let (di, _) = scene.sim.as_ref().unwrap().drones.doors.list.iter().enumerate()
        .find(|(_, d)| d.glist == 0x803f_00a0).expect("the arrival door");
    assert_eq!(scene.sim.as_ref().unwrap().drones.doors.list[di].state, crate_door_closed(), "the door starts closed at create [M 0x136100 flags 0x3]");
    let mut opened = false;
    for t in 0..160 {
        let sim = scene.sim.as_mut().unwrap();
        sim.step_bond(Input { forward: 1.0, ..Default::default() }, false, false, false);
        let (z, st) = { let s = scene.sim.as_ref().unwrap(); (s.bond.pos[2], s.drones.doors.list[di].state) };
        if t % 30 == 0 { eprintln!("t{t}: bond z {:.2} door state {}", z, st); }
        if st & 0xc != 0 { opened = true; break; }
    }
    assert!(opened, "the arrival door must open on contact with NO use key (the straddle walk-in)");
    // The leaf completes its travel: state OPEN and its origin above the doorway (closed y −0.10).
    let mut rose = false;
    for _ in 0..180 {
        scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
        let (st, ty) = { let d = &scene.sim.as_ref().unwrap().drones.doors.list[di]; (d.state, d.matrix[3][1]) };
        if st == nf_game::door::OPEN && ty > 2.0 { rose = true; break; }
    }
    let (st, ty) = { let d = &scene.sim.as_ref().unwrap().drones.doors.list[di]; (d.state, d.matrix[3][1]) };
    assert!(rose, "the leaf completed its open travel (state {st}, trans y {ty:.2})");
    // Sister: the locked door (ch_lock 0x28, its channel off) does NOT open for the same walk-in.
    let s = scene.sim.as_ref().unwrap();
    let (li, _) = s.drones.doors.list.iter().enumerate()
        .find(|(_, d)| d.glist == 0x803f_0010).expect("the locked walkway door");
    assert!(s.drones.doors.list[li].state & 0xc == 0, "a ch_lock-gated door stays shut while its channel is off");
}

fn crate_door_closed() -> u16 { nf_game::door::CLOSED }

/// Walking backwards with no fire ejects nothing (the casings rule), including after an earlier
/// burst — the still-airborne shells are the pool's own 240-tick life, not new spawns.
#[test]
fn walking_backwards_never_ejects() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000016.bin", false).expect("load");
    {
        let sim = scene.sim.as_mut().unwrap();
        give_p2k(sim);
        for _ in 0..4 { sim.step_bond(Input::default(), false, false, false); }
    }
    let c0 = alive_casings(&scene);
    for _ in 0..180 {
        let sim = scene.sim.as_mut().unwrap();
        sim.step_bond(Input { forward: -1.0, ..Default::default() }, false, false, false);
        let s = scene.sim.as_ref().unwrap();
        assert!(s.drones.player_bullets.is_empty(), "a bullet spawned while walking backwards");
        assert_eq!(alive_casings(&scene), c0, "a casing ejected while walking backwards (Bullet_init_casing runs only on the shot's release [M 0x1a4bf0:306])");
    }
    // Sister: firing THEN backpedalling — the pool holds the burst's shells (one per shot, the
    // weapon's +0x78 delay apart) and no more; the count never grows during the walk itself.
    let mut fired = 0;
    for t in 0..12 {
        let had = alive_casings(&scene);
        let sim = scene.sim.as_mut().unwrap();
        sim.step_bond(Input::default(), true, false, false);
        let now = alive_casings(&scene);
        if now > had { fired += 1; }
    }
    let after_burst = alive_casings(&scene);
    assert!(fired >= 1, "the burst ejected (the test's premise)");
    for _ in 0..180 {
        let sim = scene.sim.as_mut().unwrap();
        sim.step_bond(Input { forward: -1.0, ..Default::default() }, false, false, false);
        assert_eq!(alive_casings(&scene), after_burst, "the backpedal spawned a casing beyond the burst's own shells");
    }
}

/// 109r (fix-start-loadouts): a cold load now gets the level's own kit [M `Player_InitWeapon` 0x1a7ef8], not
/// the old default P2K. This test is about a P2K shot, so it gives and draws the P2K itself — the `--draw`
/// path's equip, then the switch request (`M_CONTROL+0x63`, the draw machine holsters and draws it).
fn give_p2k(sim: &mut nf_view::scene::Sim) {
    let wd = sim.weapons.clone();
    let rounds = wd.clip_size(2).max(1);
    if sim.drones.inventory.equip_weapon(&wd, 2, rounds, true) { sim.drones.inventory.equip_ammo(&wd, 2, rounds * 3); }
    sim.gun.switch_request = 2;
    for _ in 0..90 { sim.step_bond(nf_game::player::Input::default(), false, false, false); }
}
