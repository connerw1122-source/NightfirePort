//! The settled grenade's bounce sounds (grenade-rattle): playtests 1013-1825 "once it kind of settles, it's
//! rapidly ... contacting the ground" and 1013-2313 "when a grenade is on the ground it makes rapid bouncing
//! sounds". The game's bounce-sound call and its conditions [M Bullet_CollisionHandler 0x12ce80, the block at
//! 0x12d4bc-0x12d580, disassembled]: the flash record (+0x70 & 0x2000) plays 0x15 and the smoke record
//! (+0x70 & 0x20000) plays 0x13 on the FIRST bounce only (the pre-increment counter == 0); every other weapon
//! plays 0x51c when the pre-increment counter is EVEN and < 0xF — counters 0,2,4,…,14, 8 sounds max, gain 100.0
//! (0x42c80000), 3-D at obj+0x30. The counter (BU+0x100 u16) increments per bounce and caps the bounce arm at
//! 20 (`(ushort)(counter+1) < 0x14` skips the reflect). The rest [M 0x12d618-0x12d640]: the reflected speed is
//! `BU+0xF8 · coefficient(material @ 0x3037dc + mat·0x88)`; below `FRAME_RATE_MUL · 0.01` (= 0.01 at 60 Hz
//! [L ee_snapshot.bin 0x30d0d4 = 1.0]) the speed ZEROES (Mat_Align2Dir + `BU+0xF8 = 0`) — the settled grenade's
//! move is the gravity creep (0.00272/tick, WldGravity (0,−9.8,0) [L ee_snapshot.bin 0x2d8890]) and the
//! grenade family's hittest below half the probe radius uses the 0x101 SPHERE at the current position
//! [M Bullet_update 0x12e7b0:427-437 — radius max(0.25, obj+0x8C)], so the settled grenade re-contacts every
//! tick but the parity cap keeps it SILENT once the counter passes 14. Opt-in: set `NF_ISO`.

use nf_formats::anim::Script;
use nf_formats::iso9660::IsoImage;
use nf_formats::{disc, drone, level::Level, levelbin::LevelBin, weapon::WeaponData};
use nf_game::drone::{Data, Drone, Drones, Player, Rng};
use nf_game::sound::Cmd;
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
    hud: nf_formats::hud::HudTables,
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
        let covers = nf_game::cover::build(&level.maps[m].placements, &nav, &world);
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
            hud: nf_formats::hud::HudTables::from_action_elf(&elf).unwrap(),
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

    fn sim(&self, drones: Vec<Drone>, rng: Rng) -> Drones {
        let mut s = Drones::new(drones, rng); s.attach_level(&self.level); s.difficulty = 1; s
    }
}

fn bond_at(p: [f32; 3]) -> Player { Player { pos: p, target: [p[0], p[1] + 0.5, p[2]], eye: [p[0], p[1] + 1.0, p[2]], aim: [-1.0, 0.0, 0.0], sph: p, capsule: ([p[0], p[1] + 1.086, p[2]], p), hanging: false, cel: None } }

/// The level-start spot [L ai_live.rs: the snapshot's start position] — open ground.
const SPOT: [f32; 3] = [104.954, 25.663, 108.825];

fn frag_drop(x: f32, y: f32, z: f32) -> nf_game::grenade::Grenade {
    // The frag 0x34 [L ee_snapshot.bin weapon_data 0x2c27c0: radius 8, damage 100, speed 0.1667,
    // +0x6C 0x8c08 (gravity), +0x70 0x280 (the fuse 0x200 + the bounce 0x80)], the 210-tick fuse
    // [M Bullet_init 0x12c5a0].
    nf_game::grenade::Grenade {
        pos: [x, y, z], velocity: [0.0, 0.0, 0.0], speed: 0.1667,
        travelled: 0.0, range: 210.0, damage: 100.0, radius: 8.0,
        weapon_type: 0x34, mesh: 0, owner: 0, state: 0, bounces: 0,
        smoke_popped: false, resting: false, stuck: false, seek_target: None, phase: None,
    }
}

/// Count the 0x51c bounce-sound starts per tick over the drop's life.
fn run_drop(l: &Level05, drop: nf_game::grenade::Grenade, ticks: usize) -> (Vec<(usize, u32)>, Vec<String>) {
    let d = l.data();
    let (drones, rng) = {
        let mut rng = Rng::new(1);
        let mut lights = nf_game::light::LightList::default();
        (nf_game::drone::level_drones(&d, &l.level, &mut rng, &mut lights), rng)
    };
    let mut sim = l.sim(drones, rng);
    let away = bond_at([SPOT[0] + 40.0, SPOT[1], SPOT[2]]); // Bond away: no guard reactions
    for dr in &mut sim.list { dr.present = false; }         // a pure physics drop
    sim.grenades.push(drop);
    let mut starts = Vec::new();
    let mut trace = Vec::new();
    for t in 0..ticks {
        sim.tick(&d, &away);
        sim.audio.update(Some(&l.sfx), (SPOT, [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]), &|_| true);
        for c in sim.audio.take_commands() {
            if let Cmd::Start { id, .. } = c { starts.push((t, id)); }
        }
        if let Some(g) = sim.grenades.first() {
            if g.bounces > 0 || g.resting || t % 20 == 0 {
                trace.push(format!("t{t}: bounces={} resting={} pos=({:.3},{:.3},{:.3}) vel=({:.4},{:.4},{:.4})",
                    g.bounces, g.resting, g.pos[0], g.pos[1], g.pos[2], g.velocity[0], g.velocity[1], g.velocity[2]));
            }
        } else { break; }
        if sim.grenades.first().map_or(true, |g| g.state >= 2) { trace.push(format!("t{t}: gone (state >= 2)")); break; }
    }
    (starts, trace)
}

/// The game's sound count for a dropped frag: the parity cap keeps it "a few at most", and once the
/// grenade rests nothing sounds any more.
#[test]
fn dropped_frag_bounce_sounds_stop() {
    let Some(l) = Level05::load() else { eprintln!("skip: NF_ISO not set"); return };
    let d = l.data();
    // The ground under the level-start spot: a downward probe.
    let hit = d.world.segment_surface([SPOT[0], SPOT[1] + 5.0, SPOT[2]], [SPOT[0], SPOT[1] - 20.0, SPOT[2]], false)
        .expect("ground under the start spot");
    let floor = SPOT[1] + 5.0 - 25.0 * hit.0;
    eprintln!("floor at y={floor:.3} (surface {:#x?})", hit.2);

    let (starts, trace) = run_drop(&l, frag_drop(SPOT[0], floor + 1.2, SPOT[2]), 400);
    for line in &trace { eprintln!("{line}"); }
    let bounces: Vec<&(usize, u32)> = starts.iter().filter(|(_, id)| *id == 0x51c).collect();
    eprintln!("0x51c starts: {bounces:?}");
    let all: Vec<String> = starts.iter().map(|(t, id)| format!("t{t}:{id:#x}")).collect();
    eprintln!("all starts: {all:?}");
    // The game's gate: the parity cap allows at most 8 bounce sounds per grenade
    // [M 0x12ce80 0x12d4bc-0x12d548 — even counters < 0xF].
    assert!(bounces.len() <= 8, "the parity cap keeps the bounce sounds at a few: {:?}", bounces);
    // The settle: the drop lands at ~t29 and is done bouncing well before 2 s — NOTHING sounds
    // over the last third of its life (the parity cap [M 0x12d4bc-0x12d548: sounds only on even
    // counters < 0xF, ≤ 8] plus the slow-bounce rest); the fuse death at 210 is the blast.
    let late: Vec<(usize, u32)> = bounces.iter().filter(|(t, _)| *t > 120).map(|(t, id)| (*t, *id)).collect();
    assert!(late.is_empty(), "no bounce sounds after the grenade settles: {late:?}");
    assert!(bounces.len() >= 1, "the impact itself sounds");
}

/// The thrown frag: the guard's lob (dist < 10 → pitch +π/4 [M DroneWeap_ThrowGrenade 0x179558]),
/// speed 0.1667 — the grenade arcs, lands, rolls and settles. All sound ids are traced.
#[test]
fn thrown_frag_probe() {
    let Some(l) = Level05::load() else { eprintln!("skip: NF_ISO not set"); return };
    let d = l.data();
    let hit = d.world.segment_surface([SPOT[0], SPOT[1] + 5.0, SPOT[2]], [SPOT[0], SPOT[1] - 20.0, SPOT[2]], false)
        .expect("ground under the start spot");
    let floor = SPOT[1] + 5.0 - 25.0 * hit.0;
    // The lob: 45° up, the game's throw speed 0.1667 [L weapon_data 0x34 +0x20].
    let sp = std::f32::consts::FRAC_1_SQRT_2;
    let v = 0.1667;
    let g = frag_drop(SPOT[0], floor + 1.2, SPOT[2]);
    let (starts, trace) = run_drop(&l, nf_game::grenade::Grenade {
        velocity: [sp * v, sp * v, 0.0], ..g
    }, 400);
    for line in &trace { eprintln!("{line}"); }
    let all: Vec<String> = starts.iter().map(|(t, id)| format!("t{t}:{id:#x}")).collect();
    eprintln!("all starts: {all:?}");
}

/// The sister: a frag thrown straight AT a wall still sounds on the impact.
#[test]
fn wall_hit_still_sounds() {
    let Some(l) = Level05::load() else { eprintln!("skip: NF_ISO not set"); return };
    let d = l.data();
    let hit = d.world.segment_surface([SPOT[0], SPOT[1] + 5.0, SPOT[2]], [SPOT[0], SPOT[1] - 20.0, SPOT[2]], false)
        .expect("ground under the start spot");
    let floor = SPOT[1] + 5.0 - 25.0 * hit.0;
    // Find a wall: horizontal rays from the spot.
    let wall = (0..16).map(|k| { let a = k as f32 * 0.3927; [a.sin(), 0.0, a.cos()] })
        .find_map(|dir| d.world.segment_surface([SPOT[0], floor + 1.2, SPOT[2]],
            [SPOT[0] + dir[0] * 20.0, floor + 1.2, SPOT[2] + dir[2] * 20.0], false)
            .map(|h| (dir, h.0 * 20.0)))
        .expect("a wall within 20 units");
    eprintln!("wall at {:.2} units along {:?}", wall.1, wall.0);
    let v = 0.1667;
    let g = frag_drop(SPOT[0], floor + 1.2, SPOT[2]);
    let (starts, trace) = run_drop(&l, nf_game::grenade::Grenade {
        velocity: [wall.0[0] * v, 0.0, wall.0[2] * v], ..g
    }, 400);
    for line in &trace { eprintln!("{line}"); }
    let all: Vec<String> = starts.iter().map(|(t, id)| format!("t{t}:{id:#x}")).collect();
    eprintln!("all starts: {all:?}");
    assert!(starts.iter().any(|(_, id)| *id == 0x51c), "the wall impact sounds");
}

/// The skim: a frag moving horizontally just above the floor (the post-bounce roll) — the rattle
/// candidate. Traces every collision tick.
#[test]
fn skim_probe() {
    let Some(l) = Level05::load() else { eprintln!("skip: NF_ISO not set"); return };
    let d = l.data();
    let hit = d.world.segment_surface([SPOT[0], SPOT[1] + 5.0, SPOT[2]], [SPOT[0], SPOT[1] - 20.0, SPOT[2]], false)
        .expect("ground under the start spot");
    let floor = SPOT[1] + 5.0 - 25.0 * hit.0;
    let g = frag_drop(SPOT[0], floor + 0.05, SPOT[2]);
    let (starts, trace) = run_drop(&l, nf_game::grenade::Grenade {
        velocity: [0.1667, 0.0, 0.0], ..g
    }, 400);
    for line in &trace { eprintln!("{line}"); }
    let all: Vec<String> = starts.iter().map(|(t, id)| format!("t{t}:{id:#x}")).collect();
    eprintln!("all starts: {all:?}");
}
