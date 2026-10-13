//! The two stun-grenade guards on 07000001 (placement param 11 = 0x13 → 3× stun, and 0x11 → 1× stun,
//! decoded by `NDrone2_DefaultInit` 0x14b300 into `+0xBCC`/`+0xBCE`) never throw in play (user playtest,
//! fix-drone2 bug 1). The headless repro (this file's first version, shown failing: "no guard reached
//! GRENADE_THROW; last states [0x2c, 0x1]") and the decompile say that is the **game's own behaviour**:
//!
//! - Guard A skin 0x05000036 at (−30.058, 10.308, 13.443), channel 0x26: DTYPE 2 (param 15) with normal
//!   block mode 2 (param 16) → `NDrone2_GetDTYPENEW` 0x14c938 case 2 → dtypenew 2 = **sniper** (alertness
//!   +0x50C < 1 at creation; `GetDroneTypeAttackTypeFriend` 0x14cbd8 runs only from `DoModeSettingsNEW`
//!   0x14c600 at creation, so the type never changes). `DroneWeap_CanThrowGrenade` 0x17a798's **only**
//!   call site is `DroneFunc_CombatState` 0x148338 cases 0x10/0x18 (states 0x68/0x70) [M]; the sniper
//!   states (`NDrone2_DSTATE_SniperIdle/Aim/Fire/Reload` 0x15fab0/0x15fc18/0x15fd88/0x15ff80) never enter
//!   that family — their only exits are reload and `NDrone2_CanAltAttack` 0x1549e0 (melee, dist ≤ 1.5).
//! - Guard B skin 0x0500003A at (6.950, 6.424, 2.200), channel 5: dtypenew 0 (a normal combat guard, so he
//!   *would* throw) — but **nothing on the disc ever raises channel 5**: no trigger outputs 5 (level
//!   trigger dump: outs 0x0f/0x32/0x10/0x0c/0x0d/0x26/0x27/0x99), no NIS script event 0xD writes 5 (the
//!   only one writes ch 0x0e [D nf-extract nis]), no pickup's +0x28 is 5, no death channel (param 12) is 5.
//!   [L work/ee_start_07000001.bin] he is disabled at level start (obj+0xF0 = 0x8b0, & 0x30 = 0x30) like
//!   every drone; unlike guard A (whose plates 311/312 raise 0x26) nothing wakes him.
//!
//! So the port must NOT throw for either guard — these tests lock the game's behaviour. The grenade
//! machinery itself is covered green by `grenade_live.rs` (a normal combat guard throws through its own
//! decision tree). Opt-in: set `NF_ISO` to the disc image.

use nf_formats::anim::Script;
use nf_formats::iso9660::IsoImage;
use nf_formats::{disc, drone, level::Level, levelbin::LevelBin, weapon::WeaponData};
use nf_game::drone::{Data, Drones, Player, Rng};
use std::collections::HashMap;

struct Level01 {
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

impl Level01 {
    fn load() -> Option<Level01> {
        let path = std::env::var_os("NF_ISO")?;
        let iso = IsoImage::open(&path).unwrap();
        let elf = iso.read_file(iso.find("/ACTION.ELF").unwrap()).unwrap();
        let (entries, bin) = disc::files_bin_entries(&iso).unwrap();
        let read = |name: &str| -> Option<Vec<u8>> {
            let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(name))?;
            Some(iso.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).unwrap())
        };
        let data = read("07000001.bin")?;
        let level = Level::parse(&data).unwrap();
        let lb = LevelBin::parse(&data).unwrap();
        let skeletons: HashMap<u8, _> = lb.entries.iter().filter(|e| e.kind == 6)
            .filter_map(|e| nf_formats::character::parse_skeleton(lb.data(&data, e)).ok().map(|s| (s.id as u8, s))).collect();
        let mut sequences = HashMap::new();
        if let Some(anim) = read("07F00001.bin") {
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
        let nav = nf_game::nav::Network::build(level.maps[level.main_map().unwrap()].ai_network.as_ref().unwrap(), &world, 0x0700_0001);
        let covers = nf_game::cover::build(&level.maps[level.main_map().unwrap()].placements, &nav, &world);
        let hitboxes = nf_formats::hitbox::CollisionBones::from_action_elf(&elf).unwrap();
        let volumes = nf_game::aivolume::build(&level, 0x0700_0001);
        let models = nf_game::pickup::models(&level);
        let sfx = nf_formats::sfx::SfxTable::from_action_elf(&elf).ok();
        let tuning = nf_formats::tuning::Tuning::default();
        let skins = level.skins.clone();
        Some(Level01 { tables, props, weapons, level, scripts, world, sequences, skeletons, nav, covers, sfx, tuning, hitboxes, volumes, models, skins })
    }

    fn data(&self) -> Data<'_> {
        Data { tables: &self.tables, props: &self.props, scripts: &self.scripts, weapons: &self.weapons, world: &self.world, level: 0x0700_0001, sequences: &self.sequences, skins: &self.skins, skeletons: &self.skeletons, nav: &self.nav, covers: &self.covers, tuning: &self.tuning, hitboxes: &self.hitboxes, volumes: &self.volumes, sfx: self.sfx.as_ref(), models: &self.models, surfaces: None, voices: None }
    }

    fn sim(&self) -> Drones {
        let d = self.data();
        // One Rng through level_drones into Drones — the game's order (the shared bit-exact stream).
        let mut rng = Rng::new(0x1234_5678);
        let mut lights = nf_game::light::LightList::default();
        let drones = nf_game::drone::level_drones(&d, &self.level, &mut rng, &mut lights);
        let mut s = Drones::new(drones, rng);
        s.attach_level(&self.level);
        s
    }
}

fn bond_at(p: [f32; 3]) -> Player { Player { pos: p, target: [p[0], p[1] + 0.5, p[2]], eye: [p[0], p[1] + 1.0, p[2]], aim: [-1.0, 0.0, 0.0], sph: p, ..Default::default() } }


/// Bond 13 units from `g` on the same floor, first offset with a clear guard→Bond line of sight.
fn bond_near(world: &nf_game::world::CollisionWorld, g: [f32; 3]) -> Player {
    for off in [[13.0, 0.0, 0.0], [-13.0, 0.0, 0.0], [0.0, 0.0, 13.0], [0.0, 0.0, -13.0], [9.2, 0.0, 9.2], [-9.2, 0.0, 9.2], [9.2, 0.0, -9.2], [-9.2, 0.0, -9.2]] {
        let p = [g[0] + off[0], g[1], g[2] + off[2]];
        let fy = world.floor_below(p, 4.0).unwrap_or(g[1] - 1.0) + 1.0;
        let bp = [p[0], fy, p[2]];
        if world.line_of_sight([g[0], g[1] + 0.6, g[2]], [bp[0], bp[1] + 0.5, bp[2]]) {
            return bond_at(bp);
        }
    }
    bond_at([g[0] + 13.0, g[1], g[2]])
}

/// The two carriers, their grenade fields, and the game's structural answer: the sniper never evaluates
/// `CanThrowGrenade` (no sniper state ever reaches the combat family) and the channel-5 waiter never
/// activates (nothing on the disc raises channel 5). Sister assertions: guard A really fights (sniper
/// fire + reload), guard B really is a dtypenew-0 combat guard (he would throw if his channel fired).
#[test]
fn stun_grenade_guards_match_the_game_on_07000001() {
    use nf_game::drone::state;
    let Some(l) = Level01::load() else { eprintln!("skip: NF_ISO not set"); return };
    let d = l.data();
    let mut sim = l.sim();

    // Exactly the two param-11 carriers decode to stun 0x36 (guard A 3×, guard B 1×) — the param-11
    // fix's observable effect (`NDrone2_DefaultInit` 0x14b300, param 11 → +0xBCC/+0xBCE).
    let carriers: Vec<usize> = (0..sim.list.len()).filter(|&i| sim.list[i].grenade_weapon == 0x36 && sim.list[i].grenade_count > 0).collect();
    assert_eq!(carriers.len(), 2, "07000001 decodes exactly 2 stun-grenade carriers, got {carriers:?}");
    let (a, b) = (carriers[0], carriers[1]);
    let (ga, gb) = (sim.list[a].clone(), sim.list[b].clone());
    assert_eq!((ga.grenade_weapon, ga.grenade_count), (0x36, 3), "guard A = 0x13 → stun × 3");
    assert_eq!((gb.grenade_weapon, gb.grenade_count), (0x36, 1), "guard B = 0x11 → stun × 1");
    // And they are the documented placements.
    assert!((ga.pos[0] - (-30.058)).abs() < 0.1 && (ga.pos[2] - 13.443).abs() < 0.1, "guard A at (−30.06, 13.44), got {:?}", ga.pos);
    assert!((gb.pos[0] - 6.950).abs() < 0.1 && (gb.pos[2] - 2.200).abs() < 0.1, "guard B at (6.95, 2.20), got {:?}", gb.pos);
    // Guard A is the sniper (dtypenew 2 → start state 0x29), guard B the dtypenew-0 combat guard.
    assert_eq!(ga.mode, 2, "guard A dtypenew 2 (sniper)");
    assert_eq!(gb.mode, 0, "guard B dtypenew 0 (normal combat — he would throw, if he ever activated)");
    assert_eq!(ga.switch_channel, 0x26, "guard A waits on the plate channel 0x26");
    assert_eq!(gb.switch_channel, 5, "guard B waits on channel 5");
    // [D] No trigger on the disc outputs channel 5 (the plates 311/312 output 0x26).
    assert!(!sim.triggers.iter().any(|t| t.out == 5), "no 07000001 trigger outputs channel 5");
    assert!(sim.triggers.iter().any(|t| t.out == 0x26), "the plates that wake guard A exist");

    // Guard A: activate his plate channel (what trigger 311/312 does) and engage him at his post.
    // [M] The sniper states never call `DroneWeap_CanThrowGrenade` — the throw decision exists only in
    // `DroneFunc_CombatState` 0x148338 (states 0x68/0x70).
    sim.set_channel(0x26);
    let bond = bond_near(&l.world, ga.pos);
    let mut fought = false;
    for t in 0..2400 {
        sim.tick(&d, &bond);
        let s = sim.list[a].state;
        if matches!(s, state::SNIPER_FIRE | state::SNIPER_RELOAD) { fought = true; }
        assert!(matches!(s, state::SNIPER_IDLE | state::SNIPER_AIM | state::SNIPER_FIRE | state::SNIPER_RELOAD),
            "t{t}: sniper guard A left the sniper family into {s:#x} (the game's snipers never do)");
        assert!(sim.grenades.is_empty(), "t{t}: the sniper threw a grenade (the game's sniper states cannot)");
    }
    assert!(fought, "guard A never fought — the engagement must be real for the lock to mean anything");
    assert_eq!(sim.list[a].grenade_count, 3, "the sniper keeps his 3 stun grenades");

    // Guard B: his channel is never raised (no trigger, no script, no pickup, no death channel — [D]),
    // so he stays a disabled waiter: no AI, no draw — the game's furniture behaviour.
    for _ in 0..1200 { sim.tick(&d, &bond); }
    assert_eq!(sim.list[b].state, state::WAIT_SWITCH, "guard B never leaves WaitSwitch");
    assert!(!sim.list[b].enabled && !sim.list[b].drawn, "guard B is inactive and not drawn");
    assert_eq!(sim.list[b].grenade_count, 1, "guard B keeps his stun grenade");
}
