//! fix-progress: the difficulty factor measured LIVE — a guard's bullet damage to Bond through the
//! real AI path (`Drones::tick`), at the default difficulty (2 = Agent) and at Operative (1) [D difficulty 0x2df4c0 +0xc], against
//! `Player_HandlePain` 0x1902c8's per-difficulty `Plr_DMod_*` ([L] 07000005: 0.6 / 0.7 / 1.0,
//! ReadTuningVars 0x1d3a80). The harness is ai_live.rs's Level05, minimised.

use nf_formats::anim::Script;
use nf_formats::iso9660::IsoImage;
use nf_formats::{disc, drone, level::Level, levelbin::LevelBin, weapon::WeaponData};
use nf_game::drone::{Data, Drone, Drones, Player, Rng};
use std::collections::HashMap;

struct Level05 {
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
            tables: drone::DroneAnimData::from_action_elf(&elf).unwrap(),
            props: drone::PropertyTables::from_action_elf(&elf).unwrap(),
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

    fn sim(&self, drones: Vec<Drone>, rng: Rng) -> Drones { let mut s = Drones::new(drones, rng); s.attach_level(&self.level); s }

    fn data(&self, difficulty: u32) -> Data<'_> {
        Data { tables: &self.tables, props: &self.props, scripts: &self.scripts, weapons: &self.weapons, world: &self.world, level: 0x0700_0005,
            sequences: &self.sequences, skins: &self.level.skins, skeletons: &self.skeletons, nav: &self.nav, covers: &self.covers, tuning: &self.tuning, hitboxes: &self.hitboxes, volumes: &self.volumes,
            sfx: Some(&self.sfx), models: &self.models, surfaces: Some(&self.surfaces), voices: Some(&self.voices), difficulty }
    }

    fn drones_seeded(&self, seed: u32) -> (Vec<Drone>, usize, Rng) {
        let d = self.data(1);
        let mut rng = Rng::new(seed);
        let mut lights = nf_game::light::LightList::default();
        let drones = nf_game::drone::level_drones(&d, &self.level, &mut rng, &mut lights);
        let found = drones.iter().position(|dr| (dr.pos[0] - 61.342).abs() < 0.01).unwrap();
        (drones, found, rng)
    }
}

fn bond_at(p: [f32; 3]) -> Player { Player { pos: p, target: [p[0], p[1] + 0.5, p[2]], eye: [p[0], p[1] + 1.0, p[2]], aim: [-1.0, 0.0, 0.0], sph: p, capsule: ([p[0], p[1] + 1.086, p[2]], p), hanging: false, cel: None } }

/// The SG5 guard at x=61.342 fires on Bond standing at the live session's spot; every hit that
/// lands costs `base × the difficulty factor` ([M Player_HandlePain 0x1902c8:29-48 →
/// `Plr_DMod_*`; L 07000005: 2.75 base, Easy 0.6 = 1.65 — ai_live's measured column]).
fn guard_hits_at(l: &Level05, difficulty: u32) -> Vec<f32> {
    let d = l.data(difficulty);
    // The seed ai_live re-picked after the head-tracking draws joined the RNG (WORKLOG a8884f3).
    let (drones, gi, rng) = l.drones_seeded(5);
    let mut sim = l.sim(drones, rng);
    sim.difficulty = difficulty; // the game's GameState+0x40 [M 0x2a37a8; boot default 2 0x1c7e78:25]
    const WARM: u32 = 10;
    let away = bond_at([0.0, -500.0, 0.0]);
    for _ in 0..WARM { sim.tick(&d, &away); }
    let player = bond_at([67.31, 26.5, 99.47]);
    for _ in 0..400 { sim.tick(&d, &player); }
    assert!(sim.list[gi].shots.len() >= 4, "the guard fired at difficulty {difficulty} ({} shots)", sim.list[gi].shots.len());
    sim.player_hits.iter().filter(|(_, o, _)| *o == gi).map(|(_, _, h)| *h).collect()
}

#[test]
fn a_guards_damage_to_bond_scales_with_the_difficulty() {
    let Some(l) = Level05::load() else { eprintln!("skip: NF_ISO not set"); return };
    // The DEFAULT difficulty first (the boot default 2 [M 0x1c7e78:25]): the SG5's 2.75 × 0.7.
    let normal = guard_hits_at(&l, 2);
    assert!(!normal.is_empty(), "the default-difficulty run recorded hits");
    assert!(normal.iter().all(|h| (h - 1.925).abs() < 1e-3), "difficulty 2 hits {normal:?} (2.75 × 0.7)");
    // The switch: difficulty 1 — the same guard, same spot, 2.75 × 0.6 = 1.65 (ai_live's measured
    // value at the old hardcoded difficulty).
    let easy = guard_hits_at(&l, 1);
    assert!(!easy.is_empty(), "the easy run recorded hits");
    assert!(easy.iter().all(|h| (h - 1.65).abs() < 1e-3), "difficulty 1 hits {easy:?} (2.75 × 0.6)");
    // Sister: the ratio of the two factors is the decompile's 0.6/0.7.
    assert!((easy[0] / normal[0] - 0.6 / 0.7).abs() < 1e-3, "{} / {}", easy[0], normal[0]);
}
