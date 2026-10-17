//! 07000005 ledge cut-scene (fix-drone2 bug 3): the user's playtest shows the cut-scene on the ledge
//! left of the castle gate **missing its guard**. The cut-scene is ScriptPlayer 298's NIS 0x06000994
//! (camera, length 1400): streams 1/3/14 are `Anim { script: 0x06000996/0x06000997/0x06000995, skin:
//! 0x05000030 / 0x0500002e (guards), 0x05000007 (Bond) }` [D nf-extract nis 07000005.bin]. It is started
//! by the TOUCH_ONCE plate 301 (out 0x43, lock 0xb3) at (24–26, 31–33, 71–72) — the ledge left of the
//! castle gate — through SP 298's mask bit 1 (its p6 out channel 0x43) [M docs/atlas/script-player.md
//! SP_Hit/SP_Update]. Both guard skins parse from the level [D]. Opt-in: set `NF_ISO`.

use nf_formats::anim::Script;
use nf_formats::iso9660::IsoImage;
use nf_formats::{disc, drone, level::Level, levelbin::LevelBin, weapon::WeaponData};
use nf_game::drone::{Data, Drones, Player, Rng};
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
    sfx: Option<nf_formats::sfx::SfxTable>,
    tuning: nf_formats::tuning::Tuning,
    hitboxes: nf_formats::hitbox::CollisionBones,
    volumes: Vec<nf_game::aivolume::AiVolume>,
    models: HashMap<u32, nf_game::pickup::Model>,
    skins: HashMap<u32, nf_formats::character::Skin>,
}

impl Level05 {
    fn load() -> Option<Level05> {
        let path = std::env::var_os("NF_ISO")?;
        let iso = IsoImage::open(&path).unwrap();
        let elf = iso.read_file(iso.find("/ACTION.ELF").unwrap()).unwrap();
        let (entries, bin) = disc::files_bin_entries(&iso).unwrap();
        let read = |name: &str| -> Option<Vec<u8>> {
            let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(name))?;
            Some(iso.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).unwrap())
        };
        let data = read("07000005.bin")?;
        let level = Level::parse(&data).unwrap();
        let lb = LevelBin::parse(&data).unwrap();
        let skeletons: HashMap<u8, _> = lb.entries.iter().filter(|e| e.kind == 6)
            .filter_map(|e| nf_formats::character::parse_skeleton(lb.data(&data, e)).ok().map(|s| (s.id as u8, s))).collect();
        let mut sequences = HashMap::new();
        if let Some(anim) = read("07F00005.bin") {
            let alb = LevelBin::parse(&anim).unwrap();
            for e in alb.entries.iter().filter(|e| e.kind == 4 && e.length >= 0x40) {
                if let Ok(s) = nf_formats::anim::Sequence::parse(alb.data(&anim, e)) { sequences.insert(s.hashcode, s); }
            }
        }
        let scripts: HashMap<u32, Script> = lb.entries.iter().filter(|e| e.kind == 5)
            .filter_map(|e| Script::parse(lb.data(&data, e)).ok()).map(|s| (s.hashcode, s)).collect();
        let tables = drone::DroneAnimData::from_action_elf(&elf).unwrap();
        let props = drone::PropertyTables::from_action_elf(&elf).unwrap();
        let weapons = WeaponData::from_action_elf(&elf).unwrap();
        let world = nf_game::world::CollisionWorld::build(&level, &level.instances(true).0);
        let nav = nf_game::nav::Network::build(level.maps[level.main_map().unwrap()].ai_network.as_ref().unwrap(), &world, 0x0700_0005);
        let covers = nf_game::cover::build(&level.maps[level.main_map().unwrap()].placements, &nav, &world);
        let hitboxes = nf_formats::hitbox::CollisionBones::from_action_elf(&elf).unwrap();
        let volumes = nf_game::aivolume::build(&level, 0x0700_0005);
        let models = nf_game::pickup::models(&level);
        let sfx = nf_formats::sfx::SfxTable::from_action_elf(&elf).ok();
        let tuning = nf_formats::tuning::Tuning::default();
        let skins = level.skins.clone();
        Some(Level05 { tables, props, weapons, level, scripts, world, sequences, skeletons, nav, covers, sfx, tuning, hitboxes, volumes, models, skins })
    }

    fn data(&self) -> Data<'_> {
        Data { tables: &self.tables, props: &self.props, scripts: &self.scripts, weapons: &self.weapons, world: &self.world, level: 0x0700_0005, sequences: &self.sequences, skins: &self.skins, skeletons: &self.skeletons, nav: &self.nav, covers: &self.covers, tuning: &self.tuning, hitboxes: &self.hitboxes, volumes: &self.volumes, sfx: self.sfx.as_ref(), models: &self.models, surfaces: None, voices: None, difficulty: 1,
            ai_points: &[],
        }
    }

    fn sim(&self) -> Drones {
        let d = self.data();
        let mut rng = Rng::new(0x1234_5678);
        let mut lights = nf_game::light::LightList::default();
        let drones = nf_game::drone::level_drones(&d, &self.level, &mut rng, &mut lights);
        let mut s = Drones::new(drones, rng);
        s.attach_level(&self.level);
        s
    }
}

fn bond_standing(p: [f32; 3]) -> Player { Player { pos: p, target: [p[0], p[1] + 0.5, p[2]], eye: [p[0], p[1] + 1.0, p[2]], aim: [-1.0, 0.0, 0.0], sph: p, ..Default::default() } }

/// Touching the ledge plate (channel 0x43 = its lock 0xb3 gate is a real channel the game turns on) must
/// start NIS 0x06000994 and create all three of its Anim characters, the two guards included, animating.
#[test]
fn the_ledge_cutscene_creates_its_guard_characters() {
    let Some(l) = Level05::load() else { eprintln!("skip: NF_ISO not set"); return };
    let d = l.data();
    let mut sim = l.sim();

    // The plate's lock channel on (what the game's chain does before Bond reaches the ledge), then the
    // plate's output 0x43 (Bond stepping on plate 301).
    sim.set_channel(0xb3);
    sim.set_channel(0x43);

    let mut saw: Vec<(u32, u32, f32)> = Vec::new();
    for t in 0..1500 {
        sim.tick(&d, &bond_standing([104.0, 26.0, 108.0]));
        for o in sim.scripts.objects.list.iter().flatten() {
            if let nf_game::nis::Obj::Anim { skin, script, frame, .. } = o {
                if !saw.iter().any(|(s, sc, _)| *s == *skin && *sc == *script) {
                    saw.push((*skin, *script, *frame));
                    eprintln!("t{t}: Anim skin {skin:08x} script {script:08x} frame {frame}");
                } else if let Some(e) = saw.iter_mut().find(|(s, sc, _)| *s == *skin && *sc == *script) {
                    e.2 = *frame;
                }
            }
        }
    }
    eprintln!("final: {:?}", saw.iter().map(|(s, sc, f)| format!("{s:08x}/{sc:08x}@{f}")).collect::<Vec<_>>());
    for skin in [0x0500_0030u32, 0x0500_002e, 0x0500_0007] {
        let e = saw.iter().find(|(s, _, _)| *s == skin);
        assert!(e.is_some(), "cut-scene character {skin:08x} was never created; saw {:?}", saw.iter().map(|(s, _, _)| format!("{s:08x}")).collect::<Vec<_>>());
        assert!(e.unwrap().2 > 1.0, "cut-scene character {skin:08x} never animated (frame stuck at {:?})", e.unwrap().2);
    }
}

/// The view's pose chain for the cut-scene guards (`scene.rs::animate`): `script_frame(scripts[0x06000996],
/// frame)` → sequence hashcode → `seqs.get` → `anim::fits(seq, skeleton)`. Broken anywhere, the guard has no
/// pose and shows nothing even though the object exists.
#[test]
fn the_guard_anim_scripts_resolve_to_sequences_that_fit_their_skeletons() {
    let Some(l) = Level05::load() else { eprintln!("skip: NF_ISO not set"); return };
    for (skin_hash, script_hash) in [(0x0500_0030u32, 0x0600_0996u32), (0x0500_002e, 0x0600_0997), (0x0500_0007, 0x0600_0995)] {
        let skin = l.skins.get(&skin_hash).unwrap_or_else(|| panic!("skin {skin_hash:08x} in level"));
        let script = l.scripts.get(&script_hash).unwrap_or_else(|| panic!("anim script {script_hash:08x} in level"));
        let (q, f) = nf_game::anim::script_frame(script, 50.0).unwrap_or_else(|| panic!("{script_hash:08x}: script_frame"));
        let seq = l.sequences.get(&q);
        let fits = seq.map(|s| nf_formats::anim::fits(s, l.skeletons.get(&skin.skeleton).expect("skeleton")));
        eprintln!("skin {skin_hash:08x} script {script_hash:08x}: seq {q:08x} frame {f} in_level {} fits {:?}", seq.is_some(), fits);
        assert!(seq.is_some(), "script {script_hash:08x} needs sequence {q:08x} — not in 07F00005.bin");
        assert_eq!(fits, Some(true), "sequence {q:08x} must fit skin {skin_hash:08x}'s skeleton");
    }
    // Sister: a single-sequence danim script still resolves to exactly that sequence (the drone start
    // scripts are the live check that the last-covering pick changed nothing on the normal path).
    let start = l.scripts.get(&0x0600_02f2).expect("guard start script 060002f2");
    let (a, _, q) = start.sequences().next().expect("one sequence");
    let got = nf_game::anim::script_frame(start, a as f32 + 10.0).map(|(q, _)| q);
    assert_eq!(got, Some(q), "single-sequence scripts pick their only sequence");
}
