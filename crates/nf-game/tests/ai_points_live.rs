//! AI points (placements 0xE9, `Drone_AIPoint` 0x137550) on 07000007: the disc's 33 main-map
//! records (+ the map-1 one the census counts = the level's 34 [D-census]), the emitter build
//! (`AINetwork_EmitPath` at `Drone_PostLoad_Init` 0x136938:78-86), the `NDrone2_ClosestAIPoint`
//! (0x151b18) selection, and the scared-guest run through the real state machine
//! (`DSTATE_CivilianScared` 0x15d3a0 mode-7 arm → `HostageGoToGoalPosition` 0x64 0x165a68).
//! Opt-in: set `NF_ISO` to the disc image.

use nf_formats::anim::Script;
use nf_formats::iso9660::IsoImage;
use nf_formats::{disc, drone, level::Level, levelbin::LevelBin, weapon::WeaponData};
use nf_game::drone::{Data, Drone, Drones, Player, Rng};
use std::collections::HashMap;

const LEVEL: u32 = 0x0700_0007;

struct Level07 {
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
    ai_points: Vec<nf_game::aipoint::AiPoint>,
    sfx: nf_formats::sfx::SfxTable,
    tuning: nf_formats::tuning::Tuning,
    hitboxes: nf_formats::hitbox::CollisionBones,
    volumes: Vec<nf_game::aivolume::AiVolume>,
    models: HashMap<u32, nf_game::pickup::Model>,
    surfaces: nf_formats::sfx::SurfaceSounds,
    voices: Vec<nf_formats::sfx::VoiceLine>,
}

impl Level07 {
    fn load() -> Option<Level07> {
        let path = std::env::var_os("NF_ISO")?;
        let iso = IsoImage::open(&path).unwrap();
        let elf = iso.read_file(iso.find("/ACTION.ELF").unwrap()).unwrap();
        let (entries, bin) = disc::files_bin_entries(&iso).unwrap();
        let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case("07000007.bin")).unwrap();
        let data = iso.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).unwrap();
        let level = Level::parse(&data).unwrap();
        let lb = LevelBin::parse(&data).unwrap();
        let scripts = lb.entries.iter().filter(|e| e.kind == 5).filter_map(|e| Script::parse(lb.data(&data, e)).ok()).map(|s| (s.hashcode, s)).collect();
        let world = nf_game::world::CollisionWorld::build(&level, &level.instances(true).0);
        let skeletons = lb.entries.iter().filter(|e| e.kind == 6).filter_map(|e| nf_formats::character::parse_skeleton(lb.data(&data, e)).ok()).map(|k| (k.id as u8, k)).collect();
        let m = level.main_map().unwrap();
        let nav = nf_game::nav::Network::build(level.maps[m].ai_network.as_ref().unwrap(), &world, LEVEL);
        let covers = nf_game::cover::build(&level, &nav, &world);
        let ai_points = nf_game::aipoint::build(&level.maps[m].placements, &nav, &world);
        let volumes = nf_game::aivolume::build(&level, LEVEL);
        Some(Level07 {
            tables: drone::DroneAnimData::from_action_elf(&elf).unwrap(),
            props: drone::PropertyTables::from_action_elf(&elf).unwrap(),
            weapons: WeaponData::from_action_elf(&elf).unwrap(),
            sfx: nf_formats::sfx::SfxTable::from_action_elf(&elf).unwrap(),
            tuning: { let mut t = nf_formats::tuning::Tuning::from_action_elf(&elf).unwrap(); nf_game::tuning::read_tuning_vars(&mut t, LEVEL); t },
            hitboxes: nf_formats::hitbox::CollisionBones::from_action_elf(&elf).unwrap(),
            sequences: disc::level_sequences(&iso, "07000007.bin").unwrap(),
            models: nf_game::pickup::models(&level),
            surfaces: nf_formats::sfx::SurfaceSounds::from_action_elf(&elf).unwrap(),
            voices: nf_formats::sfx::voice_lines(&elf).unwrap(),
            level, scripts, world, skeletons, nav, covers, ai_points, volumes,
        })
    }

    fn data(&self) -> Data<'_> {
        Data { tables: &self.tables, props: &self.props, scripts: &self.scripts, weapons: &self.weapons, world: &self.world, level: LEVEL,
            sequences: &self.sequences, skins: &self.level.skins, skeletons: &self.skeletons, nav: &self.nav, covers: &self.covers, tuning: &self.tuning, hitboxes: &self.hitboxes, volumes: &self.volumes,
            sfx: Some(&self.sfx), models: &self.models, surfaces: Some(&self.surfaces), voices: Some(&self.voices), difficulty: 1,
            ai_points: &self.ai_points }
    }

    /// Every drone present at level start and the index of the first team-3 party guest (dtype 10).
    fn drones(&self) -> (Vec<Drone>, usize, Rng) {
        let d = self.data();
        let mut rng = Rng::new(1);
        let mut lights = nf_game::light::LightList::default();
        let list = nf_game::drone::level_drones(&d, &self.level, &mut rng, &mut lights);
        let guest = list.iter().position(|g| g.team == 3).expect("a team-3 party guest");
        (list, guest, rng)
    }

    fn sim(&self, drones: Vec<Drone>, rng: Rng) -> Drones { let mut s = Drones::new(drones, rng); s.attach_level_id(&self.level, LEVEL); s }
}

/// The disc's records: 33 on the main map (the level's 34th is the empty-params map-1 record the
/// census counts [D-census 07000007]), every one parameter 0 = 2 (the group), 1 = 1 (accuracy),
/// 2 = 0 (owner) [D 07000007.bin placements, `nf-extract placements 07000007.bin e9`].
#[test]
fn aipoint_records_07000007() {
    let Some(l) = Level07::load() else { eprintln!("skip: NF_ISO not set"); return };
    assert_eq!(l.ai_points.len(), 33, "the main map's 0xE9 placements");
    for (i, p) in l.ai_points.iter().enumerate() {
        assert_eq!((p.group, p.accuracy, p.owner), (2u16, 1.0f32, 0i32), "point {i}");
        // The emitter bound the navigation path for the point's position [M 0x15b1e8:36-42] and
        // Drone_PostLoad_Init emitted its per-node costs [M 0x136938:78-86].
        assert!(p.path.is_some(), "point {i} at {:?} found no nav path", p.pos);
        assert!(!p.dist.is_empty(), "point {i} has no distance bytes");
        // A point whose node search fails carries all-0xff bytes and is never eligible
        // [M 0x151b18:142]; 07000007 has one such isolated record.
        assert!(p.path.is_some() && p.dist.len() == l.nav.paths[p.path.unwrap()].nodes.len(), "point {i} dist covers its path");
    }
}

/// `NDrone2_ClosestAIPoint` (0x151b18): on 07000007 every guest-asked point (group 2) qualifies by
/// its Dijkstra byte at the drone's nearest node; the ask selects the least cost, first on ties,
/// and only among points on the drone's own nav path (Drone+0x954 [M 0x14d890:18-22]).
#[test]
fn closest_point_selection_on_07000007() {
    let Some(l) = Level07::load() else { eprintln!("skip: NF_ISO not set"); return };
    // A drone parked on a point asks from exactly that node: its own point costs 0 (g == the
    // source), so it must win over every other point on the path [M 0x151b18:152-177].
    let p0 = &l.ai_points[0];
    let path = p0.path.unwrap();
    let win = nf_game::aipoint::closest(&l.ai_points, &l.nav, &l.world, Some(path), p0.pos, 2, 0).unwrap();
    assert_eq!(win, 0, "the point the ask stands on (cost 0) beats the path's other points");
    // The group filter [M 0x151b18:129]: no group-8 (alarm) or group-0/1/9 records on this level.
    for group in [0u16, 1, 8, 9] {
        assert!(nf_game::aipoint::closest(&l.ai_points, &l.nav, &l.world, Some(path), p0.pos, group, 0).is_none(), "group {group} empty on 07000007");
    }
    // A drone whose nav path is not the point's path never sees it [M 0x151b18:130]: on a path no
    // point binds (or a pathless drone), the ask comes up empty.
    let bound: Vec<usize> = l.ai_points.iter().filter_map(|p| p.path).collect();
    let other = (0..l.nav.paths.len()).find(|&pi| l.nav.paths[pi].kind & 0xd == 0 && !bound.contains(&pi));
    if let Some(other) = other {
        let at = l.nav.paths[other].nodes[0].pos;
        assert!(nf_game::aipoint::closest(&l.ai_points, &l.nav, &l.world, Some(other), at, 2, 0).is_none(),
            "no group-2 point binds path {other}");
    }
    assert!(nf_game::aipoint::closest(&l.ai_points, &l.nav, &l.world, None, p0.pos, 2, 0).is_none(),
        "a pathless drone finds nothing [M 0x151b18:130 with +0x954 = 0]");
}

/// The scared guest through the real path: `DSTATE_CivilianScared` 0x15d3a0's mode-7 arm
/// (`+0xC6 == 7`, what `DroneFunc_HandleImpact` 0x1469b8:103-127 sets on 07000009/0A) runs
/// `FindRunToPoint` → the closest group-2 point → `HostageGoToGoalPosition` 0x64 (return state
/// 0x13) → arrives → `CivilianHiding` 0x13 [M 0x165a68:44-46]. Without the wiring the guest hides
/// at once (the old none-found arm).
#[test]
fn scared_guest_runs_to_the_closest_aipoint() {
    let Some(l) = Level07::load() else { eprintln!("skip: NF_ISO not set"); return };
    let (list, guest, rng) = l.drones();
    let mut sim = l.sim(list, rng);
    let d = l.data();
    // The level-start GLOBAL pass starts the guests in their civilian states.
    for _ in 0..10 { sim.tick(&d, &bond_at([-500.0, 0.0, 0.0])); }
    use nf_game::drone::state::*;
    // The guest's party state: the port starts mode-0 guests in Idle; seat the Civilian state the
    // SHOUT gate routes from (the guest's played state in the game's party flow) [S scaffolding —
    // the flow below is the real router].
    sim.list[guest].state = CIVILIAN;
    let g = &mut sim.list[guest];
    // The impact-set scared-run mode (the game sets +0xC6 = 7 on 09/0A [M-gated]; the marker the
    // 0x15d3a0 arm reads). Set directly — 07000007's impacts never set it [M gate].
    g.scared_run = true;
    g.send(&d, &mut sim.rng, 0, nf_game::drone::msg::SCREAM);
    assert_eq!(g.state, nf_game::drone::state::HOSTAGE_GO_TO_GOAL, "the mode-7 scared guest runs to the AI point [M 0x15d3a0:40-47]");
    let goal = g.goal.expect("the goal is the chosen point");
    // The chosen point is the registry's answer for this guest (group 2, its nav path).
    let path = g.nav_path.flatten();
    let want = nf_game::aipoint::closest(&l.ai_points, &l.nav, &l.world, path, g.pos, 2, 0).unwrap();
    assert_eq!(goal, l.ai_points[want].pos, "the goal is the closest point's position (+0x20) [M 0x153168:36-40]");
    // Run the machine until the guest arrives: HostageGoToGoalPosition returns to +0x5A2 (0x13).
    let mut arrived = false;
    for _ in 0..1200 {
        sim.tick(&d, &bond_at([-500.0, 0.0, 0.0]));
        if sim.list[guest].state == nf_game::drone::state::CIVILIAN_HIDING { arrived = true; break; }
        if sim.list[guest].state != nf_game::drone::state::HOSTAGE_GO_TO_GOAL { break; }
    }
    assert!(arrived, "the guest reaches the point and hides (0x64's arrival → +0x5A2 [M 0x165a68:44-46])");
}

/// The none-marker guest hides at once (the game's mode != 7 arm [M 0x15d3a0:49-55]) — the sister
/// pin: the registry never hijacks the plain scare.
#[test]
fn scared_guest_without_the_mode_hides_at_once() {
    let Some(l) = Level07::load() else { eprintln!("skip: NF_ISO not set"); return };
    let (list, guest, rng) = l.drones();
    let mut sim = l.sim(list, rng);
    let d = l.data();
    for _ in 0..10 { sim.tick(&d, &bond_at([-500.0, 0.0, 0.0])); }
    use nf_game::drone::state::*;
    sim.list[guest].state = CIVILIAN;
    let g = &mut sim.list[guest];
    g.send(&d, &mut sim.rng, 0, nf_game::drone::msg::SCREAM);
    assert_eq!(g.state, nf_game::drone::state::CIVILIAN_HIDING, "no scared-run mode → straight to hiding [M 0x15d3a0:53-55]");
}

fn bond_at(p: [f32; 3]) -> Player { Player { pos: p, target: [p[0], p[1] + 0.5, p[2]], eye: [p[0], p[1] + 1.0, p[2]], aim: [-1.0, 0.0, 0.0], sph: p, capsule: ([p[0], p[1] + 1.086, p[2]], p), hanging: false, cel: None } }

