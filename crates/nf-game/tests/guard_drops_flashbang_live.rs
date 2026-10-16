//! guard-drops-flashbang (task 681) on 07000005 — the playtest 1011-1655 symptoms:
//! "all enemies are dropping multiple objects" and "the flashbang item is killing enemies even if I
//! dont throw it directly at them".
//!
//! The game's drops [M]:
//! - `DroneWeap_DropWeapon` 0x17a2a0: at the death anim (Death_Anim 0x1637f8 case 3's `+0x3b`/weapon
//!   4/9 gate) the guard drops his HELD gun — one `Pickup_CreateSimple` of `weapon_data[w].base` with
//!   `clip_size · (0.25 or 0.5 + FRand(0.5))` rounds; the weapon byte is cleared so every later call
//!   is a no-op. `weapon_data[1]` (fists) carries model 0 at +0x80 [L record 1], so a flashed guard
//!   (`StunGrenadeImpact` 0x172bb0: DropWeapon + ChangeWeapon(1)) drops NOTHING further when killed.
//! - `DroneFunc_OnInitDeath` 0x145bd0:113-145: the grenade drop needs `Drone+0x19` (the captain flag,
//!   `NDrone2_DefaultInit` 0x14b300:495-531 — behaviour prop 9 on difficulty 1 / 5 on 2 / 8 on 3, kind
//!   ∈ {1,2,3,6,7}); 07000005's 16 guards are all kind 2 with props 9/5 clear [L run of this file's
//!   harness], so NO guard here drops grenades — each death is exactly ONE pickup.
//! - the stun grenade 0x35 [L record: radius 10, damage 0, `+0x70` = 0x2280]: its fuse death takes
//!   `Bullet_DoTrails` 0x12d890's `& 0x2000` flash branch — `Explode_Propagate` is never reached, so
//!   a flashbang cannot damage anyone (direct, near or far).
//!
//! Opt-in: set `NF_ISO` to the disc image.

use nf_formats::anim::Script;
use nf_formats::iso9660::IsoImage;
use nf_formats::{disc, drone, level::Level, levelbin::LevelBin, weapon::WeaponData};
use nf_game::drone::{state, Data, Drones, Player, Rng};
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
}

impl L {
    pub fn load() -> Option<L> {
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
        let skeletons = lb.entries.iter().filter(|e| e.kind == 6).filter_map(|e| nf_formats::character::parse_skeleton(lb.data(&data, e)).ok().map(|s| (s.id as u8, s))).collect();
        let m = level.main_map().unwrap();
        let nav = nf_game::nav::Network::build(level.maps[m].ai_network.as_ref().unwrap(), &world, 0x0700_0005);
        let covers = nf_game::cover::build(&level.maps[m].placements, &nav, &world);
        let volumes = nf_game::aivolume::build(&level, 0x0700_0005);
        Some(L {
            tables: drone::DroneAnimData::from_action_elf(&elf).unwrap(),
            props: drone::PropertyTables::from_action_elf(&elf).unwrap(),
            weapons: WeaponData::from_action_elf(&elf).unwrap(),
            tuning: { let mut t = nf_formats::tuning::Tuning::from_action_elf(&elf).unwrap(); nf_game::tuning::read_tuning_vars(&mut t, 0x0700_0005); t },
            hitboxes: nf_formats::hitbox::CollisionBones::from_action_elf(&elf).unwrap(),
            sequences: disc::level_sequences(&iso, "07000005.bin").unwrap(),
            models: nf_game::pickup::models(&level),
            sfx: nf_formats::sfx::SfxTable::from_action_elf(&elf).ok(),
            level, scripts, world, skeletons, nav, covers, volumes,
        })
    }

    pub fn data(&self) -> Data<'_> {
        Data { tables: &self.tables, props: &self.props, scripts: &self.scripts, weapons: &self.weapons, world: &self.world, level: 0x0700_0005,
            sequences: &self.sequences, skins: &self.level.skins, skeletons: &self.skeletons, nav: &self.nav, covers: &self.covers, tuning: &self.tuning,
            hitboxes: &self.hitboxes, volumes: &self.volumes, sfx: self.sfx.as_ref(), models: &self.models, surfaces: None, voices: None, difficulty: 1 }
    }

    /// The full roster with one Rng through `level_drones` (the game's order), then a fresh `Drones`
    /// rng for the tick stream. Difficulty 1 pinned (the harness convention; captains would need prop
    /// 9, and 07000005 has none — the roster assertion below locks that).
    pub fn sim(&self) -> (Drones, Rng) {
        let d = self.data();
        let mut rng = Rng::new(0x1234_5678);
        let mut lights = nf_game::light::LightList::default();
        let drones = nf_game::drone::level_drones(&d, &self.level, &mut rng, &mut lights);
        let s = Drones::new(drones, Rng::new(0x777));
        (s, Rng::new(0x777))
    }
}

pub fn bond_at(p: [f32; 3]) -> Player { Player { pos: p, target: [p[0], p[1] + 0.5, p[2]], eye: [p[0], p[1] + 1.0, p[2]], aim: [-1.0, 0.0, 0.0], sph: p, capsule: ([p[0], p[1] + 1.086, p[2]], p), hanging: false, cel: None } }

/// A Bond spot ~6 units from `g` whose firing line (eye → the guard's chest) passes no OTHER live
/// guard within 1.2 units AND has a clear world line — the yard pairs stand in each other's axes
/// and the tower posts are walled.
pub fn clear_bond_spot(sim: &Drones, d: &Data, i: usize) -> Player {
    let g = sim.list[i].pos;
    let offs = [[2.574, 0.03], [-2.574, 0.03], [0.03, 2.574], [0.03, -2.574], [-6.0, 0.0], [6.0, 0.0], [0.0, -6.0], [0.0, 6.0], [-4.2, -4.2], [4.2, 4.2], [-4.2, 4.2], [4.2, -4.2]];
    'outer: for o in offs {
        let b = [g[0] + o[0], g[1] + 0.5, g[2] + o[1]];
        let eye = [b[0], b[1] + 1.0, b[2]];
        if !d.world.line_of_sight(eye, [g[0], g[1] + 0.6, g[2]]) { continue; }
        let ab = [g[0] - b[0], g[1] - b[1], g[2] - b[2]];
        let l2 = ab[0] * ab[0] + ab[1] * ab[1] + ab[2] * ab[2];
        for (j, other) in sim.list.iter().enumerate() {
            if j == i || !other.present || other.corpse { continue; }
            let ac = [other.pos[0] - b[0], other.pos[1] - b[1], other.pos[2] - b[2]];
            let t = (ac[0] * ab[0] + ac[1] * ab[1] + ac[2] * ab[2]) / l2;
            if !(0.0..=1.0).contains(&t) { continue; }
            let cx = ac[0] - ab[0] * t; let cy = ac[1] - ab[1] * t; let cz = ac[2] - ab[2] * t;
            if cx * cx + cy * cy + cz * cz < 1.44 { continue 'outer; }
        }
        return bond_at(b);
    }
    bond_at([g[0] - 6.0, g[1] + 0.5, g[2]])
}

/// Kill guard `i` through the real path with the level sniper's record (0x24: damage 30 [L] — one
/// body hit ≥ the 10 health kills from any part), then run to the drop.
fn kill_guard(sim: &mut Drones, d: &Data, l: &L, i: usize) {
    let p = clear_bond_spot(sim, d, i);
    let t = sim.list[i].pos;
    let to = [t[0], t[1] + 0.3, t[2]];
    let e = p.eye;
    let n = ((to[0] - e[0]).powi(2) + (to[1] - e[1]).powi(2) + (to[2] - e[2]).powi(2)).sqrt();
    sim.fire_player(d, e, [(to[0] - e[0]) / n, (to[1] - e[1]) / n, (to[2] - e[2]) / n], 0x24, l.sfx.as_ref().map(|s| s.get(0x403)).unwrap());
    for f in 1..=450u32 {
        sim.tick(d, &p);
        if f > 60 && matches!(sim.list[i].state, state::DEAD | state::FADE | state::FADE_FAST) { break; }
    }
}

/// Five guards of the level's different loadouts (skins 0x2e/0x2f/0x30, SG5 0x16 and sniper 0x24),
/// each killed through the real path (`fire_player`, one per fresh run): exactly ONE pickup — the
/// held gun's base id with the game's random rounds — and never a grenade.
#[test]
fn five_guards_drop_exactly_the_game_drops() {
    let Some(l) = L::load() else { eprintln!("skip: NF_ISO not set"); return };
    let d = l.data();
    // placement, expected base weapon, clip_size (for the rounds band), expected rounds band.
    // [L records] 0x16 clip 30 (≥ 10 → 30·(0.25+FRand(0.5)) ∈ [7..=22]); 0x24 clip 5 (< 10 →
    // 5·(0.5+FRand(0.5)) ∈ [2..=7]).
    let cases: [(usize, u16, i16, std::ops::RangeInclusive<i16>); 5] = [
        (3, 0x16, 30, 7..=22),    // skin 0x5000030, SG5, yard
        (7, 0x24, 5, 2..=7),      // skin 0x500002f, sniper, tower post
        (5, 0x16, 30, 7..=22),    // skin 0x500002e, SG5, the gate guard ai_live kills
        (9, 0x24, 5, 2..=7),      // skin 0x5000030, sniper, another tower
        (66, 0x16, 30, 7..=22),   // skin 0x500002e, SG5, the bunker pair
    ];
    for (pl, base, clip, band) in cases {
        let (mut sim, _rng) = l.sim();
        let i = sim.list.iter().position(|dr| dr.placement == pl).expect("placement on the roster");
        let skin = sim.list[i].skin;
        assert_eq!(sim.list[i].weapon as u16, base, "pl {pl} carries the expected gun");
        // The game's captain gate on this level: prop 9 (difficulty 1) is clear for every guard
        // [L the roster run] — so the OnInitDeath grenade drop never fires here.
        assert!(!sim.list[i].prop(&d, 9) && !sim.list[i].prop(&d, 5), "pl {pl} is not a captain at difficulties 1-2");
        kill_guard(&mut sim, &d, &l, i);
        assert!(matches!(sim.list[i].state, state::DEAD | state::FADE | state::FADE_FAST | state::DEATH_ANIM),
            "pl {pl}: guard died, state {:#x}", sim.list[i].state);
        assert_eq!(sim.pickups.len(), 1,
            "pl {pl} (skin {skin:#x}): the game drops exactly ONE object — got {} ({:?})",
            sim.pickups.len(), sim.pickups.iter().map(|p| (p.weapon, p.rounds, p.kind)).collect::<Vec<_>>());
        let pk = &sim.pickups[0];
        assert_eq!((pk.weapon, pk.kind), (base, nf_game::pickup::kind::WEAPON), "pl {pl}: the held gun's pickup");
        assert!(band.contains(&pk.rounds), "pl {pl}: clip {clip} → rounds in {band:?}, got {}", pk.rounds);
        assert!(pk.weapon != 0x35 && pk.weapon != 0x34 && pk.weapon != 0x36, "pl {pl}: no grenade on a non-captain's drop list");
        eprintln!("pl {pl}: 1 drop, weapon {:#x} rounds {}", pk.weapon, pk.rounds);
    }
}

/// The flashbang among three guards (the yard cluster pl 3, 4, 302 — all within the 20-unit stun
/// radius): the fuse death takes the `& 0x2000` flash branch — NO blast, NO health loss, the stun
/// family state, the held gun dropped and the fists equipped [M 0x172bb0 case 1]. Sister: the guard
/// OUTSIDE the radius is untouched (health, no stun family, no drop).
#[test]
fn flashbang_stuns_three_guards_and_never_kills() {
    use nf_game::drone::state;
    let Some(l) = L::load() else { eprintln!("skip: NF_ISO not set"); return };
    let d = l.data();
    let (mut sim, _rng) = l.sim();
    let trio = [3usize, 4, 302];
    let mut idx = [0usize; 3];
    for (k, pl) in trio.iter().enumerate() {
        idx[k] = sim.list.iter().position(|dr| dr.placement == *pl).unwrap();
    }
    // The roster: the yard trio sits within 20 units of the blast point (between them).
    let blast = {
        let p = sim.list[idx[0]].pos;
        [p[0] + 1.9, p[1] + 0.4, p[2]] // between pl 3 (28.3, 37.0, 44.9) and pl 4 (32.1, 37.0, 44.9)
    };
    for (k, pl) in trio.iter().enumerate() {
        let g = sim.list[idx[k]].pos;
        let dist = ((g[0] - blast[0]).powi(2) + (g[1] - blast[1]).powi(2) + (g[2] - blast[2]).powi(2)).sqrt();
        assert!(dist <= 20.0, "pl {pl} inside the stun radius, dist {dist}");
    }
    // A control guard far outside the radius.
    let far_i = sim.list.iter().position(|dr| dr.placement == 271).unwrap();
    let far_dist = {
        let g = sim.list[far_i].pos;
        ((g[0] - blast[0]).powi(2) + (g[1] - blast[1]).powi(2) + (g[2] - blast[2]).powi(2)).sqrt()
    };
    assert!(far_dist > 20.0, "the control guard is outside the radius, dist {far_dist}");

    let healths: Vec<f32> = idx.iter().map(|&i| sim.list[i].health).collect();
    let far_health = sim.list[far_i].health;

    // The throw: exactly what the view's grenade path spawns for a player flashbang
    // (crates/nf-view/src/scene.rs's grenade arm + [L record 0x35]: damage +0x0C = 0, radius
    // +0x08 = 10, fuse 210 (`+0x70 & 0x200`), owner usize::MAX = the player's throw).
    let rec = &d.weapons.records[0x35];
    let f32_at = |o: usize| f32::from_le_bytes(rec[o..o + 4].try_into().unwrap());
    assert_eq!(f32_at(0x0c), 0.0, "the stun record's damage is 0 [L]");
    sim.grenades.push(nf_game::grenade::Grenade {
        pos: blast,
        velocity: [0.0, 0.0, 0.0],
        speed: 0.0,
        travelled: 0.0,
        range: 210.0,
        damage: f32_at(0x0c),
        radius: f32_at(0x08),
        weapon_type: 0x35,
        mesh: 0,
        owner: usize::MAX,
        state: 0,
        bounces: 0,
        smoke_popped: false,
        resting: true, // already at rest at the blast point: the fuse ticks from tick 1
        stuck: false,
        seek_target: None,
        phase: None,
    });

    // Bond far from the blast so his own flash does not matter to the guards' assertions.
    let p = bond_at([blast[0] + 60.0, blast[1], blast[2]]);
    let mut stunned_seen = [false; 3];
    let mut dropped_seen = [false; 3];
    for f in 1..=320u32 {
        sim.tick(&d, &p);
        for (k, &i) in idx.iter().enumerate() {
            let g = &sim.list[i];
            if matches!(g.state, state::STUN_GRENADE_IMPACT | state::STUN_GRENADE_LOOP | state::STUN_GRENADE_RECOVER) { stunned_seen[k] = true; }
            // The disarm: StunGrenadeImpact's ChangeWeapon(1) — the weapon byte is the FISTS id 1,
            // not 0 (the port's `dr.weapon = 1` [M 0x172bb0:17]).
            if g.weapon == 1 { dropped_seen[k] = true; }
            // THE core lock: a flashbang never takes health — not direct, not near, not far.
            assert_eq!(g.health, healths[k], "pl {} took flashbang damage at tick {f}!", trio[k]);
            assert!(!g.corpse && g.present, "pl {} died to a flashbang at tick {f}!", trio[k]);
        }
        assert_eq!(sim.list[far_i].health, far_health, "the control guard was touched at tick {f}");
        assert!(!matches!(sim.list[far_i].state, state::STUN_GRENADE_IMPACT | state::STUN_GRENADE_LOOP | state::STUN_GRENADE_RECOVER | state::DEATH_ANIM | state::DEAD),
            "the control guard was stunned or killed at tick {f}");
    }
    for (k, pl) in trio.iter().enumerate() {
        assert!(stunned_seen[k], "pl {pl}: never entered the stun family");
        assert!(dropped_seen[k], "pl {pl}: never dropped the held gun (StunGrenadeImpact)");
    }
    // No blast on the flash path: no explosion record, no frag script, no damage events.
    assert!(sim.explosions.is_empty(), "the flashbang must not explode: {:?}", sim.explosions);
    // The flash drops: one weapon pickup per flashed guard. The yard's pl 303 also sits within the
    // 20-unit vicinity with a clear line — 4 flashed guards, 4 gun drops, the game's own geometry.
    let flashed = sim.list.iter().filter(|g| g.weapon == 1 && g.present && !g.corpse).count();
    let flash_drops = sim.pickups.iter().filter(|p| !p.gone).count();
    assert_eq!(flash_drops, flashed, "one drop per flashed guard: {:?}", sim.pickups.iter().map(|p| (p.weapon, p.rounds)).collect::<Vec<_>>());
    assert!(flash_drops >= 3, "at least the trio dropped");
    assert!(sim.pickups.iter().all(|p| p.weapon == 0x16), "the drops are the guards' SG5s, got the list above");
}

/// Sister: the frag (0x34, radius 8 / damage 100 [L record]) at the same spot kills the trio —
/// the flashbang's harmlessness is the flash branch, not a broken blast path.
#[test]
fn frag_at_the_same_spot_kills_the_trio() {
    use nf_game::drone::state;
    let Some(l) = L::load() else { eprintln!("skip: NF_ISO not set"); return };
    let d = l.data();
    let (mut sim, _rng) = l.sim();
    let trio = [3usize, 4, 302];
    let mut idx = [0usize; 3];
    for (k, pl) in trio.iter().enumerate() { idx[k] = sim.list.iter().position(|dr| dr.placement == *pl).unwrap(); }
    let blast = { let p = sim.list[idx[0]].pos; [p[0] + 1.9, p[1] + 0.4, p[2]] };
    let rec = &d.weapons.records[0x34];
    let f32_at = |o: usize| f32::from_le_bytes(rec[o..o + 4].try_into().unwrap());
    assert_eq!(f32_at(0x0c), 100.0, "the frag's damage is 100 [L]");
    sim.grenades.push(nf_game::grenade::Grenade {
        pos: blast, velocity: [0.0; 3], speed: 0.0, travelled: 0.0, range: 210.0,
        damage: f32_at(0x0c), radius: f32_at(0x08), weapon_type: 0x34, mesh: 0,
        owner: usize::MAX, state: 0, bounces: 0, smoke_popped: false, resting: true,
        stuck: false, seek_target: None, phase: None,
    });
    let p = bond_at([blast[0] + 60.0, blast[1], blast[2]]);
    for _ in 0..320u32 { sim.tick(&d, &p); }
    for (k, pl) in trio.iter().enumerate() {
        let g = &sim.list[idx[k]];
        assert!(matches!(g.state, state::DEATH_ANIM | state::DEAD | state::FADE | state::FADE_FAST),
            "pl {pl}: the frag killed him, state {:#x} health {}", g.state, g.health);
        assert!(g.health <= 0.0, "pl {pl}: health {}", g.health);
    }
    // The deaths drop the held guns — one per dead guard (the yard's pl 303 also sits within the
    // frag's 8-unit radius and dies with them: 4 guards, 4 guns, the game's own geometry).
    let dead = sim.list.iter().filter(|g| matches!(g.state, state::DEATH_ANIM | state::DEAD | state::FADE | state::FADE_FAST)).count();
    assert!(dead >= 3, "the frag killed at least the trio, got {dead}");
    let guns = sim.pickups.iter().filter(|p| p.weapon == 0x16).count();
    assert_eq!(guns, dead, "one gun per dead guard: {:?}", sim.pickups.iter().map(|p| (p.weapon, p.rounds)).collect::<Vec<_>>());
    assert!(sim.pickups.iter().all(|p| p.weapon == 0x16), "no grenade pickups on 07000005");
}

/// Sister: the flashed (disarmed) yard, killed DOWN by a frag while every guard holds the FISTS
/// (1) — whose record has model 0 [L record 1], so `DropWeapon`'s `weapon_data[w]+0x80 != 0` gate
/// [M 0x17a2a0] skips the drop — adds NO pickups to the flash drops. (A guard who RECOVERS first
/// draws the sidearm [M 0x163350 case 1 ChangeWeapon(6)] and drops THAT when killed — the game's
/// own single object, observed (6,5) in this harness's earlier run.)
#[test]
fn a_flashed_guard_killed_after_drops_nothing_more() {
    use nf_game::drone::state;
    let Some(l) = L::load() else { eprintln!("skip: NF_ISO not set"); return };
    let d = l.data();
    let (mut sim, _rng) = l.sim();
    let gi = sim.list.iter().position(|dr| dr.placement == 3).unwrap();
    let g0 = sim.list[gi].pos;
    let blast = [g0[0] + 1.0, g0[1] + 0.4, g0[2]];
    let rec = &d.weapons.records[0x35];
    let f32_at = |o: usize| f32::from_le_bytes(rec[o..o + 4].try_into().unwrap());
    sim.grenades.push(nf_game::grenade::Grenade {
        pos: blast, velocity: [0.0; 3], speed: 0.0, travelled: 0.0, range: 210.0,
        damage: f32_at(0x0c), radius: f32_at(0x08), weapon_type: 0x35, mesh: 0,
        owner: usize::MAX, state: 0, bounces: 0, smoke_popped: false, resting: true,
        stuck: false, seek_target: None, phase: None,
    });
    let p = bond_at([blast[0] + 60.0, blast[1], blast[2]]);
    let mut flashed = false;
    for _f in 1..=260u32 {
        sim.tick(&d, &p);
        // The disarm: ChangeWeapon(1) — the fists id, not 0 [M 0x172bb0:17].
        if sim.list[gi].weapon == 1 { flashed = true; break; }
    }
    assert!(flashed, "the guard was flashed and disarmed");
    // The pickup collection runs at the tick head — run on so the flash drops land in sim.pickups.
    // The blast also flashes pl 4 and pl 303 (the yard, within 20 with a clear line): the whole
    // downed yard holds the FISTS now.
    for _ in 0..5 { sim.tick(&d, &p); }
    let drops_after_flash = sim.pickups.len();
    assert!(drops_after_flash >= 1, "the flash dropped the held gun");
    // Kill them DOWN — the frag (0x34, radius 8 / damage 100 [L record]) while every yard guard is
    // still stunned: they die holding the FISTS (1), whose record has model 0 [L record 1], so
    // `DropWeapon`'s `weapon_data[w]+0x80 != 0` gate [M 0x17a2a0] drops NOTHING — the playtest's
    // "multiple objects" never stacks pickups onto a death.
    let rec34 = &d.weapons.records[0x34];
    let f32_at34 = |o: usize| f32::from_le_bytes(rec34[o..o + 4].try_into().unwrap());
    sim.grenades.push(nf_game::grenade::Grenade {
        pos: blast, velocity: [0.0; 3], speed: 0.0, travelled: 0.0, range: 210.0,
        damage: f32_at34(0x0c), radius: f32_at34(0x08), weapon_type: 0x34, mesh: 0,
        owner: usize::MAX, state: 0, bounces: 0, smoke_popped: false, resting: true,
        stuck: false, seek_target: None, phase: None,
    });
    for _ in 0..320u32 { sim.tick(&d, &p); }
    assert!(matches!(sim.list[gi].state, state::DEATH_ANIM | state::DEAD | state::FADE | state::FADE_FAST),
        "the frag killed the stunned guard, state {:#x}", sim.list[gi].state);
    assert_eq!(sim.pickups.len(), drops_after_flash, "the deaths added NO pickup (the dying guards hold the fists): {:?}",
        sim.pickups.iter().map(|p| (p.weapon, p.rounds)).collect::<Vec<_>>());
}

/// scratch helpers (delete before merge)
pub fn trio_pl(i: usize, sim: &Drones) -> usize { sim.list[i].placement }
