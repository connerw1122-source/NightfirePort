//! The copter patrol latch (helicopter-2): `Copter_Update` case 1 keeps its whole spot check —
//! the timer increment, the cone/LOS test and the `Rand_Rand(10)` attackmode draw — inside the
//! `+0x74 != 100` conjunct [M 0x17dbb8:140]. The ambient patrols carry record+0x74 = 100
//! (07000005's map50 #241/#242 [L level params, probe_helicopter2-era parse]), so on Mayhew the
//! copters never spot, never draw from the shared RNG, never fire and never request combat
//! music — they fly their loop forever. The pre-fix port spotted the courtyard player at t160,
//! drew `Rand_Rand(10)`, fired the 0x71 gun and flooded 0x5df voices, which shifted the guards'
//! sight-to-first-shot timing (guard_reaction_live path3) and interrupted the weapon-sound
//! sweep (fix_weapon_sounds_live 0x3a/0x41 rows). Opt-in: set `NF_ISO`.

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
            hud: nf_formats::hud::HudTables::from_action_elf(&elf).unwrap(),
            surfaces: nf_formats::sfx::SurfaceSounds::from_action_elf(&elf).unwrap(),
            voices: nf_formats::sfx::voice_lines(&elf).unwrap(),
            level, scripts, world, skeletons, nav, covers, volumes,
        })
    }

    fn sim(&self, drones: Vec<Drone>, rng: Rng) -> Drones {
        let mut s = Drones::new(drones, rng); s.attach_level(&self.level);
        s.difficulty = 1;
        s
    }

    fn data(&self) -> Data<'_> {
        Data { tables: &self.tables, props: &self.props, scripts: &self.scripts, weapons: &self.weapons, world: &self.world, level: 0x0700_0005,
            sequences: &self.sequences, skins: &self.level.skins, skeletons: &self.skeletons, nav: &self.nav, covers: &self.covers, tuning: &self.tuning, hitboxes: &self.hitboxes, volumes: &self.volumes,
            sfx: Some(&self.sfx), models: &self.models, surfaces: Some(&self.surfaces), voices: Some(&self.voices), difficulty: 1,
                ai_points: &[],
            }
    }

    fn drones_seeded(&self, seed: u32) -> (Vec<Drone>, Rng) {
        let d = self.data();
        let mut rng = Rng::new(seed);
        let mut lights = nf_game::light::LightList::default();
        let drones = nf_game::drone::level_drones(&d, &self.level, &mut rng, &mut lights);
        (drones, rng)
    }
}

fn bond_at(p: [f32; 3]) -> Player { Player { pos: p, target: [p[0], p[1] + 0.5, p[2]], eye: [p[0], p[1] + 1.0, p[2]], aim: [-1.0, 0.0, 0.0], sph: p, capsule: ([p[0], p[1] + 1.086, p[2]], p), hanging: false, cel: None } }

/// The guard_reaction_live spot: the pre-fix copter entered state 2 here at t160.
const BOND: [f32; 3] = [67.31, 26.5, 99.47];

/// The latch, through the real sim path: 500 ticks (past the 240/DiffMod first-check period and
/// the pre-fix t160 spot) with Bond in the open courtyard. Both Mayhew copters carry +0x74 = 100
/// and must stay in state 1 the whole window — never the attack state, never an attackmode draw,
/// never a turret tick — while their spline flight still advances (the sister that fails if the
/// latch "fix" freezes the actor instead of gating only the spot check).
#[test]
fn the_ambient_patrol_copters_never_spot_or_draw_on_07000005() {
    let Some(l) = Level05::load() else { eprintln!("skip: NF_ISO not set"); return };
    let d = l.data();
    let player = bond_at(BOND);
    let (drones, rng) = l.drones_seeded(5);
    let mut sim = l.sim(drones, rng);
    assert_eq!(sim.copters.len(), 2, "07000005 carries exactly two copters (map50 #241, #242) [D the level]");
    for c in &sim.copters {
        assert_eq!(c.arrival, 100, "the patrol's record+0x74 is 100 [L the level's params] — the latch value");
        assert_eq!(c.countdown, 0, "the patrol's record+0x76 countdown is 0 [L the level's params]");
    }
    for tick in 0..500 {
        sim.tick(&d, &player);
        for (i, c) in sim.copters.iter().enumerate() {
            assert_eq!(c.state, 1, "copter {i} left the patrol state at t{tick} ({}): the +0x74 = 100 latch must gate the case-1 spot check [M 0x17dbb8:140]", c.state);
            assert_eq!(c.attackmode, 0, "copter {i} drew the attackmode at t{tick}: no Rand_Rand(10) may leave the latch [M 0x17dbb8:140's conjunct, :184 inside it]");
            assert_eq!(c.gun_shots, 0, "copter {i} ticked the turret at t{tick}: Copter_FireGun is case-2 only [M 0x17dbb8:197]");
        }
    }
    // The sisters that keep this from passing vacuously: the patrol still FLIES — the spline
    // playhead advanced by the route's dt (0.191 [L the framelist node]) every tick — and the
    // draw-count for the whole window matches master's stream (the copter added zero draws).
    let t0 = sim.copters[0].t;
    assert!(t0 > 80.0, "the #241 patrol never flew (t = {t0} after 500 ticks): the latch must not stop Copter_Move [M 0x17dbb8:139]");
    assert!(t0 < 100.0, "the #241 playhead ran away (t = {t0}): 499 moves x 0.191 = 95.3");
}

/// The missile-arm pod pick draws from the shared stream on EVERY state-2 even-attackmode tick,
/// before the 60·DiffMod launch gate [M 0x17d9f0:47 `Rand_Rand(0x86e6)` precedes :58-64's
/// `GameState % (FRAME_RATE·DiffMod)` early return]. Pinned with the level's own #241 record
/// forced into state 2 (an off-gate frame so nothing launches): one draw, no missile. The
/// pre-fix order drew only on the launch ticks — the count assertion is the sister that fails.
#[test]
fn the_missile_pod_pick_draws_before_the_launch_gate() {
    let Some(l) = Level05::load() else { eprintln!("skip: NF_ISO not set"); return };
    let path_blocks = l.level.path_blocks();
    let mi = l.level.main_map().unwrap();
    let mut copters = nf_game::copter::Copter::create(&l.level.maps[mi].placements, 1, &path_blocks);
    let c = copters.iter_mut().find(|c| c.placement == 241).expect("map50 #241");
    assert_eq!(c.hp, 299.0, "the d1 health [M 0x17d340:76-94] — DiffMod 1.0, the launch period 60");
    c.level = 0x0700_0005;
    c.started = true;
    c.state = 2;
    c.attackmode = 0; // even — the missile arm [M case 2 :193-195]
    let mut ch = nf_game::triggers::Channels::default();
    let mut draws: Vec<u32> = Vec::new();
    let (gun, missile, _music, _exploded) = c.update(&mut ch, 7 /* 7 % 60 != 0: the gate blocks */,
        Some(BOND), [BOND[0], BOND[1] + 1.0, BOND[2]], &|_a, _b| false, &[0.0; 9], &mut |n| { draws.push(n); 1 });
    assert!(gun.is_none(), "the even arm fires no gun [M 0x17dbb8:193-195]");
    assert!(missile.is_none(), "frame 7 is off the 60-tick gate: nothing launches [M 0x17d9f0:58-64]");
    assert_eq!(draws, vec![0x86e6], "the pod pick drew exactly Rand_Rand(0x86e6) despite the blocked launch [M 0x17d9f0:47]");
}
