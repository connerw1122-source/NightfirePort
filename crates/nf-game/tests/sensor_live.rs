//! The 0700000b sensors (sensor-volumes, task 749): playtest gap "sensor volumes (0x28)" —
//! `docs/systems/script-census.md` top-10 item 4. 0700000b carries two main-map sensors
//! [D placement 164: pos (−10.38, 49.64, 11.16), cone 15°, range 5, out ch 0x28 (40), disable
//! ch 0x3c; placement 172: pos (9.25, 49.64, 11.16), cone 15°, range 5, out ch 0x27 (39)] —
//! the read comes from the decompile, not the census plain-words: `Sensor_Update` 0x1af618
//! detects **`glb_players[0]` only** (disasm 0x1af644 loads 0x2d88e0, both `Sensor_InCone`
//! calls 0x1af97c/0x1afae0 pass it; no code path feeds it an NPC), so the trip test drives the
//! PLAYER through the cone — the census's "a drone walking through sets the out channel" is not
//! what the binary does (docs/reports/sensor-volumes.md has the contradiction and the addresses).
//!
//! Pinned here (all [M] unless noted):
//! * sweeping clears the out channel (0x1af618:169-171) — ch 40 reads 0 before the trip;
//! * 32+ consecutive in-cone ticks alert (0x1af618:211-216, `prev > 0x1e`), then 2 s of
//!   continuous sight writes `switch_channels[out] = 1` (0x1af618:154-157) — the channel
//!   therefore turns on no earlier than tick 32 + 120 = 152;
//! * the sister sensor whose cone the player never enters keeps its channel at 0 the whole run
//!   (the "empty volume" half of the task's sister pair).

use nf_formats::anim::Script;
use nf_formats::iso9660::IsoImage;
use nf_formats::{disc, drone, level::Level, levelbin::LevelBin, weapon::WeaponData};
use nf_game::door::interp;
use nf_game::drone::{Data, Drones, Player, Rng};
use std::collections::HashMap;

struct Level0b {
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
    models: HashMap<u32, nf_game::pickup::Model>,
    surfaces: nf_formats::sfx::SurfaceSounds,
    voices: Vec<nf_formats::sfx::VoiceLine>,
}

impl Level0b {
    fn load() -> Option<Level0b> {
        let path = std::env::var_os("NF_ISO")?;
        let iso = IsoImage::open(&path).unwrap();
        let elf = iso.read_file(iso.find("/ACTION.ELF").unwrap()).unwrap();
        let (entries, bin) = disc::files_bin_entries(&iso).unwrap();
        let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case("0700000b.bin")).unwrap();
        let data = iso.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).unwrap();
        let level = Level::parse(&data).unwrap();
        let lb = LevelBin::parse(&data).unwrap();
        let scripts = lb.entries.iter().filter(|e| e.kind == 5).filter_map(|e| Script::parse(lb.data(&data, e)).ok()).map(|s| (s.hashcode, s)).collect();
        let world = nf_game::world::CollisionWorld::build(&level, &level.instances(true).0);
        let skeletons = lb.entries.iter().filter(|e| e.kind == 6).filter_map(|e| nf_formats::character::parse_skeleton(lb.data(&data, e)).ok()).map(|k| (k.id as u8, k)).collect();
        let m = level.main_map().unwrap();
        let nav = nf_game::nav::Network::build(level.maps[m].ai_network.as_ref().unwrap(), &world, 0x0700_000b);
        let covers = nf_game::cover::build(&level, &nav, &world);
        let volumes = nf_game::aivolume::build(&level, 0x0700_000b);
        Some(Level0b {
            tables: drone::DroneAnimData::from_action_elf(&elf).unwrap(),
            props: drone::PropertyTables::from_action_elf(&elf).unwrap(),
            weapons: WeaponData::from_action_elf(&elf).unwrap(),
            sfx: nf_formats::sfx::SfxTable::from_action_elf(&elf).unwrap(),
            tuning: { let mut t = nf_formats::tuning::Tuning::from_action_elf(&elf).unwrap(); nf_game::tuning::read_tuning_vars(&mut t, 0x0700_000b); t },
            hitboxes: nf_formats::hitbox::CollisionBones::from_action_elf(&elf).unwrap(),
            sequences: disc::level_sequences(&iso, "0700000b.bin").unwrap(),
            models: nf_game::pickup::models(&level),
            surfaces: nf_formats::sfx::SurfaceSounds::from_action_elf(&elf).unwrap(),
            voices: nf_formats::sfx::voice_lines(&elf).unwrap(),
            level, scripts, world, skeletons, nav, covers, volumes,
        })
    }

    fn data(&self) -> Data<'_> {
        Data { tables: &self.tables, props: &self.props, scripts: &self.scripts, weapons: &self.weapons, world: &self.world, level: 0x0700_000b,
            sequences: &self.sequences, skins: &self.level.skins, skeletons: &self.skeletons, nav: &self.nav, covers: &self.covers, tuning: &self.tuning, hitboxes: &self.hitboxes, volumes: &self.volumes,
            sfx: Some(&self.sfx), models: &self.models, surfaces: Some(&self.surfaces), voices: Some(&self.voices), difficulty: 1, ai_points: &[] }
    }
}

#[test]
fn sensor_trips_after_two_seconds_of_sight_and_the_empty_sister_does_not() {
    let Some(l) = Level0b::load() else { eprintln!("skip: NF_ISO not set"); return };
    let d = l.data();
    let mut rng = Rng::new(0x1234_5678);
    let mut lights = nf_game::light::LightList::default();
    let drones = nf_game::drone::level_drones(&d, &l.level, &mut rng, &mut lights);
    let mut sim = Drones::new(drones, rng);
    sim.attach_level(&l.level);
    sim.difficulty = 1;

    // The two main-map sensors of 0700000b [D probe of the disc placements].
    assert_eq!(sim.sensors.list.len(), 2, "0700000b main map carries exactly two 0x28 placements");
    let (s164, s172) = (sim.sensors.list.iter().find(|s| s.placement == 164).expect("sensor placement 164"), sim.sensors.list.iter().find(|s| s.placement == 172).expect("sensor placement 172"));
    assert_eq!((s164.out, s164.disable), (0x28, 0x3c), "sensor 164 params [D]");
    assert_eq!((s172.out, s172.disable), (0x27, 0x3c), "sensor 172 params [D]");
    assert_eq!((s164.limit, s164.keys.len()), (4, 5), "sensor 164 sweeps a 5-key list [D]");
    assert!((s164.step - 4.0 / 600.0).abs() < 1e-9, "sweep step = (count-1)·REC·(FRAME_RATE/rate) = 4/600 [M 0x1af010:89-96], got {}", s164.step);

    // The player stands ON the sweep arc, on the bisector of sensor 164's key0→key1 segment
    // [D: the sweep tilts 30° down and yaws from (0,−0.50,0.87) to (−0.87,−0.48,0)]. At 3 units
    // on the bisector the head is within the 15° cone for ~50 consecutive ticks per pass (the
    // 90° segment takes 150 ticks, step 4/600) — over the 32-tick alert threshold
    // [M 0x1af618:211-216]. The point comes from the same `Spline_Interp` port the sim sweeps
    // with, at t = 0.5 key-units (half of the first 90° segment).
    let bisector = interp(&s164.keys, 0, s164.limit, 0.5, true).expect("sensor 164 has a sweep");
    let dir = [bisector[2][0], bisector[2][1], bisector[2][2]];
    let at = [s164.pos[0] + 3.0 * dir[0], s164.pos[1] + 3.0 * dir[1], s164.pos[2] + 3.0 * dir[2]];
    let mut bond = Player { pos: [at[0], at[1] - 0.9, at[2]], ..Default::default() };
    bond.target = bond.pos;
    bond.sph = at; // the world-sphere centre the cone test reads [M 0x1af4f0:61]
    assert!(l.world.line_of_sight(s164.pos, bond.pos) && l.world.line_of_sight(s164.pos, at),
        "the bisector point needs clear static LOS [M 0x1af4f0:84-87]");

    // Sister: park the player far away first — neither sensor may trip (the "empty volume"
    // half: a cone that never holds the player never writes its channel [M 0x1af618:169-171]).
    bond.pos = [40.0, 30.0, 40.0];
    bond.sph = [40.0, 30.9, 40.0];
    for t in 0..300u32 {
        sim.tick(&d, &bond);
        assert!(!sim.channels.on(0x28) && !sim.channels.on(0x27), "tick {t}: nothing may trip with the player far away");
    }

    // Now the in-cone run: ch 40 clears while sweeping, then trips after alert + 2 s of sight.
    // The NPCs are frozen (`Drone_EnableAll(0)`'s port switch): a patrolling guard crossing the
    // sight line breaks the LOS run — real-game behaviour, but not what this test pins.
    sim.npc_enabled = false;
    bond.pos = [at[0], at[1] - 0.9, at[2]];
    bond.sph = at;
    let mut first_on: Option<u32> = None;
    let mut sister_on: Option<u32> = None;
    for t in 0..400u32 {
        sim.tick(&d, &bond);
        if sim.channels.on(0x28) && first_on.is_none() { first_on = Some(t); }
        if sim.channels.on(0x27) && sister_on.is_none() { sister_on = Some(t); }
        let s = &sim.sensors.list[0];
        if t % 20 == 0 { eprintln!("tick {t}: sensor state {} t={:.2} ch40 {}", s.state, s.t, sim.channels.on(0x28) as u8); }
    }
    let first_on = first_on.expect("sensor 164 must trip its out channel ch 0x28 after 2 s of sight [M 0x1af618:154-157]");
    assert!(first_on >= 152, "first ch-0x28 write at tick {first_on}: the decompile needs 32 alert ticks + 120 trip ticks (≥152) [M 0x1af618:211-216,154-157]");
    assert!(first_on < 390, "first ch-0x28 write at tick {first_on}: the sweep pass must reach the player within the run");
    assert!(sister_on.is_none(), "sensor 172 (out ch 0x27) must stay off — its cone never holds the player [M 0x1af618:169-171]");
    assert!(sim.channels.on(0x28), "ch 0x28 stays 1 while the sight holds [M 0x1af618:154-157 re-writes every tick]");
    eprintln!("== sensor 164 tripped ch 0x28 at tick {first_on}; sister ch 0x27 never set");
}
