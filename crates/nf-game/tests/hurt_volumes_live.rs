//! hurt-volumes (placement 0xE4, `Hurt_Create` 0x17e870 / `Hurt_Update` 0x17e5d0, class 0x27) on the
//! real levels through the real Sim path. The game's damage flow [M]:
//! `Player_CollisionHandler` 0x1949a8:141-152 → `Player_DealWithObjHit` 0x194458:94-101 case 0x27 →
//! `Player_HandlePain(hitdata+8 + Hurt_GetDamage(v), BL, Hurt_GetType(v), -1)`.
//!
//! Disc data [D, this branch's probe]: 07000005 has 13 hurt placements (objects 151-156), all
//! type 6 damage 1000 — instant-death volumes; 07000002 has one (type 1, damage 1, channel 141);
//! 07000046 has seven animated one-shots (type 1, damage 105, keys 6, rate 90, mask 2 = bullets).
//!
//! Opt-in: set `NF_ISO` to the disc image.

use nf_formats::anim::Script;
use nf_formats::iso9660::IsoImage;
use nf_formats::{disc, drone, level::Level, levelbin::LevelBin, weapon::WeaponData};
use nf_game::drone::{Data, Drones, Player, Rng};
use std::collections::HashMap;

pub struct L {
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
    volumes: Vec<nf_game::aivolume::AiVolume>,
    models: HashMap<u32, nf_game::pickup::Model>,
    tuning: nf_formats::tuning::Tuning,
    hitboxes: nf_formats::hitbox::CollisionBones,
    sfx: Option<nf_formats::sfx::SfxTable>,
    id: u32,
}

impl L {
    pub fn load(name: &str, id: u32) -> Option<L> {
        let path = std::env::var_os("NF_ISO")?;
        let iso = IsoImage::open(&path).unwrap();
        let elf = iso.read_file(iso.find("/ACTION.ELF").unwrap()).unwrap();
        let (entries, bin) = disc::files_bin_entries(&iso).unwrap();
        let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(name)).unwrap();
        let data = iso.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).unwrap();
        let level = Level::parse(&data).unwrap();
        let lb = LevelBin::parse(&data).unwrap();
        let scripts = lb.entries.iter().filter(|e| e.kind == 5).filter_map(|e| Script::parse(lb.data(&data, e)).ok()).map(|s| (s.hashcode, s)).collect();
        let world = nf_game::world::CollisionWorld::build(&level, &level.instances(true).0);
        let skeletons = lb.entries.iter().filter(|e| e.kind == 6).filter_map(|e| nf_formats::character::parse_skeleton(lb.data(&data, e)).ok().map(|s| (s.id as u8, s))).collect();
        let m = level.main_map().unwrap();
        let nav = nf_game::nav::Network::build(level.maps[m].ai_network.as_ref().unwrap(), &world, id);
        let covers = nf_game::cover::build(&level, &nav, &world);
        let volumes = nf_game::aivolume::build(&level, id);
        Some(L {
            tables: drone::DroneAnimData::from_action_elf(&elf).unwrap(),
            props: drone::PropertyTables::from_action_elf(&elf).unwrap(),
            weapons: WeaponData::from_action_elf(&elf).unwrap(),
            tuning: { let mut t = nf_formats::tuning::Tuning::from_action_elf(&elf).unwrap(); nf_game::tuning::read_tuning_vars(&mut t, id); t },
            hitboxes: nf_formats::hitbox::CollisionBones::from_action_elf(&elf).unwrap(),
            sequences: disc::level_sequences(&iso, name).unwrap(),
            models: nf_game::pickup::models(&level),
            sfx: nf_formats::sfx::SfxTable::from_action_elf(&elf).ok(),
            level, scripts, world, skeletons, nav, covers, volumes, id,
        })
    }

    pub fn data(&self) -> Data<'_> {
        Data { tables: &self.tables, props: &self.props, scripts: &self.scripts, weapons: &self.weapons, world: &self.world, level: self.id,
            sequences: &self.sequences, skins: &self.level.skins, skeletons: &self.skeletons, nav: &self.nav, covers: &self.covers, tuning: &self.tuning,
            hitboxes: &self.hitboxes, volumes: &self.volumes, sfx: self.sfx.as_ref(), models: &self.models, surfaces: None, voices: None, difficulty: 1, ai_points: &[] }
    }

    /// The full roster through `level_drones` (the game's order); a fresh `Drones` rng with the
    /// level attached (the world objects — the hurt volumes among them — are built there).
    pub fn sim(&self) -> (Drones, Rng) {
        let d = self.data();
        let mut rng = Rng::new(0x1234_5678);
        let mut lights = nf_game::light::LightList::default();
        let drones = nf_game::drone::level_drones(&d, &self.level, &mut rng, &mut lights);
        let mut s = Drones::new(drones, Rng::new(0x777));
        s.attach_level(&self.level);
        (s, Rng::new(0x777))
    }
}

/// One level archive through the real parser (the light path for the no-Sim tests).
fn level_of(name: &str) -> Option<Level> {
    let path = std::env::var_os("NF_ISO")?;
    let iso = IsoImage::open(&path).unwrap();
    let (entries, bin) = disc::files_bin_entries(&iso).unwrap();
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(name))?;
    let data = iso.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).unwrap();
    Level::parse(&data).ok()
}

fn crate_touch(p: [f32; 3]) -> nf_game::triggers::Toucher {
    nf_game::triggers::Toucher { contain: true, ..nf_game::triggers::Toucher::player([p[0], p[1] - 0.4, p[2]]) }
}

fn bond_at(p: [f32; 3]) -> Player { Player { pos: p, target: [p[0], p[1] + 0.5, p[2]], eye: [p[0], p[1] + 1.0, p[2]], aim: [-1.0, 0.0, 0.0], sph: p, capsule: ([p[0], p[1] + 1.086, p[2]], p), hanging: false, cel: None } }

/// The hurt volumes the Sim created for a level, with their bounds.
fn hurts(sim: &Drones) -> &Vec<nf_game::worldobj::Hurt> { &sim.worldobjs.hurts }

#[test]
fn level_05_creates_the_thirteen_death_volumes() {
    let Some(l) = L::load("07000005.bin", 0x0700_0005) else { eprintln!("skip: NF_ISO not set"); return };
    let (mut sim, _rng) = l.sim();
    let d = l.data();
    // One tick so attach + update passes have run.
    sim.tick(&d, &bond_at([0.0, 5.0, 0.0]));
    let hs = hurts(&sim);
    assert_eq!(hs.len(), 13, "07000005's 0xE4 placements [D probe]");
    // The disc params: type 6, damage 1000, no channel, sound default, one-shot off, no keys.
    for h in hs {
        assert_eq!((h.kind, h.damage, h.channel, h.oneshot, h.count), (6, 1000.0, 0, 0, 0), "the 05 param block [D]");
        assert_eq!(h.sound, 0xffff, "param 3 = 0 → the 0xffff default [M 0x17e870:40-44]");
        assert!(!h.inactive, "no channel → the 0x20 gate never sets [M 0x17e870:56-58]");
        assert!(h.mask == 0 || h.mask == 1, "the 05 masks [D probe: ten 0x1, three 0x0]");
    }
    assert_eq!(hs.iter().filter(|h| h.mask == 0).count(), 3, "the three mask-0 volumes (placements 331-333) [D probe]");
    // The volumes carry world geometry (the placement object's glist) — the death planes are flat
    // 2-triangle quads, the plunge catcher (object 155) a 6-triangle box [D probe].
    assert!(hs.iter().all(|h| !h.tris.is_empty()), "every volume resolved a glist");
    assert!(hs.iter().filter(|h| h.tris.len() == 2).count() >= 10, "the flat death quads [D probe]");
}

/// Standing in one of 05's volumes kills Bond at the game's rate (1000 damage type 6 per contact
/// tick → health floors at 0 the first tick); standing outside leaves him untouched.
#[test]
fn standing_in_an_05_volume_kills_and_outside_does_nothing() {
    let Some(l) = L::load("07000005.bin", 0x0700_0005) else { eprintln!("skip: NF_ISO not set"); return };
    let d = l.data();
    // A volume to stand in: the mask-0 death quad (placement 331, object 152) whose plane sits at
    // y ≈ 5.11 spanning x 80.8..168.4, z 96.7..144.7 [D probe]. Bond's feet ON the plane.
    let (mut sim, _rng) = l.sim();
    sim.tick(&d, &bond_at([0.0, 5.0, 0.0]));
    let spot = {
        let hs = hurts(&sim);
        let i = hs.iter().position(|h| h.mask == 0
            && (124.58 - (h.lo[0] + h.hi[0]) * 0.5).abs() < 2.0
            && (120.67 - (h.lo[2] + h.hi[2]) * 0.5).abs() < 2.0).expect("the (124, 5.11, 120) death quad");
        [hurts(&sim)[i].lo[0] + 1.0, hurts(&sim)[i].hi[1], hurts(&sim)[i].lo[2] + 1.0]
    };
    // The Sim's toucher (Toucher::player(pos−0.4): a = feet+0.15, b = feet+1.46, r 0.55) reaches
    // the plane from 0.15 above it.
    let on = bond_at(spot);
    let (mut sim, _rng) = l.sim();
    assert_eq!(sim.health.health, 100.0);
    sim.tick(&d, &on);
    assert_eq!(sim.health.health, 0.0, "one contact tick of 1000·0.6 type-6 damage floors health [M 0x194458:94-101]");
    // Sister assertion: Bond at (0, 5, 0) — inside the mask-1 plunge box's AABB (x −51.6..36.8,
    // y −0.5..22.9, z −137..6.1 [D probe]) but clear of its mesh by > 0.55 on every face, the same
    // inside-a-big-volume case the shipped trigger model resolves as no contact (fix-objects2) —
    // takes no damage in 60 ticks.
    let far = bond_at([0.0, 5.0, 0.0]);
    let (mut sim, _rng) = l.sim();
    for _ in 0..60 { sim.tick(&d, &far); }
    assert_eq!(sim.health.health, 100.0, "60 ticks clear of every volume mesh: no damage");
}

/// The 07000002 trickle volume (type 1, damage 1, channel 141 [D probe]): the gate — no channel,
/// no damage; channel on, 1 damage per contact tick through the same Sim path.
#[test]
fn the_02_volume_is_channel_gated() {
    let Some(l) = L::load("07000002.bin", 0x0700_0002) else { eprintln!("skip: NF_ISO not set"); return };
    let d = l.data();
    let (mut sim, _rng) = l.sim();
    sim.tick(&d, &bond_at([0.0, 5.0, 0.0]));
    let hs = hurts(&sim);
    assert_eq!(hs.len(), 1, "07000002's single 0xE4 placement [D probe]");
    let h = &hs[0];
    assert_eq!((h.kind, h.damage, h.channel, h.mask), (1, 1.0, 141, 1), "the 02 param block [D]");
    assert!(h.inactive, "create sets the gate for an off channel [M 0x17e870:56-58]");
    assert_eq!(h.tris.len(), 12, "the volume's object carries its glist (a small box, [D probe])");
    // The gate through the Sim: the box at (−60.99, 7.33, −30.16) [D probe] — Bond's capsule inside
    // it, the gate at its created state (off): nothing (the 0x20 gate keeps the volume out of the
    // contact pass [M Collide_PickObj skip]).
    let in_box = bond_at([-60.99, 7.33, -30.16]);
    let (mut sim, _rng) = l.sim();
    sim.tick(&d, &in_box);
    assert_eq!(sim.health.health, 100.0, "gate off: standing in the volume does nothing");
    // The gate's ON side at the Hurt level (the Sim's own script machinery owns channel 141 in play
    // — this pin drives the channel directly, exactly what `Hurt::tick` reads): the real placements
    // and triangles, the Sim's toucher shape, 1 damage × 0.5 (the level's difficulty-1 factor,
    // bullet.rs damage_factor 0x02 → e) per contact tick.
    let m = l.level.main_map().unwrap();
    let mut hurts = nf_game::worldobj::Hurt::create(&l.level.maps[m].placements, &l.level.path_blocks(), |p| {
        let target = if p.by_hash() { l.level.by_hash.get(&p.hashcode).copied() }
            else if (p.index as usize) < l.level.maps[m].objects.len() { Some((m, p.index as usize)) } else { None };
        let mx = nf_formats::placement::object_matrix(p.rotation, p.translation);
        target.and_then(|(om, oi)| l.level.maps[om].objects[oi].collision.as_ref())
            .map(|c| c.triangles().into_iter().map(|t| t.map(|v| nf_formats::placement::transform(&mx, v))).collect())
            .unwrap_or_default()
    });
    assert_eq!(hurts.len(), 1);
    let ch = nf_game::triggers::Channels::default();
    let toucher = crate_touch([-60.99, 7.33, -30.16]);
    assert!(hurts[0].touched_by(&toucher), "the capsule is in contact with the box's mesh");
    let mut ch = ch;
    assert_eq!(hurts[0].pain(&toucher), None, "gate off: no pain");
    ch.set(141, 1, 0);
    hurts[0].tick(&ch); // the update pass precedes the collision pass [M 0x1336f8 pass order]
    assert_eq!(hurts[0].pain(&toucher), Some((1.0, 1)), "gate on: the volume's damage and type [M 0x194458:94-101]");
    assert_eq!(hurts[0].state, 1, "the channel arm [M 0x17e5d0:97-99]");
}

/// The 07000046 animated one-shots (type 1, damage 105, keys 6, rate 90, mask 2 [D probe]): the
/// state machine's frame math and the one-shot expiry, through `Hurt::create` + `Hurt::tick` on the
/// real placements (the level has no AI network, so no Sim here — these are the same calls
/// `attach_level` and `WorldObjects::tick` make; the triangles are irrelevant to the frame math).
#[test]
fn the_46_one_shots_sweep_once_then_go_inert() {
    let Some(level) = level_of("07000046.bin") else { eprintln!("skip: NF_ISO not set"); return };
    let m = level.main_map().unwrap();
    let path_blocks = level.path_blocks();
    let hs = nf_game::worldobj::Hurt::create(&level.maps[m].placements, &path_blocks, |_| Vec::new());
    assert_eq!(hs.len(), 7, "07000046's seven 0xE4 placements [D probe]");
    for h in &hs {
        assert_eq!((h.kind, h.damage, h.oneshot, h.count, h.rate, h.mask), (1, 105.0, 1, 6, 90, 2), "the 46 param block [D]");
        assert_eq!(h.keys.len(), 6, "the 0x19 key list resolved (m_paths[placement+0x44])");
        // The frame increment [M 0x17e870:47-50] in the game's op order at the load-time rate
        // (FRAME_RATE 30, REC_FRAME_RATE = 1/30 [L door.rs CREATE_FRAME_RATE]):
        // (5·(1/30))·(30/90) — magnitude 5/90 per tick.
        assert_eq!(h.step.to_bits(), ((5.0f32 * (1.0 / 30.0)) * (30.0f32 / 90.0)).to_bits());
        assert!((h.step - 5.0 / 90.0).abs() < 1e-6);
    }
    // Drive the state machine: arm (mask != 0 with no channel never arms in the port — force state 1
    // the way a bullet contact would [S]) and tick to expiry.
    let mut h = hs[0].clone();
    h.state = 1;
    let ch = nf_game::triggers::Channels::default();
    let mut ticks = 0;
    while h.state == 1 {
        h.tick(&ch);
        ticks += 1;
        assert!(ticks < 200, "the sweep must expire (5 frames at 5/90 per tick ≈ 90 ticks)");
    }
    assert_eq!(h.state, 2, "one-shot flag 1 → spent [M 0x17e5d0:129-132]");
    assert!((85..=95).contains(&ticks), "frame 0 → 5 at step ≈5/90: ≈90 ticks, got {ticks}");
    let spent = h.clone();
    h.tick(&ch);
    assert_eq!(spent.state, 2, "spent volumes stay spent (the class byte dropped to 1)");
}
