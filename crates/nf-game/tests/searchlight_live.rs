//! Searchlights against the live layout in `work/ee_start_05.bin` (WORKLOG 0070/0072): the 29 parts of
//! 07000005, the link structure (including the orphan spot), the dark heads' channels and the drawn flags
//! at level start, the create-time RNG consumption, and the channel/bullet behaviour.
//! Opt-in: set `NF_ISO` to the disc image.

use nf_formats::iso9660::IsoImage;
use nf_formats::{disc, drone, level::Level, levelbin::LevelBin, weapon::WeaponData};
use nf_game::drone::{Data, Player, Rng};
use nf_game::searchlight::Searchlights;
use std::collections::HashMap;

struct Level05 {
    tables: drone::DroneAnimData,
    props: drone::PropertyTables,
    weapons: WeaponData,
    level: Level,
    scripts: HashMap<u32, nf_formats::anim::Script>,
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
        let scripts = lb.entries.iter().filter(|e| e.kind == 5).filter_map(|e| nf_formats::anim::Script::parse(lb.data(&data, e)).ok()).map(|s| (s.hashcode, s)).collect();
        let world = nf_game::world::CollisionWorld::build(&level, &level.instances(true).0);
        let skeletons = lb.entries.iter().filter(|e| e.kind == 6).filter_map(|e| nf_formats::character::parse_skeleton(lb.data(&data, e)).ok()).map(|k| (k.id as u8, k)).collect();
        let m = level.main_map().unwrap();
        let nav = nf_game::nav::Network::build(level.maps[m].ai_network.as_ref().unwrap(), &world, 0x0700_0005);
        let covers = nf_game::cover::build(&level.maps[m].placements, &nav, &world);
        let volumes = nf_game::aivolume::build(&level, 0x0700_0005);
        let sequences = disc::level_sequences(&iso, "07000005.bin").unwrap();
        Some(Level05 {
            tables: drone::DroneAnimData::from_action_elf(&elf).unwrap(),
            props: drone::PropertyTables::from_action_elf(&elf).unwrap(),
            weapons: WeaponData::from_action_elf(&elf).unwrap(),
            sfx: nf_formats::sfx::SfxTable::from_action_elf(&elf).unwrap(),
            tuning: { let mut t = nf_formats::tuning::Tuning::from_action_elf(&elf).unwrap(); nf_game::tuning::read_tuning_vars(&mut t, 0x0700_0005); t },
            hitboxes: nf_formats::hitbox::CollisionBones::from_action_elf(&elf).unwrap(),
            models: nf_game::pickup::models(&level),
            level, scripts, world, sequences, skeletons, nav, covers, volumes,
        })
    }

    fn data(&self) -> Data<'_> {
        Data { tables: &self.tables, props: &self.props, scripts: &self.scripts, weapons: &self.weapons, world: &self.world, level: 0x0700_0005,
            sequences: &self.sequences, skins: &self.level.skins, skeletons: &self.skeletons, nav: &self.nav, covers: &self.covers, tuning: &self.tuning, hitboxes: &self.hitboxes, volumes: &self.volumes,
            sfx: Some(&self.sfx), models: &self.models, surfaces: None, voices: None, difficulty: 1 }
    }
}

fn bond_at(p: [f32; 3]) -> Player { Player { pos: p, target: [p[0], p[1] + 0.5, p[2]], eye: [p[0], p[1] + 1.0, p[2]], aim: [-1.0, 0.0, 0.0], ..Default::default() } }

/// The parts created exactly as `attach_level` does (the level's object boxes for the bullet stand-in).
fn parts(l: &Level05, rng: &mut Rng) -> Searchlights {
    let m = l.level.main_map().unwrap();
    Searchlights::create(&l.level.maps[m].placements, rng, |p| {
        let obj = if p.by_hash() { l.level.by_hash.get(&p.hashcode).map(|&(map, object)| &l.level.maps[map].objects[object]) }
            else { l.level.maps[m].objects.get(p.index as usize) };
        obj.map_or(([0.0; 3], [0.0; 3]), |o| (o.min, o.max))
    })
}

#[test]
fn the_live_layout_reproduces() {
    let Some(l) = Level05::load() else { eprintln!("skip: NF_ISO not set"); return };
    let mut rng = Rng::new(1);
    let s = parts(&l, &mut rng);
    // [L] 29 parts: 7 mounts, 7 heads, 6 beams, 7 coronas, 2 spots.
    let count = |k: u8| s.parts.iter().filter(|p| p.kind == k).count();
    assert_eq!((s.parts.len(), count(0), count(1), count(2), count(3), count(4)), (29, 7, 7, 2, 6, 7));
    let heads: Vec<usize> = s.parts.iter().enumerate().filter(|(_, p)| p.kind == 1).map(|(i, _)| i).collect();
    // [L] The two dark heads wait on channels 97 (0x61) and 164 (0xA4); the other five sweep.
    let dark: Vec<(u8, usize)> = heads.iter().map(|&h| (s.parts[h].on, h)).filter(|&(c, _)| c != 0).collect();
    let mut dark_chans: Vec<u8> = dark.iter().map(|&(c, _)| c).collect();
    dark_chans.sort_unstable();
    assert_eq!(dark_chans, vec![97, 164]);
    for &(c, h) in &dark {
        assert_eq!(s.parts[h].state, if c != 0 { 3 } else { 0 });
        // The dark head's non-mount parts start dark; the head and mount meshes stay drawn [L].
        for k in [2usize, 3, 4] {
            if let Some(p) = s.parts[h].links[k] { assert!(!s.parts[p].drawn, "channel {c} part {k}"); }
        }
        assert!(s.parts[h].drawn);
        assert!(s.parts[h].links[0].is_some_and(|p| s.parts[p].drawn));
    }
    assert_eq!(heads.len() - dark.len(), 5);
    // [L] Head 016e95a0 (channel 164, at (−57.03, 37.84, −59.97)) links mount, spot, beam and corona; its spot
    // was created after it (placement 31 > 27).
    let h164 = dark.iter().find(|&&(c, _)| c == 164).unwrap().1;
    let spot = s.parts[h164].links[2].expect("the channel-164 head's spot");
    assert!(s.parts[h164].links[3].is_some() && s.parts[h164].links[4].is_some());
    // The spot waits 10000 below its placement until the first ray hit [M].
    let pl = &l.level.maps[l.level.main_map().unwrap()].placements[s.parts[spot].placement];
    assert!((s.parts[spot].pos[0] - (pl.translation[0] - 10000.0)).abs() < 1e-4);
    assert!((s.parts[spot].pos[1] - (pl.translation[1] - 10000.0)).abs() < 1e-4);
    assert!((s.parts[spot].pos[2] - (pl.translation[2] - 10000.0)).abs() < 1e-4);
    assert!(s.parts[spot].light.is_some(), "the spot's flicker seed");
    // [L] Head 016dbb60 (channel 97, at (−61.98, 37.84, −55.75)) has no beam placement and no spot.
    let h97 = dark.iter().find(|&&(c, _)| c == 97).unwrap().1;
    assert_eq!((s.parts[h97].links[2].is_none(), s.parts[h97].links[3].is_none(), s.parts[h97].links[4].is_some()), (true, true, true));
    // [L] The five sweeping heads link mount, beam and corona but no spot: the courtyard spot (placement 33)
    // was created before its head, so its own −10000 move puts it out of the head's 3-unit link range —
    // the orphan (016e8ec0).
    for &h in &heads {
        if s.parts[h].on == 0 {
            assert!(s.parts[h].links[0].is_some() && s.parts[h].links[3].is_some() && s.parts[h].links[4].is_some(), "sweeping head {h}");
            assert!(s.parts[h].links[2].is_none(), "no sweeping head has a spot");
        }
    }
    let linked_spots: Vec<usize> = heads.iter().filter_map(|&h| s.parts[h].links[2]).collect();
    assert_eq!(linked_spots.len(), 1, "exactly one spot is linked");
    let orphan = (0..s.parts.len()).find(|&i| s.parts[i].kind == 2 && !linked_spots.contains(&i)).unwrap();
    assert!(!s.parts[orphan].drawn);
    // [L] The heads' (half, mid) bits (ee_start_05.bin, golden_bits.py).
    let golden: [([f32; 3], u32, u32); 7] = [
        ([-61.98, 37.84, -55.75], 0x3de8_56a0, 0x40c4_508c),
        ([62.17, 34.36, 107.01], 0x3e9c_61ab, 0x3e9c_61ab),
        ([59.80, 46.69, 53.57], 0x3f1c_61ac, 0x409f_2c8d),
        ([0.27, 46.75, 53.61], 0x3f1c_61ac, 0x409f_2c8d),
        ([22.72, 46.71, 53.61], 0x3eb2_b8c3, 0x3eb2_b8c3),
        ([37.27, 46.71, 53.67], 0x3eb2_b8c3, 0x3eb2_b8c3),
        ([-57.03, 37.84, -59.97], 0x3db2_b8c0, 0x40c3_7a15),
    ];
    for h in &heads {
        let p = &s.parts[*h];
        let g = golden.iter().find(|g| (g.0[0] - p.pos[0]).abs() < 0.05 && (g.0[1] - p.pos[1]).abs() < 0.05 && (g.0[2] - p.pos[2]).abs() < 0.05).unwrap();
        assert_eq!((p.half.to_bits(), p.mid.to_bits()), (g.1, g.2), "head at {:?}", p.pos);
    }
}

#[test]
fn create_consumes_the_shared_stream_in_order() {
    let Some(l) = Level05::load() else { eprintln!("skip: NF_ISO not set"); return };
    // 29 parts each draw Rand_Rand(480); the two spots another Rand_Rand(0): 31 draws in placement
    // order [M]. `Rand_Rand` advances y then x per call regardless of the bound.
    let mut a = Rng::new(7);
    let s = parts(&l, &mut a);
    assert_eq!(s.parts.len(), 29);
    let mut b = Rng::new(7);
    for _ in 0..31 { b.rand(480); }
    // Sister: 30 draws would leave the streams apart.
    let mut c = Rng::new(7);
    for _ in 0..30 { c.rand(480); }
    assert_eq!(a.rand(1000), b.rand(1000));
    assert_ne!(a.rand(1000), c.rand(1000));
}

#[test]
fn channels_and_tracking_behave() {
    let Some(l) = Level05::load() else { eprintln!("skip: NF_ISO not set"); return };
    let d = l.data();
    let mut rng = Rng::new(3);
    let mut s = parts(&l, &mut rng);
    let mut channels = nf_game::triggers::Channels::default();
    let far_away = bond_at([500.0, 100.0, 500.0]);
    let heads: Vec<usize> = s.parts.iter().enumerate().filter(|(_, p)| p.kind == 1 && p.on == 0).map(|(i, _)| i).collect();
    let h = heads[0];
    // Sweeping: the pitch and yaw move with the phase.
    let (p0, y0) = (s.parts[h].pitch, s.parts[h].yaw);
    let mut rrng = Rng::new(11);
    s.update(&d, &far_away, &mut channels, h, &[], &mut rrng, 1);
    assert_ne!((s.parts[h].pitch, s.parts[h].yaw), (p0, y0));
    // Its beam is drawn and oriented along the aim.
    let beam = s.parts[h].links[3].unwrap();
    assert!(s.parts[beam].drawn);
    let m = s.parts[beam].matrix;
    let aim = nf_game::searchlight::spherical(1.0, s.parts[h].yaw, -s.parts[h].pitch);
    for k in 0..3 { assert!((m[2][k] - aim[k]).abs() < 1e-5, "beam row 2 = aim"); }
    // A bullet through the head's box breaks it: state 2, parts hidden, the glass sound.
    let hp = s.parts[h].pos;
    let evs = s.update(&d, &far_away, &mut channels, h, &[(hp, [hp[0] + 5.0, hp[1], hp[2]])], &mut rrng, 2);
    assert_eq!(s.parts[h].state, 2);
    assert_eq!(evs, vec![(0xe6, hp)]);
    assert!(!s.parts[beam].drawn);
    // The dark head re-lights on its channel with the relay clunk.
    let dark: Vec<usize> = s.parts.iter().enumerate().filter(|(_, p)| p.kind == 1 && p.on == 164).map(|(i, _)| i).collect();
    let hd = dark[0];
    channels.set(164, 1, 3);
    let evs = s.update(&d, &far_away, &mut channels, hd, &[], &mut rrng, 3);
    assert_eq!(s.parts[hd].state, 0);
    assert_eq!(evs, vec![(0xe5, s.parts[hd].pos)]);
    assert!(s.parts[s.parts[hd].links[3].unwrap()].drawn, "the beam re-lights");
    // Its spot ray repositions it from −10000 onto the world (the turn-on tick aims at the create-time
    // pitch 0; the first sweep tick then moves it to the sweep cone, where the light actually points).
    let sp = s.parts[hd].links[2].unwrap();
    assert!(s.parts[sp].pos[1] > -9000.0, "the spot moved to its ray hit: {:?}", s.parts[sp].pos);
    s.update(&d, &far_away, &mut channels, hd, &[], &mut rrng, 4);
    // Bond inside the circle arms the light; the head holds its alarm channel while tracking. The spot
    // crawls ≤ 0.1 units per sweep tick, so Bond stands where it just landed.
    let at_spot = bond_at(s.parts[sp].pos);
    s.update(&d, &at_spot, &mut channels, hd, &[], &mut rrng, 5);
    assert_eq!(s.parts[hd].state, 1, "tracking");
    assert!(nf_game::triggers::Channels::on(&channels, s.parts[hd].alarm as u32) || s.parts[hd].alarm == 0);
}
