//! movie-trigger-41: NIS events 0x08 (`Drone_CoderCreate` 0x13a108) and 0x0a (`Break_DoBreak`
//! 0x12c168) through the real script path of 07000001's intro script 0x0600005b.
//!
//! Evidence base:
//! - The script's shape [D, decoded from the archive]: 12 streams; stream 7 creates the ANIM
//!   object (skin 0x05000037, script 0x060000A8); stream 8 fires `Event(0x0a, [])` at tick 127 and
//!   creates no object; stream 9 fires `Event(8, [0x0500001b, 0x060000a6, 0x10, 1])` at tick 224.
//! - `Script_EventHandler` 0x1d94c0 (disasm): events take the position/object from the PREVIOUS
//!   stream record (base = SCRIPTINFO + byte[0xAB3]·0x28 + 0x80 = SSTREAM[i−1]; the object only
//!   when its record kind is 1/2 [M :75-96]). Case 8 (@0x1d96d4) and case 0xa (@0x1d96bc) skip the
//!   call when that object is NULL (`beq s5,zero`).
//! - So the disc's only event-8 firing (stream 9, previous record = stream 8, which creates no
//!   object) is a NO-OP in the game: the coder drone does not spawn from it [M gate + D records].
//! - The disc's only event-0xa firing (stream 8, previous record = stream 7's anim object) calls
//!   `Break_DoBreak` at the anim object's position [M 0x1d96c4-0x1d96c8]; `Break_DoBreak` 0x12c168
//!   scans the control list for the first class-0x20 object whose world sphere contains it — the
//!   port's script-entity store holds none of that class in this script and its placed 0x20 glass
//!   panels are not simulated, so the port's scan finds nothing (documented stand-in; see the
//!   drain's note).
//! - `Drone_CoderCreate` 0x13a108 (disasm): builds a throwaway level_tag (0xb0 zeroed) with slot 0
//!   = p0 (skin), slot 1 = p1 (start script), slot 9 = p3 (setup) [M 0x13a160-0x13a184]; param 2
//!   is dead; `Drone_Create` 0x136be0 → `NDrone2_CreateObj` (class 2, `LList_Add` 0x298E14 [M
//!   0x14d7c8:10-14,29-30]) → `NDrone2_PostLoad_Init` [M 0x14d890]; state = IDLE (the caller's
//!   hard-coded 7th argument 4, 0x1d96ec → 0x13a1a0).
//! Opt-in via NF_ISO.

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn intro_streams(scripts: &nf_game::nis::Scripts) -> Vec<(usize, nf_formats::nis::Cmd)> {
    scripts.lib.get(&0x0600_005b).expect("0600005b ships in 01's archive")
        .streams.iter().enumerate()
        .flat_map(|(si, s)| s.decode().into_iter().map(move |c| (si, c)))
        .collect()
}

/// The disc's single event-8 firing produces NO spawn — pinned end to end: the archive bytes show
/// stream 9's `Event(8, [0x0500001b, 0x060000a6, 0x10, 1])` with stream 8 (the previous record) as
/// an events-only stream [D], the game's record(i−1) gate yields a NULL object for such a stream
/// [M 0x1d94c0:75-96], and case 8 skips the call on NULL [M 0x1d96d4]. The port must mirror that:
/// running the real script leaves the drone list empty. Sister: the request the decode raises
/// carries `obj: None` — the same gate — so no spawn can ride it.
#[test]
fn the_discs_event8_firing_spawns_nothing_through_the_real_script() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000001.bin", false).expect("load 01");
    let sim = scene.sim.as_mut().unwrap();

    // The script's shape straight from the archive [D].
    let ev8: Vec<(usize, Vec<u32>)> = intro_streams(&sim.drones.scripts).into_iter().filter_map(|(si, c)| match c {
        nf_formats::nis::Cmd::Event { kind: 8, params } => Some((si, params.clone())), _ => None,
    }).collect();
    assert_eq!(ev8, vec![(9, vec![0x0500_001b, 0x0600_00a6, 0x10, 1])], "the disc's single event-8 firing [D]");
    let s8_creates = intro_streams(&sim.drones.scripts).iter().any(|(si, c)| *si == 8 && matches!(c,
        nf_formats::nis::Cmd::Entity { .. } | nf_formats::nis::Cmd::Anim { .. } | nf_formats::nis::Cmd::Camera { .. }));
    assert!(!s8_creates, "stream 8 (the previous record) creates no object [D] — the game's gate yields NULL [M 0x1d94c0:75-96]");

    // The real script, the game's way (the same route the intro plays it). 01 has placed drones
    // [D survey] — the pin is that the event adds none.
    let placed = sim.drones.list.len();
    let ident = [[1.0f32, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]];
    sim.drones.scripts.spawn_once(sim.drones.scripts.players.len(), 0x0600_005b, ident);
    for _ in 0..700 { sim.step_bond(Default::default(), false, false, false); }
    assert_eq!(sim.drones.list.len(), placed, "the coder drone does NOT spawn from the disc's firing [M 0x1d96d4 NULL skip]");
    // Sister: the decode still raised the request with the gate's None object (the port's mirror
    // of the game's NULL) — drained without a spawn.
    assert!(sim.drones.scripts.requests.is_empty(), "the tick's requests drained [M drain_script_requests]");
}

/// The spawn path itself, driven through the real drain: a coder-create request WITH an object
/// (the shape the game's gate would pass for an entity-bearing previous stream) spawns a class-2
/// drone with the event's param map — skin 0x0500001b (param 0), start script 0x060000a6 (param
/// 1), setup 1 (param 9 → the port's `drone_setup` selector) — in state IDLE, registered on the
/// list [M 0x13a108 + 0x14d7c8 + 0x1d96ec]. Sister: a request with `obj: None` (the disc's real
/// firing) spawns nothing.
#[test]
fn coder_create_with_an_object_spawns_the_param_map_drone() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000001.bin", false).expect("load 01");
    let sim = scene.sim.as_mut().unwrap();

    // A stand-in previous-stream object at a distinctive spot (the spawn lands there: the game
    // passes obj+0xC0/+0x50 [M 0x1d96dc-0x1d96e4]).
    sim.drones.scripts.objects.list.push(Some(nf_game::nis::Obj::Entity {
        hash: 0x0200_0001, kind: 1, params: Vec::new(),
        matrix: [[1.0, 0.0, 0.0, 5.0], [0.0, 1.0, 0.0, 6.0], [0.0, 0.0, 1.0, 7.0], [0.0, 0.0, 0.0, 1.0]],
        alpha: 1.0, src: (0x0600_005b, 0),
    }));
    let obj = sim.drones.scripts.objects.list.len() - 1;
    let before = sim.drones.list.len();
    sim.drones.scripts.requests.push(nf_game::nis::Request::CoderCreate {
        obj: Some(obj), skin: 0x0500_001b, script: 0x0600_00a6, setup: 1,
    });
    sim.step_bond(Default::default(), false, false, false);

    assert_eq!(sim.drones.list.len(), before + 1, "the coder drone spawned [M 0x13a108 → NDrone2_CreateObj]");
    let dr = &sim.drones.list[sim.drones.list.len() - 1];
    assert_eq!(dr.skin, 0x0500_001b, "param 0 = the skin [M 0x13a160 slot 0]");
    assert_eq!(dr.start_script, 0x0600_00a6, "param 1 = the start script [M 0x13a168 slot 1]");
    assert_eq!(dr.state, nf_game::drone::state::IDLE, "the hard-coded start state 4 [M 0x1d96ec → 0x13a1a0]");
    assert!(dr.present && dr.enabled, "registered live [M 0x14d7c8 LList_Add]");
    assert!((dr.pos[0] - 5.0).abs() < 1e-3 && (dr.pos[1] - 6.0).abs() < 1.0 && (dr.pos[2] - 7.0).abs() < 1e-3,
        "the spawn lands at the stream object [M 0x1d96e4 a0 = s5+0xC0]");
    assert!(sim.drones.llist.contains(&(sim.drones.list.len() - 1)), "on the update list [M 0x14d7c8:29-30]");

    // Sister: the disc's real shape — obj None — spawns nothing.
    let n = sim.drones.list.len();
    sim.drones.scripts.requests.push(nf_game::nis::Request::CoderCreate {
        obj: None, skin: 0x0500_001b, script: 0x0600_00a6, setup: 1,
    });
    sim.step_bond(Default::default(), false, false, false);
    assert_eq!(sim.drones.list.len(), n, "obj None → the game's NULL skip [M 0x1d96d4]");
}

/// The disc's single event-0xa firing runs `Break_DoBreak` at stream 7's anim object and finds no
/// class-0x20 object in the port's store (this script creates none; the placed glass panels are
/// not simulated) — nothing breaks. Sister: a class-0x20 script entity within reach IS killed and
/// takes the glass sound (params[0] = 2 [M 0x12b9b4-0x12b9f0]); one out of reach survives.
#[test]
fn the_discs_break_firing_finds_nothing_and_the_scan_kills_a_nearby_breakable() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000001.bin", false).expect("load 01");
    let sim = scene.sim.as_mut().unwrap();

    // The script's shape [D]: stream 7's anim object is the position source; stream 8 fires 0xa.
    let anims: Vec<usize> = intro_streams(&sim.drones.scripts).iter().filter_map(|(si, c)| match c {
        nf_formats::nis::Cmd::Anim { .. } if *si == 7 => Some(*si), _ => None,
    }).collect();
    assert_eq!(anims, vec![7], "stream 7 creates the anim object (skin 0x05000037) [D]");
    let breaks = intro_streams(&sim.drones.scripts).into_iter().filter(|(si, c)| *si == 8 && matches!(c, nf_formats::nis::Cmd::Event { kind: 0xa, .. })).count();
    assert_eq!(breaks, 1, "the disc's single event-0xa firing [D]");
    let ent20 = intro_streams(&sim.drones.scripts).iter().any(|(_, c)| matches!(c, nf_formats::nis::Cmd::Entity { obj_type: 0x20, .. }));
    assert!(!ent20, "the script creates no class-0x20 entity [D] — the scan's script-side half is empty");

    // The real script through the real path: the firing drains, the store is untouched.
    let before = sim.drones.scripts.objects.list.iter().filter(|o| o.is_some()).count();
    let ident = [[1.0f32, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]];
    sim.drones.scripts.spawn_once(sim.drones.scripts.players.len(), 0x0600_005b, ident);
    for _ in 0..700 { sim.step_bond(Default::default(), false, false, false); }
    let after = sim.drones.scripts.objects.list.iter().filter(|o| o.is_some()).count();
    assert_eq!(before, after, "nothing broke — no class-0x20 object in reach [D records + the drain's scan]");

    // Sisters through the same drain, with a crafted position-source entity (the drain resolves
    // `at` through `script_obj_pos` — an Entity's matrix translation [M obj+0xC0's role]):
    // Sister 1: a class-0x20 entity at the source position dies (the first-sphere-hit [M
    // 0x12c180-0x12c1cc] + Break_Kill's delete [M 0x12b998-0x12b9ac]).
    sim.drones.scripts.objects.list.push(Some(nf_game::nis::Obj::Entity {
        hash: 0x0200_0004, kind: 1, params: Vec::new(),
        matrix: [[1.0, 0.0, 0.0, 100.0], [0.0, 1.0, 0.0, 1.0], [0.0, 0.0, 1.0, 100.0], [0.0, 0.0, 0.0, 1.0]],
        alpha: 1.0, src: (0x0600_005b, 0),
    }));
    let src = sim.drones.scripts.objects.list.len() - 1;
    sim.drones.scripts.objects.list.push(Some(nf_game::nis::Obj::Entity {
        hash: 0x0200_0002, kind: 0x20, params: vec![2],
        matrix: [[1.0, 0.0, 0.0, 100.0], [0.0, 1.0, 0.0, 1.0], [0.0, 0.0, 1.0, 100.0], [0.0, 0.0, 0.0, 1.0]],
        alpha: 1.0, src: (0x0600_005b, 0),
    }));
    let glass = sim.drones.scripts.objects.list.len() - 1;
    sim.drones.scripts.requests.push(nf_game::nis::Request::DoBreak { at: Some(src) });
    sim.step_bond(Default::default(), false, false, false);
    assert!(sim.drones.scripts.objects.list[glass].is_none(), "the glass breakable died [M Break_Kill obj+0xFE |= 1]");

    // Sister 2: one out of reach survives [M 0x12c1b0-0x12c1cc the sphere gate].
    sim.drones.scripts.objects.list.push(Some(nf_game::nis::Obj::Entity {
        hash: 0x0200_0003, kind: 0x20, params: vec![2],
        matrix: [[1.0, 0.0, 0.0, 150.0], [0.0, 1.0, 0.0, 1.0], [0.0, 0.0, 1.0, 150.0], [0.0, 0.0, 0.0, 1.0]],
        alpha: 1.0, src: (0x0600_005b, 0),
    }));
    let far = sim.drones.scripts.objects.list.len() - 1;
    sim.drones.scripts.requests.push(nf_game::nis::Request::DoBreak { at: Some(src) });
    sim.step_bond(Default::default(), false, false, false);
    assert!(sim.drones.scripts.objects.list[far].is_some(), "the far breakable survives [M the sphere gate]");
}
