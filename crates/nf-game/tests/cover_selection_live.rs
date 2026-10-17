//! G-028 (fix-search-cover-g028): the cover-node selection and the crouch/stand choice, pinned against the
//! game's rules. The live behaviour itself is matched by `ai_live.rs::guard_takes_cover_like_the_game`
//! (work/chase_trace.tsv: the gate guard ends behind corner node 1); here the SELECTION RULE and the
//! stance flags are pinned so a scoring or flag regression turns the tests red.
//!
//! Rules under test [M]:
//! - `NDrone2_FindCover(needLos)` 0x152548:129-157 — over unoccupied nodes on the drone's nav network with a
//!   distance byte ≠ 0xFF: the smallest byte, strictly less (first found wins ties), and only when
//!   byte² ≤ (distance to the opponent)²; `CoverNodeOK(node, 0, needLos)` must pass.
//! - `NDrone2_CoverNodeOK` 0x151d68 — the side bits, the 2.0 / 1.5×combat-range distance gates and the anim
//!   predicates (crouch lean / crouch step / stand lean / stand step / as-low; crouch forced when the corner
//!   lacks flag 8 stand).
//! - The corner flags [M drone-cover.md +0xC]: 4 = crouch, 8 = stand — the node decides the stance family.

use nf_formats::anim::Script;
use nf_formats::iso9660::IsoImage;
use nf_formats::drone::{DroneAnimData, PropertyTables};
use nf_formats::{disc, level::Level, levelbin::LevelBin, weapon::WeaponData};
use nf_game::drone::{Data, Drone, Drones, Player, Rng};

use std::collections::HashMap;

struct Level05 {
    props: PropertyTables,
    tables: DroneAnimData,
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
    surfaces: nf_formats::sfx::SurfaceSounds,
    voices: Vec<nf_formats::sfx::VoiceLine>,
}

impl Level05 {
    fn load() -> Option<Level05> {
        let path = std::env::var_os("NF_ISO")?;
        let iso = IsoImage::open(&path).unwrap();
        let elf = iso.read_file(iso.find("/ACTION.ELF").unwrap()).unwrap();
        let (entries, bin) = disc::files_bin_entries(&iso).unwrap();
        let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case("07000005.bin")).unwrap();
        let data = iso.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).unwrap();
        let level = Level::parse(&data).unwrap();
        let lb = LevelBin::parse(&data).unwrap();
        let scripts = lb.entries.iter().filter(|e| e.kind == 5).filter_map(|e| Script::parse(lb.data(&data, e)).ok()).map(|s| (s.hashcode, s)).collect();
        let world = nf_game::world::CollisionWorld::build(&level, &level.instances(true).0);
        let skeletons = lb.entries.iter().filter(|e| e.kind == 6).filter_map(|e| nf_formats::character::parse_skeleton(lb.data(&data, e)).ok()).map(|k| (k.id as u8, k)).collect();
        let m = level.main_map().unwrap();
        let nav = nf_game::nav::Network::build(level.maps[m].ai_network.as_ref().unwrap(), &world, 0x0700_0005);
        let covers = nf_game::cover::build(&level, &nav, &world);
        let volumes = nf_game::aivolume::build(&level, 0x0700_0005);
        Some(Level05 {
            props: PropertyTables::from_action_elf(&elf).unwrap(),
            tables: DroneAnimData::from_action_elf(&elf).unwrap(),
            weapons: WeaponData::from_action_elf(&elf).unwrap(),
            sfx: nf_formats::sfx::SfxTable::from_action_elf(&elf).unwrap(),
            tuning: { let mut t = nf_formats::tuning::Tuning::from_action_elf(&elf).unwrap(); nf_game::tuning::read_tuning_vars(&mut t, 0x0700_0005); t },
            hitboxes: nf_formats::hitbox::CollisionBones::from_action_elf(&elf).unwrap(),
            sequences: disc::level_sequences(&iso, "07000005.bin").unwrap(),
            models: nf_game::pickup::models(&level),
            surfaces: nf_formats::sfx::SurfaceSounds::from_action_elf(&elf).unwrap(),
            voices: nf_formats::sfx::voice_lines(&elf).unwrap(),
            level, scripts, world, skeletons, nav, covers, volumes,
        })
    }

    fn data(&self) -> Data<'_> {
        Data { tables: &self.tables, props: &self.props, scripts: &self.scripts, weapons: &self.weapons, world: &self.world, level: 0x0700_0005,
            sequences: &self.sequences, skins: &self.level.skins, skeletons: &self.skeletons, nav: &self.nav, covers: &self.covers, tuning: &self.tuning, hitboxes: &self.hitboxes, volumes: &self.volumes,
            sfx: Some(&self.sfx), models: &self.models, surfaces: Some(&self.surfaces), voices: Some(&self.voices), difficulty: 1,
                ai_points: &[],
            }
    }

    fn drones_seeded(&self, alert: Option<f32>, x: f32, seed: u32) -> (Vec<Drone>, usize, Rng) {
        let seed = std::env::var("NF_TEST_SEED").ok().and_then(|v| v.parse().ok()).unwrap_or(seed);
        let d = self.data();
        let mut rng = Rng::new(seed);
        let mut lights = nf_game::light::LightList::default();
        let mut drones = nf_game::drone::level_drones(&d, &self.level, &mut rng, &mut lights);
        if let Some(a) = alert { for dr in &mut drones { dr.alert = a; dr.alert_floor = a; } }
        let found = drones.iter().position(|dr| (dr.pos[0] - x).abs() < 0.01).unwrap();
        (drones, found, rng)
    }

    fn sim(&self, drones: Vec<Drone>, rng: Rng) -> Drones {
        let mut s = Drones::new(drones, rng); s.attach_level_id(&self.level, 0x0700_0005); s.difficulty = 1; s
    }
}

fn bond_at(p: [f32; 3]) -> Player { Player { pos: p, target: [p[0], p[1] + 0.5, p[2]], eye: [p[0], p[1] + 1.0, p[2]], aim: [-1.0, 0.0, 0.0], sph: p, capsule: ([p[0], p[1] + 1.086, p[2]], p), hanging: false, cel: None } }

/// `NDrone2_FindCover` 0x152548:129-157 replicated over the level's nodes: the unoccupied node on the drone's
/// network whose distance byte from his nav node is the smallest (strictly less, first wins ties) subject to
/// byte² ≤ (opponent distance)² and `CoverNodeOK(node, 0, needLos)`. `CoverNodeOK` 0x151d68's side/LOS gates
/// are applied through `CoverNode::usable` (bits & 0xE for a side, bits & 0x70 for needLos — the target must
/// see the spot) and the 2.0 minimum node-to-opponent distance; the anim-predicate and crowd gates are
/// all-pass on this level's all-flag corner nodes with the SG5 guard's props (all four types, prop 0x44
/// absent, no other guard within 1.5 of the traced pick).
fn expected_node(l: &Level05, guard: [f32; 3], bond: [f32; 3]) -> Option<(usize, u8)> {
    let path = l.nav.nav_path_for(guard)?;
    let from = l.nav.node_from(&l.world, path, guard)? as usize;
    let opp_d2 = (guard[0] - bond[0]).powi(2) + (guard[1] - bond[1]).powi(2) + (guard[2] - bond[2]).powi(2);
    let head = [bond[0], bond[1] + 1.0, bond[2]];
    let mut best: Option<(u8, usize)> = None;
    for (i, n) in l.covers.iter().enumerate() {
        if n.path != Some(path) { continue; }
        let dd = match n.dist.get(from) { Some(&b) if b != 0xff => b, _ => continue };
        if best.is_some_and(|(b, _)| dd >= b) { continue; }
        if (dd as f32) * (dd as f32) > opp_d2 { continue; }
        // `CoverNodeOK` 0x151d68: a side usable from Bond's position, LOS bits with needLos, ≥ 2.0 away.
        let node_d2 = (n.pos[0] - bond[0]).powi(2) + (n.pos[1] - bond[1]).powi(2) + (n.pos[2] - bond[2]).powi(2);
        if node_d2.sqrt() < 2.0 { continue; }
        let mut rt = nf_game::cover::CoverRt::default();
        n.usable(&mut rt, &l.world, &Default::default(), 0x0700_0005, bond, head);
        if rt.bits & 0xe == 0 || rt.bits & 0x70 == 0 { continue; }
        best = Some((dd, i));
    }
    best.map(|(dd, i)| (i, dd))
}

/// The trace scenario (work/chase_trace.tsv via ai_live.rs): the gate guard at x≈61.34 runs for cover when Bond
/// appears at (79.86, 22.92, 96.0) and the game picked corner node 1. The selection rule [M 0x152548], applied
/// to the level's real nodes from the guard's spawn (he does not move before `RunForCover` — Combat → OutOfRange
/// → RunForCover in 2 traced ticks), must reproduce exactly that node, and the chosen node must satisfy the
/// opponent-distance gate (byte² ≤ opp-dist²) — a cover farther than the opponent is never chosen.
#[test]
fn cover_choice_follows_the_game_scoring() {
    let Some(l) = Level05::load() else { eprintln!("skip: NF_ISO not set"); return };
    let d = l.data();
    let (drones, gi, rng) = l.drones_seeded(None, 61.342, 1);
    let spawn = drones[gi].pos;
    let mut sim = l.sim(drones, rng);
    let player = bond_at([79.86, 22.92, 96.0]);
    for _ in 0..700 { sim.tick(&d, &player); }
    let g = &sim.list[gi];
    assert_eq!(g.cover, Some(1), "the game's node for this trace (chase_trace.tsv)");
    // The replicated rule agrees with the game's pick on the real node data.
    let path = l.nav.nav_path_for(spawn).expect("the guard stands on a nav network");
    let from = l.nav.node_from(&l.world, path, spawn).expect("nearest node") as usize;
    let (want, dd) = expected_node(&l, spawn, player.pos).expect("a candidate node exists");
    assert_eq!(want, 1, "the scoring rule picks node {want} (path byte {dd}), the trace's node");
    // Sister: the chosen node must satisfy the opponent-distance gate (byte² ≤ opp-dist²), and the network
    // must carry more than one candidate so the ordering (and the gate) are load-bearing, not vacuous.
    let opp_d2 = (spawn[0] - player.pos[0]).powi(2) + (spawn[1] - player.pos[1]).powi(2) + (spawn[2] - player.pos[2]).powi(2);
    assert!((dd as f32) * (dd as f32) <= opp_d2, "winner within the opponent distance");
    let candidates = l.covers.iter().filter(|n| n.path == Some(path) && n.dist.get(from).is_some_and(|&b| b != 0xff)).count();
    assert!(candidates > 1, "{candidates} candidates on the network");
    eprintln!("cover scoring: {candidates} candidates, gated picks node {want} (byte {dd}, opp {:#.1})", opp_d2.sqrt());
}

/// The node flags decide the stance [M +0xC: 4 crouch, 8 stand; `CoverNodeOK` 0x151d68 forces crouch when the
/// corner lacks 8]. Same trace scenario, same node 1: re-flagged crouch-only the guard must take a crouch cover
/// type (crouch lean 0 / crouch step 1 / as-low 4) and never a stand type; re-flagged stand-only the mirror.
/// The guard's playing anim must follow (stand idle 0x1A / lean-outs 0x17/0x18/0x1B/0x1C vs the crouch family).
#[test]
fn cover_stance_follows_the_node_flags() {
    let Some(mut l) = Level05::load() else { eprintln!("skip: NF_ISO not set"); return };
    // Crouch-only node 1: clear stand (8), keep crouch (4) and the side/step-out/lean bits.
    l.covers[1].flags = (l.covers[1].flags & !0xc) | 4;
    let d = l.data();
    let (drones, gi, rng) = l.drones_seeded(None, 61.342, 1);
    let mut sim = l.sim(drones, rng);
    let player = bond_at([79.86, 22.92, 96.0]);
    for _ in 0..700 { sim.tick(&d, &player); }
    let g = &sim.list[gi];
    assert_eq!(g.cover, Some(1), "the same node is the scoring pick regardless of the stance flags");
    assert!(matches!(g.cover_type, 0 | 1 | 4), "crouch cover type, got {} ({:x?})", g.cover_type, g.log);
    assert!(!matches!(g.anim.state, 0x17 | 0x18 | 0x1a | 0x1b | 0x1c), "not standing (anim {:#x})", g.anim.state);

    // Sister: stand-only — clear crouch (4), keep stand (8).
    let mut l2 = Level05::load().unwrap();
    l2.covers[1].flags = (l2.covers[1].flags & !0xc) | 8;
    let d2 = l2.data();
    let (drones, gi, rng) = l2.drones_seeded(None, 61.342, 1);
    let mut sim2 = l2.sim(drones, rng);
    for _ in 0..700 { sim2.tick(&d2, &player); }
    let g2 = &sim2.list[gi];
    assert_eq!(g2.cover, Some(1), "sister: the same node");
    assert!(matches!(g2.cover_type, 2 | 3), "stand cover type, got {} ({:x?})", g2.cover_type, g2.log);
    assert!(!matches!(g2.anim.state, 0x10 | 0x11 | 0x13 | 0x15), "sister: not crouching (anim {:#x})", g2.anim.state);
}
