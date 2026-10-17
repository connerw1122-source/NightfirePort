//! difficulty-effects: the `Drone_Create` min-difficulty spawn gate through the real parse
//! path (`level_drones` over the disc's 07000011 placements), numbers only, ISO-gated.
//!
//! `Drone_Create` 0x136be0:49: a 0x0F placement whose level_tag+0x34 (param 2) is non-zero
//! spawns only when `param2 <= GameState+0x28` (the selected difficulty, u32 compare). The
//! gate sits before the create body, so skipped placements draw no RNG either side. The
//! harness is progress_live.rs's Level05, reduced to the `level_drones` inputs.

use nf_formats::anim::Script;
use nf_formats::iso9660::IsoImage;
use nf_formats::{disc, drone, level::Level, levelbin::LevelBin, weapon::WeaponData};
use nf_game::drone::{Data, Rng};
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
        let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case("07000011.bin")).unwrap();
        let data = iso.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).unwrap();
        let level = Level::parse(&data).unwrap();
        let lb = LevelBin::parse(&data).unwrap();
        let scripts = lb.entries.iter().filter(|e| e.kind == 5).filter_map(|e| Script::parse(lb.data(&data, e)).ok()).map(|s| (s.hashcode, s)).collect();
        let world = nf_game::world::CollisionWorld::build(&level, &level.instances(true).0);
        let skeletons = lb.entries.iter().filter(|e| e.kind == 6).filter_map(|e| nf_formats::character::parse_skeleton(lb.data(&data, e)).ok()).map(|k| (k.id as u8, k)).collect();
        let m = level.main_map().unwrap();
        let nav = nf_game::nav::Network::build(level.maps[m].ai_network.as_ref().unwrap(), &world, 0x0700_0011);
        let covers = nf_game::cover::build(&level, &nav, &world);
        let volumes = nf_game::aivolume::build(&level, 0x0700_0011);
        Some(Level05 {
            tables: drone::DroneAnimData::from_action_elf(&elf).unwrap(),
            props: drone::PropertyTables::from_action_elf(&elf).unwrap(),
            weapons: WeaponData::from_action_elf(&elf).unwrap(),
            sfx: nf_formats::sfx::SfxTable::from_action_elf(&elf).unwrap(),
            tuning: { let mut t = nf_formats::tuning::Tuning::from_action_elf(&elf).unwrap(); nf_game::tuning::read_tuning_vars(&mut t, 0x0700_0011); t },
            hitboxes: nf_formats::hitbox::CollisionBones::from_action_elf(&elf).unwrap(),
            sequences: disc::level_sequences(&iso, "07000011.bin").unwrap(),
            models: nf_game::pickup::models(&level),
            surfaces: nf_formats::sfx::SurfaceSounds::from_action_elf(&elf).unwrap(),
            voices: nf_formats::sfx::voice_lines(&elf).unwrap(),
            level, scripts, world, skeletons, nav, covers, volumes,
        })
    }

    fn data(&self, difficulty: u32) -> Data<'_> {
        Data { tables: &self.tables, props: &self.props, scripts: &self.scripts, weapons: &self.weapons, world: &self.world, level: 0x0700_0011,
            sequences: &self.sequences, skins: &self.level.skins, skeletons: &self.skeletons, nav: &self.nav, covers: &self.covers, tuning: &self.tuning, hitboxes: &self.hitboxes, volumes: &self.volumes,
            sfx: Some(&self.sfx), models: &self.models, surfaces: Some(&self.surfaces), voices: Some(&self.voices), difficulty, ai_points: &[] }
    }
}

/// How many of the level's 0x0F placements carry a param-2 tag of 2 or 3 (the ones the gate
/// must hold back below their difficulty).
fn tagged_counts(level: &Level) -> (usize, usize) {
    let mut tag2 = 0;
    let mut tag3 = 0;
    let m = level.main_map().unwrap();
    {
        for p in &level.maps[m].placements {
            if p.flags & 0xff != 0x0f { continue; }
            let p2 = p.params.iter().find(|(j, _)| *j == 2).map_or(0, |&(_, v)| v);
            if p2 == 2 { tag2 += 1; }
            if p2 == 3 { tag3 += 1; }
        }
    }
    (tag2, tag3)
}

#[test]
fn drone_min_difficulty_tag_gates_the_spawn() {
    let Some(f) = Level05::load() else { eprintln!("skipping: NF_ISO not set"); return; };
    let (tag2, tag3) = tagged_counts(&f.level);
    eprintln!("07000005: 0x0F placements with param2=2: {tag2}, param2=3: {tag3}");

    let mut count = |difficulty: u32| {
        let d = f.data(difficulty);
        let mut rng = Rng::new(7);
        let mut lights = nf_game::light::LightList::default();
        nf_game::drone::level_drones(&d, &f.level, &mut rng, &mut lights).len()
    };
    let c1 = count(1);
    let c2 = count(2);
    let c3 = count(3);
    eprintln!("spawn counts at d1/d2/d3: {c1}/{c2}/{c3}");

    // The game's gate, as numbers: every param-2 tag of 2 joins at difficulty 2, every 3 at 3.
    assert_eq!(c2, c1 + tag2, "difficulty 2 must add exactly the param2=2 placements");
    assert_eq!(c3, c2 + tag3, "difficulty 3 must add exactly the param2=3 placements");
    // Sister: without the gate (the pre-task code) c1 == c3 — the counts must differ when
    // any tagged placement exists on this level; if none exists the gate is untestable here.
    if tag2 + tag3 > 0 {
        assert!(c3 > c1, "the gate must hold tagged spawns back on Operative");
    } else {
        eprintln!("NOTE: 07000005 carries no param2-tagged 0x0F placements; the gate rows above are vacuous");
    }
}
