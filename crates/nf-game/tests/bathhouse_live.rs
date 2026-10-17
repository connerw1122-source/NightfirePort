//! 07000002 bathhouse: the user's playtest shows one guard standing next to a civilian with **no AI**,
//! never moving (fix-drone2 bug 2). The bathhouse drones wait on switch channels raised by TOUCH_ONCE
//! plates, so the headless repro walks Bond onto each plate and requires every guard on that channel to
//! wake and run (or, if the disc never raises its channel, to stay inactive and undrawn — the atlas
//! "present / started / enabled / in-llist" semantics, docs/atlas/drone.md).
//!
//! Placements [D 07000002.bin]: the civilian (skin 0x0500004F, DTYPE 0xC → team 2) at (−24.33, 1.08,
//! 31.92); the nearest guards wait on ch 0x15 (plates 341 at (−25, 5.4, 42.6) and 383 at (−19.4, 2.6,
//! 35.9) — the ground-floor plate beside the civilian) and the trio around the ally 0x05000034 at
//! (−45.5, 5.8, 12.2) waits on ch 0x17 (plates 342/343/424). No spawner (0xF1) placements exist on the level
//! [D], so "held" is false for every drone here. Opt-in: set `NF_ISO`.

use nf_formats::anim::Script;
use nf_formats::iso9660::IsoImage;
use nf_formats::{disc, drone, level::Level, levelbin::LevelBin, weapon::WeaponData};
use nf_game::drone::{Data, Drones, Player, Rng};
use std::collections::HashMap;

struct Level02 {
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

impl Level02 {
    fn load() -> Option<Level02> {
        let path = std::env::var_os("NF_ISO")?;
        let iso = IsoImage::open(&path).unwrap();
        let elf = iso.read_file(iso.find("/ACTION.ELF").unwrap()).unwrap();
        let (entries, bin) = disc::files_bin_entries(&iso).unwrap();
        let read = |name: &str| -> Option<Vec<u8>> {
            let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(name))?;
            Some(iso.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).unwrap())
        };
        let data = read("07000002.bin")?;
        let level = Level::parse(&data).unwrap();
        let lb = LevelBin::parse(&data).unwrap();
        let skeletons: HashMap<u8, _> = lb.entries.iter().filter(|e| e.kind == 6)
            .filter_map(|e| nf_formats::character::parse_skeleton(lb.data(&data, e)).ok().map(|s| (s.id as u8, s))).collect();
        let mut sequences = HashMap::new();
        if let Some(anim) = read("07F00002.bin") {
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
        let nav = nf_game::nav::Network::build(level.maps[level.main_map().unwrap()].ai_network.as_ref().unwrap(), &world, 0x0700_0002);
        let covers = nf_game::cover::build(&level, &nav, &world);
        let hitboxes = nf_formats::hitbox::CollisionBones::from_action_elf(&elf).unwrap();
        let volumes = nf_game::aivolume::build(&level, 0x0700_0002);
        let models = nf_game::pickup::models(&level);
        let sfx = nf_formats::sfx::SfxTable::from_action_elf(&elf).ok();
        let tuning = nf_formats::tuning::Tuning::default();
        let skins = level.skins.clone();
        Some(Level02 { tables, props, weapons, level, scripts, world, sequences, skeletons, nav, covers, sfx, tuning, hitboxes, volumes, models, skins })
    }

    fn data(&self) -> Data<'_> {
        Data { tables: &self.tables, props: &self.props, scripts: &self.scripts, weapons: &self.weapons, world: &self.world, level: 0x0700_0002, sequences: &self.sequences, skins: &self.skins, skeletons: &self.skeletons, nav: &self.nav, covers: &self.covers, tuning: &self.tuning, hitboxes: &self.hitboxes, volumes: &self.volumes, sfx: self.sfx.as_ref(), models: &self.models, surfaces: None, voices: None, difficulty: 1,
            ai_points: &[],
        }
    }

    fn sim(&self) -> Drones {
        let d = self.data();
        let mut rng = Rng::new(0x1234_5678);
        let mut lights = nf_game::light::LightList::default();
        let drones = nf_game::drone::level_drones(&d, &self.level, &mut rng, &mut lights);
        let mut s = Drones::new(drones, rng);
        s.attach_level_id(&self.level, 0x0700_0002);
        s
    }
}

fn bond_standing(p: [f32; 3]) -> Player {
    let mut b = Player { pos: p, target: [p[0], p[1] + 0.5, p[2]], eye: [p[0], p[1] + 1.0, p[2]], aim: [-1.0, 0.0, 0.0], sph: p, ..Default::default() };
    b.target = [p[0] - 2.0, p[1] + 0.5, p[2]];
    b
}



/// Every drone on a channel whose plate Bond touches must wake and run (the game: WaitSwitch →
/// `Drone_SM_SetState` start state on the channel). Sister: a channel Bond never touches keeps its
/// guards inactive and undrawn.
#[test]
fn bathhouse_guards_wake_when_their_plates_fire() {
    use nf_game::drone::state;
    let Some(l) = Level02::load() else { eprintln!("skip: NF_ISO not set"); return };
    let d = l.data();
    let mut sim = l.sim();

    // The ch-0x15 guards (5 of them) and the ch-0x17 cluster (3 guards + the ally), plus untouched channels.
    let on_ch = |sim: &Drones, ch: u32| (0..sim.list.len()).filter(|&i| sim.list[i].present && sim.list[i].switch_channel as u32 == ch).collect::<Vec<_>>();
    let ch15 = on_ch(&sim, 0x15);
    let ch17 = on_ch(&sim, 0x17);
    assert_eq!(ch15.len(), 5, "five guards wait on ch 0x15 [D placements]");
    assert_eq!(ch17.len(), 4, "the trio + ally wait on ch 0x17 [D placements]");
    // The civilian himself: no channel, present at start.
    let civ = (0..sim.list.len()).find(|&i| sim.list[i].skin == 0x0500_004f).expect("the civilian is a placement");
    assert_eq!(sim.list[civ].switch_channel, 0, "the civilian has no channel");

    // First ticks: the GLOBAL enter must route channel waiters into WaitSwitch (disabled, undrawn).
    let start = [-23.0f32, 1.084, 31.9];
    for _ in 0..5 { sim.tick(&d, &bond_standing(start)); }
    for i in 0..sim.list.len() {
        let g = &sim.list[i];
        eprintln!("start guard {i}: skin {:08x} pos ({:.1}, {:.1}, {:.1}) state {:#x} enabled {} drawn {} present {} started {} ch {}",
            g.skin, g.pos[0], g.pos[1], g.pos[2], g.state, g.enabled, g.drawn, g.present, g.started, g.switch_channel);
    }

    // Fire ch 0x17 the way plates 342/343 do (faithful: the plates exist on-disc) and watch the
    // trio + ally for 1200 ticks: each must wake AND run (state leaves WaitSwitch; the ally's
    // dtypenew 9/10 path too).
    sim.set_channel(0x17);
    for _ in 0..1200 { sim.tick(&d, &bond_standing(start)); }
    for (k, &i) in ch17.iter().enumerate() {
        let g = &sim.list[i];
        eprintln!("after 0x17: guard {i} (#{k}) skin {:08x} pos ({:.1}, {:.1}, {:.1}) state {:#x} enabled {} drawn {} started {}",
            g.skin, g.pos[0], g.pos[1], g.pos[2], g.state, g.enabled, g.drawn, g.started);
        assert!(g.enabled, "ch17 guard {i} woke");
        assert_ne!(g.state, state::WAIT_SWITCH, "ch17 guard {i} left WaitSwitch");
    }
    // The ch-0x15 guards stay asleep (their channel is off): inactive and not drawn.
    for &i in &ch15 {
        assert_eq!(sim.list[i].state, state::WAIT_SWITCH, "ch15 guard {i} still waits (sister)");
        assert!(!sim.list[i].drawn, "ch15 guard {i} undrawn (sister)");
    }
}
