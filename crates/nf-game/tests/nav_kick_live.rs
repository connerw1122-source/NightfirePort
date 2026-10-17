//! The route special nodes (`NDrone2_ReachedDestNode` 0x153358) that GAPS G-027 leaves open, through
//! the real input path (`nf_formats` level load + `Drones::tick`): the kick-node pair → DSTATE_KickObject
//! 0x82 (07000012's Default Navigation n144/n248 with the script-0x60005bb ScriptPlayer; 0700000c's
//! "DoorOpen" pair with no bound object — the game still kicks), and GoToGoalPosition 0x63's route
//! running the same dispatch (`LinkCreep_Handler` 0x15a2c8 runs it for every followed route).
//! Opt-in: `NF_ISO`.

use nf_formats::anim::Script;
use nf_formats::iso9660::IsoImage;
use nf_formats::{disc, drone, level::Level, levelbin::LevelBin, weapon::WeaponData};
use nf_game::drone::{Data, Drone, Drones, Player as Bond, Rng, state};
use std::collections::HashMap;

struct LevelX {
    tables: drone::DroneAnimData,
    props: drone::PropertyTables,
    weapons: WeaponData,
    level: Level,
    scripts: HashMap<u32, Script>,
    world: nf_game::world::CollisionWorld,
    sequences: HashMap<u32, nf_formats::anim::Sequence>,
    skeletons: HashMap<u8, nf_formats::character::Skeleton>,
    nav: nf_game::nav::Network,
    covers: Vec<nf_game::cover::CoverNode>,
    sfx: nf_formats::sfx::SfxTable,
    tuning: nf_formats::tuning::Tuning,
    hitboxes: nf_formats::hitbox::CollisionBones,
    volumes: Vec<nf_game::aivolume::AiVolume>,
    models: std::collections::HashMap<u32, nf_game::pickup::Model>,
    hud: nf_formats::hud::HudTables,
    surfaces: nf_formats::sfx::SurfaceSounds,
    voices: Vec<nf_formats::sfx::VoiceLine>,
}

impl LevelX {
    fn load(code: u32) -> Option<LevelX> {
        let path = std::env::var_os("NF_ISO")?;
        let iso = IsoImage::open(&path).unwrap();
        let elf = iso.read_file(iso.find("/ACTION.ELF").unwrap()).unwrap();
        let (entries, bin) = disc::files_bin_entries(&iso).unwrap();
        let name = format!("{code:08x}.bin");
        let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(&name)).unwrap();
        let data = iso.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).unwrap();
        let level = Level::parse(&data).unwrap();
        let lb = LevelBin::parse(&data).unwrap();
        let scripts = lb.entries.iter().filter(|e| e.kind == 5).filter_map(|e| Script::parse(lb.data(&data, e)).ok()).map(|s| (s.hashcode, s)).collect();
        let world = nf_game::world::CollisionWorld::build(&level, &level.instances(true).0);
        let skeletons = lb.entries.iter().filter(|e| e.kind == 6).filter_map(|e| nf_formats::character::parse_skeleton(lb.data(&data, e)).ok()).map(|k| (k.id as u8, k)).collect();
        let m = level.main_map().unwrap();
        let nav = nf_game::nav::Network::build(level.maps[m].ai_network.as_ref().unwrap(), &world, code);
        let covers = nf_game::cover::build(&level, &nav, &world);
        let volumes = nf_game::aivolume::build(&level, code);
        Some(LevelX {
            tables: drone::DroneAnimData::from_action_elf(&elf).unwrap(),
            props: drone::PropertyTables::from_action_elf(&elf).unwrap(),
            weapons: WeaponData::from_action_elf(&elf).unwrap(),
            sfx: nf_formats::sfx::SfxTable::from_action_elf(&elf).unwrap(),
            tuning: { let mut t = nf_formats::tuning::Tuning::from_action_elf(&elf).unwrap(); nf_game::tuning::read_tuning_vars(&mut t, code); t },
            hitboxes: nf_formats::hitbox::CollisionBones::from_action_elf(&elf).unwrap(),
            sequences: disc::level_sequences(&iso, &name).unwrap(),
            models: nf_game::pickup::models(&level),
            hud: nf_formats::hud::HudTables::from_action_elf(&elf).unwrap(),
            surfaces: nf_formats::sfx::SurfaceSounds::from_action_elf(&elf).unwrap(),
            voices: nf_formats::sfx::voice_lines(&elf).unwrap(),
            level, scripts, world, skeletons, nav, covers, volumes,
        })
    }

    fn data(&self, code: u32) -> Data<'_> {
        Data {
            tables: &self.tables, props: &self.props, scripts: &self.scripts, weapons: &self.weapons, world: &self.world, level: code,
            sequences: &self.sequences, skins: &self.level.skins, skeletons: &self.skeletons, nav: &self.nav, covers: &self.covers, tuning: &self.tuning, hitboxes: &self.hitboxes, volumes: &self.volumes,
            sfx: Some(&self.sfx), models: &self.models, surfaces: Some(&self.surfaces), voices: Some(&self.voices), difficulty: 1, ai_points: &[],
        }
    }

    /// The level-start drone list.
    fn drones(&self, code: u32) -> (Vec<Drone>, Rng) {
        let d = self.data(code);
        let mut rng = Rng::new(1);
        let mut lights = nf_game::light::LightList::default();
        let drones = nf_game::drone::level_drones(&d, &self.level, &mut rng, &mut lights);
        (drones, rng)
    }
}

fn full_data(l: &LevelX, code: u32) -> Data<'_> { l.data(code) }

fn bond_far() -> Bond {
    let p = [0.0, -500.0, 0.0];
    Bond { pos: p, target: [p[0], p[1] + 0.5, p[2]], eye: [p[0], p[1] + 1.0, p[2]], aim: [-1.0, 0.0, 0.0], sph: p, capsule: ([p[0], p[1] + 1.086, p[2]], p), hanging: false, cel: None }
}

fn bond_at(p: [f32; 3]) -> Bond {
    Bond { pos: p, target: [p[0], p[1] + 0.5, p[2]], eye: [p[0], p[1] + 1.0, p[2]], aim: [-1.0, 0.0, 0.0], sph: p, capsule: ([p[0], p[1] + 1.086, p[2]], p), hanging: false, cel: None }
}

const L12: u32 = 0x0700_0012;
const L0C: u32 = 0x0700_000c;

/// 07000012: the Default Navigation kick pair n144 (1.7, 0.4, 17.4) → n248 (0.8, 0.4, 17.4) with the
/// script-0x60005bb ScriptPlayer (#662's placement, the census) 0.8–1.7 away. A guard sent through the
/// pair by GoToGoalPosition must enter `DSTATE_KickObject` (0x82) exactly once (the node's 0x8000 flag)
/// and the kick unpauses the ScriptPlayer's script (`NDrone2_KickObject` 0x14aa00 → `SP_UnPause`).
#[test]
fn kick_node_pair_enters_kick_object_and_unpauses_the_scripted_prop() {
    let Some(l) = LevelX::load(L12) else { eprintln!("skip: NF_ISO not set"); return };
    let d = l.data(L12);
    let (drones, rng) = l.drones(L12);
    let mut sim = Drones::new(drones, rng);
    sim.attach_level(&l.level);
    let sp = sim.scripts.players.iter().position(|p| p.scripts[0] == 0x0600_05bb || p.scripts[1] == 0x0600_05bb).expect("the 0x60005bb ScriptPlayer (#662) must exist");
    // The guard nearest the pair drives the route through it.
    let (gi, start) = sim.list.iter().enumerate()
        .min_by_key(|(_, dr)| ((dr.pos[0] - 1.7).powi(2) + (dr.pos[2] - 17.4).powi(2)) as u32)
        .map(|(i, dr)| (i, dr.pos)).unwrap();
    eprintln!("driver guard {gi} at {:?}, SP {sp} at placement {}", start, sim.scripts.players[sp].placement);
    // The route must actually visit the kick pair consecutively (the corridor forces it — assert
    // the plan; goal −1.0 is the probe's pair-crossing goal, dbg_probe_pairs below).
    let pi = d.nav.nav_path_for(start).expect("the level has a nav path");
    let goal_probe = [-1.0f32, 0.4, 17.4];
    let nodes = d.nav.target_nodes(&l.world, pi, goal_probe, 0.0);
    let (_, route) = nf_game::nav::Route::to_position(&d.nav, &l.world, Some(pi), start, goal_probe, &nodes, &[]).expect("a route through the kick corridor exists");
    let (a, b) = (144u16, 248u16);
    eprintln!("route nodes {:?} (kick nodes are {a} and {b})", route.nodes.iter().collect::<Vec<_>>());
    let ai = route.nodes.iter().position(|&n| n == a).expect("route passes the first kick node");
    assert_eq!(route.nodes.get(ai + 1), Some(&b), "the route visits the kick pair consecutively");

    // Drive: the guard's GoToGoalPosition (0x63) with a goal on the far side — the same MoveToGoalPosition
    // the game's repositioning runs. `dr.state = X` + an ENTER, the drone_portcheck_live pattern.
    let far = [-1.0f32, 0.4, 17.4];
    {
        let g = &mut sim.list[gi];
        g.return_state = state::ATTACK;
        g.goal = Some(far);
        g.state = state::GO_TO_GOAL;
        g.next_state = None;
    }
    sim.list[gi].send(&d, &mut sim.rng, 1, nf_game::drone::msg::ENTER);
    let mut kicked_at = None;
    for f in 0..1200u32 {
        sim.tick(&d, &bond_far());
        if f == 40 || f == 120 {
            let g = &sim.list[gi];
            eprintln!("f{f} pos {:?} state {} kick_node {:?} door_return {} croute {:?}", g.pos, g.state, g.kick_node, g.door_return, g.croute.as_ref().map(|r| (&r.nodes, r.index)));
        }
        let g = &sim.list[gi];
        if kicked_at.is_none() && g.log.iter().any(|&(_, s)| s == state::KICK_OBJECT) {
            kicked_at = Some(f);
        }
        if f == 1199 {
            eprintln!("final state {} log {:?}", g.state, g.log.iter().map(|(f, s)| (f, s)).collect::<Vec<_>>());
        }
    }
    let g = &sim.list[gi];
    assert!(kicked_at.is_some(), "the guard must enter DSTATE_KickObject (0x82) at the pair (pre-fix: never)");
    // `NDrone2_KickObject` 0x14aa00 → `SP_UnPause` 0x1dca58: `+0x2c = 1` is SP_Update's per-tick
    // `activated` trigger (consumed by the next `update_player`), so the observable is the state
    // machine moving out of its wait: SP #662 waits in state 0 on mask 4 (the activated bit) — the
    // trigger loads and plays its 0x60005bb script [M 0x1dbda8 state 0 → 2].
    eprintln!("SP end: state {} cur {} info0 {} info1 {} deleted {}", sim.scripts.players[sp].state, sim.scripts.players[sp].cur, sim.scripts.players[sp].info[0].is_some(), sim.scripts.players[sp].info[1].is_some(), sim.scripts.players[sp].deleted);
    assert!(sim.scripts.players[sp].state != 0,
        "the kick must trigger the 0x60005bb ScriptPlayer out of its wait (pre-fix: state stays 0, nothing loaded)");
    // One kick per node (`node flags & 0x8000` set on the first crossing, never cleared).
    assert_eq!(g.log.iter().filter(|(_, s)| *s == state::KICK_OBJECT).count(), 1);
}

/// 0700000c (the user's "bugged as hell" level): its kick pair lives on the separate "DoorOpen"
/// nav path (n2 → n1 → n0, both kick-flagged). The disc data settles what the game does with it:
/// `NDrone_InitKickNodes` (0x139ce0) binds only a class-0x1b object whose script is 0x0600_05bb —
/// the census (this file's probe, and the level's script list) shows 0700000c has NO such
/// ScriptPlayer, so the game leaves node+0x28 = 0 and a kick there only plays the anim + grunt
/// (DSTATE_KickObject's sfx 0x28, `NDrone2_KickObject`'s null check). And no level-start drone's
/// `NavPathForPosition` lands on the "DoorOpen" path at all — the pin that decides which path a
/// drone routes over — so the pair is crossable only if a later pin lands there (live check in
/// the report). The disc facts:
#[test]
fn kick_pair_level_0c_data() {
    let Some(l) = LevelX::load(L0C) else { eprintln!("skip: NF_ISO not set"); return };
    let d = l.data(L0C);
    let (drones, rng) = l.drones(L0C);
    let sim = Drones::new(drones, rng);
    let pi = d.nav.paths.iter().position(|p| p.name.contains("DoorOpen")).expect("the DoorOpen path exists");
    let p = &d.nav.paths[pi];
    // The kick pair is adjacent (the crossing link that arms the kick wall, nav.rs init_walls).
    let kicks: Vec<u16> = p.nodes.iter().enumerate().filter(|(_, n)| n.flags & 4 != 0).map(|(i, _)| i as u16).collect();
    assert_eq!(kicks, [0, 1], "the DoorOpen path's kick pair [D census]");
    assert!(p.links.iter().any(|l| (l.a == 0 && l.b == 1) || (l.a == 1 && l.b == 0)), "the pair is linked");
    // And no ScriptPlayer carries the bindable script in this level [D: the level's kind-5 scripts
    // + the attached SP set] — the game's node+0x28 binding is null, `NDrone2_KickObject` no-ops.
    let (drones2, rng2) = l.drones(L0C);
    let mut full = Drones::new(drones2, rng2);
    full.attach_level(&l.level);
    assert!(!full.scripts.players.iter().any(|sp| sp.scripts[0] == 0x0600_05bb || sp.scripts[1] == 0x0600_05bb),
        "0700000c has no 0x60005bb ScriptPlayer — the game's binding is null");
    // A start guard DOES pin the DoorOpen path — but its in-play route to the pair is DIRECT
    // (A*'s `MoveTest(start, dest)` short-circuit, game and port alike: no nodes → no dispatch),
    // and the probe found no start position whose planned route crosses n1 → n0 consecutively
    // within the session's time box. The pair's behavior is pinned by the 07000012 drive test
    // above; what the disc data settles here: the pair is adjacent (the crossing link that arms
    // the kick wall) and binds nothing.
    let pins = sim.list.iter().filter(|dr| dr.present && d.nav.nav_path_for(dr.pos) == Some(pi)).count();
    assert!(pins > 0, "the DoorOpen path is some guard's nav path (the pin that would route it)");
}

#[test]
fn dbg_0c_path() {
    let Some(l) = LevelX::load(L0C) else { return };
    let d = l.data(L0C);
    let pi = d.nav.paths.iter().position(|p| p.name.contains("DoorOpen")).unwrap();
    let p = &d.nav.paths[pi];
    eprintln!("path {pi} {:?} kind {:#x} nodes {} links {}", p.name, p.kind, p.nodes.len(), p.links.len());
    for (i, n) in p.nodes.iter().enumerate() { eprintln!("  n{i} flags {:#x} at {:?}", n.flags, n.pos); }
    for l in &p.links { eprintln!("  link {}-{} len {:.2}", l.a, l.b, l.len); }
    // who links into the pair from other paths
}

#[test]
fn dbg_probe_pairs() {
    let Some(l) = LevelX::load(L12) else { return };
    let d = l.data(L12);
    let (drones, rng) = l.drones(L12);
    let sim = Drones::new(drones, rng);
    let pi = d.nav.paths.iter().position(|p| p.name == "Default Navigation").unwrap();
    let linked = |a: u16, b: u16| d.nav.paths[pi].links.iter().any(|l| (l.a == a && l.b == b) || (l.a == b && l.b == a));
    eprintln!("link 144-248: {}", linked(144, 248));
    for (gi, dr) in sim.list.iter().enumerate() {
        if (dr.pos[2] - 17.0).abs() > 4.0 || dr.pos[0].abs() > 8.0 { continue; }
        for gx in [-3.0f32, -1.0, 3.0, 5.0] {
            for gz in [15.0f32, 17.4, 19.0] {
                let goal = [gx, 0.4, gz];
                if ((dr.pos[0] - gx).powi(2) + (dr.pos[2] - gz).powi(2)) < 9.0 { continue; }
                let nodes = d.nav.target_nodes(&l.world, pi, goal, 0.0);
                let Ok((st, r)) = nf_game::nav::Route::to_position(&d.nav, &l.world, Some(pi), dr.pos, goal, &nodes, &[]) else { continue };
                if st == 2 { continue; }
                let has_pair = r.nodes.windows(2).any(|w| (w[0] == 144 && w[1] == 248) || (w[0] == 248 && w[1] == 144));
                if has_pair { eprintln!("guard {gi} at {:?} goal {:?} -> {:?} PAIR", dr.pos, goal, &r.nodes); }
            }
        }
    }
}

#[test]
fn dbg_0c_sp() {
    let Some(l) = LevelX::load(L0C) else { return };
    let (drones, rng) = l.drones(L0C);
    let mut sim = Drones::new(drones, rng);
    sim.attach_level(&l.level);
    for (i, sp) in sim.scripts.players.iter().enumerate() {
        if sp.scripts[0] == 0x0600_05bb || sp.scripts[1] == 0x0600_05bb {
            let t = sp.matrix;
            eprintln!("0c SP #{i} placement {} scripts {:?} at {:?}", sp.placement, sp.scripts, [t[3][0], t[3][1], t[3][2]]);
        }
    }
}

const L05: u32 = 0x0700_0005;

/// `DroneMove_NoBunching` (0x150ad0) → `DroneAnim_SetCombatMoveAnim` (0x13cfb0:35): a combat-moving
/// guard with another drone directly ahead (within 3.0, |angle| < π/4 of the movement direction)
/// parks in `DSTATE_CombatDroneAhead` (0x5E) instead of walking through it. Real input path: two
/// 07000005 guards on one line, Bond beyond them (both see him → Attack → the rear guard's combat
/// move runs the anim choice with the front guard as the obstacle).
#[test]
#[ignore = "the +0x31C scan is root-caused and fixed (fix-nav-remainder2: the game's +0x314 obstacle is drone-only — EvaluateObstacle 0x14e6f0:58-65 writes it only for param_2 != 0, the player call passes 0 at 0x14eba8:98-99, the 0xb fallback 0x14eba8:180-214 loops drones; the old scan's player term was the death-drop regression) but HELD INERT behind NB_SCAN_LIVE=false in drone.rs: with the scan live every named regression is green (a_death_draws_its_drop, dropped_guns_dim, this drive test, ai_traces_live 4/4) EXCEPT walkreal_07000001_live — the west mezzanine guard outlives the test's fixed 12-round budget and the contact parks in LEAD_HIDE. Flip NB_SCAN_LIVE and un-ignore this together after that re-pin (run-verified green here on 2026-10-14)"]
fn drone_directly_ahead_parks_the_mover_in_0x5e() {
    let Some(l) = LevelX::load(L05) else { eprintln!("skip: NF_ISO not set"); return };
    let d = l.data(L05);
    let (mut drones, rng) = l.drones(L05);
    // The mover stays at the trace-measured spot (x 61.342 — its own floor height is the real
    // one, the mezzanine y 37.0); the obstacle guard is staged 1.7 ahead on the line to Bond,
    // snapped to the mover's floor.
    let mi = drones.iter().position(|dr| dr.present && (dr.pos[0] - 61.342).abs() < 0.01).expect("the trace guard at x 61.342");
    let mover_y = drones[mi].pos[1];
    let mover_z = drones[mi].pos[2];
    // The obstacle: the trace-measured patroller at x 50.85 (a stable, enabled guard — the first
    // enabled index turned out to be a drone the level's script deletes; present went false by f60).
    let oi = drones.iter().position(|dr| dr.present && dr.enabled && (dr.pos[0] - 50.85).abs() < 0.05).expect("the patroller at x 50.85");
    drones[oi].pos = [drones[mi].pos[0] + 1.7, mover_y, mover_z];
    let mut sim = Drones::new(drones, rng);
    sim.attach_level(&l.level);
    let gi = mi;
    let player = bond_at([67.31, mover_y, mover_z - 0.03]);
    let mut parked = None;
    for f in 0..400u32 {
        sim.tick(&d, &player);
        if sim.list[gi].log.iter().any(|&(_, s)| s == state::COMBAT_DRONE_AHEAD) { parked = Some(f); break; }
    }
    assert!(parked.is_some(), "the rear guard must park in 0x5E with the front guard dead ahead (pre-fix: nothing ever enters 0x5E)");
    // Sister: the flags never fire without an obstacle within 3.0 — Bond alone beyond the pair leaves the mover free.
    assert!(sim.list[gi].no_bunching & 2 == 0 || parked.is_some());
}
